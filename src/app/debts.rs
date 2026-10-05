use crate::app::fields::{AddEditField, DebtField, ReconcileField};
use crate::app::investments::{STATUS_ACTIVE, STATUS_ARCHIVED};
use crate::app::state::{App, AppMode};
use crate::db::account_store::AccountStore;
use crate::debt::{self, DebtActivity, DebtInput, DebtTerms, PayoffPlan, PayoffStrategy};
use crate::model::{
    Account, AccountClass, AccountDraft, DATE_FORMAT, Transaction, TransactionType,
};
use crate::ui::helpers::format_amount;
use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;

pub(crate) const EXTRA_STEP: i64 = 25;
pub(crate) const PLAN_INCLUDE: &str = "In plan";
pub(crate) const PLAN_TRACK_ONLY: &str = "Track only";
pub(crate) const EXTRA_BIG_STEP: i64 = 100;

impl App {
    pub(crate) fn enter_debts_mode(&mut self) {
        if let Err(err) = self.reload_accounts() {
            self.set_status_message(format!("Error loading debts: {}", err), None);
            return;
        }
        self.mode = AppMode::Debts;
        self.debt_detail_id = None;
        self.clamp_debt_selection();
        self.clear_status_message();
    }

    pub(crate) fn exit_debts_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.debt_detail_id = None;
        self.clear_status_message();
    }

    pub(crate) fn debt_accounts(&self) -> Vec<&Account> {
        self.accounts
            .all()
            .iter()
            .filter(|account| account.class == AccountClass::Credit)
            .filter(|account| self.show_archived_debts || !account.archived)
            .collect()
    }

    fn visible_debt_ids(&self) -> Vec<i64> {
        self.debt_accounts()
            .iter()
            .map(|account| account.id)
            .collect()
    }

    fn selected_debt_id(&self) -> Option<i64> {
        let ids = self.visible_debt_ids();
        self.debt_table_state
            .selected()
            .and_then(|index| ids.get(index).copied())
    }

    pub(crate) fn active_debt(&self) -> Option<&Account> {
        self.debt_detail_id
            .or_else(|| self.selected_debt_id())
            .and_then(|id| self.accounts.get(id))
            .filter(|account| account.class == AccountClass::Credit)
    }

    fn clamp_debt_selection(&mut self) {
        let len = self.visible_debt_ids().len();
        let index = self.debt_table_state.selected().unwrap_or(0);
        self.debt_table_state
            .select((len > 0).then(|| index.min(len - 1)));
    }

    pub(crate) fn next_debt(&mut self) {
        let len = self.visible_debt_ids().len();
        if len == 0 {
            return;
        }
        let index = match self.debt_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.debt_table_state.select(Some(index));
    }

    pub(crate) fn previous_debt(&mut self) {
        let len = self.visible_debt_ids().len();
        if len == 0 {
            return;
        }
        let index = match self.debt_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.debt_table_state.select(Some(index));
    }

    pub(crate) fn toggle_archived_debts(&mut self) {
        self.show_archived_debts = !self.show_archived_debts;
        self.clamp_debt_selection();
        let message = if self.show_archived_debts {
            "Showing archived debts."
        } else {
            "Hiding archived debts."
        };
        self.set_status_message(message, Some(Duration::seconds(2)));
    }

    pub(crate) fn debt_owed(&self, account: &Account) -> Decimal {
        debt::owed_on(account, &self.transactions, self.today())
    }

    pub(crate) fn debt_activity(&self, account: &Account) -> DebtActivity {
        debt::activity(account, &self.transactions, self.today())
    }

    pub(crate) fn debt_in_plan(&self, id: i64) -> bool {
        self.debt_terms.get(&id).is_some_and(|terms| terms.in_plan)
    }

    fn debt_input(&self, account: &Account) -> Option<DebtInput> {
        let (apr, payment) = self.debt_terms.get(&account.id)?.projectable()?;
        Some(DebtInput {
            id: account.id,
            owed: self.debt_owed(account),
            apr,
            payment,
        })
    }

    // Keep paid-off debts so their payments can roll over.
    fn debt_plan_inputs(&self) -> Vec<DebtInput> {
        self.debt_accounts()
            .into_iter()
            .filter(|account| self.debt_in_plan(account.id))
            .filter_map(|account| {
                self.debt_input(account)
                    .filter(|input| input.owed > Decimal::ZERO || !account.archived)
            })
            .collect()
    }

    pub(crate) fn debt_plan(&self, strategy: PayoffStrategy) -> PayoffPlan {
        debt::simulate(&self.debt_plan_inputs(), strategy, self.debt_extra)
    }

    pub(crate) fn debt_plan_payments(&self) -> Decimal {
        self.debt_plan_inputs()
            .iter()
            .filter(|input| self.debt_strategy.uses_extra() || input.owed > Decimal::ZERO)
            .map(|input| input.payment)
            .sum()
    }

    pub(crate) fn standalone_debt_plan(&self, account: &Account) -> Option<PayoffPlan> {
        if self.debt_in_plan(account.id) {
            return None;
        }
        let input = self
            .debt_input(account)
            .filter(|input| input.owed > Decimal::ZERO)?;
        Some(debt::simulate(
            &[input],
            PayoffStrategy::Minimums,
            Decimal::ZERO,
        ))
    }

    pub(crate) fn cycle_debt_strategy(&mut self) {
        self.debt_strategy =
            crate::app::util::cycle(&PayoffStrategy::all(), self.debt_strategy, true);
    }

    pub(crate) fn adjust_debt_extra(&mut self, amount: i64) {
        if !self.debt_strategy.uses_extra() {
            self.set_status_message(
                "Minimums only ignores extra. Press s to pick a strategy.",
                Some(Duration::seconds(3)),
            );
            return;
        }
        self.debt_extra = (self.debt_extra + Decimal::from(amount)).max(Decimal::ZERO);
    }

    pub(crate) fn open_debt_detail(&mut self) {
        let Some(id) = self.selected_debt_id() else {
            return;
        };
        self.debt_detail_id = Some(id);
        self.mode = AppMode::DebtDetail;
        let has_rows = !self.debt_history().is_empty();
        self.debt_history_table_state.select(has_rows.then_some(0));
        self.debt_schedule_focused = false;
        self.debt_schedule_table_state = Default::default();
        self.clear_status_message();
    }

    pub(crate) fn toggle_debt_detail_focus(&mut self) {
        self.debt_schedule_focused = !self.debt_schedule_focused;
        if self.debt_schedule_focused && self.debt_schedule_table_state.selected().is_none() {
            let has_rows = self.debt_schedule_len() > 0;
            self.debt_schedule_table_state.select(has_rows.then_some(0));
        }
    }

    pub(crate) fn debt_schedule_len(&self) -> usize {
        let Some(account) = self.debt_detail_id.and_then(|id| self.accounts.get(id)) else {
            return 0;
        };
        let plan = if self.debt_in_plan(account.id) {
            Some(self.debt_plan(self.debt_strategy))
        } else {
            self.standalone_debt_plan(account)
        };
        plan.and_then(|plan| plan.outcome(account.id).map(|o| o.schedule.len()))
            .unwrap_or(0)
    }

    pub(crate) fn next_debt_detail_row(&mut self) {
        if self.debt_schedule_focused {
            let len = self.debt_schedule_len();
            step_selection(&mut self.debt_schedule_table_state, len, true);
        } else {
            self.next_debt_history_row();
        }
    }

    pub(crate) fn previous_debt_detail_row(&mut self) {
        if self.debt_schedule_focused {
            let len = self.debt_schedule_len();
            step_selection(&mut self.debt_schedule_table_state, len, false);
        } else {
            self.previous_debt_history_row();
        }
    }

    pub(crate) fn exit_debt_detail(&mut self) {
        self.debt_detail_id = None;
        self.mode = AppMode::Debts;
        self.clear_status_message();
    }

    pub(crate) fn debt_history(&self) -> Vec<(&Transaction, Decimal)> {
        let Some(account) = self.debt_detail_id.and_then(|id| self.accounts.get(id)) else {
            return Vec::new();
        };
        let today = self.today();
        let mut rows: Vec<(&Transaction, Decimal)> = self
            .transactions
            .iter()
            .filter(|tx| tx.date <= today && account.tracks(tx.date))
            .filter_map(|tx| {
                let change = match tx.transaction_type {
                    TransactionType::Expense if tx.account_id == account.id => tx.amount,
                    TransactionType::Income if tx.account_id == account.id => -tx.amount,
                    TransactionType::Transfer if tx.to_account_id == Some(account.id) => -tx.amount,
                    TransactionType::Transfer if tx.account_id == account.id => tx.amount,
                    _ => return None,
                };
                Some((tx, change))
            })
            .collect();
        rows.sort_by_key(|(tx, _)| std::cmp::Reverse(tx.date));
        rows
    }

    fn next_debt_history_row(&mut self) {
        let len = self.debt_history().len();
        if len == 0 {
            return;
        }
        let index = match self.debt_history_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.debt_history_table_state.select(Some(index));
    }

    fn previous_debt_history_row(&mut self) {
        let len = self.debt_history().len();
        if len == 0 {
            return;
        }
        let index = match self.debt_history_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.debt_history_table_state.select(Some(index));
    }

    pub(crate) fn start_adding_debt(&mut self) {
        self.debt_fields.reset();
        self.debt_fields[DebtField::Status] = STATUS_ACTIVE.to_string();
        self.debt_fields[DebtField::Plan] = PLAN_INCLUDE.to_string();
        self.editing_debt_id = None;
        self.debt_cursor = 0;
        self.mode = AppMode::DebtEditor;
        self.clear_status_message();
    }

    pub(crate) fn start_editing_debt(&mut self) {
        let Some(account) = self.active_debt().cloned() else {
            return;
        };
        let terms = self.debt_terms.get(&account.id).copied();

        self.debt_fields.reset();
        self.debt_fields[DebtField::Name] = account.name;
        self.debt_fields[DebtField::Kind] = account.kind;
        let owed = -account.opening_balance;
        if !owed.is_zero() {
            self.debt_fields[DebtField::Owed] = format!("{:.2}", owed);
        }
        self.debt_fields[DebtField::AsOf] = account
            .tracked_from
            .map(|date| date.format(DATE_FORMAT).to_string())
            .unwrap_or_default();
        if let Some(apr) = terms.and_then(|t| t.apr) {
            self.debt_fields[DebtField::Apr] = apr.normalize().to_string();
        }
        if let Some(payment) = terms.and_then(|t| t.payment) {
            self.debt_fields[DebtField::Payment] = format!("{:.2}", payment);
        }
        self.debt_fields[DebtField::Plan] = if terms.is_some_and(|t| !t.in_plan) {
            PLAN_TRACK_ONLY
        } else {
            PLAN_INCLUDE
        }
        .to_string();
        self.debt_fields[DebtField::Status] = if account.archived {
            STATUS_ARCHIVED
        } else {
            STATUS_ACTIVE
        }
        .to_string();
        self.editing_debt_id = Some(account.id);
        self.debt_cursor = self.debt_fields.focused_value().len();
        self.mode = AppMode::DebtEditor;
        self.clear_status_message();
    }

    pub(crate) fn cancel_debt_editor(&mut self) {
        self.editing_debt_id = None;
        self.debt_fields.reset();
        self.debt_cursor = 0;
        self.mode = self.debt_return_mode();
        self.clear_status_message();
    }

    pub(crate) fn next_debt_field(&mut self) {
        self.debt_fields.focus_next();
        self.debt_cursor = self.debt_fields.focused_value().len();
    }

    pub(crate) fn previous_debt_field(&mut self) {
        self.debt_fields.focus_previous();
        self.debt_cursor = self.debt_fields.focused_value().len();
    }

    pub(crate) fn toggle_debt_status(&mut self) {
        let field = &mut self.debt_fields[DebtField::Status];
        *field = if field == STATUS_ARCHIVED {
            STATUS_ACTIVE
        } else {
            STATUS_ARCHIVED
        }
        .to_string();
    }

    pub(crate) fn toggle_debt_plan(&mut self) {
        let field = &mut self.debt_fields[DebtField::Plan];
        *field = if field == PLAN_TRACK_ONLY {
            PLAN_INCLUDE
        } else {
            PLAN_TRACK_ONLY
        }
        .to_string();
    }

    pub(crate) fn save_debt(&mut self) {
        let (draft, terms) = match self.debt_draft() {
            Ok(parsed) => parsed,
            Err(message) => {
                self.set_status_message(format!("Error: {}", message), None);
                return;
            }
        };

        let store = self.account_store();
        let saved = match self.editing_debt_id {
            Some(id) => store.update_account(id, &draft).map(|_| id),
            None => store.create_account(&draft),
        };
        let id = match saved {
            Ok(id) => id,
            Err(err) => {
                self.set_status_message(format!("Error saving debt: {}", err), None);
                return;
            }
        };
        let terms_saved = match terms {
            Some(terms) => store.save_debt_terms(&DebtTerms {
                account_id: id,
                ..terms
            }),
            None => store.delete_debt_terms(id),
        };

        let was_edit = self.editing_debt_id.is_some();
        self.editing_debt_id = None;
        self.debt_fields.reset();
        self.debt_cursor = 0;
        self.mode = self.debt_return_mode();

        let message = match terms_saved {
            Err(err) => format!("Debt saved, but its terms failed: {}", err),
            Ok(()) => format!(
                "Debt '{}' {}.",
                draft.name,
                if was_edit { "updated" } else { "added" }
            ),
        };
        self.finish_debt_write(Some(id), message);
        if let Some(warning) = self.debt_terms_warning(id) {
            self.set_status_message(warning, None);
        }
    }

    fn debt_draft(&self) -> Result<(AccountDraft, Option<DebtTerms>), String> {
        let field = |key: DebtField| self.debt_fields[key].trim();
        let name = field(DebtField::Name);
        if name.is_empty() {
            return Err("Debt name cannot be empty".to_string());
        }
        let owed = match field(DebtField::Owed) {
            "" => Decimal::ZERO,
            value => crate::validation::validate_non_negative_amount_string(value)?,
        };
        let tracked_from = match field(DebtField::AsOf) {
            "" => None,
            value => Some(
                NaiveDate::parse_from_str(value, DATE_FORMAT)
                    .map_err(|_| format!("Invalid As Of date (expected {})", DATE_FORMAT))?,
            ),
        };
        let draft = AccountDraft {
            name: name.to_string(),
            kind: field(DebtField::Kind).to_string(),
            archived: self.debt_fields[DebtField::Status] == STATUS_ARCHIVED,
            class: AccountClass::Credit,
            opening_balance: -owed,
            tracked_from,
        };

        let in_plan = self.debt_fields[DebtField::Plan] != PLAN_TRACK_ONLY;
        let (apr, payment, months) = (
            field(DebtField::Apr),
            field(DebtField::Payment),
            field(DebtField::MonthsLeft),
        );
        if in_plan && apr.is_empty() && payment.is_empty() && months.is_empty() {
            return Ok((draft, None));
        }

        let apr = match apr {
            "" => None,
            value => {
                let apr = crate::validation::validate_non_negative_amount_string(value)?;
                if apr > Decimal::from(100) {
                    return Err("The interest rate is a yearly percent, like 19.99".to_string());
                }
                Some(apr)
            }
        };
        let payment = if !payment.is_empty() {
            Some(crate::validation::validate_amount_string(payment)?)
        } else if !months.is_empty() {
            let apr = apr.ok_or("Set an interest rate to work out a payment from months left")?;
            let months: u32 = months
                .parse()
                .ok()
                .filter(|months| (1..=debt::MAX_MONTHS as u32).contains(months))
                .ok_or(format!(
                    "Months left must be a whole number from 1 to {}",
                    debt::MAX_MONTHS
                ))?;
            let account = Account {
                id: self.editing_debt_id.unwrap_or(-1),
                name: draft.name.clone(),
                kind: draft.kind.clone(),
                position: 0,
                archived: draft.archived,
                class: AccountClass::Credit,
                opening_balance: draft.opening_balance,
                tracked_from: draft.tracked_from,
            };
            let owed_now = debt::owed_on(&account, &self.transactions, self.today());
            Some(
                debt::payment_for_term(owed_now, apr, months)
                    .ok_or("Nothing is owed, so there's no payment to work out")?,
            )
        } else {
            None
        };

        if in_plan && apr.is_none() {
            return Err(
                "Set an interest rate (0 if it's interest free), or choose Track only".to_string(),
            );
        }
        if in_plan && payment.is_none() {
            return Err("Set a monthly payment or months left, or choose Track only".to_string());
        }
        Ok((
            draft,
            Some(DebtTerms {
                account_id: 0,
                apr,
                payment,
                in_plan,
            }),
        ))
    }

    fn debt_terms_warning(&self, id: i64) -> Option<String> {
        let account = self.accounts.get(id)?;
        let (apr, payment) = self.debt_terms.get(&id)?.projectable()?;
        let owed = self.debt_owed(account);
        let interest = (owed * apr / Decimal::from(1200)).round_dp(2);
        (owed > Decimal::ZERO && payment <= interest).then(|| {
            format!(
                "Heads up: {}/mo doesn't cover the {} monthly interest on '{}', so it never gets paid off.",
                format_amount(&payment),
                format_amount(&interest),
                account.name
            )
        })
    }

    pub(crate) fn prepare_delete_debt(&mut self) {
        let Some(account) = self
            .selected_debt_id()
            .and_then(|id| self.accounts.get(id))
            .cloned()
        else {
            return;
        };

        let used = self.account_transaction_count(account.id);
        if used > 0 {
            self.set_status_message(
                format!(
                    "{} transaction{} '{}'. Archive it instead (e).",
                    used,
                    if used == 1 { " uses" } else { "s use" },
                    account.name
                ),
                None,
            );
            return;
        }
        let spending = self
            .accounts
            .all()
            .iter()
            .filter(|a| a.class.holds_spending())
            .count();
        if spending == 1 {
            self.set_status_message("A ledger needs at least one cash or credit account.", None);
            return;
        }

        self.debt_delete_prompt = format!("Delete '{}'? (y/n)", account.name);
        self.debt_delete_id = Some(account.id);
        self.mode = AppMode::ConfirmDebtDelete;
    }

    pub(crate) fn cancel_delete_debt(&mut self) {
        self.debt_delete_id = None;
        self.mode = AppMode::Debts;
        self.clear_status_message();
    }

    pub(crate) fn confirm_delete_debt(&mut self) {
        let Some(id) = self.debt_delete_id.take() else {
            self.cancel_delete_debt();
            return;
        };
        self.mode = AppMode::Debts;
        if let Err(err) = self.account_store().delete_account(id) {
            self.set_status_message(format!("Error deleting debt: {}", err), None);
            return;
        }
        self.finish_debt_write(None, "Debt deleted.".to_string());
    }

    pub(crate) fn start_debt_payment(&mut self) {
        let Some(account) = self.active_debt().cloned() else {
            self.set_status_message("Add a debt first (press a).", None);
            return;
        };
        let cash = |include_archived: bool| {
            self.accounts
                .all()
                .iter()
                .find(|a| a.class == AccountClass::Cash && (include_archived || !a.archived))
                .map(|a| a.name.clone())
        };
        let Some(from) = cash(false).or_else(|| cash(true)) else {
            self.set_status_message(
                "Add a cash account to pay from first (Settings > Manage Accounts).",
                None,
            );
            return;
        };

        let owed = self.debt_owed(&account);
        let amount = self
            .debt_terms
            .get(&account.id)
            .and_then(|terms| terms.payment)
            .unwrap_or(owed)
            .min(owed);
        let amount = (amount > Decimal::ZERO).then_some(amount);
        let subcategory = debt::payment_subcategory(&account)
            .filter(|sub| self.has_category(TransactionType::Transfer, debt::DEBT_CATEGORY, sub))
            .unwrap_or_default();

        let category_listed = self.has_category(TransactionType::Transfer, debt::DEBT_CATEGORY, "");

        let origin = self.mode;
        self.start_adding();
        self.add_return_mode = origin;
        let fields = &mut self.add_edit_fields;
        fields[AddEditField::Description] = format!("{} payment", account.name);
        fields[AddEditField::Amount] = amount.map(|a| format!("{:.2}", a)).unwrap_or_default();
        fields[AddEditField::TransactionType] = TransactionType::Transfer.as_str().to_string();
        if category_listed {
            fields[AddEditField::Category] = debt::DEBT_CATEGORY.to_string();
            fields[AddEditField::Subcategory] = subcategory.to_string();
        }
        fields[AddEditField::Account] = from;
        fields[AddEditField::ToAccount] = account.name.clone();
        self.focus_add_field(AddEditField::Amount);
        if !account.tracks(self.today()) {
            self.set_status_message(
                format!(
                    "'{}' has As Of set to today, so a payment dated today won't lower the balance. Date it later or move As Of back.",
                    account.name
                ),
                None,
            );
        }
    }

    fn has_category(&self, kind: TransactionType, category: &str, subcategory: &str) -> bool {
        self.categories.iter().any(|info| {
            info.transaction_type == kind
                && info.category.eq_ignore_ascii_case(category)
                && (subcategory.is_empty() || info.subcategory.eq_ignore_ascii_case(subcategory))
        })
    }

    fn focus_add_field(&mut self, field: AddEditField) {
        self.add_edit_fields.focus(field);
        self.add_edit_cursor = self.add_edit_fields[field].len();
    }

    pub(crate) fn start_reconcile(&mut self) {
        if self.active_debt().is_none() {
            self.set_status_message("Add a debt first (press a).", None);
            return;
        }
        self.reconcile_fields.reset();
        self.reconcile_fields[ReconcileField::Date] = self.today().format(DATE_FORMAT).to_string();
        self.reconcile_fields.focus(ReconcileField::Balance);
        self.reconcile_cursor = 0;
        self.mode = AppMode::DebtReconcile;
        self.clear_status_message();
    }

    pub(crate) fn cancel_reconcile(&mut self) {
        self.reconcile_fields.reset();
        self.reconcile_cursor = 0;
        self.mode = self.debt_return_mode();
        self.clear_status_message();
    }

    pub(crate) fn next_reconcile_field(&mut self) {
        self.reconcile_fields.focus_next();
        self.reconcile_cursor = self.reconcile_fields.focused_value().len();
    }

    pub(crate) fn previous_reconcile_field(&mut self) {
        self.reconcile_fields.focus_previous();
        self.reconcile_cursor = self.reconcile_fields.focused_value().len();
    }

    pub(crate) fn save_reconcile(&mut self) {
        let Some(account) = self.active_debt().cloned() else {
            self.cancel_reconcile();
            return;
        };
        let statement = match crate::validation::validate_non_negative_amount_string(
            self.reconcile_fields[ReconcileField::Balance].trim(),
        ) {
            Ok(amount) => amount,
            Err(message) => {
                self.set_status_message(format!("Error: {}", message), None);
                return;
            }
        };
        let date = match NaiveDate::parse_from_str(
            self.reconcile_fields[ReconcileField::Date].trim(),
            DATE_FORMAT,
        ) {
            Ok(date) => date,
            Err(_) => {
                self.set_status_message(
                    format!("Error: Invalid date (expected {})", DATE_FORMAT),
                    None,
                );
                return;
            }
        };
        if !account.tracks(date) {
            self.set_status_message(
                "That's on or before the As Of date, so it's already in the starting amount. Edit the debt (e) instead.",
                None,
            );
            return;
        }

        let recorded = debt::signed_owed_on(&account, &self.transactions, date);
        let gap = statement - recorded;
        let origin = self.debt_return_mode();
        self.reconcile_fields.reset();
        self.reconcile_cursor = 0;

        if gap.is_zero() {
            self.mode = origin;
            self.set_status_message(
                format!("'{}' matches the statement. Nothing to add.", account.name),
                Some(Duration::seconds(4)),
            );
            return;
        }

        self.start_adding();
        self.add_return_mode = origin;
        let date_text = date.format(DATE_FORMAT).to_string();
        let interest_category = debt::interest_category(&account)
            .into_iter()
            .find(|(category, sub)| self.has_category(TransactionType::Expense, category, sub));
        let fields = &mut self.add_edit_fields;
        fields[AddEditField::Date] = date_text;
        fields[AddEditField::Amount] = format!("{:.2}", gap.abs());
        fields[AddEditField::Account] = account.name.clone();

        if gap > Decimal::ZERO {
            fields[AddEditField::TransactionType] = TransactionType::Expense.as_str().to_string();
            fields[AddEditField::Description] = format!("{} interest", account.name);
            if let Some((category, sub)) = interest_category {
                fields[AddEditField::Category] = category.to_string();
                fields[AddEditField::Subcategory] = sub.to_string();
            }
            self.focus_add_field(AddEditField::Description);
            self.set_status_message(
                format!(
                    "The statement is {} higher than recorded. Change the category if it wasn't interest.",
                    format_amount(&gap)
                ),
                None,
            );
        } else {
            fields[AddEditField::TransactionType] = TransactionType::Income.as_str().to_string();
            fields[AddEditField::Description] = format!("{} statement credit", account.name);
            self.focus_add_field(AddEditField::Category);
            self.set_status_message(
                format!(
                    "The statement is {} lower than recorded. Pick a category and save.",
                    format_amount(&gap.abs())
                ),
                None,
            );
        }
    }

    fn debt_return_mode(&self) -> AppMode {
        if self.debt_detail_id.is_some() {
            AppMode::DebtDetail
        } else {
            AppMode::Debts
        }
    }

    fn finish_debt_write(&mut self, select: Option<i64>, message: String) {
        if let Err(err) = self.reload_accounts() {
            self.set_status_message(format!("Saved, but reloading failed: {}", err), None);
            return;
        }
        if let Some(id) = select
            && let Some(index) = self.visible_debt_ids().iter().position(|v| *v == id)
        {
            self.debt_table_state.select(Some(index));
        }
        if self
            .debt_detail_id
            .is_some_and(|id| self.accounts.get(id).is_none())
        {
            self.debt_detail_id = None;
            self.mode = AppMode::Debts;
        }
        self.clamp_debt_selection();
        self.set_status_message(message, Some(Duration::seconds(3)));
    }
}

fn step_selection(state: &mut ratatui::widgets::TableState, len: usize, forward: bool) {
    if len == 0 {
        state.select(None);
        return;
    }
    let current = state.selected().unwrap_or(0).min(len - 1);
    let next = if forward {
        (current + 1).min(len - 1)
    } else {
        current.saturating_sub(1)
    };
    state.select(Some(next));
}
