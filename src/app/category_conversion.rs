use super::state::{App, AppMode};
use crate::db::transaction_store::TransactionStore;
use crate::model::{CategoryRecord, InvestmentEntry, Transaction, TransactionType};
use chrono::Duration;
use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub struct CategoryConversion {
    pub record: CategoryRecord,
    pub account_id: Option<i64>,
    pub transaction_ids: Vec<i64>,
    pub duplicate_entries: Vec<i64>,
    pub prompt: String,
}

fn uses_record(tx: &Transaction, record: &CategoryRecord) -> bool {
    tx.transaction_type == record.transaction_type
        && tx
            .category
            .trim()
            .eq_ignore_ascii_case(record.category.trim())
        && tx
            .subcategory
            .trim()
            .eq_ignore_ascii_case(record.subcategory.trim())
}

fn record_label(record: &CategoryRecord) -> String {
    if record.subcategory.is_empty() {
        record.category.clone()
    } else {
        format!("{} / {}", record.category, record.subcategory)
    }
}

fn as_transfer(tx: &Transaction, other: i64) -> Transaction {
    let (from, to) = match tx.transaction_type {
        TransactionType::Income => (other, tx.account_id),
        TransactionType::Expense | TransactionType::Transfer => (tx.account_id, other),
    };
    Transaction {
        transaction_type: TransactionType::Transfer,
        account_id: from,
        to_account_id: Some(to),
        ..tx.clone()
    }
}

impl App {
    pub(crate) fn suggests_conversion(&self) -> bool {
        let Some(record) = self.selected_category_record() else {
            return false;
        };
        record.transaction_type != TransactionType::Transfer
            && self.category_records.iter().any(|other| {
                other.transaction_type == TransactionType::Transfer
                    && other
                        .category
                        .trim()
                        .eq_ignore_ascii_case(record.category.trim())
            })
            && self.transactions.iter().any(|tx| uses_record(tx, record))
    }

    pub(crate) fn start_category_conversion(&mut self) {
        let Some(record) = self.selected_category_record().cloned() else {
            self.set_status_message("Select a category first.", None);
            return;
        };
        if record.transaction_type == TransactionType::Transfer {
            self.set_status_message("This is already a transfer category.", None);
            return;
        }
        if !self.transactions.iter().any(|tx| uses_record(tx, &record)) {
            self.set_status_message(
                format!(
                    "No transactions in this ledger use {}.",
                    record_label(&record)
                ),
                None,
            );
            return;
        }

        let options: Vec<String> = self
            .accounts
            .all()
            .iter()
            .filter(|account| !account.archived)
            .map(|account| account.name.clone())
            .collect();

        self.category_conversion = Some(CategoryConversion {
            record,
            account_id: None,
            transaction_ids: Vec::new(),
            duplicate_entries: Vec::new(),
            prompt: String::new(),
        });
        self.type_to_select.clear();
        self.current_selection_list = options;
        self.selection_list_state = ListState::default();
        self.selection_list_state.select(Some(0));
        self.mode = AppMode::SelectingConversionAccount;
        self.clear_status_message();
    }

    pub(crate) fn choose_conversion_account(&mut self) {
        let Some((other_id, other_name)) = self
            .selection_list_state
            .selected()
            .and_then(|index| self.current_selection_list.get(index))
            .and_then(|name| self.accounts.named(name))
            .map(|account| (account.id, account.name.clone()))
        else {
            self.cancel_category_conversion();
            return;
        };
        let Some(conversion) = self.category_conversion.as_mut() else {
            self.cancel_category_conversion();
            return;
        };

        let record = &conversion.record;
        let sources: Vec<&Transaction> = self
            .transactions
            .iter()
            .filter(|tx| !tx.is_generated_from_recurring && uses_record(tx, record))
            .collect();
        if sources.iter().any(|tx| tx.account_id == other_id) {
            self.set_status_message(
                format!(
                    "Some of these transactions are already in {}. Pick the account the money moved to or from.",
                    other_name
                ),
                None,
            );
            return;
        }
        let recurring = sources.iter().filter(|tx| tx.is_recurring).count();
        conversion.transaction_ids = sources.iter().filter_map(|tx| tx.id).collect();

        let today = chrono::Local::now().date_naive();
        let flows: Vec<InvestmentEntry> = self
            .transactions
            .iter()
            .filter(|tx| tx.date <= today && uses_record(tx, record))
            .flat_map(|tx| InvestmentEntry::transfer_flows(&as_transfer(tx, other_id)))
            .filter(|flow| {
                flow.account_id == other_id && self.portfolio.counts_transfer(other_id, flow.date)
            })
            .collect();
        conversion.duplicate_entries = self.portfolio.duplicate_entries(other_id, &flows);
        conversion.account_id = Some(other_id);

        let count = conversion.transaction_ids.len();
        let direction = match record.transaction_type {
            TransactionType::Income => "from",
            TransactionType::Expense | TransactionType::Transfer => "to",
        };
        let mut prompt = format!(
            "Turn {} transaction{}{} in {} into transfers {} {}? They keep the category.",
            count,
            if count == 1 { "" } else { "s" },
            match recurring {
                0 => String::new(),
                n => format!(" ({} recurring)", n),
            },
            record_label(record),
            direction,
            other_name
        );
        let duplicates = conversion.duplicate_entries.len();
        if duplicates > 0 {
            prompt.push_str(&format!(
                " {} matching manual entr{} on {} will be removed so nothing counts twice.",
                duplicates,
                if duplicates == 1 { "y" } else { "ies" },
                other_name
            ));
        }
        prompt.push_str(" (y/n)");
        conversion.prompt = prompt;

        self.current_selection_list.clear();
        self.mode = AppMode::ConfirmCategoryConversion;
    }

    pub(crate) fn cancel_category_conversion(&mut self) {
        self.category_conversion = None;
        self.current_selection_list.clear();
        self.mode = AppMode::CategoryCatalog;
        self.clear_status_message();
    }

    pub(crate) fn confirm_category_conversion(&mut self) {
        let Some(conversion) = self.category_conversion.take() else {
            self.cancel_category_conversion();
            return;
        };
        let Some(account_id) = conversion.account_id else {
            self.cancel_category_conversion();
            return;
        };
        self.mode = AppMode::CategoryCatalog;

        let record = &conversion.record;
        let result = self
            .transaction_store()
            .convert_to_transfers(
                &conversion.transaction_ids,
                account_id,
                &record.category,
                &record.subcategory,
                &conversion.duplicate_entries,
            )
            .and_then(|_| self.reload_transactions_from_db())
            .and_then(|_| self.reload_categories_from_store());
        if let Err(err) = result {
            self.set_status_message(format!("Error converting: {}", err), None);
            return;
        }

        let count = conversion.transaction_ids.len();
        self.set_status_message(
            format!(
                "Converted {} transaction{} to transfers. The {} category stays until you delete it.",
                count,
                if count == 1 { "" } else { "s" },
                record.transaction_type.as_str().to_lowercase()
            ),
            Some(Duration::seconds(5)),
        );
    }
}
