use super::state::App;
use crate::app::fields::{AddEditField, FieldKey, FieldSet};
use crate::db::transaction_store::TransactionStore;
use crate::model::TransactionType;
use crate::model::{DATE_FORMAT, TransactionDraft};
use chrono::{Duration, NaiveDate};

impl App {
    // Helper function to find the original recurring transaction for a generated one
    pub(crate) fn find_original_recurring_transaction(
        &self,
        generated_tx: &crate::model::Transaction,
    ) -> Option<usize> {
        // Only generated occurrences point back to a source.
        let parent_id = generated_tx.parent_id?;
        self.transactions
            .iter()
            .position(|tx| tx.id == Some(parent_id) && !tx.is_generated_from_recurring)
    }

    // Shared function for jumping to original recurring transaction
    pub(crate) fn jump_to_original_if_needed(
        &mut self,
        tx: &crate::model::Transaction,
        original_index: usize,
        action: crate::app::util::JumpToOriginalAction,
    ) -> Option<usize> {
        if !tx.is_generated_from_recurring {
            return Some(original_index); // Not generated, use current index
        }

        if let Some(original_recurring_index) = self.find_original_recurring_transaction(tx) {
            // Find the view index for the original transaction
            if let Some(original_view_index) = self
                .filtered_indices
                .iter()
                .position(|&idx| idx == original_recurring_index)
            {
                // Select the original transaction in the table
                self.table_state.select(Some(original_view_index));
                self.set_status_message(action.message(), None);
                Some(original_recurring_index)
            } else {
                self.set_status_message(
                    "Original recurring transaction not visible in current filter.",
                    None,
                );
                None
            }
        } else {
            self.set_status_message("Could not find original recurring transaction.", None);
            None
        }
    }

    // --- Adding Logic ---
    // Handles entering add mode, setting up default values, and resetting state.
    pub(crate) fn start_adding(&mut self) {
        self.mode = crate::app::state::AppMode::Adding;
        self.editing_index = None;
        self.add_edit_fields = Default::default();
        let today = chrono::Local::now().date_naive();
        self.add_edit_fields[AddEditField::Date] = today.format(DATE_FORMAT).to_string();
        self.add_edit_fields[AddEditField::TransactionType] = "Expense".to_string();
        let scoped = self
            .account_scope
            .and_then(|id| self.accounts.get(id))
            .filter(|account| account.class.holds_spending())
            .map(|account| account.name.clone());
        self.add_edit_fields[AddEditField::Account] =
            scoped.unwrap_or_else(|| self.default_account_name());
        self.add_edit_cursor = self.add_edit_fields[AddEditField::Date].len();
        self.clear_status_message();
    }
    pub(crate) fn exit_adding(&mut self, cancelled: bool) {
        self.mode = crate::app::state::AppMode::Normal;
        self.editing_index = None;
        self.add_edit_fields = Default::default();
        if cancelled {
            self.set_status_message("Add transaction cancelled.", Some(Duration::seconds(3)));
        }
    }
    pub(crate) fn add_edit_type(&self) -> TransactionType {
        TransactionType::try_from(self.add_edit_fields[AddEditField::TransactionType].trim())
            .unwrap_or(TransactionType::Expense)
    }

    fn add_edit_draft(&self) -> Result<TransactionDraft, String> {
        let amount = crate::validation::validate_amount_string(
            self.add_edit_fields[AddEditField::Amount].trim(),
        )?;
        let date =
            NaiveDate::parse_from_str(&self.add_edit_fields[AddEditField::Date], DATE_FORMAT)
                .map_err(|_| format!("Invalid Date Format (Expected {})", DATE_FORMAT))?;
        let description = self.add_edit_fields[AddEditField::Description].trim();
        if description.is_empty() {
            return Err("Description cannot be empty".to_string());
        }

        let transaction_type = self.add_edit_type();
        let category = self.add_edit_fields[AddEditField::Category].trim();
        let subcategory = self.add_edit_fields[AddEditField::Subcategory].trim();
        crate::validation::validate_category(
            &self.categories,
            transaction_type,
            category,
            subcategory,
        )?;

        let account = |field: AddEditField| {
            let name = self.add_edit_fields[field].trim();
            self.accounts
                .named(name)
                .map(|account| account.id)
                .ok_or_else(|| {
                    if name.is_empty() {
                        format!("Select {}", field.label().to_lowercase())
                    } else {
                        format!("No account named '{}'", name)
                    }
                })
        };
        let account_id = account(AddEditField::Account)?;
        let to_account_id = match transaction_type {
            TransactionType::Transfer => Some(account(AddEditField::ToAccount)?),
            TransactionType::Income | TransactionType::Expense => None,
        };

        Ok(TransactionDraft {
            date,
            description: description.to_string(),
            amount,
            transaction_type,
            category: category.to_string(),
            subcategory: subcategory.to_string(),
            is_recurring: false,
            recurrence_frequency: None,
            recurrence_end_date: None,
            account_id,
            to_account_id,
        })
    }

    pub(crate) fn add_transaction(&mut self) {
        let draft = match self.add_edit_draft() {
            Ok(draft) => draft,
            Err(msg) => {
                self.set_status_message(format!("Error: {}", msg), None);
                return;
            }
        };

        match self.transaction_store().insert(&draft) {
            Ok(_) => match self.reload_transactions_from_db() {
                Ok(_) => {
                    self.set_status_message(
                        "Transaction added successfully.",
                        Some(Duration::seconds(3)),
                    );
                    self.exit_adding(false);
                }
                Err(e) => {
                    self.set_status_message(format!("Error reloading transactions: {}", e), None);
                }
            },
            Err(e) => {
                self.set_status_message(format!("Error saving transaction: {}", e), None);
            }
        }
    }
    // --- Editing Logic ---
    pub(crate) fn start_editing(&mut self) {
        if let Some(view_index) = self.table_state.selected() {
            if let Some(original_index) = self.get_original_index(view_index) {
                let tx = self.transactions[original_index].clone();

                // Jump to original if this is a generated transaction, or use current if not
                if let Some(target_index) = self.jump_to_original_if_needed(
                    &tx,
                    original_index,
                    crate::app::util::JumpToOriginalAction::Edit,
                ) {
                    let target_tx = &self.transactions[target_index];

                    self.mode = crate::app::state::AppMode::Editing;
                    self.editing_index = Some(target_index);
                    self.add_edit_fields = FieldSet::new([
                        target_tx.date.format(DATE_FORMAT).to_string(),
                        target_tx.description.clone(),
                        format!("{:.2}", target_tx.amount),
                        target_tx.transaction_type.as_str().to_string(),
                        target_tx.category.clone(),
                        target_tx.subcategory.clone(),
                        self.accounts.name(target_tx.account_id).to_string(),
                        target_tx
                            .to_account_id
                            .map(|id| self.accounts.name(id).to_string())
                            .unwrap_or_default(),
                    ]);
                    self.add_edit_cursor = self.add_edit_fields[AddEditField::Date].len();

                    if target_index == original_index {
                        self.clear_status_message();
                    }
                }
            } else {
                self.set_status_message("Error: Could not map view index to transaction", None);
            }
        } else {
            self.set_status_message("Select a transaction to edit first", None);
        }
    }
    pub(crate) fn exit_editing(&mut self, cancelled: bool) {
        self.mode = crate::app::state::AppMode::Normal;
        self.editing_index = None;
        self.add_edit_fields = Default::default();
        if cancelled {
            self.set_status_message("Edit transaction cancelled.", Some(Duration::seconds(3)));
        } else {
            self.clear_status_message();
        }
    }
    pub(crate) fn update_transaction(&mut self) {
        if let Some(index) = self.editing_index {
            let mut draft = match self.add_edit_draft() {
                Ok(draft) => draft,
                Err(msg) => {
                    self.set_status_message(format!("Error: {}", msg), None);
                    return;
                }
            };

            // Update transaction
            if index < self.transactions.len() {
                let existing_tx = &self.transactions[index];
                // The edit form only edits the core fields; preserve the recurring rule.
                draft.is_recurring = existing_tx.is_recurring;
                draft.recurrence_frequency = existing_tx.recurrence_frequency;
                draft.recurrence_end_date = existing_tx.recurrence_end_date;
                let Some(id) = existing_tx.id else {
                    self.set_status_message("Error: transaction has no database id", None);
                    self.exit_editing(true);
                    return;
                };

                match self
                    .transaction_store()
                    .update(id, &draft)
                    .and_then(|_| self.reload_transactions_from_db())
                {
                    Ok(_) => {
                        self.set_status_message(
                            "Transaction updated successfully.",
                            Some(Duration::seconds(3)),
                        );
                        self.exit_editing(false);
                    }
                    Err(e) => {
                        self.set_status_message(
                            format!("Error saving updated transaction: {}", e),
                            None,
                        );
                    }
                }
            } else {
                self.set_status_message("Error: Invalid index during edit", None);
                self.exit_editing(true);
            }
        } else {
            self.set_status_message("Error: No transaction selected for editing", None);
            self.exit_editing(true);
        }
    }
    // --- Field Navigation ---
    pub(crate) fn next_add_edit_field(&mut self) {
        self.add_edit_fields.focus_next();
        self.add_edit_cursor = self.add_edit_fields.focused_value().len();
    }

    pub(crate) fn previous_add_edit_field(&mut self) {
        self.add_edit_fields.focus_previous();
        self.add_edit_cursor = self.add_edit_fields.focused_value().len();
    }

    pub(crate) fn cycle_transaction_type(&mut self, forward: bool) {
        if self.add_edit_fields.focused() != AddEditField::TransactionType {
            return;
        }
        let next = crate::app::util::cycle(&TransactionType::all(), self.add_edit_type(), forward);
        self.add_edit_fields[AddEditField::TransactionType] = next.as_str().to_string();
        self.add_edit_fields[AddEditField::Category] = String::new();
        self.add_edit_fields[AddEditField::Subcategory] = String::new();
        if next != TransactionType::Transfer {
            self.add_edit_fields[AddEditField::ToAccount] = String::new();
            let spends = self
                .accounts
                .named(&self.add_edit_fields[AddEditField::Account])
                .is_some_and(|account| account.class.holds_spending());
            if !spends {
                self.add_edit_fields[AddEditField::Account] = self.default_account_name();
            }
        }
    }

    pub(crate) fn default_account_name(&self) -> String {
        self.accounts
            .default_id()
            .map(|id| self.accounts.name(id).to_string())
            .unwrap_or_default()
    }

    // --- Copying Logic ---
    pub(crate) fn copy_transaction(&mut self) {
        if let Some(view_index) = self.table_state.selected() {
            if let Some(original_index) = self.get_original_index(view_index) {
                let tx = self.transactions[original_index].clone();
                let today = chrono::Local::now().date_naive();

                // A copy is always a plain (non-recurring) transaction dated today.
                let draft = TransactionDraft {
                    date: today,
                    description: tx.description.clone(),
                    amount: tx.amount,
                    transaction_type: tx.transaction_type,
                    category: tx.category.clone(),
                    subcategory: tx.subcategory.clone(),
                    is_recurring: false,
                    recurrence_frequency: None,
                    recurrence_end_date: None,
                    account_id: tx.account_id,
                    to_account_id: tx.to_account_id,
                };

                match self
                    .transaction_store()
                    .insert(&draft)
                    .and_then(|_| self.reload_transactions_from_db())
                {
                    Ok(_) => {
                        if let Some(new_view_index) =
                            self.filtered_indices.iter().position(|&idx| {
                                let t = &self.transactions[idx];
                                t.date == today
                                    && t.description == tx.description
                                    && t.amount == tx.amount
                                    && t.transaction_type == tx.transaction_type
                                    && t.category == tx.category
                                    && t.subcategory == tx.subcategory
                                    && t.account_id == tx.account_id
                                    && t.to_account_id == tx.to_account_id
                                    && !t.is_recurring
                                    && !t.is_generated_from_recurring
                            })
                        {
                            self.table_state.select(Some(new_view_index));
                        }
                        self.set_status_message(
                            "Transaction copied with today's date.",
                            Some(Duration::seconds(3)),
                        );
                    }
                    Err(e) => {
                        self.set_status_message(
                            format!("Error saving copied transaction: {}", e),
                            None,
                        );
                    }
                }
            } else {
                self.set_status_message("Error: Could not map view index to transaction", None);
            }
        } else {
            self.set_status_message("Select a transaction to copy first", None);
        }
    }

    // --- Deleting Logic ---
    // Handles preparing, confirming, and canceling transaction deletion.
    pub(crate) fn prepare_for_delete(&mut self) {
        if let Some(view_index) = self.table_state.selected() {
            if let Some(original_index) = self.get_original_index(view_index) {
                // Clone the transaction to avoid borrowing issues
                let tx = self.transactions[original_index].clone();

                // Jump to original if this is a generated transaction, or use current if not
                if let Some(target_index) = self.jump_to_original_if_needed(
                    &tx,
                    original_index,
                    crate::app::util::JumpToOriginalAction::Delete,
                ) {
                    self.delete_index = Some(target_index);
                    self.mode = crate::app::state::AppMode::ConfirmDelete;

                    // Only show delete confirmation if we didn't jump (to preserve jump message)
                    if target_index == original_index {
                        self.set_status_message("Confirm Delete? (y/n)", None);
                    }
                }
            } else {
                self.set_status_message("Error: Could not map view index to transaction", None);
            }
        } else {
            self.set_status_message("Select a transaction to delete first", None);
        }
    }
    pub(crate) fn confirm_delete(&mut self) {
        if let Some(original_index) = self.delete_index {
            // Deletion always targets a real row (generated rows jump to their source first).
            let Some(id) = self.transactions[original_index].id else {
                self.set_status_message("Error: transaction has no database id", None);
                self.cancel_delete();
                return;
            };

            match self
                .transaction_store()
                .delete(id)
                .and_then(|_| self.reload_transactions_from_db())
            {
                Ok(_) => {
                    if let Some(selected) = self.table_state.selected()
                        && selected >= self.filtered_indices.len()
                        && !self.filtered_indices.is_empty()
                    {
                        self.table_state
                            .select(Some(self.filtered_indices.len() - 1));
                    }
                    self.set_status_message(
                        "Transaction deleted successfully.",
                        Some(Duration::seconds(3)),
                    );
                    self.delete_index = None;
                    self.mode = crate::app::state::AppMode::Normal;
                }
                Err(e) => {
                    self.set_status_message(format!("Error deleting transaction: {}", e), None);
                }
            }
        } else {
            self.cancel_delete();
        }
    }
    pub(crate) fn cancel_delete(&mut self) {
        self.mode = crate::app::state::AppMode::Normal;
        self.delete_index = None;
        self.clear_status_message();
    }
}
