use super::state::App;
use crate::app::fields::{AddEditField, SelectingField};
use crate::model::TransactionType;
use ratatui::widgets::ListState;
use std::collections::HashSet;

impl App {
    fn report_no_categories(&mut self, transaction_type: TransactionType) {
        self.cancel_selection();
        self.set_status_message(
            format!(
                "No {} categories yet. Add some in Settings > Manage Categories.",
                transaction_type.as_str().to_lowercase()
            ),
            None,
        );
    }

    // --- Category/Subcategory Selection Logic ---
    pub(crate) fn start_category_selection(&mut self) {
        // If fuzzy search is enabled, redirect to that mode
        if self.fuzzy_search_mode {
            if !self
                .categories
                .iter()
                .any(|category| category.transaction_type == self.add_edit_type())
            {
                self.report_no_categories(self.add_edit_type());
                return;
            }
            self.start_fuzzy_selection();
            return;
        }
        self.type_to_select.clear();

        self.selecting_field = Some(SelectingField::AddEdit(AddEditField::Category));
        self.mode = crate::app::state::AppMode::SelectingCategory;
        let current_type_str = self.add_edit_fields[AddEditField::TransactionType].trim();
        let Ok(current_type) = TransactionType::try_from(current_type_str) else {
            self.set_status_message("Error: Invalid transaction type for category lookup.", None);
            self.mode = if self.editing_index.is_some() {
                crate::app::state::AppMode::Editing
            } else {
                crate::app::state::AppMode::Adding
            };
            return;
        };
        let mut unique_categories: HashSet<String> = self
            .categories
            .iter()
            .filter(|cat_info| cat_info.transaction_type == current_type)
            .map(|cat_info| cat_info.category.clone())
            .collect();
        if unique_categories.is_empty() {
            self.report_no_categories(current_type);
            return;
        }
        let mut options: Vec<String> = unique_categories.drain().collect();
        options.sort_unstable();
        self.current_selection_list = options;
        self.selection_list_state = ListState::default();
        if !self.current_selection_list.is_empty() {
            self.selection_list_state.select(Some(0));
        }
    }
    pub(crate) fn start_account_selection(&mut self, field: AddEditField) {
        let transfer = self.add_edit_type() == TransactionType::Transfer;
        if field == AddEditField::ToAccount && !transfer {
            return;
        }
        let current = self.add_edit_fields[field].trim().to_string();
        let other = match field {
            AddEditField::ToAccount => self.add_edit_fields[AddEditField::Account].trim(),
            _ => self.add_edit_fields[AddEditField::ToAccount].trim(),
        };
        let options: Vec<String> = self
            .accounts
            .all()
            .iter()
            .filter(|account| transfer || account.class.holds_spending())
            .filter(|account| !(transfer && account.name.eq_ignore_ascii_case(other)))
            .filter(|account| !account.archived || account.name.eq_ignore_ascii_case(&current))
            .map(|account| account.name.clone())
            .collect();
        if options.is_empty() {
            self.set_status_message("No other account to pick. Add one in Settings.", None);
            return;
        }

        self.type_to_select.clear();
        self.selecting_field = Some(SelectingField::AddEdit(field));
        self.mode = crate::app::state::AppMode::SelectingCategory;
        let selected = options
            .iter()
            .position(|name| name.eq_ignore_ascii_case(&current))
            .unwrap_or(0);
        self.current_selection_list = options;
        self.selection_list_state = ListState::default();
        self.selection_list_state.select(Some(selected));
    }

    pub(crate) fn start_subcategory_selection(&mut self) {
        self.type_to_select.clear();
        self.selecting_field = Some(SelectingField::AddEdit(AddEditField::Subcategory));
        self.mode = crate::app::state::AppMode::SelectingSubcategory;
        let current_type_str = self.add_edit_fields[AddEditField::TransactionType].trim();
        let current_category = self.add_edit_fields[AddEditField::Category].trim();
        let Ok(current_type) = TransactionType::try_from(current_type_str) else {
            self.set_status_message(
                "Error: Invalid transaction type for subcategory lookup.",
                None,
            );
            self.mode = if self.editing_index.is_some() {
                crate::app::state::AppMode::Editing
            } else {
                crate::app::state::AppMode::Adding
            };
            return;
        };
        if current_category.is_empty() || current_category.eq_ignore_ascii_case("Uncategorized") {
            self.current_selection_list = vec!["(None)".to_string()];
        } else {
            let mut unique_subcategories: HashSet<String> = self
                .categories
                .iter()
                .filter(|cat_info| {
                    cat_info.transaction_type == current_type
                        && cat_info.category.eq_ignore_ascii_case(current_category)
                        && !cat_info.subcategory.is_empty()
                })
                .map(|cat_info| cat_info.subcategory.clone())
                .collect();
            let mut options: Vec<String> = unique_subcategories.drain().collect();
            options.sort_unstable();
            options.insert(0, "(None)".to_string());
            self.current_selection_list = options;
        }
        self.selection_list_state = ListState::default();
        if !self.current_selection_list.is_empty() {
            self.selection_list_state.select(Some(0));
        }
    }
    pub(crate) fn confirm_selection(&mut self) {
        if let Some(selected_index) = self.selection_list_state.selected()
            && let Some(field) = self.selecting_field.and_then(SelectingField::add_edit)
            && let Some(selected_value) = self.current_selection_list.get(selected_index)
        {
            let value_to_set = if field == AddEditField::Subcategory && selected_value == "(None)" {
                ""
            } else {
                selected_value.as_str()
            };
            self.add_edit_fields[field] = value_to_set.to_string();
            if field == AddEditField::Category {
                self.add_edit_fields.focus(AddEditField::Subcategory);
                self.start_subcategory_selection();
                return;
            } else if field == AddEditField::Subcategory
                && self.add_edit_type() == TransactionType::Transfer
                && self.add_edit_fields[AddEditField::ToAccount].is_empty()
            {
                self.add_edit_fields.focus(AddEditField::ToAccount);
                self.start_account_selection(AddEditField::ToAccount);
                return;
            } else if field == AddEditField::Subcategory {
                self.add_edit_fields.focus(AddEditField::Date);
                self.add_edit_cursor = self.add_edit_fields[AddEditField::Date].len();
            }
        }
        self.mode = if self.editing_index.is_some() {
            crate::app::state::AppMode::Editing
        } else {
            crate::app::state::AppMode::Adding
        };
        self.selecting_field = None;
        self.current_selection_list.clear();
    }
    pub(crate) fn cancel_selection(&mut self) {
        self.mode = if self.editing_index.is_some() {
            crate::app::state::AppMode::Editing
        } else {
            crate::app::state::AppMode::Adding
        };
        if let Some(field) = self.selecting_field.and_then(SelectingField::add_edit) {
            self.add_edit_fields.focus(field);
        }
        self.selecting_field = None;
        self.current_selection_list.clear();
    }
    pub(crate) fn select_next_list_item(&mut self) {
        let list_len = self.current_selection_list.len();
        if list_len == 0 {
            return;
        }
        let i = match self.selection_list_state.selected() {
            Some(i) => {
                if i >= list_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selection_list_state.select(Some(i));
    }
    pub(crate) fn select_previous_list_item(&mut self) {
        let list_len = self.current_selection_list.len();
        if list_len == 0 {
            return;
        }
        let i = match self.selection_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    list_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selection_list_state.select(Some(i));
    }

    pub(crate) fn handle_type_to_select(&mut self, c: char) {
        if let Some(index) =
            self.type_to_select
                .handle_char(c, &self.current_selection_list, |item| item.as_str())
        {
            self.selection_list_state.select(Some(index));
        }
    }
}
