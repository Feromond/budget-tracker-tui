use super::state::App;
use crate::app::fields::{CategoryEditField, FieldSet};
use crate::app::state::AppMode;
use crate::db::category_store::CategoryStore;
use crate::model::{CategoryDraft, CategoryRecord, CategorySortColumn, SortOrder, TransactionType};
use chrono::Duration;

impl App {
    /// Page size for category catalog navigation (PageUp/PageDown)
    const CATEGORY_PAGE_SIZE: usize = 20;

    pub(crate) fn open_category_catalog(&mut self, origin: AppMode) {
        // Start each visit with a clean filter so all categories are visible;
        // the reload below re-applies it to the fresh records.
        self.category_filter_query.clear();
        self.category_filter_cursor = 0;
        if let Err(err) = self.reload_categories_from_store() {
            self.set_status_message(format!("Error loading categories: {}", err), None);
            return;
        }

        self.mode = AppMode::CategoryCatalog;
        self.category_catalog_origin = origin;
        self.editing_category_id = None;
        self.category_delete_id = None;
        self.clear_status_message();
    }

    pub(crate) fn exit_category_catalog(&mut self) {
        self.mode = self.category_catalog_origin;
        self.category_delete_id = None;
        if self.mode == AppMode::Budget {
            // Category budgets may have changed; re-validate the month and row selection.
            self.refresh_budget_years();
        }
        self.clear_status_message();
    }

    pub(crate) fn next_category_record(&mut self) {
        let len = self.filtered_category_indices.len();
        if len == 0 {
            return;
        }

        let index = match self.category_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.category_table_state.select(Some(index));
    }

    pub(crate) fn previous_category_record(&mut self) {
        let len = self.filtered_category_indices.len();
        if len == 0 {
            return;
        }

        let index = match self.category_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.category_table_state.select(Some(index));
    }

    pub(crate) fn jump_to_first_category(&mut self) {
        if !self.filtered_category_indices.is_empty() {
            self.category_table_state.select(Some(0));
        }
    }

    pub(crate) fn jump_to_last_category(&mut self) {
        let len = self.filtered_category_indices.len();
        if len > 0 {
            self.category_table_state.select(Some(len - 1));
        }
    }

    pub(crate) fn page_up_category(&mut self) {
        if self.filtered_category_indices.is_empty() {
            return;
        }
        let current = self.category_table_state.selected().unwrap_or(0);
        self.category_table_state
            .select(Some(current.saturating_sub(Self::CATEGORY_PAGE_SIZE)));
    }

    pub(crate) fn page_down_category(&mut self) {
        let len = self.filtered_category_indices.len();
        if len == 0 {
            return;
        }
        let current = self.category_table_state.selected().unwrap_or(0);
        self.category_table_state
            .select(Some((current + Self::CATEGORY_PAGE_SIZE).min(len - 1)));
    }

    // --- Catalog Filtering ---
    // Mirrors the simple transaction filter: live filtering while typing, Enter keeps
    // the filter applied, Esc clears it.
    pub(crate) fn is_category_filter_active(&self) -> bool {
        !self.category_filter_query.is_empty()
    }

    pub(crate) fn start_category_filtering(&mut self) {
        self.mode = AppMode::CategoryCatalogFilter;
        self.category_filter_cursor = self.category_filter_query.len();
        self.clear_status_message();
    }

    pub(crate) fn finish_category_filtering(&mut self) {
        self.mode = AppMode::CategoryCatalog;
        self.clear_status_message();
    }

    pub(crate) fn reset_category_filter(&mut self) {
        let was_active = self.is_category_filter_active();
        self.category_filter_query.clear();
        self.category_filter_cursor = 0;
        self.apply_category_filter();
        self.mode = AppMode::CategoryCatalog;
        if was_active {
            self.set_status_message("Category filter cleared", Some(Duration::seconds(3)));
        } else {
            self.clear_status_message();
        }
    }

    pub(crate) fn apply_category_filter(&mut self) {
        let query = self.category_filter_query.to_lowercase();
        self.filtered_category_indices = self
            .category_records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                if query.is_empty() {
                    return true;
                }
                record.category.to_lowercase().contains(&query)
                    || record.subcategory.to_lowercase().contains(&query)
                    || record
                        .tag
                        .as_ref()
                        .is_some_and(|tag| tag.to_lowercase().contains(&query))
                    || record
                        .transaction_type
                        .to_string()
                        .to_lowercase()
                        .contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.sort_category_records();
        self.clamp_category_catalog_selection();
    }

    // --- Catalog Sorting ---
    // Similar to the transaction table: same key again flips the direction.
    pub(crate) fn set_category_sort_column(&mut self, column: CategorySortColumn) {
        if self.category_sort_by == column {
            self.category_sort_order = match self.category_sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.category_sort_by = column;
            self.category_sort_order = SortOrder::Ascending;
        }

        // Keep the same row selected after the re-sort.
        let selected_id = self.selected_category_record().map(|record| record.id);
        self.sort_category_records();
        let new_selection = selected_id.and_then(|id| {
            self.filtered_category_indices.iter().position(|&index| {
                self.category_records
                    .get(index)
                    .is_some_and(|record| record.id == id)
            })
        });
        if new_selection.is_some() {
            self.category_table_state.select(new_selection);
        } else {
            self.clamp_category_catalog_selection();
        }
    }

    fn sort_category_records(&mut self) {
        // Taken out of `self` so the schedule can be read while the list is sorted.
        let mut indices = std::mem::take(&mut self.filtered_category_indices);
        let month = Self::current_budget_key();
        crate::app::util::sort_category_indices_impl(
            &mut indices,
            &self.category_records,
            |record| self.budget_schedule.category_budget(record.id, month),
            self.category_sort_by,
            self.category_sort_order,
        );
        self.filtered_category_indices = indices;
    }

    pub(crate) fn start_adding_category(&mut self) {
        self.mode = AppMode::CategoryEditor;
        self.editing_category_id = None;
        self.category_edit_fields = FieldSet::new([
            TransactionType::Expense.to_string(),
            String::new(),
            String::new(),
            String::new(),
            "Not set".to_string(),
        ]);
        self.category_edit_cursor =
            self.category_edit_fields[CategoryEditField::TransactionType].len();
        self.clear_status_message();
    }

    pub(crate) fn start_editing_category(&mut self) {
        let Some(record) = self.selected_category_record().cloned() else {
            self.set_status_message("Select a category first.", None);
            return;
        };

        self.mode = AppMode::CategoryEditor;
        self.editing_category_id = Some(record.id);
        let draft = record.to_draft();
        // Shown for reference; the popup is what actually changes it.
        let budget = self
            .budget_schedule
            .category_budget(record.id, Self::current_budget_key())
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "Not set".to_string());
        self.category_edit_fields = FieldSet::new([
            draft.transaction_type.to_string(),
            draft.category,
            draft.subcategory,
            draft.tag.unwrap_or_default(),
            budget,
        ]);
        self.category_edit_cursor =
            self.category_edit_fields[CategoryEditField::TransactionType].len();
        self.clear_status_message();
    }

    pub(crate) fn exit_category_editor(&mut self, cancelled: bool) {
        self.mode = AppMode::CategoryCatalog;
        self.editing_category_id = None;
        self.category_edit_fields = Default::default();
        self.category_edit_cursor = 0;

        if cancelled {
            self.set_status_message("Category edit cancelled.", Some(Duration::seconds(3)));
        } else {
            self.clear_status_message();
        }
    }

    pub(crate) fn next_category_field(&mut self) {
        self.category_edit_fields.focus_next();
        self.category_edit_cursor = self.category_edit_fields.focused_value().len();
    }

    pub(crate) fn previous_category_field(&mut self) {
        self.category_edit_fields.focus_previous();
        self.category_edit_cursor = self.category_edit_fields.focused_value().len();
    }

    pub(crate) fn toggle_category_transaction_type(&mut self) {
        if self.category_edit_fields.focused() != CategoryEditField::TransactionType {
            return;
        }

        let current = TransactionType::try_from(
            self.category_edit_fields[CategoryEditField::TransactionType].as_str(),
        )
        .unwrap_or(TransactionType::Expense);
        self.category_edit_fields[CategoryEditField::TransactionType] =
            crate::app::util::cycle(&TransactionType::all(), current, true).to_string();
        self.category_edit_cursor =
            self.category_edit_fields[CategoryEditField::TransactionType].len();
    }

    pub(crate) fn prepare_delete_category(&mut self) {
        let Some(record) = self.selected_category_record().cloned() else {
            self.set_status_message("Select a category first.", None);
            return;
        };

        self.category_delete_id = Some(record.id);
        self.mode = AppMode::ConfirmCategoryDelete;
        self.set_status_message(
            format!(
                "Delete {} / {}? Press y to confirm.",
                record.category,
                if record.subcategory.is_empty() {
                    "(No subcategory)"
                } else {
                    record.subcategory.as_str()
                }
            ),
            None,
        );
    }

    pub(crate) fn cancel_delete_category(&mut self) {
        self.mode = AppMode::CategoryCatalog;
        self.category_delete_id = None;
        self.clear_status_message();
    }

    pub(crate) fn confirm_delete_category(&mut self) {
        let Some(id) = self.category_delete_id else {
            self.cancel_delete_category();
            return;
        };

        let store = self.category_store();
        if let Err(err) = store.delete(id) {
            self.set_status_message(format!("Error deleting category: {}", err), None);
            return;
        }

        if let Err(err) = self.reload_transactions_from_db() {
            self.set_status_message(
                format!(
                    "Category deleted, but reloading transactions failed: {}",
                    err
                ),
                None,
            );
            return;
        }

        if let Err(err) = self.reload_categories_from_store() {
            self.set_status_message(
                format!("Category deleted, but refresh failed: {}", err),
                None,
            );
            return;
        }

        self.mode = AppMode::CategoryCatalog;
        self.category_delete_id = None;
        self.set_status_message("Category deleted successfully.", Some(Duration::seconds(3)));
    }

    /// Returns the saved id, since an active filter can leave a different row selected.
    pub(crate) fn save_category(&mut self) -> Option<i64> {
        let draft = match self.build_category_draft_from_editor() {
            Ok(draft) => draft,
            Err(message) => {
                self.set_status_message(format!("Error: {}", message), None);
                return None;
            }
        };

        let editing_category_id = self.editing_category_id;

        // The schema's UNIQUE is case-sensitive but rename and delete propagate with LOWER(), so
        // a case-variant pair would let an edit to one record rewrite the other's transactions.
        let duplicate = self.category_records.iter().any(|record| {
            Some(record.id) != editing_category_id
                && record.transaction_type == draft.transaction_type
                && record.category.eq_ignore_ascii_case(&draft.category)
                && record.subcategory.eq_ignore_ascii_case(&draft.subcategory)
        });
        if duplicate {
            self.set_status_message(
                "Error: that category already exists (names are matched without case).",
                None,
            );
            return None;
        }

        let store = self.category_store();
        let result = if let Some(id) = editing_category_id {
            store.update(id, &draft).map(|_| id)
        } else {
            store.insert(&draft).map(|record| record.id)
        };

        let saved_id = match result {
            Ok(saved_id) => saved_id,
            Err(err) => {
                self.set_status_message(format!("Error saving category: {}", err), None);
                return None;
            }
        };

        if editing_category_id.is_some()
            && let Err(err) = self.reload_transactions_from_db()
        {
            self.set_status_message(
                format!("Category saved, but reloading transactions failed: {}", err),
                None,
            );
            return None;
        }

        if let Err(err) = self.reload_categories_from_store() {
            self.set_status_message(format!("Category saved, but refresh failed: {}", err), None);
            return None;
        }

        self.mode = AppMode::CategoryCatalog;
        self.editing_category_id = Some(saved_id);
        self.category_edit_fields = Default::default();
        self.category_edit_cursor = 0;
        self.select_saved_category();
        self.editing_category_id = None;
        self.set_status_message("Category saved successfully.", Some(Duration::seconds(3)));
        Some(saved_id)
    }

    pub(crate) fn selected_category_record(&self) -> Option<&CategoryRecord> {
        self.category_table_state
            .selected()
            .and_then(|index| self.filtered_category_indices.get(index))
            .and_then(|&record_index| self.category_records.get(record_index))
    }

    fn clamp_category_catalog_selection(&mut self) {
        let selection = if self.filtered_category_indices.is_empty() {
            None
        } else {
            Some(
                self.category_table_state
                    .selected()
                    .unwrap_or(0)
                    .min(self.filtered_category_indices.len() - 1),
            )
        };
        self.category_table_state.select(selection);
    }

    /// A catalog sorted by Budget would otherwise show new values in the old order.
    pub(crate) fn resort_catalog_keeping(&mut self, id: i64) {
        self.apply_category_filter();
        let position = self.filtered_category_indices.iter().position(|&index| {
            self.category_records
                .get(index)
                .is_some_and(|record| record.id == id)
        });
        if position.is_some() {
            self.category_table_state.select(position);
        }
    }

    fn select_saved_category(&mut self) {
        let selection = self.editing_category_id.and_then(|id| {
            self.filtered_category_indices.iter().position(|&index| {
                self.category_records
                    .get(index)
                    .is_some_and(|record| record.id == id)
            })
        });

        if selection.is_some() {
            self.category_table_state.select(selection);
        } else {
            self.clamp_category_catalog_selection();
        }
    }

    fn build_category_draft_from_editor(&self) -> Result<CategoryDraft, String> {
        let transaction_type = TransactionType::try_from(
            self.category_edit_fields[CategoryEditField::TransactionType].trim(),
        )
        .map_err(|_| "Transaction type must be Income, Expense, or Transfer.".to_string())?;
        let category = self.category_edit_fields[CategoryEditField::Category]
            .trim()
            .to_string();
        let subcategory = self.category_edit_fields[CategoryEditField::Subcategory]
            .trim()
            .to_string();
        let tag = self.category_edit_fields[CategoryEditField::Tag].trim();

        if category.is_empty() {
            return Err("Category cannot be empty.".to_string());
        }

        Ok(CategoryDraft {
            transaction_type,
            category,
            subcategory,
            tag: if tag.is_empty() {
                None
            } else {
                Some(tag.to_string())
            },
        })
    }
}
