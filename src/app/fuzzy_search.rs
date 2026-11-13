use super::state::App;
use crate::app::fields::{AddEditField, SelectingField};
use crate::model::TransactionType;
use ratatui::widgets::ListState;
use std::collections::HashSet;

impl App {
    // --- Fuzzy Search Logic ---

    pub(crate) fn start_fuzzy_selection(&mut self) {
        self.selecting_field = Some(SelectingField::AddEdit(AddEditField::Category));
        self.mode = crate::app::state::AppMode::FuzzyFinding;
        self.search_query = String::new();
        self.update_fuzzy_search_results();

        // Reset list state
        self.selection_list_state = ListState::default();
        if !self.current_selection_list.is_empty() {
            self.selection_list_state.select(Some(0));
        }
    }

    pub(crate) fn update_fuzzy_search_results(&mut self) {
        let current_type_str = self.add_edit_fields[AddEditField::TransactionType].trim();
        let Ok(current_type) = TransactionType::try_from(current_type_str) else {
            self.current_selection_list.clear();
            self.fuzzy_selection_pairs.clear();
            return;
        };

        let query = self.search_query.to_lowercase();

        let pairs: HashSet<(String, String)> = self
            .categories
            .iter()
            .filter(|cat_info| cat_info.transaction_type == current_type)
            .map(|cat_info| (cat_info.category.clone(), cat_info.subcategory.clone()))
            .collect();
        let mut options: Vec<(String, (String, String))> = pairs
            .into_iter()
            .map(|(cat, sub)| {
                let display = if sub.is_empty() {
                    cat.clone()
                } else {
                    format!("{} > {}", cat, sub)
                };
                (display, (cat, sub))
            })
            .filter(|(display, _)| query.is_empty() || display.to_lowercase().contains(&query))
            .collect();
        options.sort_unstable();
        (self.current_selection_list, self.fuzzy_selection_pairs) = options.into_iter().unzip();

        // Adjust selection if out of bounds
        let len = self.current_selection_list.len();
        if len > 0 {
            let current = self.selection_list_state.selected().unwrap_or(0);
            if current >= len {
                self.selection_list_state.select(Some(len - 1));
            } else {
                self.selection_list_state.select(Some(current));
            }
        } else {
            self.selection_list_state.select(None);
        }
    }

    pub(crate) fn confirm_fuzzy_selection(&mut self) {
        if let Some(selected_index) = self.selection_list_state.selected()
            && let Some((category, subcategory)) = self.fuzzy_selection_pairs.get(selected_index)
        {
            self.add_edit_fields[AddEditField::Category] = category.clone();
            self.add_edit_fields[AddEditField::Subcategory] = subcategory.clone();
            self.add_edit_fields.focus(AddEditField::Date);
        }

        self.exit_fuzzy_mode();
    }

    pub(crate) fn cancel_fuzzy_selection(&mut self) {
        self.exit_fuzzy_mode();
        // Return to category field focus
        self.add_edit_fields.focus(AddEditField::Category);
    }

    fn exit_fuzzy_mode(&mut self) {
        self.mode = if self.editing_index.is_some() {
            crate::app::state::AppMode::Editing
        } else {
            crate::app::state::AppMode::Adding
        };
        self.selecting_field = None;
        self.current_selection_list.clear();
        self.fuzzy_selection_pairs.clear();
        self.search_query.clear();
    }
}
