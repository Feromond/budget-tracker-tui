use super::state::App;
use crate::app::fields::AdvancedFilterField;
use crate::app::state::{AppMode, CategorySummaryItem};
use crate::app::util::category_summary_keys;
use crate::model::{CategorySummarySortColumn, DATE_FORMAT, MonthlySummary, SortOrder};
use crate::ui::helpers::month_to_short_str;
use chrono;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use std::cmp::Ordering;

fn same_category_summary_row(a: &CategorySummaryItem, b: &CategorySummaryItem) -> bool {
    match (a, b) {
        (CategorySummaryItem::Month(a, _), CategorySummaryItem::Month(b, _)) => a == b,
        (
            CategorySummaryItem::Subcategory(a_month, a_cat, a_sub, _),
            CategorySummaryItem::Subcategory(b_month, b_cat, b_sub, _),
        ) => a_month == b_month && a_cat == b_cat && a_sub == b_sub,
        (
            CategorySummaryItem::Transaction(a_month, a_index),
            CategorySummaryItem::Transaction(b_month, b_index),
        ) => a_month == b_month && a_index == b_index,
        _ => false,
    }
}

impl App {
    // --- Private Helpers for Summary Navigation ---
    pub fn sorted_months_for_year(&self, year: i32) -> Vec<u32> {
        let mut months: Vec<u32> = self
            .monthly_summaries
            .keys()
            .filter_map(|(y, m)| if *y == year { Some(*m) } else { None })
            .collect();
        months.sort_unstable();
        months
    }

    /// Stay on the same month across a year change, unless that year has no data for it.
    fn keep_or_pick_summary_month(&mut self, year: i32) {
        let months = self.sorted_months_for_year(year);
        if !matches!(self.selected_summary_month, Some(month) if months.contains(&month)) {
            self.update_selected_summary_month(year);
        }
    }

    fn update_selected_summary_month(&mut self, year: i32) {
        let current_date = chrono::Local::now();
        let current_year = current_date.year();
        let current_month = current_date.month();
        let months = self.sorted_months_for_year(year);
        if year == current_year && months.contains(&current_month) {
            self.selected_summary_month = Some(current_month);
        } else {
            self.selected_summary_month = months.last().copied();
        }
    }

    // --- Private Helpers for Category Summary Navigation ---
    pub fn sorted_category_months_for_year(&self, year: i32) -> Vec<u32> {
        let mut months: Vec<u32> = self
            .category_summaries
            .keys()
            .filter_map(|(y, m)| if *y == year { Some(*m) } else { None })
            .collect();
        months.sort_unstable();
        months.dedup();
        months
    }

    // --- Summary Logic ---
    // Handles entering/exiting summary mode, navigating years, and updating monthly summaries.
    pub(crate) fn enter_summary_mode(&mut self) {
        self.mode = AppMode::Summary;
        self.calculate_monthly_summaries();
        if !self.summary_years.is_empty() {
            let current_year = chrono::Local::now().year();
            if let Some(idx) = self.summary_years.iter().position(|&y| y == current_year) {
                self.selected_summary_year_index = idx;
            } else {
                self.selected_summary_year_index = self.summary_years.len() - 1;
            }
        }
        // Set selected_summary_month to current month if present, else latest month with data
        if let Some(year) = self
            .summary_years
            .get(self.selected_summary_year_index)
            .copied()
        {
            self.update_selected_summary_month(year);
        } else {
            self.selected_summary_month = None;
        }
        self.table_state.select(Some(0));
        self.clear_status_message();
    }
    pub(crate) fn exit_summary_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.clear_status_message();
    }
    pub(crate) fn next_summary_year(&mut self) {
        if !self.summary_years.is_empty() {
            self.selected_summary_year_index =
                (self.selected_summary_year_index + 1) % self.summary_years.len();
            // Reset month selection for new year
            if let Some(year) = self
                .summary_years
                .get(self.selected_summary_year_index)
                .copied()
            {
                self.keep_or_pick_summary_month(year);
            } else {
                self.selected_summary_month = None;
            }
            self.table_state.select(Some(0));
        }
    }
    pub(crate) fn previous_summary_year(&mut self) {
        if !self.summary_years.is_empty() {
            if self.selected_summary_year_index > 0 {
                self.selected_summary_year_index -= 1;
            } else {
                self.selected_summary_year_index = self.summary_years.len() - 1;
            }
            // Reset month selection for new year
            if let Some(year) = self
                .summary_years
                .get(self.selected_summary_year_index)
                .copied()
            {
                self.keep_or_pick_summary_month(year);
            } else {
                self.selected_summary_month = None;
            }
            self.table_state.select(Some(0));
        }
    }
    pub(crate) fn next_summary_month(&mut self) {
        if let Some(year) = self
            .summary_years
            .get(self.selected_summary_year_index)
            .copied()
        {
            let months = self.sorted_months_for_year(year);
            if let Some(current) = self.selected_summary_month {
                if let Some(idx) = months.iter().position(|&m| m == current) {
                    let next_idx = (idx + 1) % months.len();
                    self.selected_summary_month = Some(months[next_idx]);
                }
            } else if let Some(&first) = months.first() {
                self.selected_summary_month = Some(first);
            }
        }
    }
    pub(crate) fn previous_summary_month(&mut self) {
        if let Some(year) = self
            .summary_years
            .get(self.selected_summary_year_index)
            .copied()
        {
            let months = self.sorted_months_for_year(year);
            if let Some(current) = self.selected_summary_month {
                if let Some(idx) = months.iter().position(|&m| m == current) {
                    let prev_idx = if idx == 0 { months.len() - 1 } else { idx - 1 };
                    self.selected_summary_month = Some(months[prev_idx]);
                }
            } else if let Some(&first) = months.first() {
                self.selected_summary_month = Some(first);
            }
        }
    }
    // --- Category Summary Logic ---
    // Handles entering/exiting category summary mode, navigating years/items, and updating category summaries.
    pub(crate) fn enter_category_summary_mode(&mut self) {
        self.mode = AppMode::CategorySummary;
        self.calculate_category_summaries();
        if !self.category_summary_years.is_empty() {
            let current_year = chrono::Local::now().year();
            if let Some(idx) = self
                .category_summary_years
                .iter()
                .position(|&y| y == current_year)
            {
                self.category_summary_year_index = idx;
            } else {
                self.category_summary_year_index = self.category_summary_years.len() - 1;
            }
        }
        self.rebuild_category_summary_items(None);
        self.clear_status_message();
    }
    pub(crate) fn exit_category_summary_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.clear_status_message();
    }
    pub(crate) fn next_category_summary_item(&mut self) {
        let list_len = self.cached_visible_category_items.len();
        if list_len == 0 {
            return;
        }
        let i = match self.category_summary_table_state.selected() {
            Some(i) => {
                if i >= list_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.category_summary_table_state.select(Some(i));
    }
    pub(crate) fn previous_category_summary_item(&mut self) {
        let list_len = self.cached_visible_category_items.len();
        if list_len == 0 {
            return;
        }
        let i = match self.category_summary_table_state.selected() {
            Some(i) => {
                if i == 0 {
                    list_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.category_summary_table_state.select(Some(i));
    }
    fn selected_category_summary_month(&self) -> Option<u32> {
        let index = self.category_summary_table_state.selected()?;
        match self.cached_visible_category_items.get(index)? {
            CategorySummaryItem::Month(month, _) => Some(*month),
            CategorySummaryItem::Subcategory(month, _, _, _) => Some(*month),
            CategorySummaryItem::Transaction(month, _) => Some(*month),
        }
    }

    fn rebuild_category_summary_items(&mut self, month: Option<u32>) {
        self.cached_visible_category_items = self.get_visible_category_summary_items();
        if self.cached_visible_category_items.is_empty() {
            self.category_summary_table_state.select(None);
            return;
        }
        let position = self.category_summary_month_row(month);
        self.category_summary_table_state
            .select(Some(position.unwrap_or(0)));
    }

    fn category_summary_month_row(&self, month: Option<u32>) -> Option<usize> {
        let month = self.target_category_summary_month(month)?;
        self.cached_visible_category_items
            .iter()
            .position(|item| matches!(item, CategorySummaryItem::Month(m, _) if *m == month))
    }

    /// Same fallback the monthly summary uses: keep the month, else this year's current
    /// month, else its latest.
    fn target_category_summary_month(&self, month: Option<u32>) -> Option<u32> {
        let year = self
            .category_summary_years
            .get(self.category_summary_year_index)
            .copied()?;
        let months = self.sorted_category_months_for_year(year);
        if let Some(month) = month
            && months.contains(&month)
        {
            return Some(month);
        }
        let now = chrono::Local::now();
        if year == now.year() && months.contains(&now.month()) {
            return Some(now.month());
        }
        months.last().copied()
    }

    pub(crate) fn next_category_summary_year(&mut self) {
        if !self.category_summary_years.is_empty() {
            let month = self.selected_category_summary_month();
            self.category_summary_year_index =
                (self.category_summary_year_index + 1) % self.category_summary_years.len();
            self.rebuild_category_summary_items(month);
        }
    }
    pub(crate) fn previous_category_summary_year(&mut self) {
        if !self.category_summary_years.is_empty() {
            let month = self.selected_category_summary_month();
            if self.category_summary_year_index > 0 {
                self.category_summary_year_index -= 1;
            } else {
                self.category_summary_year_index = self.category_summary_years.len() - 1;
            }
            self.rebuild_category_summary_items(month);
        }
    }
    pub(crate) fn get_visible_category_summary_items(&self) -> Vec<CategorySummaryItem> {
        let mut items = Vec::new();
        let Some(year) = self
            .category_summary_years
            .get(self.category_summary_year_index)
            .copied()
        else {
            return items;
        };
        for (month, month_total) in self.sorted_category_month_totals(year) {
            items.push(CategorySummaryItem::Month(month, month_total));
            if !self.expanded_category_summary_months.contains(&month) {
                continue;
            }
            for (category, subcategory, summary) in self.sorted_category_rows_for(year, month) {
                items.push(CategorySummaryItem::Subcategory(
                    month,
                    category.clone(),
                    subcategory.clone(),
                    summary,
                ));
                let key = (month, category.clone(), subcategory.clone());
                if self.expanded_category_summary_subcategories.contains(&key) {
                    items.extend(self.transaction_items_for(year, month, &category, &subcategory));
                }
            }
        }
        items
    }

    pub(crate) fn set_category_summary_sort_column(&mut self, column: CategorySummarySortColumn) {
        if self.category_summary_sort_by == column {
            self.category_summary_sort_order = match self.category_summary_sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.category_summary_sort_by = column;
            self.category_summary_sort_order = SortOrder::Ascending;
        }
        self.resort_category_summary_items();
    }

    fn resort_category_summary_items(&mut self) {
        let selected = self
            .category_summary_table_state
            .selected()
            .and_then(|index| self.cached_visible_category_items.get(index).cloned());
        let month = self.selected_category_summary_month();
        self.cached_visible_category_items = self.get_visible_category_summary_items();
        if self.cached_visible_category_items.is_empty() {
            self.category_summary_table_state.select(None);
            return;
        }
        let position = selected
            .and_then(|row| {
                self.cached_visible_category_items
                    .iter()
                    .position(|candidate| same_category_summary_row(candidate, &row))
            })
            .or_else(|| self.category_summary_month_row(month));
        self.category_summary_table_state
            .select(Some(position.unwrap_or(0)));
    }

    fn applied_category_summary_order(&self, ordering: Ordering) -> Ordering {
        match self.category_summary_sort_order {
            SortOrder::Ascending => ordering,
            SortOrder::Descending => ordering.reverse(),
        }
    }

    fn category_summary_amount_key(&self, summary: &MonthlySummary) -> Decimal {
        match self.category_summary_sort_by {
            CategorySummarySortColumn::Income => summary.income,
            CategorySummarySortColumn::Expense => summary.expense,
            _ => summary.income - summary.expense,
        }
    }

    fn sorted_category_month_totals(&self, year: i32) -> Vec<(u32, MonthlySummary)> {
        let mut months: Vec<(u32, MonthlySummary)> = self
            .sorted_category_months_for_year(year)
            .into_iter()
            .map(|month| {
                let mut total = MonthlySummary::default();
                if let Some(month_map) = self.category_summaries.get(&(year, month)) {
                    for summary in month_map.values() {
                        total.income += summary.income;
                        total.expense += summary.expense;
                    }
                }
                (month, total)
            })
            .collect();
        match self.category_summary_sort_by {
            CategorySummarySortColumn::Month => {
                if self.category_summary_sort_order == SortOrder::Descending {
                    months.reverse();
                }
            }
            CategorySummarySortColumn::Category | CategorySummarySortColumn::Subcategory => {}
            _ => months.sort_by(|(a_month, a_total), (b_month, b_total)| {
                self.applied_category_summary_order(
                    self.category_summary_amount_key(a_total)
                        .cmp(&self.category_summary_amount_key(b_total)),
                )
                .then(a_month.cmp(b_month))
            }),
        }
        months
    }

    fn sorted_category_rows_for(
        &self,
        year: i32,
        month: u32,
    ) -> Vec<(String, String, MonthlySummary)> {
        let Some(month_map) = self.category_summaries.get(&(year, month)) else {
            return Vec::new();
        };
        let mut rows: Vec<(String, String, MonthlySummary)> = month_map
            .iter()
            .map(|((category, subcategory), summary)| {
                (category.clone(), subcategory.clone(), *summary)
            })
            .collect();
        rows.sort_by(|(a_cat, a_sub, a_summary), (b_cat, b_sub, b_summary)| {
            match self.category_summary_sort_by {
                CategorySummarySortColumn::Month => a_cat.cmp(b_cat).then(a_sub.cmp(b_sub)),
                CategorySummarySortColumn::Category => self
                    .applied_category_summary_order(a_cat.cmp(b_cat))
                    .then(a_sub.cmp(b_sub)),
                CategorySummarySortColumn::Subcategory => self
                    .applied_category_summary_order(a_sub.cmp(b_sub))
                    .then(a_cat.cmp(b_cat)),
                _ => self
                    .applied_category_summary_order(
                        self.category_summary_amount_key(a_summary)
                            .cmp(&self.category_summary_amount_key(b_summary)),
                    )
                    .then(a_cat.cmp(b_cat))
                    .then(a_sub.cmp(b_sub)),
            }
        });
        rows
    }

    fn transaction_items_for(
        &self,
        year: i32,
        month: u32,
        category: &str,
        subcategory: &str,
    ) -> Vec<CategorySummaryItem> {
        let mut matches: Vec<usize> = self
            .filtered_indices
            .iter()
            .copied()
            .filter(|&index| {
                let tx = &self.transactions[index];
                if tx.date.year() != year || tx.date.month() != month {
                    return false;
                }
                let (tx_category, tx_subcategory) = category_summary_keys(tx);
                tx_category == category && tx_subcategory == subcategory
            })
            .collect();
        match self.category_summary_sort_by {
            CategorySummarySortColumn::Income
            | CategorySummarySortColumn::Expense
            | CategorySummarySortColumn::Net => matches.sort_by(|&a, &b| {
                self.applied_category_summary_order(
                    self.transaction_sort_key(a)
                        .cmp(&self.transaction_sort_key(b)),
                )
                .then_with(|| self.transactions[a].date.cmp(&self.transactions[b].date))
            }),
            _ => matches.sort_by_key(|&index| self.transactions[index].date),
        }
        matches
            .into_iter()
            .map(|index| CategorySummaryItem::Transaction(month, index))
            .collect()
    }

    fn transaction_sort_key(&self, index: usize) -> Decimal {
        let tx = &self.transactions[index];
        let income = tx.transaction_type == crate::model::TransactionType::Income;
        match (self.category_summary_sort_by, income) {
            (CategorySummarySortColumn::Income, true) => tx.amount,
            (CategorySummarySortColumn::Expense, false) => tx.amount,
            (CategorySummarySortColumn::Net, true) => tx.amount,
            (CategorySummarySortColumn::Net, false) => -tx.amount,
            _ => Decimal::ZERO,
        }
    }

    pub(crate) fn toggle_category_summary_row(&mut self) {
        let Some(selected) = self.category_summary_table_state.selected() else {
            return;
        };
        match self.cached_visible_category_items.get(selected) {
            Some(CategorySummaryItem::Month(month, _)) => {
                let month = *month;
                if !self.expanded_category_summary_months.remove(&month) {
                    self.expanded_category_summary_months.insert(month);
                }
            }
            Some(CategorySummaryItem::Subcategory(month, category, subcategory, _)) => {
                let key = (*month, category.clone(), subcategory.clone());
                if !self.expanded_category_summary_subcategories.remove(&key) {
                    self.expanded_category_summary_subcategories.insert(key);
                }
            }
            Some(CategorySummaryItem::Transaction(_, index)) => {
                self.open_transaction_from_category_summary(*index);
                return;
            }
            None => return,
        }
        self.cached_visible_category_items = self.get_visible_category_summary_items();
        let len = self.cached_visible_category_items.len();
        if len == 0 {
            self.category_summary_table_state.select(None);
        } else {
            self.category_summary_table_state
                .select(Some(selected.min(len - 1)));
        }
    }

    fn open_transaction_from_category_summary(&mut self, index: usize) {
        if let Some(position) = self.filtered_indices.iter().position(|&i| i == index) {
            self.table_state.select(Some(position));
        }
        self.mode = AppMode::Normal;
        self.clear_status_message();
    }

    pub(crate) fn filter_transactions_from_category_summary(&mut self) {
        let Some(year) = self
            .category_summary_years
            .get(self.category_summary_year_index)
            .copied()
        else {
            return;
        };
        let Some(selected) = self.category_summary_table_state.selected() else {
            return;
        };
        let (month, category, subcategory) = match self.cached_visible_category_items.get(selected)
        {
            Some(CategorySummaryItem::Month(month, _)) => (*month, String::new(), String::new()),
            Some(CategorySummaryItem::Subcategory(month, category, subcategory, _)) => {
                (*month, category.clone(), subcategory.clone())
            }
            Some(CategorySummaryItem::Transaction(month, index)) => {
                let Some(tx) = self.transactions.get(*index) else {
                    return;
                };
                let (category, subcategory) = category_summary_keys(tx);
                (*month, category.to_string(), subcategory.to_string())
            }
            None => return,
        };

        let last_day = crate::validation::days_in_month(year, month);
        let (Some(from), Some(to)) = (
            NaiveDate::from_ymd_opt(year, month, 1),
            NaiveDate::from_ymd_opt(year, month, last_day),
        ) else {
            return;
        };

        // Leave category unrestricted: the filter cannot match missing categories exactly.
        let category = if category == "Uncategorized" {
            String::new()
        } else {
            category
        };

        self.clear_all_filter_fields();
        self.advanced_filter_fields[AdvancedFilterField::DateFrom] =
            from.format(DATE_FORMAT).to_string();
        self.advanced_filter_fields[AdvancedFilterField::DateTo] =
            to.format(DATE_FORMAT).to_string();
        self.advanced_filter_fields[AdvancedFilterField::Category] = category.clone();
        self.advanced_filter_fields[AdvancedFilterField::Subcategory] = subcategory.clone();
        self.apply_advanced_filter();
        self.mode = AppMode::Normal;

        let scope = match (category.is_empty(), subcategory.is_empty()) {
            (true, _) => month_to_short_str(month).to_string(),
            (false, true) => format!("{} {}", month_to_short_str(month), category),
            (false, false) => {
                format!(
                    "{} {} / {}",
                    month_to_short_str(month),
                    category,
                    subcategory
                )
            }
        };
        self.set_status_message(
            format!("Filtered to {} {}", scope, year),
            Some(chrono::Duration::seconds(4)),
        );
    }

    /// Jump to the next month in CategorySummary mode
    /// Finds the next Month item in the visible items list
    pub(crate) fn next_category_summary_month(&mut self) {
        let current_selection = self.category_summary_table_state.selected().unwrap_or(0);
        let items = &self.cached_visible_category_items;

        // Find the next month item after the current selection
        for (index, item) in items.iter().enumerate().skip(current_selection + 1) {
            if matches!(item, CategorySummaryItem::Month(_, _)) {
                self.category_summary_table_state.select(Some(index));
                return;
            }
        }

        // If no month found after current selection, wrap to first month
        for (index, item) in items.iter().enumerate() {
            if matches!(item, CategorySummaryItem::Month(_, _)) {
                self.category_summary_table_state.select(Some(index));
                return;
            }
        }
    }

    /// Jump to the previous month in CategorySummary mode
    /// Finds the previous Month item in the visible items list
    pub(crate) fn previous_category_summary_month(&mut self) {
        let current_selection = self.category_summary_table_state.selected().unwrap_or(0);
        let items = &self.cached_visible_category_items;

        // Find the previous month item before the current selection
        for (index, item) in items.iter().enumerate().take(current_selection).rev() {
            if matches!(item, CategorySummaryItem::Month(_, _)) {
                self.category_summary_table_state.select(Some(index));
                return;
            }
        }

        // If no month found before current selection, wrap to last month
        for (index, item) in items.iter().enumerate().rev() {
            if matches!(item, CategorySummaryItem::Month(_, _)) {
                self.category_summary_table_state.select(Some(index));
                return;
            }
        }
    }
}
