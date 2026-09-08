use crate::db::database::SqliteDatabase;
use rusqlite::{Connection, Row, params};
use std::io::{Error, ErrorKind, Result};

const ACTIVE_LEDGER_KEY: &str = "active_ledger_id";
const DEFAULT_LEDGER_NAME: &str = "Main";

/// The ledger seeded by migration v3, which every pre-existing transaction is attributed to.
pub const DEFAULT_LEDGER_ID: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRecord {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone)]
pub struct LedgerSelection {
    pub ledgers: Vec<LedgerRecord>,
    pub active_id: i64,
}

/// Persistence for ledgers: independent sets of transactions within one database file. The
/// category catalog is shared across every ledger, so only transactions are partitioned.
pub trait LedgerStore {
    /// Guarantee at least one ledger exists and return them alongside a valid active id,
    /// repairing the stored selection if it is missing or points at a deleted ledger.
    fn initialize(&self) -> Result<LedgerSelection>;
    fn create(&self, name: &str) -> Result<LedgerRecord>;
    /// Create a ledger holding a copy of every transaction in `source_id`.
    fn copy(&self, source_id: i64, name: &str) -> Result<LedgerRecord>;
    fn rename(&self, id: i64, name: &str) -> Result<()>;
    /// Delete a ledger and everything it holds. Refuses to delete the last ledger.
    fn delete(&self, id: i64) -> Result<()>;
    fn transaction_count(&self, id: i64) -> Result<i64>;
    fn investment_account_count(&self, id: i64) -> Result<i64>;
    fn set_active_id(&self, id: i64) -> Result<()>;
}

pub struct SqliteLedgerStore {
    database: SqliteDatabase,
}

impl SqliteLedgerStore {
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    fn ready_connection(&self) -> Result<Connection> {
        self.database.ready_connection("ledger")
    }

    fn row_to_record(row: &Row<'_>) -> rusqlite::Result<LedgerRecord> {
        Ok(LedgerRecord {
            id: row.get(0)?,
            name: row.get(1)?,
            position: row.get(2)?,
        })
    }

    fn list_with_conn(conn: &Connection) -> Result<Vec<LedgerRecord>> {
        let mut stmt = conn
            .prepare("SELECT id, name, position FROM ledgers ORDER BY position, id")
            .map_err(|err| Error::other(format!("Failed to prepare ledger query: {}", err)))?;

        stmt.query_map([], Self::row_to_record)
            .map_err(|err| Error::other(format!("Failed to load ledgers: {}", err)))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| Error::other(format!("Failed to read ledgers: {}", err)))
    }

    fn create_with_conn(conn: &Connection, name: &str) -> Result<LedgerRecord> {
        let name = validate_name(name)?;
        let position: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM ledgers",
                [],
                |row| row.get(0),
            )
            .map_err(|err| Error::other(format!("Failed to position new ledger: {}", err)))?;

        conn.execute(
            "INSERT INTO ledgers (name, position, created_at) VALUES (?1, ?2, datetime('now'))",
            params![&name, position],
        )
        .map_err(|err| match err {
            rusqlite::Error::SqliteFailure(inner, _)
                if inner.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Error::new(
                    ErrorKind::AlreadyExists,
                    format!("A ledger named '{}' already exists.", name),
                )
            }
            other => Error::other(format!("Failed to create ledger: {}", other)),
        })?;

        Ok(LedgerRecord {
            id: conn.last_insert_rowid(),
            name,
            position,
        })
    }
}

impl LedgerStore for SqliteLedgerStore {
    fn initialize(&self) -> Result<LedgerSelection> {
        let conn = self.ready_connection()?;

        let mut ledgers = Self::list_with_conn(&conn)?;
        if ledgers.is_empty() {
            ledgers.push(Self::create_with_conn(&conn, DEFAULT_LEDGER_NAME)?);
        }

        let stored_id = self
            .database
            .metadata_value(&conn, ACTIVE_LEDGER_KEY)?
            .and_then(|value| value.trim().parse::<i64>().ok());

        let active_id = match stored_id {
            Some(id) if ledgers.iter().any(|ledger| ledger.id == id) => id,
            _ => {
                let fallback = ledgers[0].id;
                self.database.set_metadata_value(
                    &conn,
                    ACTIVE_LEDGER_KEY,
                    &fallback.to_string(),
                )?;
                fallback
            }
        };

        Ok(LedgerSelection { ledgers, active_id })
    }

    fn create(&self, name: &str) -> Result<LedgerRecord> {
        let conn = self.ready_connection()?;
        Self::create_with_conn(&conn, name)
    }

    fn copy(&self, source_id: i64, name: &str) -> Result<LedgerRecord> {
        let mut conn = self.ready_connection()?;
        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin ledger copy: {}", err)))?;

        let ledger = Self::create_with_conn(&tx, name)?;
        tx.execute(
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
            )
            SELECT
                ?1,
                date,
                description,
                amount,
                transaction_type,
                category,
                subcategory,
                is_recurring,
                recurrence_frequency,
                recurrence_end_date
            FROM transactions
            WHERE ledger_id = ?2
            ",
            params![ledger.id, source_id],
        )
        .map_err(|err| Error::other(format!("Failed to copy transactions: {}", err)))?;

        // Same transaction, or a failure leaves a ledger the caller was told did not save.
        tx.execute(
            "
            INSERT INTO budget_periods (ledger_id, category_id, start_year, start_month, amount)
            SELECT ?1, category_id, start_year, start_month, amount
            FROM budget_periods
            WHERE ledger_id = ?2
            ",
            params![ledger.id, source_id],
        )
        .map_err(|err| Error::other(format!("Failed to copy budgets: {}", err)))?;

        // Accounts first, then their entries remapped onto the new ids.
        tx.execute(
            "
            INSERT INTO investment_accounts (ledger_id, name, kind, position, archived)
            SELECT ?1, name, kind, position, archived
            FROM investment_accounts
            WHERE ledger_id = ?2
            ",
            params![ledger.id, source_id],
        )
        .map_err(|err| Error::other(format!("Failed to copy investment accounts: {}", err)))?;

        tx.execute(
            "
            INSERT INTO investment_entries
                (account_id, date, entry_kind, amount, note, transaction_id)
            SELECT copy.id, e.date, e.entry_kind, e.amount, e.note, e.transaction_id
            FROM investment_entries e
            JOIN investment_accounts source ON source.id = e.account_id
            JOIN investment_accounts copy
                ON copy.ledger_id = ?1 AND copy.name = source.name
            WHERE source.ledger_id = ?2
            ",
            params![ledger.id, source_id],
        )
        .map_err(|err| Error::other(format!("Failed to copy investment entries: {}", err)))?;

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit ledger copy: {}", err)))?;
        Ok(ledger)
    }

    fn rename(&self, id: i64, name: &str) -> Result<()> {
        let name = validate_name(name)?;
        let conn = self.ready_connection()?;
        let updated = conn
            .execute(
                "UPDATE ledgers SET name = ?1 WHERE id = ?2",
                params![&name, id],
            )
            .map_err(|err| match err {
                rusqlite::Error::SqliteFailure(inner, _)
                    if inner.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Error::new(
                        ErrorKind::AlreadyExists,
                        format!("A ledger named '{}' already exists.", name),
                    )
                }
                other => Error::other(format!("Failed to rename ledger: {}", other)),
            })?;

        if updated == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Ledger with id {} was not found.", id),
            ));
        }
        Ok(())
    }

    fn delete(&self, id: i64) -> Result<()> {
        let mut conn = self.ready_connection()?;

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM ledgers", [], |row| row.get(0))
            .map_err(|err| Error::other(format!("Failed to count ledgers: {}", err)))?;
        if remaining <= 1 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "At least one ledger must remain.",
            ));
        }

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin ledger delete: {}", err)))?;

        tx.execute("DELETE FROM transactions WHERE ledger_id = ?1", [id])
            .map_err(|err| {
                Error::other(format!("Failed to delete ledger transactions: {}", err))
            })?;
        // Entries go through the accounts' cascade, but only if foreign keys are on for this
        // connection, so clear them explicitly rather than relying on the pragma.
        tx.execute(
            "
            DELETE FROM investment_entries
            WHERE account_id IN (SELECT id FROM investment_accounts WHERE ledger_id = ?1)
            ",
            [id],
        )
        .map_err(|err| Error::other(format!("Failed to delete investment entries: {}", err)))?;
        tx.execute("DELETE FROM investment_accounts WHERE ledger_id = ?1", [id])
            .map_err(|err| {
                Error::other(format!("Failed to delete investment accounts: {}", err))
            })?;
        let deleted = tx
            .execute("DELETE FROM ledgers WHERE id = ?1", [id])
            .map_err(|err| Error::other(format!("Failed to delete ledger: {}", err)))?;

        if deleted == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Ledger with id {} was not found.", id),
            ));
        }

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit ledger delete: {}", err)))
    }

    fn transaction_count(&self, id: i64) -> Result<i64> {
        let conn = self.ready_connection()?;
        conn.query_row(
            "SELECT COUNT(*) FROM transactions WHERE ledger_id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|err| Error::other(format!("Failed to count ledger transactions: {}", err)))
    }

    fn investment_account_count(&self, id: i64) -> Result<i64> {
        let conn = self.ready_connection()?;
        conn.query_row(
            "SELECT COUNT(*) FROM investment_accounts WHERE ledger_id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(|err| Error::other(format!("Failed to count investment accounts: {}", err)))
    }

    fn set_active_id(&self, id: i64) -> Result<()> {
        let conn = self.ready_connection()?;
        self.database
            .set_metadata_value(&conn, ACTIVE_LEDGER_KEY, &id.to_string())
    }
}

fn validate_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Ledger name cannot be empty.",
        ));
    }
    Ok(trimmed.to_string())
}
