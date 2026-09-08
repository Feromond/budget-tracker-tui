/// Input validation utilities
///
/// This module provides centralized validation for user input across the application.
/// All input validation logic should be placed here for consistency and reusability.
use crate::model::{CategoryInfo, TransactionType};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

/// Validates and inserts a date character with proper formatting
/// Returns the new field content with auto-inserted hyphens and validation
pub fn validate_and_insert_date_char(field: &str, c: char) -> Option<String> {
    // Only allow ASCII digits for date input
    if !c.is_ascii_digit() {
        return None;
    }

    let len = field.len();

    if len >= 10 {
        return None;
    }

    let mut result = field.to_string();

    // Validate month digits as they're entered
    if len == 5 {
        let month_digit = c.to_digit(10).unwrap_or(0);
        if month_digit > 1 {
            // For digits 2-9, auto-prepend 0 to make it 02, 03, etc.
            result.push('0');
            result.push(c);
            result.push('-');
            return Some(result);
        } else {
            // For 0 or 1, just add the digit (could be 01, 10, 11, 12)
            result.push(c);
            return Some(result);
        }
    } else if len == 6 {
        let first_digit = field
            .chars()
            .nth(5)
            .and_then(|ch| ch.to_digit(10))
            .unwrap_or(0);
        let month = first_digit * 10 + c.to_digit(10).unwrap_or(0);

        if month == 0 || month > 12 {
            return None;
        }
    }

    // Validate day digits
    if len == 8 {
        let day_digit = c.to_digit(10).unwrap_or(0);
        if day_digit > 3 {
            // First digit of day can only be 0, 1, 2, or 3
            return None;
        }
    } else if len == 9
        && let (Ok(year), Ok(month)) = (field[0..4].parse::<i32>(), field[5..7].parse::<u32>())
    {
        let first_digit = field
            .chars()
            .nth(8)
            .and_then(|ch| ch.to_digit(10))
            .unwrap_or(0);
        let day = first_digit * 10 + c.to_digit(10).unwrap_or(0);
        // Use days_in_month utility for validation
        let last_day = days_in_month(year, month);

        if day == 0 || day > last_day {
            return None;
        }
    }

    // Add the digit
    result.push(c);

    // Auto-insert hyphens
    if result.len() == 4 {
        // Validate year
        if let Ok(year) = result.parse::<i32>()
            && !(1900..=2100).contains(&year)
        {
            return None; // Reject invalid year
        }
        result.push('-');
    } else if result.len() == 7 {
        result.push('-');
    }

    Some(result)
}

/// Handles backspace for date fields with special hyphen logic
pub fn handle_date_backspace(field: &mut String) {
    let len = field.len();

    if field.ends_with('-') && (len == 5 || len == 8) {
        if field
            .chars()
            .nth(len - 2)
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            field.pop(); // Remove the hyphen
            field.pop(); // Remove the preceding digit
        } else {
            field.pop();
        }
    } else if !field.is_empty() {
        field.pop();
    }
}

// --- Amount Validation ---

/// Validates an amount character for input
pub fn validate_amount_char(field: &str, c: char) -> bool {
    c.is_ascii_digit() || (c == '.' && !field.contains('.'))
}

/// Validates and parses an amount string
pub fn validate_amount_string(amount_str: &str) -> Result<Decimal, String> {
    match amount_str.parse::<Decimal>() {
        Ok(amount) if amount > Decimal::ZERO => Ok(amount),
        Ok(_) => Err("Amount must be positive".to_string()),
        Err(_) => Err("Invalid amount format".to_string()),
    }
}

// --- Category Validation ---

/// Validates a category/subcategory combination for a transaction type
pub fn validate_category(
    categories: &[CategoryInfo],
    transaction_type: TransactionType,
    category: &str,
    subcategory: &str,
) -> Result<(), String> {
    // Allow "Uncategorized" or empty category
    if category.is_empty() || category.eq_ignore_ascii_case("Uncategorized") {
        return Ok(());
    }

    let category_lower = category.to_lowercase();
    let subcategory_lower = subcategory.to_lowercase();

    if subcategory.is_empty() {
        let category_exists = categories.iter().any(|cat_info| {
            cat_info.transaction_type == transaction_type
                && cat_info.category.to_lowercase() == category_lower
        });
        if category_exists {
            return Ok(());
        }
    } else {
        let pair_exists = categories.iter().any(|cat_info| {
            cat_info.transaction_type == transaction_type
                && cat_info.category.to_lowercase() == category_lower
                && cat_info.subcategory.to_lowercase() == subcategory_lower
        });
        if pair_exists {
            return Ok(());
        }
    }

    Err(format!(
        "Invalid Category/Subcategory: '{}' / '{}' for {:?}",
        category, subcategory, transaction_type
    ))
}

// --- Date Utilities ---

/// Check if a year is a leap year
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get the number of days in a month for a given year
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 31,
    }
}

/// Add months to a date with proper overflow handling
pub fn add_months(date: NaiveDate, months_to_add: i32) -> NaiveDate {
    let mut year = date.year();
    let mut month = date.month() as i32 + months_to_add;

    // Prevent infinite loops with extreme values
    if months_to_add.abs() > 120000 {
        // ~10000 years
        return date; // Return original date for extreme values
    }

    // Handle year overflow/underflow
    while month > 12 {
        year = year.saturating_add(1);
        month -= 12;
        if year > 3000 {
            // Reasonable upper bound
            return date;
        }
    }
    while month < 1 {
        year = year.saturating_sub(1);
        month += 12;
        if year < 1000 {
            // Reasonable lower bound
            return date;
        }
    }

    let day = date.day();
    let days_in_target_month = days_in_month(year, month as u32);
    let actual_day = day.min(days_in_target_month);

    NaiveDate::from_ymd_opt(year, month as u32, actual_day)
        .or_else(|| NaiveDate::from_ymd_opt(year, month as u32, 28))
        .or_else(|| NaiveDate::from_ymd_opt(year, month as u32, 1))
        .unwrap_or(date) // Final fallback: return original date
}

// --- Path Validation ---

/// Strips quotes from file paths to handle copy-paste scenarios
/// Removes leading and trailing single or double quotes from paths
pub fn strip_path_quotes(path: &str) -> String {
    let mut result = path.to_string();

    // Remove leading quotes
    if result.starts_with('"') || result.starts_with('\'') {
        result = result[1..].to_string();
    }

    // Remove trailing quotes
    if result.ends_with('"') || result.ends_with('\'') {
        result = result[..result.len() - 1].to_string();
    }

    result
}

/// Like [`validate_amount_string`] but admits zero, for values that can legitimately be nothing
/// (an investment account you have sold out of is worth 0, not "invalid").
pub fn validate_non_negative_amount_string(amount_str: &str) -> Result<Decimal, String> {
    match amount_str.parse::<Decimal>() {
        Ok(amount) if amount >= Decimal::ZERO => Ok(amount),
        Ok(_) => Err("Amount cannot be negative".to_string()),
        Err(_) => Err("Invalid amount format".to_string()),
    }
}
