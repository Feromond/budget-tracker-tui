use rusqlite::{Connection, OptionalExtension};
use std::fs::create_dir_all;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

/// The latest schema version understood by this build. Bump this and add a matching arm in
/// [`SqliteDatabase::apply_migration`] whenever the schema changes.
pub const SCHEMA_VERSION: i64 = 5;

#[derive(Debug, Clone)]
pub struct SqliteDatabase {
    path: PathBuf,
}

impl SqliteDatabase {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn open_connection(&self, purpose: &str) -> Result<Connection> {
        self.ensure_parent_dir()?;
        let conn = Connection::open(&self.path).map_err(|err| {
            Error::other(format!(
                "Failed to open {} database '{}': {}",
                purpose,
                self.path.display(),
                err
            ))
        })?;
        // Off by default in SQLite, and per connection. Must be set outside a transaction.
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|err| Error::other(format!("Failed to enable foreign keys: {}", err)))?;
        Ok(conn)
    }

    /// Open a connection with the schema guaranteed up to date.
    pub fn ready_connection(&self, purpose: &str) -> Result<Connection> {
        let mut conn = self.open_connection(purpose)?;
        self.run_migrations(&mut conn)?;
        Ok(conn)
    }

    pub fn ensure_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Apply any pending schema migrations, keyed on SQLite's built-in `PRAGMA user_version`.
    /// Steps run in order inside a single transaction; the version is bumped on success. This
    /// is the one place schema is created, so it is safe to call before every operation
    /// (it is a cheap version read once the database is up to date).
    pub fn run_migrations(&self, conn: &mut Connection) -> Result<()> {
        let current: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|err| Error::other(format!("Failed to read schema version: {}", err)))?;

        // A newer build may have added columns this one does not know about, so refuse the
        // file rather than reading or writing a schema we cannot honour.
        if current > SCHEMA_VERSION {
            return Err(Error::new(
                ErrorKind::Unsupported,
                format!(
                    "Database '{}' was created by a newer version of Budget Tracker \
                     (data version {}; this build supports up to {}). Update Budget Tracker to open it.",
                    self.path.display(),
                    current,
                    SCHEMA_VERSION
                ),
            ));
        }

        if current == SCHEMA_VERSION {
            return Ok(());
        }

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin migration: {}", err)))?;

        for version in (current + 1)..=SCHEMA_VERSION {
            Self::apply_migration(&tx, version)?;
        }

        // `user_version` does not accept bound parameters, so format it into the statement.
        tx.execute_batch(&format!("PRAGMA user_version = {};", SCHEMA_VERSION))
            .map_err(|err| Error::other(format!("Failed to set schema version: {}", err)))?;
        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit migration: {}", err)))
    }

    fn apply_migration(conn: &Connection, version: i64) -> Result<()> {
        match version {
            // v1: metadata + categories (idempotent so pre-existing databases upgrade cleanly).
            1 => {
                conn.execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS database_meta (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS categories (
                        id INTEGER PRIMARY KEY,
                        transaction_type TEXT NOT NULL CHECK (transaction_type IN ('Income', 'Expense')),
                        category TEXT NOT NULL,
                        subcategory TEXT NOT NULL DEFAULT '',
                        tag TEXT NULL,
                        target_budget TEXT NULL,
                        UNIQUE(transaction_type, category, subcategory)
                    );
                    ",
                )
                .map_err(|err| Error::other(format!("Migration v1 failed: {}", err)))?;
                // Databases created before target_budget existed need the column added.
                Self::ensure_column(conn, "categories", "target_budget", "TEXT NULL")
            }
            // v2: transactions (real rows only; generated occurrences are derived in-memory).
            2 => conn
                .execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS transactions (
                        id INTEGER PRIMARY KEY,
                        date TEXT NOT NULL,
                        description TEXT NOT NULL,
                        amount TEXT NOT NULL,
                        transaction_type TEXT NOT NULL CHECK (transaction_type IN ('Income', 'Expense')),
                        category TEXT NOT NULL DEFAULT 'Uncategorized',
                        subcategory TEXT NOT NULL DEFAULT '',
                        is_recurring INTEGER NOT NULL DEFAULT 0,
                        recurrence_frequency TEXT NULL,
                        recurrence_end_date TEXT NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
                    ",
                )
                .map_err(|err| Error::other(format!("Migration v2 failed: {}", err))),
            // v3: ledgers (independent sets of transactions sharing one category catalog).
            3 => {
                conn.execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS ledgers (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL COLLATE NOCASE,
                        position INTEGER NOT NULL DEFAULT 0,
                        created_at TEXT NOT NULL,
                        UNIQUE(name)
                    );
                    INSERT INTO ledgers (id, name, position, created_at)
                    SELECT 1, 'Main', 0, datetime('now')
                    WHERE NOT EXISTS (SELECT 1 FROM ledgers);
                    ",
                )
                .map_err(|err| Error::other(format!("Migration v3 failed: {}", err)))?;
                // SQLite rejects a REFERENCES clause on an added NOT NULL column, so the link
                // to `ledgers` is enforced by the store rather than by a foreign key.
                Self::ensure_column(conn, "transactions", "ledger_id", "INTEGER NOT NULL DEFAULT 1")?;
                conn.execute_batch(
                    "
                    CREATE INDEX IF NOT EXISTS idx_transactions_ledger_date
                        ON transactions(ledger_id, date);
                    ",
                )
                .map_err(|err| Error::other(format!("Migration v3 failed: {}", err)))
            }
            // v4: budget history. The monthly budget and every category budget become
            // effective-dated per ledger, so changing one stops rewriting what past months
            // were budgeted at.
            4 => {
                conn.execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS budget_periods (
                        id INTEGER PRIMARY KEY,
                        ledger_id INTEGER NOT NULL REFERENCES ledgers(id) ON DELETE CASCADE,
                        category_id INTEGER NULL REFERENCES categories(id) ON DELETE CASCADE,
                        start_year INTEGER NOT NULL,
                        start_month INTEGER NOT NULL,
                        -- NULL means the budget was cleared from this month on, which is
                        -- different from having no period at all (inherit the previous one).
                        amount TEXT NULL
                    );
                    CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_periods_target
                        ON budget_periods(ledger_id, start_year, start_month)
                        WHERE category_id IS NULL;
                    CREATE UNIQUE INDEX IF NOT EXISTS idx_budget_periods_category
                        ON budget_periods(ledger_id, category_id, start_year, start_month)
                        WHERE category_id IS NOT NULL;
                    ",
                )
                .map_err(|err| Error::other(format!("Migration v4 failed: {}", err)))?;

                // A database that never ran v1 has no categories to carry over.
                if Self::has_column(conn, "categories", "target_budget")? {
                    conn.execute_batch(
                        "
                        INSERT INTO budget_periods (ledger_id, category_id, start_year, start_month, amount)
                        SELECT l.id, c.id, 0, 1, c.target_budget
                        FROM ledgers l
                        CROSS JOIN categories c
                        WHERE c.target_budget IS NOT NULL;
                        ",
                    )
                    .map_err(|err| Error::other(format!("Migration v4 failed: {}", err)))?;
                    Self::drop_column(conn, "categories", "target_budget")?;
                }
                Ok(())
            }
            // v5: manually tracked investments. Valuations and cash flows are separate rows so
            // growth is always derived, never typed in.
            5 => conn
                .execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS investment_accounts (
                        id INTEGER PRIMARY KEY,
                        ledger_id INTEGER NOT NULL REFERENCES ledgers(id) ON DELETE CASCADE,
                        name TEXT NOT NULL COLLATE NOCASE,
                        kind TEXT NOT NULL DEFAULT '',
                        position INTEGER NOT NULL DEFAULT 0,
                        archived INTEGER NOT NULL DEFAULT 0,
                        UNIQUE(ledger_id, name)
                    );
                    CREATE TABLE IF NOT EXISTS investment_entries (
                        id INTEGER PRIMARY KEY,
                        account_id INTEGER NOT NULL
                            REFERENCES investment_accounts(id) ON DELETE CASCADE,
                        date TEXT NOT NULL,
                        entry_kind TEXT NOT NULL
                            CHECK (entry_kind IN ('Valuation', 'Contribution', 'Withdrawal')),
                        amount TEXT NOT NULL,
                        note TEXT NOT NULL DEFAULT '',
                        -- Reserved for linking a flow to the transaction that funded it.
                        -- Nothing reads it yet; it is here so adding that needs no migration.
                        transaction_id INTEGER NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_investment_entries_account
                        ON investment_entries(account_id, date);
                    -- Two different values for one account on one day is meaningless, so a
                    -- second valuation replaces the first rather than stacking.
                    CREATE UNIQUE INDEX IF NOT EXISTS idx_investment_valuation_day
                        ON investment_entries(account_id, date)
                        WHERE entry_kind = 'Valuation';
                    ",
                )
                .map_err(|err| Error::other(format!("Migration v5 failed: {}", err))),
            _ => Ok(()),
        }
    }

    /// Drop a column if it is still present, so re-running the migration is harmless.
    fn drop_column(conn: &Connection, table: &str, column: &str) -> Result<()> {
        if !Self::has_column(conn, table, column)? {
            return Ok(());
        }
        conn.execute(&format!("ALTER TABLE {} DROP COLUMN {}", table, column), [])
            .map_err(|err| Error::other(format!("Failed to drop {}.{}: {}", table, column, err)))?;
        Ok(())
    }

    fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({})", table))
            .map_err(|err| Error::other(format!("Failed to inspect {} schema: {}", table, err)))?;
        let exists = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|err| Error::other(format!("Failed to read {} schema: {}", table, err)))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| Error::other(format!("Failed to collect {} schema: {}", table, err)))?
            .into_iter()
            .any(|name| name == column);
        Ok(exists)
    }

    /// Add a column to a table if it is not already present (for upgrading legacy databases).
    fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
        if !Self::has_column(conn, table, column)? {
            conn.execute(
                &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
                [],
            )
            .map_err(|err| Error::other(format!("Failed to add {}.{}: {}", table, column, err)))?;
        }
        Ok(())
    }

    pub fn metadata_value(&self, conn: &Connection, key: &str) -> Result<Option<String>> {
        conn.query_row(
            "SELECT value FROM database_meta WHERE key = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|err| {
            Error::other(format!(
                "Failed to read database metadata '{}': {}",
                key, err
            ))
        })
    }

    pub fn set_metadata_value(&self, conn: &Connection, key: &str, value: &str) -> Result<()> {
        conn.execute(
            "
            INSERT INTO database_meta (key, value)
            VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            ",
            [key, value],
        )
        .map_err(|err| {
            Error::other(format!(
                "Failed to write database metadata '{}': {}",
                key, err
            ))
        })?;
        Ok(())
    }
}
