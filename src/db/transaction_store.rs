use crate::db::account_store::account_class;
use crate::db::database::SqliteDatabase;
use crate::model::{
    DATE_FORMAT, RecurrenceFrequency, Transaction, TransactionDraft, TransactionType,
};
use chrono::NaiveDate;
use rusqlite::{Connection, Error as SqlError, Row, params, types::Type};
use rust_decimal::Decimal;
use std::io::{Error, ErrorKind, Result};
use std::str::FromStr;

/// Outcome of a merge-dedupe import.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub added: usize,
    pub skipped: usize,
}

/// Persistence for transactions. Only **real** rows are stored (regular transactions and
/// recurring sources); generated occurrences are derived in-memory and never written here.
/// Every method is scoped to the single ledger the store was built for.
pub trait TransactionStore {
    fn list(&self) -> Result<Vec<Transaction>>;
    fn insert(&self, draft: &TransactionDraft) -> Result<i64>;
    fn update(&self, id: i64, draft: &TransactionDraft) -> Result<()>;
    fn delete(&self, id: i64) -> Result<()>;
    /// Insert every row that is not already present (matched on its natural key). Runs in a
    /// single transaction; duplicates within the batch are skipped too.
    fn import_merge(&self, rows: &[Transaction]) -> Result<ImportSummary>;
    /// Convert expenses to transfers out and income to transfers in, using `other_account_id`.
    /// Remove matching manual investment entries in the same transaction.
    fn convert_to_transfers(
        &self,
        ids: &[i64],
        other_account_id: i64,
        category: &str,
        subcategory: &str,
        duplicate_entries: &[i64],
    ) -> Result<()>;
}

pub struct SqliteTransactionStore {
    database: SqliteDatabase,
    ledger_id: i64,
}

impl SqliteTransactionStore {
    pub fn new(database: SqliteDatabase, ledger_id: i64) -> Self {
        Self {
            database,
            ledger_id,
        }
    }

    fn ready_connection(&self) -> Result<Connection> {
        self.database.ready_connection("transaction")
    }

    fn row_to_transaction(row: &Row<'_>) -> rusqlite::Result<Transaction> {
        let id: i64 = row.get(0)?;
        let date = parse_date(1, &row.get::<_, String>(1)?)?;
        let amount = parse_decimal(3, &row.get::<_, String>(3)?)?;
        let transaction_type = parse_transaction_type(4, &row.get::<_, String>(4)?)?;
        let is_recurring: i64 = row.get(7)?;
        let recurrence_frequency = row
            .get::<_, Option<String>>(8)?
            .and_then(|label| RecurrenceFrequency::from_label(&label));
        let recurrence_end_date = match row.get::<_, Option<String>>(9)? {
            Some(value) if !value.trim().is_empty() => Some(parse_date(9, value.trim())?),
            _ => None,
        };

        Ok(Transaction {
            date,
            description: row.get(2)?,
            amount,
            transaction_type,
            category: row.get(5)?,
            subcategory: row.get(6)?,
            is_recurring: is_recurring != 0,
            recurrence_frequency,
            recurrence_end_date,
            is_generated_from_recurring: false,
            id: Some(id),
            parent_id: None,
            account_id: row.get(10)?,
            to_account_id: row.get(11)?,
        })
    }

    fn check_accounts(conn: &Connection, ledger_id: i64, draft: &TransactionDraft) -> Result<()> {
        let class = account_class(conn, ledger_id, draft.account_id)?;
        match (draft.transaction_type, draft.to_account_id) {
            (TransactionType::Transfer, None) => Err(Error::new(
                ErrorKind::InvalidInput,
                "A transfer needs an account to go to.",
            )),
            (TransactionType::Transfer, Some(to)) if to == draft.account_id => Err(Error::new(
                ErrorKind::InvalidInput,
                "A transfer has to go to a different account.",
            )),
            (TransactionType::Transfer, Some(to)) => account_class(conn, ledger_id, to).map(|_| ()),
            (_, Some(_)) => Err(Error::new(
                ErrorKind::InvalidInput,
                "Only transfers go to a second account.",
            )),
            (_, None) if !class.holds_spending() => Err(Error::new(
                ErrorKind::InvalidInput,
                "Income and expenses belong in a cash or credit account. Use a transfer to move money into an investment.",
            )),
            (_, None) => Ok(()),
        }
    }

    fn insert_with_conn(
        conn: &Connection,
        ledger_id: i64,
        draft: &TransactionDraft,
    ) -> Result<i64> {
        Self::check_accounts(conn, ledger_id, draft)?;
        conn.execute(
            "
            INSERT INTO transactions (
                ledger_id,
                date,
                description,
                amount,
                transaction_type,
                category,
                subcategory,
                is_recurring,
                recurrence_frequency,
                recurrence_end_date,
                account_id,
                to_account_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ",
            params![
                ledger_id,
                draft.date.format(DATE_FORMAT).to_string(),
                &draft.description,
                draft.amount.normalize().to_string(),
                draft.transaction_type.as_str(),
                &draft.category,
                &draft.subcategory,
                draft.is_recurring as i64,
                draft.recurrence_frequency.map(|freq| freq.to_string()),
                draft
                    .recurrence_end_date
                    .map(|date| date.format(DATE_FORMAT).to_string()),
                draft.account_id,
                draft.to_account_id,
            ],
        )
        .map_err(|err| Error::other(format!("Failed to insert transaction: {}", err)))?;

        Ok(conn.last_insert_rowid())
    }

    /// Does a row with the same natural key already exist? Amounts are compared in their
    /// canonical `Decimal` string form so "10" and "10.00" are treated as equal.
    fn natural_key_exists(conn: &Connection, ledger_id: i64, tx: &Transaction) -> Result<bool> {
        conn.query_row(
            "
            SELECT 1 FROM transactions
            WHERE ledger_id = ?1
              AND date = ?2
              AND description = ?3
              AND amount = ?4
              AND transaction_type = ?5
              AND category = ?6
              AND subcategory = ?7
              AND account_id = ?8
              AND to_account_id IS ?9
            LIMIT 1
            ",
            params![
                ledger_id,
                tx.date.format(DATE_FORMAT).to_string(),
                &tx.description,
                tx.amount.normalize().to_string(),
                tx.transaction_type.as_str(),
                &tx.category,
                &tx.subcategory,
                tx.account_id,
                tx.to_account_id,
            ],
            |_| Ok(()),
        )
        .map(|_| true)
        .or_else(|err| match err {
            SqlError::QueryReturnedNoRows => Ok(false),
            other => Err(Error::other(format!(
                "Failed to check for existing transaction: {}",
                other
            ))),
        })
    }
}

impl TransactionStore for SqliteTransactionStore {
    fn list(&self) -> Result<Vec<Transaction>> {
        let conn = self.ready_connection()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, date, description, amount, transaction_type, category, subcategory,
                       is_recurring, recurrence_frequency, recurrence_end_date, account_id,
                       to_account_id
                FROM transactions
                WHERE ledger_id = ?1
                ORDER BY date, id
                ",
            )
            .map_err(|err| Error::other(format!("Failed to prepare transaction query: {}", err)))?;

        let rows = stmt
            .query_map([self.ledger_id], Self::row_to_transaction)
            .map_err(|err| Error::other(format!("Failed to load transactions: {}", err)))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| Error::other(format!("Failed to read transactions: {}", err)))
    }

    fn insert(&self, draft: &TransactionDraft) -> Result<i64> {
        let conn = self.ready_connection()?;
        Self::insert_with_conn(&conn, self.ledger_id, draft)
    }

    fn update(&self, id: i64, draft: &TransactionDraft) -> Result<()> {
        let conn = self.ready_connection()?;
        Self::check_accounts(&conn, self.ledger_id, draft)?;
        let updated = conn
            .execute(
                "
                UPDATE transactions
                SET
                    date = ?1,
                    description = ?2,
                    amount = ?3,
                    transaction_type = ?4,
                    category = ?5,
                    subcategory = ?6,
                    is_recurring = ?7,
                    recurrence_frequency = ?8,
                    recurrence_end_date = ?9,
                    account_id = ?10,
                    to_account_id = ?11
                WHERE id = ?12 AND ledger_id = ?13
                ",
                params![
                    draft.date.format(DATE_FORMAT).to_string(),
                    &draft.description,
                    draft.amount.normalize().to_string(),
                    draft.transaction_type.as_str(),
                    &draft.category,
                    &draft.subcategory,
                    draft.is_recurring as i64,
                    draft.recurrence_frequency.map(|freq| freq.to_string()),
                    draft
                        .recurrence_end_date
                        .map(|date| date.format(DATE_FORMAT).to_string()),
                    draft.account_id,
                    draft.to_account_id,
                    id,
                    self.ledger_id,
                ],
            )
            .map_err(|err| Error::other(format!("Failed to update transaction: {}", err)))?;

        if updated == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Transaction with id {} was not found.", id),
            ));
        }
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        let conn = self.ready_connection()?;
        let deleted = conn
            .execute(
                "DELETE FROM transactions WHERE id = ?1 AND ledger_id = ?2",
                [id, self.ledger_id],
            )
            .map_err(|err| Error::other(format!("Failed to delete transaction: {}", err)))?;

        if deleted == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Transaction with id {} was not found.", id),
            ));
        }
        Ok(())
    }

    fn import_merge(&self, rows: &[Transaction]) -> Result<ImportSummary> {
        let mut conn = self.ready_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin import: {}", err)))?;

        // Insert oldest first so auto-increment ids line up with chronological order
        // (otherwise a newest-first CSV would give the most recent row the lowest id).
        let mut ordered: Vec<&Transaction> = rows.iter().collect();
        ordered.sort_by_key(|row| row.date);

        let mut summary = ImportSummary::default();
        for row in ordered {
            if Self::natural_key_exists(&tx, self.ledger_id, row)? {
                summary.skipped += 1;
            } else {
                Self::insert_with_conn(&tx, self.ledger_id, &row.to_draft())?;
                summary.added += 1;
            }
        }

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit import: {}", err)))?;
        Ok(summary)
    }

    fn convert_to_transfers(
        &self,
        ids: &[i64],
        other_account_id: i64,
        category: &str,
        subcategory: &str,
        duplicate_entries: &[i64],
    ) -> Result<()> {
        let mut conn = self.ready_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin conversion: {}", err)))?;

        // Assignments read the original row, so swapping the income accounts is safe.
        for &id in ids {
            let converted = tx
                .execute(
                    "
                    UPDATE transactions
                    SET to_account_id = CASE transaction_type
                            WHEN 'Expense' THEN ?1 ELSE account_id END,
                        account_id = CASE transaction_type
                            WHEN 'Expense' THEN account_id ELSE ?1 END,
                        transaction_type = 'Transfer',
                        category = ?4,
                        subcategory = ?5
                    WHERE id = ?2
                      AND ledger_id = ?3
                      AND transaction_type IN ('Income', 'Expense')
                      AND account_id != ?1
                      AND EXISTS (SELECT 1 FROM accounts WHERE id = ?1 AND ledger_id = ?3)
                    ",
                    params![other_account_id, id, self.ledger_id, category, subcategory],
                )
                .map_err(|err| Error::other(format!("Failed to convert transaction: {}", err)))?;
            if converted == 0 {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("Transaction {} could not be converted.", id),
                ));
            }
        }

        tx.execute(
            "
            INSERT INTO categories (transaction_type, category, subcategory)
            SELECT 'Transfer', ?1, ?2
            WHERE NOT EXISTS (
                SELECT 1 FROM categories
                WHERE transaction_type = 'Transfer'
                  AND LOWER(category) = LOWER(?1)
                  AND LOWER(subcategory) = LOWER(?2)
            )
            ",
            params![category, subcategory],
        )
        .map_err(|err| Error::other(format!("Failed to add transfer category: {}", err)))?;

        for &id in duplicate_entries {
            tx.execute(
                "
                DELETE FROM investment_entries
                WHERE id = ?1 AND account_id = ?2 AND entry_kind != 'Valuation'
                ",
                params![id, other_account_id],
            )
            .map_err(|err| Error::other(format!("Failed to remove duplicate entry: {}", err)))?;
        }

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit conversion: {}", err)))
    }
}

fn parse_date(index: usize, value: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(value, DATE_FORMAT).map_err(|err| {
        SqlError::FromSqlConversionFailure(
            index,
            Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid date '{}' in transaction database: {}", value, err),
            )),
        )
    })
}

fn parse_decimal(index: usize, value: &str) -> rusqlite::Result<Decimal> {
    Decimal::from_str(value.trim()).map_err(|err| {
        SqlError::FromSqlConversionFailure(
            index,
            Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Invalid amount '{}' in transaction database: {}",
                    value, err
                ),
            )),
        )
    })
}

fn parse_transaction_type(index: usize, value: &str) -> rusqlite::Result<TransactionType> {
    TransactionType::try_from(value).map_err(|_| {
        SqlError::FromSqlConversionFailure(
            index,
            Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Invalid transaction type '{}' in transaction database.",
                    value
                ),
            )),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::account_store::{AccountStore, SqliteAccountStore};
    use crate::db::backup::{self, BackupKind};
    use crate::db::category_store::CategoryStore;
    use crate::db::database::SCHEMA_VERSION;
    use crate::db::ledger_store::{DEFAULT_LEDGER_ID, LedgerStore, SqliteLedgerStore};
    use crate::model::{
        AccountClass, AccountDraft, Accounts, BudgetSchedule, InvestmentEntry,
        InvestmentEntryDraft, InvestmentEntryKind, MonthlySummary, Portfolio,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A temporary on-disk database that deletes itself (and its sidecar files) when dropped.
    struct TempDb {
        path: PathBuf,
    }

    impl TempDb {
        fn new() -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "budget_tracker_test_{}_{}_{}.db",
                std::process::id(),
                nanos,
                unique
            ));
            Self { path }
        }

        fn store(&self) -> SqliteTransactionStore {
            self.store_for(DEFAULT_LEDGER_ID)
        }

        fn store_for(&self, ledger_id: i64) -> SqliteTransactionStore {
            SqliteTransactionStore::new(SqliteDatabase::new(&self.path), ledger_id)
        }

        fn create_ledger(&self, name: &str) -> i64 {
            SqliteLedgerStore::new(SqliteDatabase::new(&self.path))
                .create(name)
                .unwrap()
                .id
        }

        fn main_account(&self, ledger_id: i64) -> i64 {
            SqliteAccountStore::new(SqliteDatabase::new(&self.path), ledger_id)
                .list_accounts()
                .unwrap()[0]
                .id
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(self.path.with_extension("db-wal"));
            let _ = std::fs::remove_file(self.path.with_extension("db-shm"));
            let _ = std::fs::remove_dir_all(crate::db::backup::backups_dir(&self.path));
        }
    }

    /// Assigned by migration v6 in a fresh database.
    const MAIN_ACCOUNT: i64 = 1;

    fn draft(date: &str, description: &str, amount: &str, category: &str) -> TransactionDraft {
        TransactionDraft {
            date: NaiveDate::parse_from_str(date, DATE_FORMAT).unwrap(),
            description: description.to_string(),
            amount: Decimal::from_str(amount).unwrap(),
            transaction_type: TransactionType::Expense,
            category: category.to_string(),
            subcategory: String::new(),
            is_recurring: false,
            recurrence_frequency: None,
            recurrence_end_date: None,
            account_id: MAIN_ACCOUNT,
            to_account_id: None,
        }
    }

    fn transfer(date: &str, amount: &str, from: i64, to: i64) -> TransactionDraft {
        TransactionDraft {
            transaction_type: TransactionType::Transfer,
            account_id: from,
            to_account_id: Some(to),
            ..draft(date, "Transfer", amount, "Savings")
        }
    }

    fn open_account(temp: &TempDb, ledger_id: i64, name: &str, class: AccountClass) -> i64 {
        investments(temp, ledger_id)
            .create_account(&AccountDraft {
                name: name.to_string(),
                kind: String::new(),
                archived: false,
                class,
                opening_balance: Decimal::ZERO,
                tracked_from: None,
            })
            .unwrap()
    }

    #[test]
    fn backup_round_trips_through_restore() {
        let temp = TempDb::new();
        let store = temp.store();
        store
            .insert(&draft("2026-01-05", "Rent", "1200.00", "Housing"))
            .unwrap();

        let snapshot = backup::create(&temp.path, "device-a", BackupKind::Manual).unwrap();
        assert!(snapshot.path.exists());

        store
            .insert(&draft("2026-02-05", "Oops", "9999.00", "Housing"))
            .unwrap();
        assert_eq!(store.list().unwrap().len(), 2);

        let safety = backup::restore(&temp.path, &snapshot.path, "device-a").unwrap();
        let restored = store.list().unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].description, "Rent");

        let safety = safety.expect("restoring over a real database keeps a safety copy");
        backup::restore(&temp.path, &safety, "device-a").unwrap();
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn pruning_only_touches_this_devices_automatic_backups() {
        let temp = TempDb::new();
        temp.store()
            .insert(&draft("2026-01-05", "Rent", "1200.00", "Housing"))
            .unwrap();

        for _ in 0..3 {
            backup::create(&temp.path, "device-a", BackupKind::Auto).unwrap();
        }
        backup::create(&temp.path, "device-a", BackupKind::Manual).unwrap();
        backup::create(&temp.path, "device-b", BackupKind::Auto).unwrap();

        assert_eq!(backup::prune(&temp.path, "device-a", 1).unwrap(), 2);

        let remaining = backup::list(&temp.path, "device-a").unwrap();
        assert_eq!(remaining.len(), 3);
        let other = remaining
            .iter()
            .find(|entry| !entry.is_this_device)
            .expect("the other device's backup is kept");
        assert_eq!(other.instance, "device-b");
        assert!(
            remaining
                .iter()
                .any(|entry| entry.kind == BackupKind::Manual)
        );
    }

    #[test]
    fn migration_creates_schema_at_latest_version() {
        let temp = TempDb::new();
        // Listing forces the schema/migrations to run.
        assert!(temp.store().list().unwrap().is_empty());

        let conn = SqliteDatabase::new(&temp.path)
            .open_connection("test")
            .unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

        // Migration v3 seeds the ledger that pre-existing transactions are attributed to.
        let (id, name): (i64, String) = conn
            .query_row("SELECT id, name FROM ledgers", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(id, DEFAULT_LEDGER_ID);
        assert_eq!(name, "Main");
    }

    #[test]
    fn upgrading_a_v2_database_keeps_rows_on_the_default_ledger() {
        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);

        // Build a database at the pre-ledger schema, exactly as an existing install has it.
        let mut conn = database.open_connection("test").unwrap();
        conn.execute_batch(
            "
            CREATE TABLE transactions (
                id INTEGER PRIMARY KEY,
                date TEXT NOT NULL,
                description TEXT NOT NULL,
                amount TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'Uncategorized',
                subcategory TEXT NOT NULL DEFAULT '',
                is_recurring INTEGER NOT NULL DEFAULT 0,
                recurrence_frequency TEXT NULL,
                recurrence_end_date TEXT NULL
            );
            INSERT INTO transactions (date, description, amount, transaction_type, category)
            VALUES ('2026-01-05', 'Coffee', '4.50', 'Expense', 'Food');
            PRAGMA user_version = 2;
            ",
        )
        .unwrap();
        database.run_migrations(&mut conn).unwrap();
        drop(conn);

        let rows = temp.store().list().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description, "Coffee");
    }

    #[test]
    fn upgrading_to_v4_moves_category_budgets_into_periods() {
        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let mut conn = database.open_connection("test").unwrap();

        // A v3 database: categories carrying budgets, shared by two ledgers.
        conn.execute_batch(
            "
            CREATE TABLE categories (
                id INTEGER PRIMARY KEY,
                transaction_type TEXT NOT NULL,
                category TEXT NOT NULL,
                subcategory TEXT NOT NULL DEFAULT '',
                tag TEXT NULL,
                target_budget TEXT NULL,
                UNIQUE(transaction_type, category, subcategory)
            );
            INSERT INTO categories (id, transaction_type, category, subcategory, target_budget)
            VALUES (1, 'Expense', 'Food', 'Groceries', '600.00'),
                   (2, 'Expense', 'Fun', 'Dining', NULL);
            CREATE TABLE ledgers (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL COLLATE NOCASE,
                position INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                UNIQUE(name)
            );
            INSERT INTO ledgers (id, name, position, created_at)
            VALUES (1, 'Main', 0, datetime('now')), (2, 'Scenario', 1, datetime('now'));
            PRAGMA user_version = 3;
            ",
        )
        .unwrap();
        database.run_migrations(&mut conn).unwrap();

        // The budgeted category lands on both ledgers, starting before any real month so
        // history keeps the amount it already showed. The unbudgeted one brings nothing.
        let mut stmt = conn
            .prepare(
                "SELECT ledger_id, category_id, start_year, start_month, amount
                 FROM budget_periods ORDER BY ledger_id",
            )
            .unwrap();
        let periods: Vec<(i64, i64, i64, i64, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        drop(stmt);
        assert_eq!(
            periods,
            vec![
                (1, 1, 0, 1, "600.00".to_string()),
                (2, 1, 0, 1, "600.00".to_string()),
            ]
        );

        // The old column is gone, so no stale budget can be read back from it.
        let mut stmt = conn.prepare("PRAGMA table_info(categories)").unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        drop(stmt);
        assert!(!columns.contains(&"target_budget".to_string()));

        // Running the migration again must not duplicate the periods.
        database.run_migrations(&mut conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM budget_periods", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn deleting_a_ledger_or_category_clears_its_budget_periods() {
        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let conn = database.ready_connection("test").unwrap();
        conn.execute_batch(
            "
            INSERT INTO ledgers (id, name, position, created_at)
            VALUES (2, 'Scenario', 1, datetime('now'));
            INSERT INTO categories (id, transaction_type, category, subcategory)
            VALUES (7, 'Expense', 'Food', 'Groceries');
            INSERT INTO budget_periods (ledger_id, category_id, start_year, start_month, amount)
            VALUES (1, NULL, 2026, 3, '2000.00'),
                   (2, NULL, 2026, 3, '3000.00'),
                   (1, 7, 0, 1, '600.00');
            ",
        )
        .unwrap();
        drop(conn);

        let remaining = |database: &SqliteDatabase| -> Vec<(i64, Option<i64>)> {
            let conn = database.ready_connection("test").unwrap();
            let mut stmt = conn
                .prepare("SELECT ledger_id, category_id FROM budget_periods ORDER BY id")
                .unwrap();
            let rows = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            drop(stmt);
            rows
        };

        // Dropping a ledger takes its budgets with it, leaving the other ledger alone.
        SqliteLedgerStore::new(database.clone()).delete(2).unwrap();
        assert_eq!(remaining(&database), vec![(1, None), (1, Some(7))]);

        // Dropping a category clears its budget without touching the monthly budget.
        crate::db::category_store::SqliteCategoryStore::new(database.clone())
            .delete(7)
            .unwrap();
        assert_eq!(remaining(&database), vec![(1, None)]);
    }

    #[test]
    fn budget_periods_round_trip_for_the_target_and_a_category() {
        use crate::db::budget_store::{BudgetStore, SqliteBudgetStore};
        use crate::model::{BudgetMonth, BudgetWrite};

        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let conn = database.ready_connection("test").unwrap();
        conn.execute_batch(
            "
            INSERT INTO categories (id, transaction_type, category, subcategory)
            VALUES (7, 'Expense', 'Food', 'Groceries');
            INSERT INTO ledgers (id, name, position, created_at)
            VALUES (2, 'Scenario', 1, datetime('now'));
            ",
        )
        .unwrap();
        drop(conn);

        let store = SqliteBudgetStore::new(database.clone());
        let jan = BudgetMonth::new(2026, 1);
        let mar = BudgetMonth::new(2026, 3);
        let ledger = DEFAULT_LEDGER_ID;

        // The monthly budget is keyed on a NULL category, which only matches with `IS`.
        store
            .set(ledger, None, jan, Some("2000".parse().unwrap()))
            .unwrap();
        store
            .set(ledger, None, mar, Some("2500".parse().unwrap()))
            .unwrap();
        store
            .set(ledger, Some(7), jan, Some("600".parse().unwrap()))
            .unwrap();

        let schedule = BudgetSchedule::new(store.list(ledger).unwrap());
        assert_eq!(schedule.monthly_budget(jan), Some("2000".parse().unwrap()));
        assert_eq!(
            schedule.monthly_budget(BudgetMonth::new(2026, 2)),
            Some("2000".parse().unwrap())
        );
        assert_eq!(schedule.monthly_budget(mar), Some("2500".parse().unwrap()));
        assert_eq!(
            schedule.monthly_budget(BudgetMonth::new(2026, 4)),
            Some("2500".parse().unwrap())
        );
        assert_eq!(
            schedule.category_budget(7, mar),
            Some("600".parse().unwrap())
        );

        // Replacing a start month must not leave the old row behind.
        store
            .set(ledger, None, mar, Some("2600".parse().unwrap()))
            .unwrap();
        let schedule = BudgetSchedule::new(store.list(ledger).unwrap());
        assert_eq!(schedule.monthly_budget(mar), Some("2600".parse().unwrap()));

        // Clearing from a month is not the same as having no period there.
        store.set(ledger, Some(7), mar, None).unwrap();
        let schedule = BudgetSchedule::new(store.list(ledger).unwrap());
        assert_eq!(
            schedule.category_budget(7, jan),
            Some("600".parse().unwrap())
        );
        assert_eq!(schedule.category_budget(7, mar), None);

        // Removing that period lets March inherit January again.
        store
            .apply(ledger, Some(7), &[BudgetWrite::Remove(mar)])
            .unwrap();
        let schedule = BudgetSchedule::new(store.list(ledger).unwrap());
        assert_eq!(
            schedule.category_budget(7, mar),
            Some("600".parse().unwrap())
        );

        // A copied ledger gets its own rows, unaffected by later edits to the source.
        let copied = SqliteLedgerStore::new(database.clone())
            .copy(ledger, "Copy")
            .unwrap();
        store
            .apply(ledger, None, &[BudgetWrite::Remove(mar)])
            .unwrap();

        let source = BudgetSchedule::new(store.list(ledger).unwrap());
        assert_eq!(source.monthly_budget(mar), Some("2000".parse().unwrap()));
        // Removing the target left the category budget alone, so `IS` matched the NULL key.
        assert_eq!(source.category_budget(7, mar), Some("600".parse().unwrap()));

        let copy = BudgetSchedule::new(store.list(copied.id).unwrap());
        assert_eq!(copy.monthly_budget(mar), Some("2600".parse().unwrap()));

        // A replace plan is two writes; both have to land or history would be lost.
        store
            .apply(
                ledger,
                None,
                &[
                    BudgetWrite::RemoveAll,
                    BudgetWrite::Set(BudgetMonth::BEGINNING, Some("1800".parse().unwrap())),
                ],
            )
            .unwrap();
        let schedule = BudgetSchedule::new(store.list(ledger).unwrap());
        assert_eq!(schedule.monthly_budget(jan), Some("1800".parse().unwrap()));
        assert_eq!(
            schedule.monthly_budget(BudgetMonth::new(2030, 1)),
            Some("1800".parse().unwrap())
        );

        assert_eq!(schedule.years(), vec![2026]);
    }

    #[test]
    fn edit_scopes_resolve_to_the_right_writes() {
        use crate::model::{BudgetEditScope, BudgetMonth, BudgetPeriod, BudgetWrite};

        let amount = |v: &str| Some(v.parse::<rust_decimal::Decimal>().unwrap());
        let period = |id, start, value: Option<&str>| BudgetPeriod {
            id,
            category_id: None,
            start,
            amount: value.map(|v| v.parse().unwrap()),
        };
        let jan = BudgetMonth::new(2026, 1);
        let mar = BudgetMonth::new(2026, 3);
        let dec = BudgetMonth::new(2026, 12);
        let schedule = BudgetSchedule::new(vec![
            period(1, jan, Some("2000")),
            period(2, mar, Some("2500")),
        ]);

        assert_eq!(
            schedule.plan_edit(None, mar, amount("3000"), BudgetEditScope::FromThisMonth),
            vec![BudgetWrite::Set(mar, amount("3000"))]
        );

        // Only April must be restored to what it inherits today (2500), not to the new value.
        assert_eq!(
            schedule.plan_edit(None, mar, amount("3000"), BudgetEditScope::ThisMonthOnly),
            vec![
                BudgetWrite::Set(mar, amount("3000")),
                BudgetWrite::Set(BudgetMonth::new(2026, 4), amount("2500")),
            ]
        );

        // December rolls into January of the next year rather than month 13.
        assert_eq!(
            schedule.plan_edit(None, dec, amount("100"), BudgetEditScope::ThisMonthOnly),
            vec![
                BudgetWrite::Set(dec, amount("100")),
                BudgetWrite::Set(BudgetMonth::new(2027, 1), amount("2500")),
            ]
        );

        // Replacing wipes history first, so a later period cannot survive and override.
        assert_eq!(
            schedule.plan_edit(None, mar, amount("1800"), BudgetEditScope::ReplaceAllMonths),
            vec![
                BudgetWrite::RemoveAll,
                BudgetWrite::Set(BudgetMonth::BEGINNING, amount("1800")),
            ]
        );

        assert_eq!(
            schedule.plan_edit(None, mar, None, BudgetEditScope::RemoveChange),
            vec![BudgetWrite::Remove(mar)]
        );

        // A scope with no periods of its own is unaffected by the target's.
        assert_eq!(
            schedule.plan_edit(Some(7), mar, amount("600"), BudgetEditScope::ThisMonthOnly),
            vec![
                BudgetWrite::Set(mar, amount("600")),
                BudgetWrite::Set(BudgetMonth::new(2026, 4), None),
            ]
        );
    }

    #[test]
    fn a_recreated_category_does_not_inherit_a_deleted_budget() {
        use crate::db::budget_store::{BudgetStore, SqliteBudgetStore};
        use crate::db::category_store::SqliteCategoryStore;
        use crate::model::{BudgetMonth, CategoryDraft, TransactionType};

        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let categories = SqliteCategoryStore::new(database.clone());
        let budgets = SqliteBudgetStore::new(database.clone());
        let draft = |name: &str| CategoryDraft {
            transaction_type: TransactionType::Expense,
            category: name.to_string(),
            subcategory: String::new(),
            tag: None,
        };

        let created = categories.insert(&draft("Food")).unwrap();
        budgets
            .set(
                DEFAULT_LEDGER_ID,
                Some(created.id),
                BudgetMonth::new(2026, 1),
                Some("600".parse().unwrap()),
            )
            .unwrap();

        categories.delete(created.id).unwrap();
        assert!(budgets.list(DEFAULT_LEDGER_ID).unwrap().is_empty());

        // `categories.id` has no AUTOINCREMENT, so SQLite hands the id straight back.
        let recreated = categories.insert(&draft("Fun")).unwrap();
        assert_eq!(recreated.id, created.id);
        let schedule = BudgetSchedule::new(budgets.list(DEFAULT_LEDGER_ID).unwrap());
        assert_eq!(
            schedule.category_budget(recreated.id, BudgetMonth::new(2026, 6)),
            None
        );
    }

    #[test]
    fn turning_a_category_into_income_drops_its_budgets() {
        use crate::db::budget_store::{BudgetStore, SqliteBudgetStore};
        use crate::db::category_store::SqliteCategoryStore;
        use crate::model::{BudgetMonth, CategoryDraft, TransactionType};

        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let conn = database.ready_connection("test").unwrap();
        conn.execute_batch(
            "INSERT INTO ledgers (id, name, position, created_at)
             VALUES (2, 'Scenario', 1, datetime('now'));",
        )
        .unwrap();
        drop(conn);

        let categories = SqliteCategoryStore::new(database.clone());
        let budgets = SqliteBudgetStore::new(database.clone());
        let mut draft = CategoryDraft {
            transaction_type: TransactionType::Expense,
            category: "Food".to_string(),
            subcategory: String::new(),
            tag: None,
        };
        let record = categories.insert(&draft).unwrap();

        // Budgets are per ledger, but the category type is not, so both must go.
        for ledger in [DEFAULT_LEDGER_ID, 2] {
            budgets
                .set(
                    ledger,
                    Some(record.id),
                    BudgetMonth::new(2026, 1),
                    Some("600".parse().unwrap()),
                )
                .unwrap();
        }

        draft.transaction_type = TransactionType::Income;
        categories.update(record.id, &draft).unwrap();

        for ledger in [DEFAULT_LEDGER_ID, 2] {
            let schedule = BudgetSchedule::new(budgets.list(ledger).unwrap());
            assert_eq!(
                schedule.category_budget(record.id, BudgetMonth::new(2026, 6)),
                None
            );
        }

        // Switching back must not bring back the old amounts.
        draft.transaction_type = TransactionType::Expense;
        categories.update(record.id, &draft).unwrap();
        let schedule = BudgetSchedule::new(budgets.list(DEFAULT_LEDGER_ID).unwrap());
        assert_eq!(
            schedule.category_budget(record.id, BudgetMonth::new(2026, 6)),
            None
        );
    }

    fn investments(temp: &TempDb, ledger_id: i64) -> SqliteAccountStore {
        SqliteAccountStore::new(SqliteDatabase::new(&temp.path), ledger_id)
    }

    fn entry(
        account_id: i64,
        date: &str,
        entry_kind: InvestmentEntryKind,
        amount: &str,
    ) -> InvestmentEntryDraft {
        InvestmentEntryDraft {
            account_id,
            date: NaiveDate::parse_from_str(date, DATE_FORMAT).unwrap(),
            entry_kind,
            amount: Decimal::from_str(amount).unwrap(),
            note: String::new(),
        }
    }

    fn day(date: &str) -> NaiveDate {
        NaiveDate::parse_from_str(date, DATE_FORMAT).unwrap()
    }

    /// The identity the whole investments view rests on: money paid in is never growth.
    #[test]
    fn contributions_are_never_counted_as_investment_growth() {
        let temp = TempDb::new();
        let store = investments(&temp, DEFAULT_LEDGER_ID);
        let account = store
            .create_account(&AccountDraft {
                name: "TFSA".to_string(),
                kind: "Brokerage".to_string(),
                archived: false,
                class: AccountClass::Investment,
                opening_balance: Decimal::ZERO,
                tracked_from: None,
            })
            .unwrap();

        // Opening position, then a deposit, then a fresh valuation.
        store
            .save_entry(&entry(
                account,
                "2024-01-01",
                InvestmentEntryKind::Contribution,
                "10000",
            ))
            .unwrap();
        store
            .save_entry(&entry(
                account,
                "2024-01-01",
                InvestmentEntryKind::Valuation,
                "10000",
            ))
            .unwrap();
        store
            .save_entry(&entry(
                account,
                "2024-07-01",
                InvestmentEntryKind::Contribution,
                "5000",
            ))
            .unwrap();
        store
            .save_entry(&entry(
                account,
                "2025-01-01",
                InvestmentEntryKind::Valuation,
                "17000",
            ))
            .unwrap();

        let portfolio = Portfolio::new(
            store.list_accounts().unwrap(),
            store.list_entries().unwrap(),
        );

        // Between the deposit and the next valuation the money shows up as value, not gain.
        assert_eq!(
            portfolio.value_on(account, day("2024-08-01")),
            Decimal::from(15000)
        );
        assert_eq!(
            portfolio.gain_between(Some(account), day("2024-01-01"), day("2024-08-01"), false),
            Decimal::ZERO
        );

        // Over the full span only the 2,000 the account actually earned counts.
        let position = portfolio.position(account, day("2025-01-01"));
        assert_eq!(position.value, Decimal::from(17000));
        assert_eq!(position.invested, Decimal::from(15000));
        assert_eq!(position.gain(), Decimal::from(2000));
        assert_eq!(
            portfolio.gain_between(Some(account), day("2024-01-01"), day("2025-01-01"), false),
            Decimal::from(2000)
        );
    }

    #[test]
    fn copying_a_ledger_carries_its_investments_and_deleting_one_clears_them() {
        let temp = TempDb::new();
        let source = investments(&temp, DEFAULT_LEDGER_ID);
        let account = source
            .create_account(&AccountDraft {
                name: "Brokerage".to_string(),
                kind: "Taxable".to_string(),
                archived: false,
                class: AccountClass::Investment,
                opening_balance: Decimal::ZERO,
                tracked_from: None,
            })
            .unwrap();
        source
            .save_entry(&entry(
                account,
                "2025-01-01",
                InvestmentEntryKind::Contribution,
                "500",
            ))
            .unwrap();

        temp.store()
            .insert(&transfer("2025-02-01", "100", MAIN_ACCOUNT, account))
            .unwrap();

        let ledger_store = SqliteLedgerStore::new(SqliteDatabase::new(&temp.path));
        let copy = ledger_store.copy(DEFAULT_LEDGER_ID, "Copy").unwrap();

        let copied = investments(&temp, copy.id);
        let copied_accounts = Accounts::new(copied.list_accounts().unwrap());
        assert_eq!(copied_accounts.all().len(), 2);
        let copied_transfer = &temp.store_for(copy.id).list().unwrap()[0];
        assert_eq!(
            copied_transfer.account_id,
            copied_accounts.named("Main Account").unwrap().id
        );
        assert_eq!(
            copied_transfer.to_account_id,
            Some(copied_accounts.named("Brokerage").unwrap().id)
        );
        let copied_entries = copied.list_entries().unwrap();
        assert_eq!(copied_entries.len(), 1);
        assert_eq!(copied_entries[0].amount, Decimal::from(500));
        // The copy stands on its own rows, not the source's.
        assert_ne!(copied_entries[0].account_id, account);

        ledger_store.delete(copy.id).unwrap();
        assert!(copied.list_accounts().unwrap().is_empty());
        assert!(copied.list_entries().unwrap().is_empty());
        // The ledger that was copied from is untouched.
        assert_eq!(source.list_entries().unwrap().len(), 1);
    }

    // Includes an account name conflict and a transaction whose ledger is missing.
    const V5_FIXTURE: &str = "
        CREATE TABLE database_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE categories (
            id INTEGER PRIMARY KEY,
            transaction_type TEXT NOT NULL CHECK (transaction_type IN ('Income', 'Expense')),
            category TEXT NOT NULL,
            subcategory TEXT NOT NULL DEFAULT '',
            tag TEXT NULL,
            UNIQUE(transaction_type, category, subcategory)
        );
        INSERT INTO categories (id, transaction_type, category) VALUES (1, 'Expense', 'Food');
        CREATE TABLE transactions (
            id INTEGER PRIMARY KEY,
            date TEXT NOT NULL,
            description TEXT NOT NULL,
            amount TEXT NOT NULL,
            transaction_type TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'Uncategorized',
            subcategory TEXT NOT NULL DEFAULT '',
            is_recurring INTEGER NOT NULL DEFAULT 0,
            recurrence_frequency TEXT NULL,
            recurrence_end_date TEXT NULL,
            ledger_id INTEGER NOT NULL DEFAULT 1
        );
        INSERT INTO transactions (id, ledger_id, date, description, amount, transaction_type, category)
        VALUES (7, 1, '2026-01-05', 'Coffee', '4.50', 'Expense', 'Food'),
               (9, 1, '2026-01-06', 'Pay', '100', 'Income', 'Salary'),
               (11, 2, '2026-01-07', 'Rent', '900', 'Expense', 'Housing'),
               (13, 5, '2026-01-08', 'Lost', '1', 'Expense', 'Food');
        CREATE TABLE ledgers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL COLLATE NOCASE,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            UNIQUE(name)
        );
        INSERT INTO ledgers (id, name, created_at)
        VALUES (1, 'Main', datetime('now')), (2, 'Scenario', datetime('now'));
        CREATE TABLE budget_periods (
            id INTEGER PRIMARY KEY,
            ledger_id INTEGER NOT NULL REFERENCES ledgers(id) ON DELETE CASCADE,
            category_id INTEGER NULL REFERENCES categories(id) ON DELETE CASCADE,
            start_year INTEGER NOT NULL,
            start_month INTEGER NOT NULL,
            amount TEXT NULL
        );
        INSERT INTO budget_periods (ledger_id, category_id, start_year, start_month, amount)
        VALUES (1, 1, 0, 1, '600'), (1, NULL, 2026, 1, '2000'), (2, 1, 2026, 1, '400');
        CREATE TABLE investment_accounts (
            id INTEGER PRIMARY KEY,
            ledger_id INTEGER NOT NULL REFERENCES ledgers(id) ON DELETE CASCADE,
            name TEXT NOT NULL COLLATE NOCASE,
            kind TEXT NOT NULL DEFAULT '',
            position INTEGER NOT NULL DEFAULT 0,
            archived INTEGER NOT NULL DEFAULT 0,
            UNIQUE(ledger_id, name)
        );
        INSERT INTO investment_accounts (id, ledger_id, name)
        VALUES (3, 1, 'TFSA'), (4, 2, 'Main Account'), (5, 2, 'main account 2');
        CREATE TABLE investment_entries (
            id INTEGER PRIMARY KEY,
            account_id INTEGER NOT NULL REFERENCES investment_accounts(id) ON DELETE CASCADE,
            date TEXT NOT NULL,
            entry_kind TEXT NOT NULL,
            amount TEXT NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            transaction_id INTEGER NULL
        );
        INSERT INTO investment_entries (account_id, date, entry_kind, amount, note)
        VALUES (3, '2026-01-01', 'Valuation', '1000', 'Opening position');
        PRAGMA user_version = 5;
    ";

    #[test]
    fn upgrading_to_v6_gives_every_transaction_an_account_and_loses_nothing() {
        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let mut conn = database.open_connection("test").unwrap();
        conn.execute_batch(V5_FIXTURE).unwrap();
        database.run_migrations(&mut conn).unwrap();

        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
        assert_eq!(count("SELECT COUNT(*) FROM budget_periods"), 3);
        assert_eq!(count("SELECT COUNT(*) FROM categories"), 1);
        assert_eq!(count("SELECT COUNT(*) FROM investment_entries"), 1);
        assert_eq!(count("SELECT COUNT(*) FROM pragma_foreign_key_check"), 0);
        assert_eq!(count("PRAGMA foreign_keys"), 1);
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM pragma_foreign_key_list('investment_entries') WHERE \"table\" = 'accounts'"
            ),
            1
        );
        assert_eq!(
            count(
                "SELECT COUNT(*) FROM pragma_table_info('investment_entries') WHERE name = 'transaction_id'"
            ),
            0
        );
        assert_eq!(count("SELECT COUNT(*) FROM ledgers WHERE id = 5"), 1);
        drop(conn);

        let main = Accounts::new(
            investments(&temp, DEFAULT_LEDGER_ID)
                .list_accounts()
                .unwrap(),
        );
        let cash = main.default_id().unwrap();
        assert_eq!(main.name(cash), "Main Account");
        let tfsa = main.named("TFSA").unwrap();
        assert_eq!(tfsa.class, AccountClass::Investment);
        assert_eq!(tfsa.tracked_from, Some(day("2026-01-01")));
        let rows = temp.store().list().unwrap();
        assert_eq!(
            rows.iter()
                .map(|tx| (tx.id, tx.transaction_type, tx.account_id))
                .collect::<Vec<_>>(),
            vec![
                (Some(7), TransactionType::Expense, cash),
                (Some(9), TransactionType::Income, cash),
            ]
        );

        let scenario = Accounts::new(investments(&temp, 2).list_accounts().unwrap());
        let scenario_cash = scenario.default_id().unwrap();
        assert_eq!(scenario.name(scenario_cash), "Main Account 3");
        assert_eq!(
            temp.store_for(2).list().unwrap()[0].account_id,
            scenario_cash
        );
        assert_eq!(temp.store_for(5).list().unwrap().len(), 1);

        let tfsa = main.named("TFSA").unwrap().id;
        temp.store()
            .insert(&transfer("2026-02-01", "250", cash, tfsa))
            .unwrap();
        assert_eq!(temp.store().list().unwrap()[2].to_account_id, Some(tfsa));
    }

    #[test]
    fn a_failed_upgrade_changes_nothing() {
        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let mut conn = database.open_connection("test").unwrap();
        conn.execute_batch(V5_FIXTURE).unwrap();
        // Force a table name conflict during the rebuild.
        conn.execute_batch("CREATE TABLE transactions_v6 (id INTEGER);")
            .unwrap();

        assert!(database.run_migrations(&mut conn).is_err());
        let count = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
        assert_eq!(count("PRAGMA user_version"), 5);
        assert_eq!(count("PRAGMA foreign_keys"), 1);
        assert_eq!(count("SELECT COUNT(*) FROM transactions"), 4);
        assert_eq!(count("SELECT COUNT(*) FROM budget_periods"), 3);
        assert_eq!(count("SELECT COUNT(*) FROM investment_accounts"), 3);
    }

    #[test]
    fn a_card_purchase_counts_once_and_its_payment_is_a_transfer() {
        let temp = TempDb::new();
        let visa = open_account(&temp, DEFAULT_LEDGER_ID, "Visa", AccountClass::Credit);
        let store = temp.store();
        store
            .insert(&TransactionDraft {
                account_id: visa,
                ..draft("2026-03-02", "Phone", "1200", "Shopping")
            })
            .unwrap();
        store
            .insert(&TransactionDraft {
                category: "Debt Payments".to_string(),
                ..transfer("2026-03-20", "1200", MAIN_ACCOUNT, visa)
            })
            .unwrap();

        let rows = store.list().unwrap();
        let mut totals = MonthlySummary::default();
        rows.iter().for_each(|tx| totals.add(tx));
        assert_eq!(totals.expense, Decimal::from(1200));
        assert_eq!(totals.transferred, Decimal::from(1200));
        assert_eq!(rows[0].category, "Shopping");
        assert_eq!(rows[1].category, "Debt Payments");

        let accounts = investments(&temp, DEFAULT_LEDGER_ID);
        for (id, name, class, opening, tracked_from) in [
            (
                MAIN_ACCOUNT,
                "Main Account",
                AccountClass::Cash,
                "4100",
                Some(day("2026-03-25")),
            ),
            (visa, "Visa", AccountClass::Credit, "-300", None),
        ] {
            accounts
                .update_account(
                    id,
                    &AccountDraft {
                        name: name.to_string(),
                        kind: String::new(),
                        archived: false,
                        class,
                        opening_balance: Decimal::from_str(opening).unwrap(),
                        tracked_from,
                    },
                )
                .unwrap();
        }
        let all = Accounts::new(accounts.list_accounts().unwrap());
        let balance = |id, on| all.balance(id, &rows, day(on));
        assert_eq!(balance(MAIN_ACCOUNT, "2026-03-31"), Decimal::from(4100));
        assert_eq!(balance(visa, "2026-03-10"), Decimal::from(-1500));
        assert_eq!(balance(visa, "2026-03-31"), Decimal::from(-300));
    }

    #[test]
    fn transfers_become_investment_flows_and_pin_their_accounts() {
        let temp = TempDb::new();
        let accounts = investments(&temp, DEFAULT_LEDGER_ID);
        let rrsp = open_account(&temp, DEFAULT_LEDGER_ID, "RRSP", AccountClass::Investment);
        accounts
            .save_entry(&entry(
                rrsp,
                "2025-01-01",
                InvestmentEntryKind::Valuation,
                "1000",
            ))
            .unwrap();

        accounts
            .update_account(
                rrsp,
                &AccountDraft {
                    name: "RRSP".to_string(),
                    kind: String::new(),
                    archived: false,
                    class: AccountClass::Investment,
                    opening_balance: Decimal::ZERO,
                    tracked_from: Some(day("2025-01-01")),
                },
            )
            .unwrap();

        let store = temp.store();
        store
            .insert(&transfer("2024-11-01", "800", MAIN_ACCOUNT, rrsp))
            .unwrap();
        store
            .insert(&transfer("2025-02-01", "500", MAIN_ACCOUNT, rrsp))
            .unwrap();
        store
            .insert(&transfer("2025-03-01", "200", rrsp, MAIN_ACCOUNT))
            .unwrap();

        let transactions = store.list().unwrap();
        let mut entries = accounts.list_entries().unwrap();
        entries.extend(
            transactions
                .iter()
                .flat_map(InvestmentEntry::transfer_flows),
        );
        let all = Accounts::new(accounts.list_accounts().unwrap());
        let portfolio = Portfolio::new(all.investments(), entries);

        let position = portfolio.position(rrsp, day("2025-04-01"));
        assert_eq!(position.value, Decimal::from(1300));
        assert_eq!(position.invested, Decimal::from(300));
        assert_eq!(
            portfolio.gain_between(Some(rrsp), day("2025-01-01"), day("2025-04-01"), false),
            Decimal::ZERO
        );
        assert_eq!(portfolio.entries_for(MAIN_ACCOUNT).count(), 0);

        assert_eq!(
            accounts.delete_account(rrsp).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        let mut retyped = AccountDraft {
            name: "RRSP".to_string(),
            kind: String::new(),
            archived: false,
            class: AccountClass::Cash,
            opening_balance: Decimal::ZERO,
            tracked_from: None,
        };
        assert_eq!(
            accounts.update_account(rrsp, &retyped).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        retyped.class = AccountClass::Investment;
        retyped.archived = true;
        accounts.update_account(rrsp, &retyped).unwrap();
    }

    #[test]
    fn converting_a_category_to_transfers_drops_manual_duplicates() {
        let temp = TempDb::new();
        let accounts = investments(&temp, DEFAULT_LEDGER_ID);
        let rrsp = open_account(&temp, DEFAULT_LEDGER_ID, "RRSP", AccountClass::Investment);
        for (date, amount) in [
            ("2025-01-15", "500"),
            ("2025-02-15", "500"),
            ("2025-03-01", "80"),
        ] {
            accounts
                .save_entry(&entry(
                    rrsp,
                    date,
                    InvestmentEntryKind::Contribution,
                    amount,
                ))
                .unwrap();
        }

        let store = temp.store();
        let saving = store
            .insert(&draft("2025-01-15", "Saving", "500", "savings"))
            .unwrap();
        let payout = store
            .insert(&TransactionDraft {
                transaction_type: TransactionType::Income,
                ..draft("2025-02-20", "Withdrawal", "100", "savings")
            })
            .unwrap();
        store
            .insert(&draft("2025-02-15", "Groceries", "500", "Food"))
            .unwrap();

        let mut converted = transfer("2025-01-15", "500", MAIN_ACCOUNT, rrsp).into_transaction();
        converted.id = Some(saving);
        let flows = InvestmentEntry::transfer_flows(&converted);
        let all = Accounts::new(accounts.list_accounts().unwrap());
        let portfolio = Portfolio::new(all.investments(), accounts.list_entries().unwrap());
        let duplicates = portfolio.duplicate_entries(rrsp, &flows);
        // Only January matches. February's 500 was groceries.
        assert_eq!(duplicates.len(), 1);

        store
            .convert_to_transfers(&[saving, payout], rrsp, "Savings", "", &duplicates)
            .unwrap();

        let rows = store.list().unwrap();
        let shape: Vec<_> = rows
            .iter()
            .map(|tx| {
                (
                    tx.description.as_str(),
                    tx.transaction_type,
                    tx.account_id,
                    tx.to_account_id,
                    tx.category.as_str(),
                )
            })
            .collect();
        assert!(shape.contains(&(
            "Saving",
            TransactionType::Transfer,
            MAIN_ACCOUNT,
            Some(rrsp),
            "Savings"
        )));
        assert!(shape.contains(&(
            "Withdrawal",
            TransactionType::Transfer,
            rrsp,
            Some(MAIN_ACCOUNT),
            "Savings"
        )));
        assert!(shape.contains(&(
            "Groceries",
            TransactionType::Expense,
            MAIN_ACCOUNT,
            None,
            "Food"
        )));
        assert_eq!(accounts.list_entries().unwrap().len(), 2);
        let categories =
            crate::db::category_store::SqliteCategoryStore::new(SqliteDatabase::new(&temp.path));
        assert!(categories.list().unwrap().iter().any(|record| {
            record.transaction_type == TransactionType::Transfer && record.category == "Savings"
        }));

        let remaining = accounts.list_entries().unwrap()[0].id;
        assert!(
            store
                .convert_to_transfers(&[saving], rrsp, "Savings", "", &[remaining])
                .is_err()
        );
        assert_eq!(accounts.list_entries().unwrap().len(), 2);
    }

    #[test]
    fn upgrading_the_default_catalog_only_adds_what_is_new() {
        use crate::db::category_store::{CATEGORY_SEED_VERSION, SqliteCategoryStore};

        assert!(
            crate::csv_io::seed_categories_added_after(CATEGORY_SEED_VERSION)
                .unwrap()
                .is_empty()
        );
        let added = crate::csv_io::seed_categories_added_after(1).unwrap();
        assert!(!added.is_empty());

        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let conn = database.ready_connection("test").unwrap();
        // An older catalog with one new default already added in lowercase.
        conn.execute_batch(
            "
            INSERT INTO categories (transaction_type, category, subcategory)
            VALUES ('Expense', 'Food & Dining', 'Groceries'),
                   ('Transfer', 'debt payments', 'credit card payments');
            INSERT INTO database_meta (key, value) VALUES ('category_seed_version', '1');
            ",
        )
        .unwrap();
        drop(conn);

        let categories = SqliteCategoryStore::new(database.clone());
        categories.initialize(&[]).unwrap();
        categories.initialize(&[]).unwrap();

        let records = categories.list().unwrap();
        assert_eq!(records.len(), 1 + added.len());
        let count = |category: &str, subcategory: &str| {
            records
                .iter()
                .filter(|record| {
                    record.category.eq_ignore_ascii_case(category)
                        && record.subcategory.eq_ignore_ascii_case(subcategory)
                })
                .count()
        };
        assert_eq!(count("Debt Payments", "Credit Card Payments"), 1);
        assert_eq!(count("Debt Payments", "Interest Charges"), 1);
        assert_eq!(count("Food & Dining", "Groceries"), 1);
        assert_eq!(count("Housing", "Rent / Mortgage"), 0);

        let conn = database.ready_connection("test").unwrap();
        assert_eq!(
            database
                .metadata_value(&conn, "category_seed_version")
                .unwrap(),
            Some(CATEGORY_SEED_VERSION.to_string())
        );
    }

    #[test]
    fn a_database_from_a_newer_build_is_refused() {
        let temp = TempDb::new();
        let database = SqliteDatabase::new(&temp.path);
        let mut conn = database.open_connection("test").unwrap();
        conn.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION + 1))
            .unwrap();

        let err = database.run_migrations(&mut conn).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unsupported);
        drop(conn);

        // Every store operation refuses too, so nothing can read or write the file.
        assert!(temp.store().list().is_err());
        assert!(
            temp.store()
                .insert(&draft("2026-01-05", "Coffee", "4.50", "Food"))
                .is_err()
        );
    }

    #[test]
    fn ledgers_do_not_see_each_others_rows() {
        let temp = TempDb::new();
        temp.store()
            .insert(&draft("2026-01-05", "Coffee", "4.50", "Food"))
            .unwrap();

        let forecast = temp.create_ledger("Forecast");
        let forecast_store = temp.store_for(forecast);
        assert!(forecast_store.list().unwrap().is_empty());

        let id = forecast_store
            .insert(&TransactionDraft {
                account_id: temp.main_account(forecast),
                ..draft("2026-02-01", "Rent", "1000", "Housing")
            })
            .unwrap();
        assert_eq!(
            forecast_store
                .insert(&draft("2026-02-01", "Rent", "1000", "Housing"))
                .unwrap_err()
                .kind(),
            ErrorKind::NotFound
        );
        assert_eq!(temp.store().list().unwrap().len(), 1);
        assert_eq!(forecast_store.list().unwrap().len(), 1);

        assert!(temp.store().delete(id).is_err());
        assert_eq!(forecast_store.list().unwrap().len(), 1);
    }

    #[test]
    fn insert_list_update_delete_roundtrip() {
        let temp = TempDb::new();
        let store = temp.store();

        let id = store
            .insert(&draft("2026-01-05", "Coffee", "4.50", "Food"))
            .unwrap();

        let rows = store.list().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, Some(id));
        assert_eq!(rows[0].description, "Coffee");
        assert_eq!(rows[0].amount, Decimal::from_str("4.50").unwrap());
        assert!(!rows[0].is_recurring);

        let mut updated = draft("2026-01-06", "Latte", "5.25", "Food");
        updated.is_recurring = true;
        updated.recurrence_frequency = Some(RecurrenceFrequency::Monthly);
        store.update(id, &updated).unwrap();

        let rows = store.list().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].description, "Latte");
        assert!(rows[0].is_recurring);
        assert_eq!(
            rows[0].recurrence_frequency,
            Some(RecurrenceFrequency::Monthly)
        );

        store.delete(id).unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn import_merge_skips_duplicates() {
        let temp = TempDb::new();
        let store = temp.store();
        store
            .insert(&draft("2026-01-05", "Coffee", "4.50", "Food"))
            .unwrap();

        // One duplicate (note "4.5" vs stored "4.50", which canonicalize equal) and one new row.
        let dup = draft("2026-01-05", "Coffee", "4.5", "Food").into_transaction();
        let fresh = draft("2026-02-01", "Books", "20", "Education").into_transaction();

        let summary = store.import_merge(&[dup, fresh]).unwrap();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(store.list().unwrap().len(), 2);

        // Dedupe is per-ledger, so the same rows import cleanly into a different ledger.
        let forecast = temp.create_ledger("Forecast");
        let other = temp.store_for(forecast);
        let in_forecast = |draft: TransactionDraft| TransactionDraft {
            account_id: temp.main_account(forecast),
            ..draft
        };
        let dup = in_forecast(draft("2026-01-05", "Coffee", "4.5", "Food")).into_transaction();
        let fresh = in_forecast(draft("2026-02-01", "Books", "20", "Education")).into_transaction();
        let summary = other.import_merge(&[dup, fresh]).unwrap();
        assert_eq!(summary.added, 2);
        assert_eq!(summary.skipped, 0);
    }

    #[test]
    fn importing_a_csv_drops_generated_rows() {
        let temp = TempDb::new();
        let csv_path = temp.path.with_extension("csv");
        std::fs::write(
            &csv_path,
            "date,description,amount,transaction_type,category,subcategory,is_recurring,recurrence_frequency,recurrence_end_date,is_generated_from_recurring\n\
             2026-01-01,Rent,1000,Expense,Housing,Rent,true,Monthly,,false\n\
             2026-02-01,Rent,1000,Expense,Housing,Rent,true,Monthly,,true\n\
             2026-03-01,Rent,1000,Expense,Housing,Rent,true,Monthly,,true\n\
             2026-01-15,Coffee,4.50,Expense,Food,Coffee,false,,,false\n",
        )
        .unwrap();

        let rows = crate::csv_io::load_transactions(&csv_path).unwrap();
        assert_eq!(rows.len(), 4, "all CSV rows parse");

        // The import path drops generated occurrences, keeping only real rows (source + normal).
        let accounts = Accounts::new(
            investments(&temp, DEFAULT_LEDGER_ID)
                .list_accounts()
                .unwrap(),
        );
        let real_rows: Vec<Transaction> = rows
            .iter()
            .filter(|row| !row.is_generated_from_recurring)
            .map(|row| accounts.link_csv(row).unwrap())
            .collect();
        let summary = temp.store().import_merge(&real_rows).unwrap();
        assert_eq!(summary.added, 2);

        let stored = temp.store().list().unwrap();
        assert_eq!(stored.len(), 2);
        assert!(stored.iter().all(|tx| !tx.is_generated_from_recurring));
        // The recurring source survived with its rule intact.
        assert!(
            stored.iter().any(|tx| tx.is_recurring
                && tx.recurrence_frequency == Some(RecurrenceFrequency::Monthly))
        );

        let _ = std::fs::remove_file(&csv_path);
    }

    #[test]
    fn accounts_round_trip_through_csv_by_name() {
        let temp = TempDb::new();
        let rrsp = open_account(&temp, DEFAULT_LEDGER_ID, "RRSP", AccountClass::Investment);
        temp.store()
            .insert(&transfer("2026-01-01", "250", MAIN_ACCOUNT, rrsp))
            .unwrap();
        let accounts = Accounts::new(
            investments(&temp, DEFAULT_LEDGER_ID)
                .list_accounts()
                .unwrap(),
        );

        let csv_path = temp.path.with_extension("csv");
        let exported: Vec<_> = temp
            .store()
            .list()
            .unwrap()
            .iter()
            .map(|tx| accounts.csv_row(tx))
            .collect();
        crate::csv_io::save_transactions(&exported, &csv_path).unwrap();
        assert!(
            std::fs::read_to_string(&csv_path)
                .unwrap()
                .contains("Transfer,Savings,,false,,,false,Main Account,RRSP")
        );

        let rows: Vec<Transaction> = crate::csv_io::load_transactions(&csv_path)
            .unwrap()
            .iter()
            .map(|row| accounts.link_csv(row).unwrap())
            .collect();
        assert_eq!(rows[0].account_id, MAIN_ACCOUNT);
        assert_eq!(rows[0].to_account_id, Some(rrsp));
        assert_eq!(temp.store().import_merge(&rows).unwrap().added, 0);

        std::fs::write(
            &csv_path,
            "date,description,amount,transaction_type,category\n\
             2026-01-05,Coffee,4.50,Expense,Food\n",
        )
        .unwrap();
        let old = crate::csv_io::load_transactions(&csv_path).unwrap();
        assert_eq!(accounts.link_csv(&old[0]).unwrap().account_id, MAIN_ACCOUNT);
        let mut unknown = old[0].clone();
        unknown.account = "Nowhere".to_string();
        assert!(accounts.link_csv(&unknown).is_err());
        let mut headless = old[0].clone();
        headless.transaction_type = TransactionType::Transfer;
        assert!(accounts.link_csv(&headless).is_err());

        let _ = std::fs::remove_file(&csv_path);
    }

    // Small helper to turn a draft into a Transaction for import tests.
    impl TransactionDraft {
        fn into_transaction(self) -> Transaction {
            Transaction {
                date: self.date,
                description: self.description,
                amount: self.amount,
                transaction_type: self.transaction_type,
                category: self.category,
                subcategory: self.subcategory,
                is_recurring: self.is_recurring,
                recurrence_frequency: self.recurrence_frequency,
                recurrence_end_date: self.recurrence_end_date,
                is_generated_from_recurring: false,
                id: None,
                parent_id: None,
                account_id: self.account_id,
                to_account_id: self.to_account_id,
            }
        }
    }
}
