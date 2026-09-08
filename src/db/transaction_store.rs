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
        })
    }

    fn insert_with_conn(
        conn: &Connection,
        ledger_id: i64,
        draft: &TransactionDraft,
    ) -> Result<i64> {
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
                recurrence_end_date
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
                       is_recurring, recurrence_frequency, recurrence_end_date
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
                    recurrence_end_date = ?9
                WHERE id = ?10 AND ledger_id = ?11
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
    use crate::db::category_store::CategoryStore;
    use crate::db::database::SCHEMA_VERSION;
    use crate::db::investment_store::{InvestmentStore, SqliteInvestmentStore};
    use crate::db::ledger_store::{DEFAULT_LEDGER_ID, LedgerStore, SqliteLedgerStore};
    use crate::model::{
        BudgetSchedule, InvestmentAccountDraft, InvestmentEntryDraft, InvestmentEntryKind,
        Portfolio,
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
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(self.path.with_extension("db-wal"));
            let _ = std::fs::remove_file(self.path.with_extension("db-shm"));
        }
    }

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
        }
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

    fn investments(temp: &TempDb, ledger_id: i64) -> SqliteInvestmentStore {
        SqliteInvestmentStore::new(SqliteDatabase::new(&temp.path), ledger_id)
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
            .create_account(&InvestmentAccountDraft {
                name: "TFSA".to_string(),
                kind: "Brokerage".to_string(),
                archived: false,
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
            .create_account(&InvestmentAccountDraft {
                name: "Brokerage".to_string(),
                kind: "Taxable".to_string(),
                archived: false,
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

        let ledger_store = SqliteLedgerStore::new(SqliteDatabase::new(&temp.path));
        let copy = ledger_store.copy(DEFAULT_LEDGER_ID, "Copy").unwrap();

        let copied = investments(&temp, copy.id);
        assert_eq!(copied.list_accounts().unwrap().len(), 1);
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
            .insert(&draft("2026-02-01", "Rent", "1000", "Housing"))
            .unwrap();
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
        let other = temp.store_for(temp.create_ledger("Forecast"));
        let dup = draft("2026-01-05", "Coffee", "4.5", "Food").into_transaction();
        let fresh = draft("2026-02-01", "Books", "20", "Education").into_transaction();
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
        let real_rows: Vec<_> = rows
            .into_iter()
            .filter(|tx| !tx.is_generated_from_recurring)
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
            }
        }
    }
}
