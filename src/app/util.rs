use crate::model::*;
/// App-specific utility functions
///
/// This module contains utilities that are specific to app state management
/// and operations, as opposed to general validation or business logic.
use chrono::Datelike;
use ratatui::widgets::TableState;
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

/// Totals for the current selection, optionally limited to a year.
pub fn calculate_totals(
    app: &crate::app::state::App,
    year_filter: Option<i32>,
) -> crate::model::MonthlySummary {
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
        .fold(crate::model::MonthlySummary::default(), |mut totals, tx| {
            totals.add(tx);
            totals
        })
}

pub fn cycle<T: Copy + PartialEq>(options: &[T], current: T, forward: bool) -> T {
    let index = options
        .iter()
        .position(|option| *option == current)
        .unwrap_or(0);
    let step = if forward { 1 } else { options.len() - 1 };
    options[(index + step) % options.len()]
}

pub fn step_selection(state: &mut TableState, len: usize, forward: bool, wrap: bool) {
    if len == 0 {
        state.select(None);
        return;
    }
    let last = len - 1;
    let current = state.selected().map(|index| index.min(last));
    let next = match (current, forward) {
        (None, false) if wrap => last,
        (None, _) => 0,
        (Some(index), true) if index < last => index + 1,
        (Some(_), true) if wrap => 0,
        (Some(index), false) if index > 0 => index - 1,
        (Some(_), false) if wrap => last,
        (Some(index), _) => index,
    };
    state.select(Some(next));
}

pub fn toggle_between(field: &mut String, first: &str, second: &str) {
    *field = if field == second { first } else { second }.to_string();
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
