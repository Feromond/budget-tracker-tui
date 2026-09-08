use super::state::{App, AppMode};
use crate::app::settings_types::SettingKey;
use crate::db::ledger_store::LedgerStore;
use chrono::Duration;

impl App {
    pub(crate) fn open_ledger_manager(&mut self) {
        if let Err(err) = self.refresh_ledgers() {
            self.set_status_message(format!("Error loading ledgers: {}", err), None);
            return;
        }

        self.mode = AppMode::LedgerManager;
        self.editing_ledger_id = None;
        self.ledger_delete_id = None;
        self.select_active_ledger_row();
        self.clear_status_message();
    }

    /// Rebuilds the settings form so its ledger row reflects any switch or rename made here.
    pub(crate) fn exit_ledger_manager(&mut self) {
        self.editing_ledger_id = None;
        self.ledger_copy_source_id = None;
        self.ledger_delete_id = None;
        self.ledger_name_input.clear();
        self.ledger_name_cursor = 0;
        self.enter_settings_mode();
        self.select_settings_row(SettingKey::ManageLedgers);
    }

    fn select_active_ledger_row(&mut self) {
        let index = self
            .ledgers
            .iter()
            .position(|ledger| ledger.id == self.active_ledger_id)
            .unwrap_or(0);
        self.ledger_table_state.select(Some(index));
    }

    fn selected_ledger_id(&self) -> Option<i64> {
        self.ledger_table_state
            .selected()
            .and_then(|index| self.ledgers.get(index))
            .map(|ledger| ledger.id)
    }

    pub(crate) fn next_ledger(&mut self) {
        let len = self.ledgers.len();
        if len == 0 {
            return;
        }
        let index = match self.ledger_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.ledger_table_state.select(Some(index));
    }

    pub(crate) fn previous_ledger(&mut self) {
        let len = self.ledgers.len();
        if len == 0 {
            return;
        }
        let index = match self.ledger_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.ledger_table_state.select(Some(index));
    }

    pub(crate) fn activate_selected_ledger(&mut self) {
        let Some(id) = self.selected_ledger_id() else {
            return;
        };

        if id == self.active_ledger_id {
            self.exit_ledger_manager();
            return;
        }

        if let Err(err) = self.switch_ledger(id) {
            self.set_status_message(format!("Error switching ledger: {}", err), None);
            return;
        }

        let name = self.active_ledger_name().to_string();
        self.exit_ledger_manager();
        self.set_status_message(
            format!("Switched to ledger '{}'.", name),
            Some(Duration::seconds(3)),
        );
    }

    pub(crate) fn start_adding_ledger(&mut self) {
        self.mode = AppMode::LedgerEditor;
        self.editing_ledger_id = None;
        self.ledger_copy_source_id = None;
        self.ledger_name_input.clear();
        self.ledger_name_cursor = 0;
        self.clear_status_message();
    }

    pub(crate) fn start_copying_ledger(&mut self) {
        let Some(index) = self.ledger_table_state.selected() else {
            return;
        };
        let Some(ledger) = self.ledgers.get(index) else {
            return;
        };

        let source_id = ledger.id;
        let suggested = self.unique_copy_name(&ledger.name);

        self.mode = AppMode::LedgerEditor;
        self.editing_ledger_id = None;
        self.ledger_copy_source_id = Some(source_id);
        self.ledger_name_input = suggested;
        self.ledger_name_cursor = self.ledger_name_input.len();
        self.clear_status_message();
    }

    fn unique_copy_name(&self, base: &str) -> String {
        let taken = |candidate: &str| {
            self.ledgers
                .iter()
                .any(|ledger| ledger.name.eq_ignore_ascii_case(candidate))
        };

        let first = format!("{} (copy)", base);
        if !taken(&first) {
            return first;
        }
        (2..)
            .map(|n| format!("{} (copy {})", base, n))
            .find(|candidate| !taken(candidate))
            .unwrap_or(first)
    }

    pub(crate) fn start_renaming_ledger(&mut self) {
        let Some(index) = self.ledger_table_state.selected() else {
            return;
        };
        let Some(ledger) = self.ledgers.get(index) else {
            return;
        };

        self.mode = AppMode::LedgerEditor;
        self.editing_ledger_id = Some(ledger.id);
        self.ledger_copy_source_id = None;
        self.ledger_name_input = ledger.name.clone();
        self.ledger_name_cursor = self.ledger_name_input.len();
        self.clear_status_message();
    }

    pub(crate) fn cancel_ledger_editor(&mut self) {
        self.mode = AppMode::LedgerManager;
        self.editing_ledger_id = None;
        self.ledger_copy_source_id = None;
        self.ledger_name_input.clear();
        self.ledger_name_cursor = 0;
        self.clear_status_message();
    }

    pub(crate) fn save_ledger(&mut self) {
        let name = self.ledger_name_input.trim().to_string();
        if name.is_empty() {
            self.set_status_message("Error: ledger name cannot be empty.", None);
            return;
        }

        let store = self.ledger_store();
        let result = match (self.editing_ledger_id, self.ledger_copy_source_id) {
            (Some(id), _) => store.rename(id, &name).map(|_| id),
            (None, Some(source_id)) => store.copy(source_id, &name).map(|ledger| ledger.id),
            (None, None) => store.create(&name).map(|ledger| ledger.id),
        };

        let saved_id = match result {
            Ok(id) => id,
            Err(err) => {
                self.set_status_message(format!("Error saving ledger: {}", err), None);
                return;
            }
        };

        let was_rename = self.editing_ledger_id.is_some();
        let was_copy = self.ledger_copy_source_id.is_some();
        if let Err(err) = self.refresh_ledgers() {
            self.set_status_message(format!("Ledger saved, but refresh failed: {}", err), None);
            return;
        }

        self.mode = AppMode::LedgerManager;
        self.editing_ledger_id = None;
        self.ledger_copy_source_id = None;
        self.ledger_name_input.clear();
        self.ledger_name_cursor = 0;
        if let Some(index) = self.ledgers.iter().position(|ledger| ledger.id == saved_id) {
            self.ledger_table_state.select(Some(index));
        }

        let message = if was_rename {
            format!("Ledger renamed to '{}'.", name)
        } else if was_copy {
            format!("Ledger copied to '{}'. Press Enter to switch to it.", name)
        } else {
            format!("Ledger '{}' created. Press Enter to switch to it.", name)
        };
        self.set_status_message(message, Some(Duration::seconds(4)));
    }

    pub(crate) fn prepare_delete_ledger(&mut self) {
        let Some(id) = self.selected_ledger_id() else {
            return;
        };

        if self.ledgers.len() <= 1 {
            self.set_status_message("At least one ledger must remain.", None);
            return;
        }

        let name = self
            .ledgers
            .iter()
            .find(|ledger| ledger.id == id)
            .map(|ledger| ledger.name.clone())
            .unwrap_or_default();
        let store = self.ledger_store();
        let count = store.transaction_count(id).unwrap_or(0);
        let accounts = store.investment_account_count(id).unwrap_or(0);

        let plural = |n: i64| if n == 1 { "" } else { "s" };
        self.ledger_delete_prompt = if accounts > 0 {
            format!(
                "Delete '{}' with its {} transaction{} and {} investment account{}? (y/n)",
                name,
                count,
                plural(count),
                accounts,
                plural(accounts)
            )
        } else {
            format!(
                "Delete '{}' and its {} transaction{}? (y/n)",
                name,
                count,
                plural(count)
            )
        };
        self.ledger_delete_id = Some(id);
        self.mode = AppMode::ConfirmLedgerDelete;
    }

    pub(crate) fn cancel_delete_ledger(&mut self) {
        self.ledger_delete_id = None;
        self.mode = AppMode::LedgerManager;
        self.clear_status_message();
    }

    pub(crate) fn confirm_delete_ledger(&mut self) {
        let Some(id) = self.ledger_delete_id else {
            self.cancel_delete_ledger();
            return;
        };

        let name = self
            .ledgers
            .iter()
            .find(|ledger| ledger.id == id)
            .map(|ledger| ledger.name.clone())
            .unwrap_or_default();

        let delete_result = self.ledger_store().delete(id);

        // Leave the confirmation before reporting anything, so no error path can strand the
        // dialog with an id that has already been deleted.
        self.mode = AppMode::LedgerManager;
        self.ledger_delete_id = None;

        if let Err(err) = delete_result {
            self.set_status_message(format!("Error deleting ledger: {}", err), None);
            return;
        }

        // Deleting the open ledger leaves the stored selection dangling; refresh_ledgers
        // repairs it, and the reload then pulls in whichever ledger it landed on.
        let was_active = id == self.active_ledger_id;
        if let Err(err) = self.refresh_ledgers() {
            self.set_status_message(format!("Ledger deleted, but refresh failed: {}", err), None);
            return;
        }
        if was_active {
            self.clear_all_filter_fields();
            if let Err(err) = self.reload_working_set() {
                self.set_status_message(
                    format!("Ledger deleted, but reloading failed: {}", err),
                    None,
                );
                return;
            }
        }

        self.select_active_ledger_row();
        self.set_status_message(
            format!("Ledger '{}' deleted.", name),
            Some(Duration::seconds(3)),
        );
    }
}
