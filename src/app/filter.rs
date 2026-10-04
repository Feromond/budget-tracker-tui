use super::state::App;
use crate::app::fields::{AdvancedFilterField, FieldKey, FieldKind, SelectingField};
use crate::model::{DATE_FORMAT, TransactionType};
use chrono::{Duration, NaiveDate};
use ratatui::widgets::ListState;
use rust_decimal::Decimal;
use std::collections::HashSet;

const MANAGE_ACCOUNTS: &str = "Manage accounts…";

impl App {
    pub(crate) fn in_account_scope(&self, tx: &crate::model::Transaction) -> bool {
        self.account_scope
            .is_none_or(|id| tx.account_id == id || tx.to_account_id == Some(id))
    }

    pub(crate) fn open_account_scope_picker(&mut self) {
        let mut options = vec!["All accounts".to_string()];
        options.extend(self.accounts.all().iter().map(|account| {
            if account.archived {
                format!("{} (archived)", account.name)
            } else {
                account.name.clone()
            }
        }));
        options.push(MANAGE_ACCOUNTS.to_string());
        let selected = self
            .account_scope
            .and_then(|id| self.accounts.all().iter().position(|a| a.id == id))
            .map_or(0, |index| index + 1);
        self.type_to_select.clear();
        self.current_selection_list = options;
        self.selection_list_state = ListState::default();
        self.selection_list_state.select(Some(selected));
        self.mode = crate::app::state::AppMode::SelectingAccountScope;
    }

    pub(crate) fn choose_account_scope(&mut self) {
        let Some(index) = self.selection_list_state.selected() else {
            self.cancel_account_scope_picker();
            return;
        };
        if index + 1 == self.current_selection_list.len() {
            self.current_selection_list.clear();
            self.open_account_manager(crate::app::state::AppMode::Normal);
            return;
        }
        self.account_scope = index
            .checked_sub(1)
            .and_then(|index| self.accounts.all().get(index))
            .map(|account| account.id);
        self.cancel_account_scope_picker();
        self.refresh_filter();
        self.reset_table_selection();
    }

    pub(crate) fn cancel_account_scope_picker(&mut self) {
        self.current_selection_list.clear();
        self.mode = crate::app::state::AppMode::Normal;
    }

    pub(crate) fn account_scope_label(&self) -> String {
        match self.account_scope {
            Some(id) => self.accounts.name(id).to_string(),
            None => "all accounts".to_string(),
        }
    }

    pub(crate) fn refresh_filter(&mut self) {
        if self.advanced_filter_fields.all_empty() {
            self.apply_filter();
        } else {
            self.apply_advanced_filter();
        }
    }

    pub(crate) fn is_filter_active(&self) -> bool {
        !self.simple_filter_content.is_empty() || !self.advanced_filter_fields.all_empty()
    }
    // --- Filtering Logic ---
    // Handles entering/exiting filtering mode, applying basic filter, and updating filtered indices.
    pub(crate) fn start_filtering(&mut self) {
        self.mode = crate::app::state::AppMode::Filtering;
        self.simple_filter_cursor = self.simple_filter_content.len();
        self.clear_status_message();
    }
    pub(crate) fn exit_filtering(&mut self) {
        self.mode = crate::app::state::AppMode::Normal;
        self.clear_status_message();
    }
    pub(crate) fn apply_filter(&mut self) {
        crate::app::util::sort_transactions_impl(
            &mut self.transactions,
            self.sort_by,
            self.sort_order,
        );
        let query = self.simple_filter_content.to_lowercase();
        self.filtered_indices = self
            .transactions
            .iter()
            .enumerate()
            .filter(|(_, tx)| {
                self.in_account_scope(tx)
                    && (query.is_empty() || tx.description.to_lowercase().contains(&query))
            })
            .map(|(index, _)| index)
            .collect();
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            let current_selection = self.table_state.selected().unwrap_or(0);
            self.table_state
                .select(Some(current_selection.min(self.filtered_indices.len() - 1)));
        }
        self.calculate_category_summaries();
        self.calculate_monthly_summaries();
    }
    // --- Advanced Filtering Logic ---
    // Handles advanced filter UI, field navigation, and applying advanced filters to transactions.
    pub(crate) fn start_advanced_filtering(&mut self) {
        self.mode = crate::app::state::AppMode::AdvancedFiltering;
        self.advanced_filter_fields
            .focus(AdvancedFilterField::DateFrom);
        self.advanced_filter_cursor = self.advanced_filter_fields.focused_value().len();
        self.clear_status_message()
    }
    pub(crate) fn cancel_advanced_filtering(&mut self) {
        self.mode = crate::app::state::AppMode::Normal;
        self.clear_status_message()
    }
    pub(crate) fn finish_advanced_filtering(&mut self) {
        self.clear_simple_filter_field_only();
        self.apply_advanced_filter();
        self.mode = crate::app::state::AppMode::Normal;
        self.clear_status_message()
    }

    /// Clear both filter forms without touching the current mode or the visible rows.
    pub(crate) fn clear_all_filter_fields(&mut self) {
        self.simple_filter_content.clear();
        self.simple_filter_cursor = 0;
        self.clear_advanced_filter_fields_only();
    }

    pub(crate) fn reset_all_filters(&mut self) {
        let was_active = self.is_filter_active();
        self.clear_all_filter_fields();

        // Apply basic filter (shows all transactions) and return to normal mode
        self.apply_filter();
        self.mode = crate::app::state::AppMode::Normal;
        if was_active {
            self.set_status_message("All filters cleared", Some(Duration::seconds(3)));
        } else {
            self.clear_status_message();
        }
    }
    pub(crate) fn clear_advanced_filter_fields_only(&mut self) {
        // Clear advanced filter fields without changing mode
        self.advanced_filter_fields.reset();
    }
    pub(crate) fn clear_simple_filter_field_only(&mut self) {
        // Clear simple filter field without changing mode
        self.simple_filter_content.clear();
        self.simple_filter_cursor = 0;
    }
    pub(crate) fn next_advanced_filter_field(&mut self) {
        self.advanced_filter_fields.focus_next();
        self.advanced_filter_cursor = self.advanced_filter_fields.focused_value().len();
    }
    pub(crate) fn previous_advanced_filter_field(&mut self) {
        self.advanced_filter_fields.focus_previous();
        self.advanced_filter_cursor = self.advanced_filter_fields.focused_value().len();
    }
    pub(crate) fn toggle_advanced_transaction_type(&mut self) {
        self.clear_simple_filter_field_only();
        let ft = self.advanced_filter_fields[AdvancedFilterField::TransactionType].trim();
        let new_val = if ft.is_empty() {
            "Income"
        } else if ft.eq_ignore_ascii_case("Income") {
            "Expense"
        } else if ft.eq_ignore_ascii_case("Expense") {
            "Transfer"
        } else {
            ""
        };
        self.advanced_filter_fields[AdvancedFilterField::TransactionType] = new_val.to_string();
    }
    pub(crate) fn toggle_advanced_recurring(&mut self) {
        self.clear_simple_filter_field_only();
        let fr = self.advanced_filter_fields[AdvancedFilterField::Recurring].trim();
        let new_val = if fr.is_empty() {
            "Recurring"
        } else if fr.eq_ignore_ascii_case("Recurring") {
            "One-Time"
        } else {
            ""
        };
        self.advanced_filter_fields[AdvancedFilterField::Recurring] = new_val.to_string();
    }
    pub(crate) fn start_advanced_category_selection(&mut self) {
        self.type_to_select.clear();
        self.selecting_field = Some(SelectingField::AdvancedFilter(
            AdvancedFilterField::Category,
        ));
        self.mode = crate::app::state::AppMode::SelectingFilterCategory;
        let mut unique: HashSet<String> =
            self.categories.iter().map(|c| c.category.clone()).collect();
        let mut opts: Vec<String> = unique.drain().collect();
        opts.sort_unstable();
        self.current_selection_list = opts;
        self.selection_list_state = ListState::default();
        if !self.current_selection_list.is_empty() {
            self.selection_list_state.select(Some(0));
        }
    }
    pub(crate) fn start_advanced_subcategory_selection(&mut self) {
        self.type_to_select.clear();
        self.selecting_field = Some(SelectingField::AdvancedFilter(
            AdvancedFilterField::Subcategory,
        ));
        self.mode = crate::app::state::AppMode::SelectingFilterSubcategory;
        let current_cat = self.advanced_filter_fields[AdvancedFilterField::Category].trim();
        let mut unique: HashSet<String> = self
            .categories
            .iter()
            .filter(|c| current_cat.is_empty() || c.category.eq_ignore_ascii_case(current_cat))
            .filter(|c| !c.subcategory.is_empty())
            .map(|c| c.subcategory.clone())
            .collect();
        let mut opts: Vec<String> = unique.drain().collect();
        opts.sort_unstable();
        opts.insert(0, "(None)".to_string());
        self.current_selection_list = opts;
        self.selection_list_state = ListState::default();
        if !self.current_selection_list.is_empty() {
            self.selection_list_state.select(Some(0));
        }
    }
    pub(crate) fn confirm_advanced_selection(&mut self) {
        if let Some(idx) = self.selection_list_state.selected()
            && let Some(field) = self
                .selecting_field
                .and_then(SelectingField::advanced_filter)
            && let Some(val) = self.current_selection_list.get(idx)
        {
            let val_clone = val.clone();
            self.clear_simple_filter_field_only();
            let v = if field == AdvancedFilterField::Subcategory && val_clone == "(None)" {
                ""
            } else {
                val_clone.as_str()
            };
            self.advanced_filter_fields[field] = v.to_string();
            if field == AdvancedFilterField::Category {
                self.start_advanced_subcategory_selection();
                return;
            }
        }
        self.mode = crate::app::state::AppMode::AdvancedFiltering;
        if let Some(field) = self
            .selecting_field
            .and_then(SelectingField::advanced_filter)
        {
            self.advanced_filter_fields.focus(field);
            self.advanced_filter_cursor = self.advanced_filter_fields.focused_value().len();
        }
        self.selecting_field = None;
        self.current_selection_list.clear();
    }
    pub(crate) fn cancel_advanced_selection(&mut self) {
        self.mode = crate::app::state::AppMode::AdvancedFiltering;
        if let Some(field) = self
            .selecting_field
            .and_then(SelectingField::advanced_filter)
        {
            self.advanced_filter_fields.focus(field);
        }
        self.selecting_field = None;
        self.current_selection_list.clear();
    }
    pub(crate) fn apply_advanced_filter(&mut self) {
        crate::app::util::sort_transactions_impl(
            &mut self.transactions,
            self.sort_by,
            self.sort_order,
        );
        let date_from = NaiveDate::parse_from_str(
            &self.advanced_filter_fields[AdvancedFilterField::DateFrom],
            DATE_FORMAT,
        )
        .ok();
        let date_to = NaiveDate::parse_from_str(
            &self.advanced_filter_fields[AdvancedFilterField::DateTo],
            DATE_FORMAT,
        )
        .ok();
        let desc_q = self.advanced_filter_fields[AdvancedFilterField::Description].to_lowercase();
        let cat_q = self.advanced_filter_fields[AdvancedFilterField::Category].to_lowercase();
        let sub_q = self.advanced_filter_fields[AdvancedFilterField::Subcategory].to_lowercase();
        let type_q = self.advanced_filter_fields[AdvancedFilterField::TransactionType].trim();
        let recurring_q = self.advanced_filter_fields[AdvancedFilterField::Recurring].trim();
        let amt_from = self.advanced_filter_fields[AdvancedFilterField::AmountFrom]
            .parse::<Decimal>()
            .ok();
        let amt_to = self.advanced_filter_fields[AdvancedFilterField::AmountTo]
            .parse::<Decimal>()
            .ok();
        self.filtered_indices = self
            .transactions
            .iter()
            .enumerate()
            .filter(|(_, tx)| {
                if let Some(d) = date_from
                    && tx.date < d
                {
                    return false;
                }
                if let Some(d) = date_to
                    && tx.date > d
                {
                    return false;
                }
                if !desc_q.is_empty() && !tx.description.to_lowercase().contains(&desc_q) {
                    return false;
                }
                if !cat_q.is_empty() && !tx.category.to_lowercase().contains(&cat_q) {
                    return false;
                }
                if !sub_q.is_empty() && !tx.subcategory.to_lowercase().contains(&sub_q) {
                    return false;
                }
                if type_q.eq_ignore_ascii_case("Income")
                    && tx.transaction_type != TransactionType::Income
                {
                    return false;
                }
                if type_q.eq_ignore_ascii_case("Expense")
                    && tx.transaction_type != TransactionType::Expense
                {
                    return false;
                }
                if type_q.eq_ignore_ascii_case("Transfer")
                    && tx.transaction_type != TransactionType::Transfer
                {
                    return false;
                }
                if !self.in_account_scope(tx) {
                    return false;
                }
                if recurring_q.eq_ignore_ascii_case("Recurring") && !tx.is_recurring {
                    return false;
                }
                if recurring_q.eq_ignore_ascii_case("One-Time") && tx.is_recurring {
                    return false;
                }
                if let Some(f) = amt_from
                    && tx.amount < f
                {
                    return false;
                }
                if let Some(t) = amt_to
                    && tx.amount > t
                {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            let cur = self.table_state.selected().unwrap_or(0);
            self.table_state
                .select(Some(cur.min(self.filtered_indices.len() - 1)));
        }
        self.calculate_category_summaries();
        self.calculate_monthly_summaries();
    }
    pub(crate) fn increment_advanced_date(&mut self) {
        let field = self.advanced_filter_fields.focused();
        if field.kind() == FieldKind::Date {
            self.clear_simple_filter_field_only();
            if let Some(new_date) = self.increment_date_field(&self.advanced_filter_fields[field]) {
                self.advanced_filter_fields[field] = new_date;
                self.advanced_filter_cursor = self.advanced_filter_fields[field].len();
            }
        }
    }
    pub(crate) fn decrement_advanced_date(&mut self) {
        let field = self.advanced_filter_fields.focused();
        if field.kind() == FieldKind::Date {
            self.clear_simple_filter_field_only();
            if let Some(new_date) = self.decrement_date_field(&self.advanced_filter_fields[field]) {
                self.advanced_filter_fields[field] = new_date;
                self.advanced_filter_cursor = self.advanced_filter_fields[field].len();
            }
        }
    }
    pub(crate) fn increment_advanced_month(&mut self) {
        let field = self.advanced_filter_fields.focused();
        if field.kind() == FieldKind::Date {
            self.clear_simple_filter_field_only();
            if let Some(new_date) = self.increment_month_field(&self.advanced_filter_fields[field])
            {
                self.advanced_filter_fields[field] = new_date;
                self.advanced_filter_cursor = self.advanced_filter_fields[field].len();
            }
        }
    }
    pub(crate) fn decrement_advanced_month(&mut self) {
        let field = self.advanced_filter_fields.focused();
        if field.kind() == FieldKind::Date {
            self.clear_simple_filter_field_only();
            if let Some(new_date) = self.decrement_month_field(&self.advanced_filter_fields[field])
            {
                self.advanced_filter_fields[field] = new_date;
                self.advanced_filter_cursor = self.advanced_filter_fields[field].len();
            }
        }
    }
}
