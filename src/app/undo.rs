use super::state::{App, AppMode};
use crate::db::backup;
use crate::db::ledger_store::LedgerStore;
use chrono::Duration;
use ratatui::widgets::TableState;
use std::fs;
use std::io::Error;

const MAX_STEPS: usize = 50;
const MAX_BYTES: usize = 64 * 1024 * 1024;

struct Step {
    bytes: Vec<u8>,
    area: &'static str,
    ledger_id: i64,
}

#[derive(Default)]
pub(crate) struct UndoHistory {
    undo: Vec<Step>,
    redo: Vec<Step>,
    baseline: Option<(Vec<u8>, [u8; 4])>,
    pub(crate) key_mode: Option<AppMode>,
}

fn area_of(mode: AppMode) -> &'static str {
    use AppMode::*;
    match mode {
        Budget | BudgetCategoryEditor => "budgets",
        CategoryCatalog
        | CategoryCatalogFilter
        | CategoryEditor
        | ConfirmCategoryDelete
        | SelectingConversionAccount
        | ConfirmCategoryConversion => "categories",
        AccountManager | AccountEditor | ConfirmAccountDelete => "accounts",
        LedgerManager | LedgerEditor | ConfirmLedgerDelete => "ledgers",
        Investments
        | InvestmentDetail
        | InvestmentAccountEditor
        | InvestmentEntryEditor
        | ConfirmInvestmentDelete => "investments",
        Debts | DebtDetail | DebtEditor | DebtReconcile | ConfirmDebtDelete => "debts",
        Settings => "settings",
        _ => "transactions",
    }
}

impl App {
    pub(crate) fn reset_undo_history(&mut self) {
        self.undo_history.undo.clear();
        self.undo_history.redo.clear();
        self.refresh_undo_baseline();
    }

    pub(crate) fn refresh_undo_baseline(&mut self) {
        let path = &self.database_path;
        self.undo_history.baseline =
            backup::change_counter(path).and_then(|counter| Some((fs::read(path).ok()?, counter)));
    }

    pub(crate) fn record_undo_step(&mut self) {
        let Some(counter) = backup::change_counter(&self.database_path) else {
            return;
        };
        if self
            .undo_history
            .baseline
            .as_ref()
            .is_some_and(|(_, seen)| *seen == counter)
        {
            return;
        }
        if let Some((bytes, _)) = self.undo_history.baseline.take() {
            let area = area_of(self.undo_history.key_mode.unwrap_or(self.mode));
            let steps = &mut self.undo_history.undo;
            steps.push(Step {
                bytes,
                area,
                ledger_id: self.active_ledger_id,
            });
            self.undo_history.redo.clear();
            let mut total: usize = steps.iter().map(|step| step.bytes.len()).sum();
            while steps.len() > 1 && (steps.len() > MAX_STEPS || total > MAX_BYTES) {
                total -= steps.remove(0).bytes.len();
            }
        }
        self.refresh_undo_baseline();
    }

    pub(crate) fn undo(&mut self) {
        self.step_history(true);
    }

    pub(crate) fn redo(&mut self) {
        self.step_history(false);
    }

    fn step_history(&mut self, undo: bool) {
        let verb = if undo { "undo" } else { "redo" };
        if !self.mode_allows_undo() {
            self.set_status_message(format!("Close this first to {}.", verb), None);
            return;
        }
        let history = &mut self.undo_history;
        let (from, to) = if undo {
            (&mut history.undo, &mut history.redo)
        } else {
            (&mut history.redo, &mut history.undo)
        };
        let Some((current, _)) = history.baseline.as_ref() else {
            self.set_status_message(
                format!("Couldn't {}: the database file can't be read.", verb),
                None,
            );
            return;
        };
        let current = current.clone();
        let Some(step) = from.pop() else {
            self.set_status_message(format!("Nothing to {}.", verb), Some(Duration::seconds(3)));
            return;
        };

        if let Err(err) = backup::overwrite(&self.database_path, &step.bytes) {
            from.push(step);
            self.set_status_message(format!("Couldn't {}: {}", verb, err), None);
            return;
        }
        to.push(Step {
            bytes: current,
            area: step.area,
            ledger_id: step.ledger_id,
        });

        let reloaded = self.reload_after_history_step();
        self.refresh_undo_baseline();
        if let Err(err) = &reloaded {
            self.set_status_message(format!("Reloading failed after {}: {}", verb, err), None);
            return;
        }

        let elsewhere = match reloaded {
            Ok(true) => format!(". Now in '{}'", self.active_ledger_name()),
            _ => (step.area != "ledgers" && step.ledger_id != self.active_ledger_id)
                .then(|| {
                    self.ledgers
                        .iter()
                        .find(|ledger| ledger.id == step.ledger_id)
                })
                .flatten()
                .map(|ledger| format!(" in '{}'", ledger.name))
                .unwrap_or_default(),
        };
        let done = if undo { "Undid" } else { "Redid" };
        self.set_status_message(
            format!("{} a change to {}{}.", done, step.area, elsewhere),
            Some(Duration::seconds(4)),
        );
    }

    // A reload can remove the row a form or prompt is using.
    fn mode_allows_undo(&self) -> bool {
        use AppMode::*;
        matches!(
            self.mode,
            Normal
                | Summary
                | CategorySummary
                | Budget
                | Settings
                | CategoryCatalog
                | AccountManager
                | LedgerManager
                | BackupManager
                | Investments
                | InvestmentDetail
                | Debts
                | DebtDetail
        )
    }

    /// Returns true if the current ledger no longer exists.
    fn reload_after_history_step(&mut self) -> Result<bool, Error> {
        let open_ledger = self.active_ledger_id;
        self.refresh_ledgers()?;
        let moved = if self.ledgers.iter().any(|ledger| ledger.id == open_ledger) {
            if self.active_ledger_id != open_ledger {
                self.ledger_store().set_active_id(open_ledger)?;
                self.active_ledger_id = open_ledger;
            }
            false
        } else {
            self.clear_all_filter_fields();
            self.account_scope = None;
            true
        };
        self.reload_categories_from_store()?;
        self.reload_transactions_from_db()?;
        self.reload_budget_schedule()?;
        self.refresh_budget_years();

        clamp(&mut self.table_state, self.filtered_indices.len());
        clamp(&mut self.ledger_table_state, self.ledgers.len());
        self.clamp_account_selection();
        self.clamp_category_catalog_selection();
        self.clamp_debt_selection();
        self.clamp_investment_selection();
        if self.mode == AppMode::CategorySummary {
            self.resort_category_summary_items();
        }
        if self.mode == AppMode::InvestmentDetail
            && self
                .investment_detail_id
                .is_none_or(|id| self.accounts.get(id).is_none())
        {
            self.investment_detail_id = None;
            self.mode = AppMode::Investments;
        }
        if self.mode == AppMode::DebtDetail && self.active_debt().is_none() {
            self.debt_detail_id = None;
            self.mode = AppMode::Debts;
        }
        if let Some(id) = self.investment_detail_id {
            let len = self.portfolio.entries_for(id).count();
            clamp(&mut self.investment_entry_table_state, len);
        }
        Ok(moved)
    }
}

fn clamp(state: &mut TableState, len: usize) {
    if let Some(selected) = state.selected() {
        state.select((len > 0).then(|| selected.min(len - 1)));
    }
}
