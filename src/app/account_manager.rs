use super::state::{App, AppMode};
use crate::app::fields::AccountField;
use crate::app::investments::{STATUS_ACTIVE, STATUS_ARCHIVED};
use crate::app::settings_types::SettingKey;
use crate::db::account_store::AccountStore;
use crate::model::{Account, AccountClass, AccountDraft};
use chrono::Duration;
use rust_decimal::Decimal;

impl App {
    pub(crate) fn open_account_manager(&mut self, origin: AppMode) {
        if let Err(err) = self.reload_accounts() {
            self.set_status_message(format!("Error loading accounts: {}", err), None);
            return;
        }
        self.account_manager_origin = origin;
        self.mode = AppMode::AccountManager;
        self.editing_account_id = None;
        self.clamp_account_selection();
        self.clear_status_message();
    }

    pub(crate) fn account_balance(&self, account: &Account) -> Decimal {
        let today = self.today();
        match account.class {
            AccountClass::Investment => self.portfolio.value_on(account.id, today),
            AccountClass::Cash | AccountClass::Credit => {
                self.accounts.balance(account.id, &self.transactions, today)
            }
        }
    }

    pub(crate) fn exit_account_manager(&mut self) {
        self.editing_account_id = None;
        if self.account_manager_origin == AppMode::Settings {
            self.enter_settings_mode();
            self.select_settings_row(SettingKey::ManageAccounts);
        } else {
            self.mode = self.account_manager_origin;
            self.clear_status_message();
        }
    }

    fn clamp_account_selection(&mut self) {
        let len = self.accounts.all().len();
        let index = self.account_table_state.selected().unwrap_or(0);
        self.account_table_state
            .select((len > 0).then(|| index.min(len - 1)));
    }

    fn selected_account_id(&self) -> Option<i64> {
        self.account_table_state
            .selected()
            .and_then(|index| self.accounts.all().get(index))
            .map(|account| account.id)
    }

    pub(crate) fn next_account(&mut self) {
        let len = self.accounts.all().len();
        if len == 0 {
            return;
        }
        let index = match self.account_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.account_table_state.select(Some(index));
    }

    pub(crate) fn previous_account(&mut self) {
        let len = self.accounts.all().len();
        if len == 0 {
            return;
        }
        let index = match self.account_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.account_table_state.select(Some(index));
    }

    pub(crate) fn account_transaction_count(&self, id: i64) -> usize {
        self.transactions
            .iter()
            .filter(|tx| {
                !tx.is_generated_from_recurring
                    && (tx.account_id == id || tx.to_account_id == Some(id))
            })
            .count()
    }

    pub(crate) fn start_adding_account(&mut self) {
        self.account_fields.reset();
        self.account_fields[AccountField::Class] = AccountClass::Cash.as_str().to_string();
        self.account_fields[AccountField::Status] = STATUS_ACTIVE.to_string();
        self.editing_account_id = None;
        self.account_cursor = 0;
        self.mode = AppMode::AccountEditor;
        self.clear_status_message();
    }

    pub(crate) fn start_editing_account(&mut self) {
        let Some(account) = self
            .selected_account_id()
            .and_then(|id| self.accounts.get(id))
            .cloned()
        else {
            return;
        };
        self.account_fields.reset();
        self.account_fields[AccountField::Name] = account.name;
        self.account_fields[AccountField::Class] = account.class.as_str().to_string();
        self.account_fields[AccountField::Kind] = account.kind;
        let opening = match account.class {
            AccountClass::Credit => -account.opening_balance,
            AccountClass::Cash | AccountClass::Investment => account.opening_balance,
        };
        if !opening.is_zero() {
            self.account_fields[AccountField::OpeningBalance] = format!("{:.2}", opening);
        }
        self.account_fields[AccountField::Status] = if account.archived {
            STATUS_ARCHIVED
        } else {
            STATUS_ACTIVE
        }
        .to_string();
        self.editing_account_id = Some(account.id);
        self.account_cursor = self.account_fields.focused_value().len();
        self.mode = AppMode::AccountEditor;
        self.clear_status_message();
    }

    pub(crate) fn cancel_account_editor(&mut self) {
        self.editing_account_id = None;
        self.account_fields.reset();
        self.account_cursor = 0;
        self.mode = AppMode::AccountManager;
        self.clear_status_message();
    }

    pub(crate) fn next_account_field(&mut self) {
        self.account_fields.focus_next();
        self.account_cursor = self.account_fields.focused_value().len();
    }

    pub(crate) fn previous_account_field(&mut self) {
        self.account_fields.focus_previous();
        self.account_cursor = self.account_fields.focused_value().len();
    }

    pub(crate) fn cycle_account_toggle(&mut self, forward: bool) {
        match self.account_fields.focused() {
            AccountField::Class => {
                if let Some(id) = self.editing_account_id
                    && (self.account_transaction_count(id) > 0
                        || self.portfolio.entries_for(id).next().is_some())
                {
                    self.set_status_message(
                        "An account's class can't change once it has transactions or entries.",
                        Some(Duration::seconds(3)),
                    );
                    return;
                }
                let current = AccountClass::from_label(&self.account_fields[AccountField::Class])
                    .unwrap_or(AccountClass::Cash);
                self.account_fields[AccountField::Class] =
                    crate::app::util::cycle(&AccountClass::all(), current, forward)
                        .as_str()
                        .to_string();
            }
            AccountField::Status => {
                let field = &mut self.account_fields[AccountField::Status];
                *field = if field == STATUS_ARCHIVED {
                    STATUS_ACTIVE
                } else {
                    STATUS_ARCHIVED
                }
                .to_string();
            }
            AccountField::Name | AccountField::Kind | AccountField::OpeningBalance => {}
        }
    }

    pub(crate) fn save_account(&mut self) {
        let class = AccountClass::from_label(&self.account_fields[AccountField::Class])
            .unwrap_or(AccountClass::Cash);
        let opening_str = self.account_fields[AccountField::OpeningBalance].trim();
        let opening = if opening_str.is_empty() {
            Decimal::ZERO
        } else {
            match crate::validation::validate_non_negative_amount_string(opening_str) {
                Ok(amount) => amount,
                Err(msg) => {
                    self.set_status_message(format!("Error: {}", msg), None);
                    return;
                }
            }
        };
        let draft = AccountDraft {
            name: self.account_fields[AccountField::Name].trim().to_string(),
            kind: self.account_fields[AccountField::Kind].trim().to_string(),
            archived: self.account_fields[AccountField::Status] == STATUS_ARCHIVED,
            class,
            opening_balance: match class {
                AccountClass::Credit => -opening,
                AccountClass::Cash | AccountClass::Investment => opening,
            },
        };

        let store = self.account_store();
        let result = match self.editing_account_id {
            Some(id) => store.update_account(id, &draft).map(|_| id),
            None => store.create_account(&draft),
        };
        let id = match result {
            Ok(id) => id,
            Err(err) => {
                self.set_status_message(format!("Error saving account: {}", err), None);
                return;
            }
        };

        let was_edit = self.editing_account_id.is_some();
        self.editing_account_id = None;
        self.account_fields.reset();
        self.account_cursor = 0;
        self.mode = AppMode::AccountManager;
        if let Err(err) = self.reload_accounts() {
            self.set_status_message(format!("Saved, but reloading failed: {}", err), None);
            return;
        }
        if let Some(index) = self.accounts.all().iter().position(|a| a.id == id) {
            self.account_table_state.select(Some(index));
        }
        self.set_status_message(
            format!(
                "Account '{}' {}.",
                draft.name,
                if was_edit { "updated" } else { "added" }
            ),
            Some(Duration::seconds(3)),
        );
    }

    pub(crate) fn prepare_delete_account(&mut self) {
        let Some(account) = self
            .selected_account_id()
            .and_then(|id| self.accounts.get(id))
            .cloned()
        else {
            return;
        };

        let transactions = self.account_transaction_count(account.id);
        if transactions > 0 {
            self.set_status_message(
                format!(
                    "{} transaction{} '{}'. Archive it instead (e).",
                    transactions,
                    if transactions == 1 { " uses" } else { "s use" },
                    account.name
                ),
                None,
            );
            return;
        }
        let spending_accounts = self
            .accounts
            .all()
            .iter()
            .filter(|a| a.class.holds_spending())
            .count();
        if account.class.holds_spending() && spending_accounts == 1 {
            self.set_status_message("A ledger needs at least one cash or credit account.", None);
            return;
        }

        let entries = self.portfolio.entries_for(account.id).count();
        self.account_delete_prompt = if entries > 0 {
            format!(
                "Delete '{}' and its {} entr{}? (y/n)",
                account.name,
                entries,
                if entries == 1 { "y" } else { "ies" }
            )
        } else {
            format!("Delete '{}'? (y/n)", account.name)
        };
        self.account_delete_id = Some(account.id);
        self.mode = AppMode::ConfirmAccountDelete;
    }

    pub(crate) fn cancel_delete_account(&mut self) {
        self.account_delete_id = None;
        self.mode = AppMode::AccountManager;
        self.clear_status_message();
    }

    pub(crate) fn confirm_delete_account(&mut self) {
        let Some(id) = self.account_delete_id.take() else {
            self.cancel_delete_account();
            return;
        };
        self.mode = AppMode::AccountManager;
        if let Err(err) = self
            .account_store()
            .delete_account(id)
            .and_then(|_| self.reload_accounts())
        {
            self.set_status_message(format!("Error deleting account: {}", err), None);
            return;
        }
        self.clamp_account_selection();
        self.set_status_message("Account deleted.", Some(Duration::seconds(3)));
    }
}
