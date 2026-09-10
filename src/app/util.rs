use crate::model::*;
/// App-specific utility functions
///
/// This module contains utilities that are specific to app state management
/// and operations, as opposed to general validation or business logic.
use chrono::Datelike;
use rust_decimal::Decimal;
use std::cmp::Ordering;

use std::time::{Duration, Instant};

pub struct TypeToSelect {
    buffer: String,
    last_type_time: Option<Instant>,
    timeout: Duration,
}

impl Default for TypeToSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeToSelect {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            last_type_time: None,
            timeout: Duration::from_secs(1),
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.last_type_time = None;
    }

    pub fn handle_char<T, F>(&mut self, c: char, items: &[T], extractor: F) -> Option<usize>
    where
        F: Fn(&T) -> &str,
    {
        let now = Instant::now();
        if let Some(last_time) = self.last_type_time
            && now.duration_since(last_time) > self.timeout
        {
            self.buffer.clear();
        }
        self.buffer.push(c);
        self.last_type_time = Some(now);

        let search_term = self.buffer.to_lowercase();
        items
            .iter()
            .position(|item| extractor(item).to_lowercase().starts_with(&search_term))
    }
}

/// Calculates the total income and expenses for the current filter view,
/// optionally filtered by a specific year.
///
/// # Arguments
///
/// * `app` - The application state containing transactions and filtered indices.
/// * `year_filter` - Optional year to filter transactions by. If None, includes all filtered transactions.
///
/// # Returns
///
/// A tuple `(total_income, total_expense)`
pub fn calculate_totals(
    app: &crate::app::state::App,
    year_filter: Option<i32>,
) -> (Decimal, Decimal) {
    app.filtered_indices
        .iter()
        .filter_map(|&idx| app.transactions.get(idx))
        .filter(|tx| {
            // Apply year filter if specified, otherwise include all transactions
            match year_filter {
                Some(year) => tx.date.year() == year,
                None => true,
            }
        })
        .fold((Decimal::ZERO, Decimal::ZERO), |(inc, exp), tx| {
            match tx.transaction_type {
                crate::model::TransactionType::Income => (inc + tx.amount, exp),
                crate::model::TransactionType::Expense => (inc, exp + tx.amount),
            }
        })
}

/// The `(category, subcategory)` a transaction is aggregated under in the category summary.
pub fn category_summary_keys(tx: &Transaction) -> (&str, &str) {
    let category = tx.category.trim();
    let category = if category.is_empty() {
        "Uncategorized"
    } else {
        category
    };
    (category, tx.subcategory.trim())
}

/// Sorts transactions by the selected column and order
pub fn sort_transactions_impl(
    transactions: &mut [Transaction],
    sort_by: SortColumn,
    sort_order: SortOrder,
) {
    transactions.sort_by(|a, b| {
        let ordering = match sort_by {
            SortColumn::Date => a.date.cmp(&b.date),
            SortColumn::Description => a.description.cmp(&b.description),
            SortColumn::Amount => a.amount.partial_cmp(&b.amount).unwrap_or(Ordering::Equal),
            SortColumn::Type => a.transaction_type.cmp(&b.transaction_type),
            SortColumn::Category => a.category.cmp(&b.category),
            SortColumn::Subcategory => a.subcategory.cmp(&b.subcategory),
        };
        if sort_order == SortOrder::Descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
}

/// Sorts catalog row indices. Blank tags and budgets always sink to the bottom so
/// the rows that have a value stay grouped.
pub fn sort_category_indices_impl(
    indices: &mut [usize],
    records: &[CategoryRecord],
    budget_of: impl Fn(&CategoryRecord) -> Option<Decimal>,
    sort_by: CategorySortColumn,
    sort_order: SortOrder,
) {
    indices.sort_by(|&a, &b| {
        let (a, b) = (&records[a], &records[b]);
        let ordering = match sort_by {
            CategorySortColumn::Type => a.transaction_type.cmp(&b.transaction_type),
            CategorySortColumn::Category => compare_ignore_case(&a.category, &b.category),
            CategorySortColumn::Subcategory => compare_ignore_case(&a.subcategory, &b.subcategory),
            CategorySortColumn::Tag => {
                return compare_blank_last(tag_of(a), tag_of(b), sort_order, |x, y| {
                    compare_ignore_case(x, y)
                })
                .then_with(|| category_tiebreak(a, b));
            }
            CategorySortColumn::TargetBudget => {
                return compare_blank_last(budget_of(a), budget_of(b), sort_order, Ord::cmp)
                    .then_with(|| category_tiebreak(a, b));
            }
        };
        let ordering = if sort_order == SortOrder::Descending {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| category_tiebreak(a, b))
    });
}

fn tag_of(record: &CategoryRecord) -> Option<&str> {
    record
        .tag
        .as_deref()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
}

fn compare_ignore_case(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

/// `None` sorts last in both directions.
fn compare_blank_last<T>(
    a: Option<T>,
    b: Option<T>,
    sort_order: SortOrder,
    compare: impl Fn(&T, &T) -> Ordering,
) -> Ordering {
    match (&a, &b) {
        (Some(a), Some(b)) => {
            let ordering = compare(a, b);
            if sort_order == SortOrder::Descending {
                ordering.reverse()
            } else {
                ordering
            }
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Keeps tied rows from shuffling between renders.
fn category_tiebreak(a: &CategoryRecord, b: &CategoryRecord) -> Ordering {
    compare_ignore_case(&a.category, &b.category)
        .then_with(|| compare_ignore_case(&a.subcategory, &b.subcategory))
        .then_with(|| a.id.cmp(&b.id))
}

/// Opens a URL in the default browser using system commands.
/// Returns true if successful, false otherwise.
pub fn open_url(url: &str) -> bool {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open").arg(url).spawn()
    } else {
        return false;
    };

    result.map(|_| true).unwrap_or(false)
}

// --- Recurring Transaction Utilities ---

/// Action types for jumping to original recurring transactions
pub enum JumpToOriginalAction {
    Edit,
    Delete,
    RecurringSettings,
}

impl JumpToOriginalAction {
    pub fn message(&self) -> &'static str {
        match self {
            JumpToOriginalAction::Edit => "Jumped to original recurring transaction for editing.",
            JumpToOriginalAction::Delete => {
                "Jumped to original recurring transaction for deletion."
            }
            JumpToOriginalAction::RecurringSettings => {
                "Jumped to original recurring transaction for settings."
            }
        }
    }
}
