use crate::app::fields::{InvestmentAccountField, InvestmentEntryField};
use crate::app::state::{App, AppMode, InvestmentDeleteTarget};
use crate::db::investment_store::InvestmentStore;
use crate::model::{
    DATE_FORMAT, InvestmentAccountDraft, InvestmentEntryDraft, InvestmentEntryKind,
    InvestmentPosition, InvestmentRange,
};
use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;

pub const STALE_VALUATION_DAYS: i64 = 30;

const STATUS_ACTIVE: &str = "Active";
const STATUS_ARCHIVED: &str = "Archived";

impl App {
    pub(crate) fn today(&self) -> NaiveDate {
        chrono::Local::now().date_naive()
    }

    pub(crate) fn enter_investments_mode(&mut self) {
        if let Err(err) = self.reload_portfolio() {
            self.set_status_message(format!("Error loading investments: {}", err), None);
            return;
        }

        self.mode = AppMode::Investments;
        self.investment_detail_id = None;
        self.clamp_investment_selection();
        self.clamp_investment_range();
        self.clear_status_message();
    }

    pub(crate) fn exit_investments_mode(&mut self) {
        self.mode = AppMode::Normal;
        self.investment_detail_id = None;
        self.clear_status_message();
    }

    // --- Overview navigation ---

    pub(crate) fn visible_investment_ids(&self) -> Vec<i64> {
        self.portfolio
            .visible_accounts(self.show_archived_investments)
            .iter()
            .map(|account| account.id)
            .collect()
    }

    pub(crate) fn selected_investment_account_id(&self) -> Option<i64> {
        let ids = self.visible_investment_ids();
        self.investment_table_state
            .selected()
            .and_then(|index| ids.get(index).copied())
    }

    pub(crate) fn active_investment_account_id(&self) -> Option<i64> {
        self.investment_detail_id
            .or_else(|| self.selected_investment_account_id())
    }

    fn clamp_investment_selection(&mut self) {
        let len = self.visible_investment_ids().len();
        if len == 0 {
            self.investment_table_state.select(None);
            return;
        }
        let index = self
            .investment_table_state
            .selected()
            .unwrap_or(0)
            .min(len - 1);
        self.investment_table_state.select(Some(index));
    }

    pub(crate) fn next_investment_account(&mut self) {
        let len = self.visible_investment_ids().len();
        if len == 0 {
            return;
        }
        let index = match self.investment_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.investment_table_state.select(Some(index));
    }

    pub(crate) fn previous_investment_account(&mut self) {
        let len = self.visible_investment_ids().len();
        if len == 0 {
            return;
        }
        let index = match self.investment_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.investment_table_state.select(Some(index));
    }

    pub(crate) fn available_investment_ranges(&self) -> Vec<InvestmentRange> {
        let today = self.today();
        let earliest = self
            .portfolio
            .earliest_date(self.investment_detail_id, self.show_archived_investments);
        let mut ranges: Vec<InvestmentRange> = InvestmentRange::all()
            .into_iter()
            .filter(|range| range.crops(today, earliest))
            .collect();
        ranges.push(InvestmentRange::All);
        ranges
    }

    pub(crate) fn cycle_investment_range(&mut self, forward: bool) {
        let ranges = self.available_investment_ranges();
        let index = ranges
            .iter()
            .position(|range| *range == self.investment_range)
            .unwrap_or(ranges.len() - 1);
        let next = if forward {
            (index + 1) % ranges.len()
        } else {
            (index + ranges.len() - 1) % ranges.len()
        };
        self.investment_range = ranges[next];
    }

    fn clamp_investment_range(&mut self) {
        if !self
            .available_investment_ranges()
            .contains(&self.investment_range)
        {
            self.investment_range = InvestmentRange::All;
        }
    }

    pub(crate) fn toggle_archived_investments(&mut self) {
        self.show_archived_investments = !self.show_archived_investments;
        self.clamp_investment_selection();
        self.clamp_investment_range();
        let message = if self.show_archived_investments {
            "Showing archived accounts."
        } else {
            "Hiding archived accounts."
        };
        self.set_status_message(message, Some(Duration::seconds(2)));
    }

    pub(crate) fn investment_window(&self) -> (NaiveDate, NaiveDate) {
        let today = self.today();
        let start = self.investment_range.start(
            today,
            self.portfolio
                .earliest_date(self.investment_detail_id, self.show_archived_investments),
        );
        (start, today)
    }

    pub(crate) fn investment_position(&self) -> InvestmentPosition {
        let today = self.today();
        match self.investment_detail_id {
            Some(id) => self.portfolio.position(id, today),
            None => self
                .portfolio
                .total_position(today, self.show_archived_investments),
        }
    }

    pub(crate) fn is_valuation_stale(&self, account_id: i64) -> bool {
        match self.portfolio.last_valuation_date(account_id) {
            Some(date) => (self.today() - date).num_days() > STALE_VALUATION_DAYS,
            None => false,
        }
    }

    // --- Detail view ---

    pub(crate) fn open_investment_detail(&mut self) {
        let Some(id) = self.selected_investment_account_id() else {
            return;
        };
        self.investment_detail_id = Some(id);
        self.mode = AppMode::InvestmentDetail;
        self.clamp_investment_range();
        let has_entries = !self.detail_entry_ids().is_empty();
        self.investment_entry_table_state
            .select(has_entries.then_some(0));
        self.clear_status_message();
    }

    pub(crate) fn exit_investment_detail(&mut self) {
        self.investment_detail_id = None;
        self.mode = AppMode::Investments;
        self.clamp_investment_range();
        self.clear_status_message();
    }

    /// Newest first, matching the table.
    pub(crate) fn detail_entry_ids(&self) -> Vec<i64> {
        let Some(account_id) = self.investment_detail_id else {
            return Vec::new();
        };
        let mut ids: Vec<i64> = self
            .portfolio
            .entries_for(account_id)
            .map(|entry| entry.id)
            .collect();
        ids.reverse();
        ids
    }

    pub(crate) fn selected_investment_entry_id(&self) -> Option<i64> {
        let ids = self.detail_entry_ids();
        self.investment_entry_table_state
            .selected()
            .and_then(|index| ids.get(index).copied())
    }

    pub(crate) fn next_investment_entry(&mut self) {
        let len = self.detail_entry_ids().len();
        if len == 0 {
            return;
        }
        let index = match self.investment_entry_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.investment_entry_table_state.select(Some(index));
    }

    pub(crate) fn previous_investment_entry(&mut self) {
        let len = self.detail_entry_ids().len();
        if len == 0 {
            return;
        }
        let index = match self.investment_entry_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.investment_entry_table_state.select(Some(index));
    }

    // --- Account editor ---

    pub(crate) fn start_adding_investment_account(&mut self) {
        self.investment_account_fields.reset();
        self.investment_account_fields[InvestmentAccountField::Status] = STATUS_ACTIVE.to_string();
        self.investment_account_fields[InvestmentAccountField::OpeningDate] =
            self.today().format(DATE_FORMAT).to_string();
        self.editing_investment_account_id = None;
        self.investment_account_cursor = 0;
        self.mode = AppMode::InvestmentAccountEditor;
        self.clear_status_message();
    }

    pub(crate) fn start_editing_investment_account(&mut self) {
        let Some(account) = self
            .active_investment_account_id()
            .and_then(|id| self.portfolio.account(id))
        else {
            return;
        };

        let (id, name, kind, archived) = (
            account.id,
            account.name.clone(),
            account.kind.clone(),
            account.archived,
        );

        self.investment_account_fields.reset();
        self.investment_account_fields[InvestmentAccountField::Name] = name;
        self.investment_account_fields[InvestmentAccountField::Kind] = kind;
        self.investment_account_fields[InvestmentAccountField::Status] = if archived {
            STATUS_ARCHIVED.to_string()
        } else {
            STATUS_ACTIVE.to_string()
        };
        self.editing_investment_account_id = Some(id);
        self.investment_account_cursor = self.investment_account_fields.focused_value().len();
        self.mode = AppMode::InvestmentAccountEditor;
        self.clear_status_message();
    }

    pub(crate) fn cancel_investment_account_editor(&mut self) {
        self.editing_investment_account_id = None;
        self.investment_account_fields.reset();
        self.investment_account_cursor = 0;
        self.mode = self.investment_return_mode();
        self.clear_status_message();
    }

    pub(crate) fn investment_opening_fields_active(&self) -> bool {
        self.editing_investment_account_id.is_none()
    }

    pub(crate) fn next_investment_account_field(&mut self) {
        self.investment_account_fields.focus_next();
        self.investment_account_cursor = self.investment_account_fields.focused_value().len();
    }

    pub(crate) fn previous_investment_account_field(&mut self) {
        self.investment_account_fields.focus_previous();
        self.investment_account_cursor = self.investment_account_fields.focused_value().len();
    }

    pub(crate) fn toggle_investment_status(&mut self) {
        let field = &mut self.investment_account_fields[InvestmentAccountField::Status];
        *field = if field == STATUS_ARCHIVED {
            STATUS_ACTIVE.to_string()
        } else {
            STATUS_ARCHIVED.to_string()
        };
    }

    pub(crate) fn save_investment_account(&mut self) {
        let name = self.investment_account_fields[InvestmentAccountField::Name]
            .trim()
            .to_string();
        if name.is_empty() {
            self.set_status_message("Error: Account name cannot be empty", None);
            return;
        }

        let draft = InvestmentAccountDraft {
            name: name.clone(),
            kind: self.investment_account_fields[InvestmentAccountField::Kind]
                .trim()
                .to_string(),
            archived: self.investment_account_fields[InvestmentAccountField::Status]
                == STATUS_ARCHIVED,
        };

        let opening = match self.parse_opening_position() {
            Ok(opening) => opening,
            Err(message) => {
                self.set_status_message(format!("Error: {}", message), None);
                return;
            }
        };

        let store = self.investment_store();
        let saved_id = match self.editing_investment_account_id {
            Some(id) => store.update_account(id, &draft).map(|_| id),
            None => store.create_account(&draft),
        };

        let account_id = match saved_id {
            Ok(id) => id,
            Err(err) => {
                self.set_status_message(format!("Error saving account: {}", err), None);
                return;
            }
        };

        let opening_error = opening.and_then(|(date, value, invested)| {
            let mut entries = Vec::new();
            // Nothing contributed is a real position: a gift, a match, something inherited.
            if invested > Decimal::ZERO {
                entries.push((InvestmentEntryKind::Contribution, invested));
            }
            entries.push((InvestmentEntryKind::Valuation, value));

            entries
                .into_iter()
                .try_for_each(|(entry_kind, amount)| {
                    store
                        .save_entry(&InvestmentEntryDraft {
                            account_id,
                            date,
                            entry_kind,
                            amount,
                            note: "Opening position".to_string(),
                        })
                        .map(|_| ())
                })
                .err()
        });

        let was_edit = self.editing_investment_account_id.is_some();
        self.editing_investment_account_id = None;
        self.investment_account_fields.reset();
        self.investment_account_cursor = 0;
        self.mode = self.investment_return_mode();

        let message = match (&opening_error, was_edit) {
            (Some(err), _) => format!("Account saved, but the opening position failed: {}", err),
            (None, true) => format!("Account '{}' updated.", name),
            (None, false) => format!("Account '{}' added.", name),
        };
        self.finish_investment_write(Some(account_id), message);
        if opening_error.is_some() {
            // A failed opening position needs to stay on screen.
            self.status_expiry = None;
        }
    }

    fn parse_opening_position(&self) -> Result<Option<(NaiveDate, Decimal, Decimal)>, String> {
        if !self.investment_opening_fields_active() {
            return Ok(None);
        }

        let value_str = self.investment_account_fields[InvestmentAccountField::OpeningValue].trim();
        let invested_str =
            self.investment_account_fields[InvestmentAccountField::OpeningInvested].trim();
        if value_str.is_empty() {
            if !invested_str.is_empty() {
                return Err("Set a starting value to go with the contributed amount".to_string());
            }
            return Ok(None);
        }

        let value = crate::validation::validate_non_negative_amount_string(value_str)?;
        let invested = if invested_str.is_empty() {
            value
        } else {
            crate::validation::validate_non_negative_amount_string(invested_str)?
        };

        let date_str = self.investment_account_fields[InvestmentAccountField::OpeningDate].trim();
        let date = NaiveDate::parse_from_str(date_str, DATE_FORMAT)
            .map_err(|_| format!("Invalid As Of date (expected {})", DATE_FORMAT))?;

        Ok(Some((date, value, invested)))
    }

    // --- Entry editor ---

    pub(crate) fn start_recording_valuation(&mut self) {
        self.open_entry_editor(InvestmentEntryKind::Valuation);
    }

    pub(crate) fn start_adding_investment_entry(&mut self) {
        self.open_entry_editor(InvestmentEntryKind::Contribution);
    }

    fn open_entry_editor(&mut self, kind: InvestmentEntryKind) {
        if self.active_investment_account_id().is_none() {
            self.set_status_message("Add an investment account first (press a).", None);
            return;
        }

        self.investment_entry_fields.reset();
        self.investment_entry_fields[InvestmentEntryField::Date] =
            self.today().format(DATE_FORMAT).to_string();
        self.investment_entry_fields[InvestmentEntryField::EntryKind] = kind.as_str().to_string();
        self.investment_entry_fields
            .focus(InvestmentEntryField::Amount);
        self.editing_investment_entry_id = None;
        self.investment_entry_cursor = 0;
        self.mode = AppMode::InvestmentEntryEditor;
        self.clear_status_message();
    }

    pub(crate) fn start_editing_investment_entry(&mut self) {
        let Some(entry) = self.selected_investment_entry_id().and_then(|id| {
            self.investment_detail_id
                .and_then(|account| self.portfolio.entries_for(account).find(|e| e.id == id))
        }) else {
            return;
        };

        let (id, date, kind, amount, note) = (
            entry.id,
            entry.date,
            entry.entry_kind,
            entry.amount,
            entry.note.clone(),
        );

        self.investment_entry_fields.reset();
        self.investment_entry_fields[InvestmentEntryField::Date] =
            date.format(DATE_FORMAT).to_string();
        self.investment_entry_fields[InvestmentEntryField::EntryKind] = kind.as_str().to_string();
        self.investment_entry_fields[InvestmentEntryField::Amount] = amount.normalize().to_string();
        self.investment_entry_fields[InvestmentEntryField::Note] = note;
        self.investment_entry_fields
            .focus(InvestmentEntryField::Amount);
        self.editing_investment_entry_id = Some(id);
        self.investment_entry_cursor = self.investment_entry_fields.focused_value().len();
        self.mode = AppMode::InvestmentEntryEditor;
        self.clear_status_message();
    }

    pub(crate) fn cancel_investment_entry_editor(&mut self) {
        self.editing_investment_entry_id = None;
        self.investment_entry_fields.reset();
        self.investment_entry_cursor = 0;
        self.mode = self.investment_return_mode();
        self.clear_status_message();
    }

    pub(crate) fn next_investment_entry_field(&mut self) {
        self.investment_entry_fields.focus_next();
        self.investment_entry_cursor = self.investment_entry_fields.focused_value().len();
    }

    pub(crate) fn previous_investment_entry_field(&mut self) {
        self.investment_entry_fields.focus_previous();
        self.investment_entry_cursor = self.investment_entry_fields.focused_value().len();
    }

    pub(crate) fn cycle_investment_entry_kind(&mut self, forward: bool) {
        let all = InvestmentEntryKind::all();
        let current = InvestmentEntryKind::from_label(
            &self.investment_entry_fields[InvestmentEntryField::EntryKind],
        )
        .unwrap_or(InvestmentEntryKind::Valuation);
        let index = all.iter().position(|kind| *kind == current).unwrap_or(0);
        let next = if forward {
            (index + 1) % all.len()
        } else {
            (index + all.len() - 1) % all.len()
        };
        self.investment_entry_fields[InvestmentEntryField::EntryKind] =
            all[next].as_str().to_string();
    }

    pub(crate) fn save_investment_entry(&mut self) {
        let Some(account_id) = self.active_investment_account_id() else {
            self.set_status_message("Error: No investment account selected", None);
            return;
        };

        let kind = InvestmentEntryKind::from_label(
            &self.investment_entry_fields[InvestmentEntryField::EntryKind],
        )
        .unwrap_or(InvestmentEntryKind::Valuation);

        let date_str = self.investment_entry_fields[InvestmentEntryField::Date].trim();
        let date = match NaiveDate::parse_from_str(date_str, DATE_FORMAT) {
            Ok(date) => date,
            Err(_) => {
                self.set_status_message(
                    format!("Error: Invalid Date Format (Expected {})", DATE_FORMAT),
                    None,
                );
                return;
            }
        };

        let amount_str = self.investment_entry_fields[InvestmentEntryField::Amount].trim();
        let parsed = match kind {
            InvestmentEntryKind::Valuation => {
                crate::validation::validate_non_negative_amount_string(amount_str)
            }
            _ => crate::validation::validate_amount_string(amount_str),
        };
        let amount = match parsed {
            Ok(amount) => amount,
            Err(message) => {
                self.set_status_message(format!("Error: {}", message), None);
                return;
            }
        };

        let draft = InvestmentEntryDraft {
            account_id,
            date,
            entry_kind: kind,
            amount,
            note: self.investment_entry_fields[InvestmentEntryField::Note]
                .trim()
                .to_string(),
        };

        let store = self.investment_store();
        let result = match self.editing_investment_entry_id {
            Some(id) => store.update_entry(id, &draft).map(|_| id),
            None => store.save_entry(&draft),
        };

        if let Err(err) = result {
            self.set_status_message(format!("Error saving entry: {}", err), None);
            return;
        }

        let was_edit = self.editing_investment_entry_id.is_some();
        self.editing_investment_entry_id = None;
        self.investment_entry_fields.reset();
        self.investment_entry_cursor = 0;
        self.mode = self.investment_return_mode();
        self.finish_investment_write(
            None,
            if was_edit {
                format!("{} updated.", kind.as_str())
            } else {
                format!("{} recorded.", kind.as_str())
            },
        );
    }

    // --- Deleting ---

    pub(crate) fn prepare_delete_investment(&mut self) {
        let target = match self.mode {
            AppMode::InvestmentDetail => self
                .selected_investment_entry_id()
                .map(InvestmentDeleteTarget::Entry),
            _ => self
                .selected_investment_account_id()
                .map(InvestmentDeleteTarget::Account),
        };

        let Some(target) = target else {
            return;
        };

        self.investment_delete_prompt = match target {
            InvestmentDeleteTarget::Account(id) => {
                let name = self
                    .portfolio
                    .account(id)
                    .map(|account| account.name.clone())
                    .unwrap_or_default();
                let count = self.portfolio.entries_for(id).count();
                format!(
                    "Delete '{}' and its {} entr{}? (y/n)",
                    name,
                    count,
                    if count == 1 { "y" } else { "ies" }
                )
            }
            InvestmentDeleteTarget::Entry(_) => "Delete selected entry? (y/n)".to_string(),
        };
        self.investment_delete_target = Some(target);
        self.mode = AppMode::ConfirmInvestmentDelete;
    }

    pub(crate) fn cancel_delete_investment(&mut self) {
        self.investment_delete_target = None;
        self.mode = self.investment_return_mode();
        self.clear_status_message();
    }

    pub(crate) fn confirm_delete_investment(&mut self) {
        let Some(target) = self.investment_delete_target else {
            self.cancel_delete_investment();
            return;
        };

        let store = self.investment_store();
        let (result, message) = match target {
            InvestmentDeleteTarget::Account(id) => {
                (store.delete_account(id), "Account deleted.".to_string())
            }
            InvestmentDeleteTarget::Entry(id) => {
                (store.delete_entry(id), "Entry deleted.".to_string())
            }
        };

        // Leave the dialog first, so no error path strands it on a deleted id.
        self.investment_delete_target = None;
        if let InvestmentDeleteTarget::Account(id) = target
            && self.investment_detail_id == Some(id)
        {
            self.investment_detail_id = None;
        }
        self.mode = self.investment_return_mode();

        if let Err(err) = result {
            self.set_status_message(format!("Error deleting: {}", err), None);
            return;
        }
        self.finish_investment_write(None, message);
    }

    // --- Shared plumbing ---

    fn investment_return_mode(&self) -> AppMode {
        if self.investment_detail_id.is_some() {
            AppMode::InvestmentDetail
        } else {
            AppMode::Investments
        }
    }

    fn finish_investment_write(&mut self, select: Option<i64>, message: String) {
        if let Err(err) = self.reload_portfolio() {
            self.set_status_message(format!("Saved, but reloading failed: {}", err), None);
            return;
        }

        if let Some(id) = select
            && let Some(index) = self.visible_investment_ids().iter().position(|v| *v == id)
        {
            self.investment_table_state.select(Some(index));
        }
        self.clamp_investment_selection();
        self.clamp_investment_range();

        let entries = self.detail_entry_ids().len();
        if entries == 0 {
            self.investment_entry_table_state.select(None);
        } else {
            let index = self
                .investment_entry_table_state
                .selected()
                .unwrap_or(0)
                .min(entries - 1);
            self.investment_entry_table_state.select(Some(index));
        }

        self.set_status_message(message, Some(Duration::seconds(3)));
    }
}
