use crate::db::database::SqliteDatabase;
use crate::model::{
    Account, AccountClass, AccountDraft, DATE_FORMAT, InvestmentEntry, InvestmentEntryDraft,
    InvestmentEntryKind,
};
use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};
use rust_decimal::Decimal;
use std::io::{Error, ErrorKind, Result};
use std::str::FromStr;

pub trait AccountStore {
    fn list_accounts(&self) -> Result<Vec<Account>>;
    fn list_entries(&self) -> Result<Vec<InvestmentEntry>>;
    fn create_account(&self, draft: &AccountDraft) -> Result<i64>;
    fn update_account(&self, id: i64, draft: &AccountDraft) -> Result<()>;
    fn delete_account(&self, id: i64) -> Result<()>;
    /// Replaces any valuation the account already has on that date.
    fn save_entry(&self, draft: &InvestmentEntryDraft) -> Result<i64>;
    fn update_entry(&self, id: i64, draft: &InvestmentEntryDraft) -> Result<()>;
    fn delete_entry(&self, id: i64) -> Result<()>;
}

pub struct SqliteAccountStore {
    database: SqliteDatabase,
    ledger_id: i64,
}

impl SqliteAccountStore {
    pub fn new(database: SqliteDatabase, ledger_id: i64) -> Self {
        Self {
            database,
            ledger_id,
        }
    }

    fn ready_connection(&self) -> Result<Connection> {
        self.database.ready_connection("account")
    }

    fn row_to_account(row: &Row<'_>) -> rusqlite::Result<Account> {
        Ok(Account {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            position: row.get(3)?,
            archived: row.get::<_, i64>(4)? != 0,
            class: parse_class(5, &row.get::<_, String>(5)?)?,
            opening_balance: parse_decimal(6, &row.get::<_, String>(6)?)?,
            tracked_from: row
                .get::<_, Option<String>>(7)?
                .map(|value| parse_date(7, &value))
                .transpose()?,
        })
    }

    fn row_to_entry(row: &Row<'_>) -> rusqlite::Result<InvestmentEntry> {
        Ok(InvestmentEntry {
            id: row.get(0)?,
            account_id: row.get(1)?,
            date: parse_date(2, &row.get::<_, String>(2)?)?,
            entry_kind: parse_kind(3, &row.get::<_, String>(3)?)?,
            amount: parse_decimal(4, &row.get::<_, String>(4)?)?,
            note: row.get(5)?,
            transaction_id: None,
        })
    }

    fn assert_owns_investment(&self, conn: &Connection, account_id: i64) -> Result<()> {
        let owned: i64 = conn
            .query_row(
                "
                SELECT COUNT(*) FROM accounts
                WHERE id = ?1 AND ledger_id = ?2 AND class = 'Investment'
                ",
                params![account_id, self.ledger_id],
                |row| row.get(0),
            )
            .map_err(|err| Error::other(format!("Failed to check investment account: {}", err)))?;

        if owned == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Investment account {} was not found.", account_id),
            ));
        }
        Ok(())
    }

    fn class_of(&self, conn: &Connection, id: i64) -> Result<AccountClass> {
        account_class(conn, self.ledger_id, id)
    }

    fn assert_other_spending_account(&self, conn: &Connection, id: i64) -> Result<()> {
        let others: i64 = conn
            .query_row(
                "
                SELECT COUNT(*) FROM accounts
                WHERE ledger_id = ?1 AND id != ?2 AND class IN ('Cash', 'Credit')
                ",
                params![self.ledger_id, id],
                |row| row.get(0),
            )
            .map_err(|err| Error::other(format!("Failed to count accounts: {}", err)))?;
        if others == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "A ledger needs at least one cash or credit account.",
            ));
        }
        Ok(())
    }

    fn is_used(conn: &Connection, id: i64) -> Result<bool> {
        conn.query_row(
            "
            SELECT EXISTS (SELECT 1 FROM transactions WHERE account_id = ?1 OR to_account_id = ?1)
                OR EXISTS (SELECT 1 FROM investment_entries WHERE account_id = ?1)
            ",
            [id],
            |row| row.get(0),
        )
        .map_err(|err| Error::other(format!("Failed to check account use: {}", err)))
    }
}

impl AccountStore for SqliteAccountStore {
    fn list_accounts(&self) -> Result<Vec<Account>> {
        let conn = self.ready_connection()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, name, kind, position, archived, class, opening_balance, tracked_from
                FROM accounts
                WHERE ledger_id = ?1
                ORDER BY position, id
                ",
            )
            .map_err(|err| Error::other(format!("Failed to prepare account query: {}", err)))?;

        stmt.query_map([self.ledger_id], Self::row_to_account)
            .map_err(|err| Error::other(format!("Failed to load accounts: {}", err)))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| Error::other(format!("Failed to read accounts: {}", err)))
    }

    fn list_entries(&self) -> Result<Vec<InvestmentEntry>> {
        let conn = self.ready_connection()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT e.id, e.account_id, e.date, e.entry_kind, e.amount, e.note
                FROM investment_entries e
                JOIN accounts a ON a.id = e.account_id
                WHERE a.ledger_id = ?1
                ORDER BY e.account_id, e.date, e.id
                ",
            )
            .map_err(|err| Error::other(format!("Failed to prepare entry query: {}", err)))?;

        stmt.query_map([self.ledger_id], Self::row_to_entry)
            .map_err(|err| Error::other(format!("Failed to load investment entries: {}", err)))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| Error::other(format!("Failed to read investment entries: {}", err)))
    }

    fn create_account(&self, draft: &AccountDraft) -> Result<i64> {
        let name = validate_name(&draft.name)?;
        let conn = self.ready_connection()?;
        let position: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM accounts WHERE ledger_id = ?1",
                [self.ledger_id],
                |row| row.get(0),
            )
            .map_err(|err| Error::other(format!("Failed to position new account: {}", err)))?;

        conn.execute(
            "
            INSERT INTO accounts (
                ledger_id, name, kind, position, archived, class, opening_balance, tracked_from
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                self.ledger_id,
                &name,
                draft.kind.trim(),
                position,
                draft.archived as i64,
                draft.class.as_str(),
                opening_balance(draft).to_string(),
                tracked_from(draft)
            ],
        )
        .map_err(|err| name_conflict(err, &name, "create"))?;

        Ok(conn.last_insert_rowid())
    }

    fn update_account(&self, id: i64, draft: &AccountDraft) -> Result<()> {
        let name = validate_name(&draft.name)?;
        let conn = self.ready_connection()?;
        let class = self.class_of(&conn, id)?;
        if class != draft.class {
            if Self::is_used(&conn, id)? {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "An account's class can't change once it has transactions or entries.",
                ));
            }
            if class.holds_spending() && !draft.class.holds_spending() {
                self.assert_other_spending_account(&conn, id)?;
            }
        }

        conn.execute(
            "
            UPDATE accounts
            SET name = ?1, kind = ?2, archived = ?3, class = ?4, opening_balance = ?5,
                tracked_from = ?6
            WHERE id = ?7 AND ledger_id = ?8
            ",
            params![
                &name,
                draft.kind.trim(),
                draft.archived as i64,
                draft.class.as_str(),
                opening_balance(draft).to_string(),
                tracked_from(draft),
                id,
                self.ledger_id
            ],
        )
        .map_err(|err| name_conflict(err, &name, "rename"))?;
        Ok(())
    }

    fn delete_account(&self, id: i64) -> Result<()> {
        let conn = self.ready_connection()?;
        if self.class_of(&conn, id)?.holds_spending() {
            self.assert_other_spending_account(&conn, id)?;
        }
        let deleted = conn
            .execute(
                "DELETE FROM accounts WHERE id = ?1 AND ledger_id = ?2",
                params![id, self.ledger_id],
            )
            .map_err(|err| match err {
                rusqlite::Error::SqliteFailure(inner, _)
                    if inner.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Error::new(
                        ErrorKind::InvalidInput,
                        "Transactions use this account. Archive it instead.",
                    )
                }
                other => Error::other(format!("Failed to delete account: {}", other)),
            })?;

        if deleted == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Investment account {} was not found.", id),
            ));
        }
        Ok(())
    }

    fn save_entry(&self, draft: &InvestmentEntryDraft) -> Result<i64> {
        let amount = validate_amount(draft.amount, draft.entry_kind)?;
        let mut conn = self.ready_connection()?;
        self.assert_owns_investment(&conn, draft.account_id)?;

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin entry write: {}", err)))?;

        // A partial unique index can't be an ON CONFLICT target, so clear then insert.
        if draft.entry_kind == InvestmentEntryKind::Valuation {
            tx.execute(
                "
                DELETE FROM investment_entries
                WHERE account_id = ?1 AND date = ?2 AND entry_kind = 'Valuation'
                ",
                params![draft.account_id, draft.date.format(DATE_FORMAT).to_string()],
            )
            .map_err(|err| Error::other(format!("Failed to replace valuation: {}", err)))?;
        }

        tx.execute(
            "
            INSERT INTO investment_entries (account_id, date, entry_kind, amount, note)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                draft.account_id,
                draft.date.format(DATE_FORMAT).to_string(),
                draft.entry_kind.as_str(),
                amount.to_string(),
                draft.note.trim()
            ],
        )
        .map_err(|err| Error::other(format!("Failed to save investment entry: {}", err)))?;

        let id = tx.last_insert_rowid();
        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit entry write: {}", err)))?;
        Ok(id)
    }

    fn update_entry(&self, id: i64, draft: &InvestmentEntryDraft) -> Result<()> {
        let amount = validate_amount(draft.amount, draft.entry_kind)?;
        let mut conn = self.ready_connection()?;
        self.assert_owns_investment(&conn, draft.account_id)?;

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin entry update: {}", err)))?;

        if draft.entry_kind == InvestmentEntryKind::Valuation {
            tx.execute(
                "
                DELETE FROM investment_entries
                WHERE account_id = ?1 AND date = ?2 AND entry_kind = 'Valuation' AND id != ?3
                ",
                params![
                    draft.account_id,
                    draft.date.format(DATE_FORMAT).to_string(),
                    id
                ],
            )
            .map_err(|err| Error::other(format!("Failed to replace valuation: {}", err)))?;
        }

        let updated = tx
            .execute(
                "
                UPDATE investment_entries
                SET account_id = ?1, date = ?2, entry_kind = ?3, amount = ?4, note = ?5
                WHERE id = ?6 AND account_id IN (
                    SELECT id FROM accounts WHERE ledger_id = ?7
                )
                ",
                params![
                    draft.account_id,
                    draft.date.format(DATE_FORMAT).to_string(),
                    draft.entry_kind.as_str(),
                    amount.to_string(),
                    draft.note.trim(),
                    id,
                    self.ledger_id
                ],
            )
            .map_err(|err| Error::other(format!("Failed to update investment entry: {}", err)))?;

        if updated == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Investment entry {} was not found.", id),
            ));
        }

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit entry update: {}", err)))
    }

    fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.ready_connection()?;
        let deleted = conn
            .execute(
                "
                DELETE FROM investment_entries
                WHERE id = ?1 AND account_id IN (
                    SELECT id FROM accounts WHERE ledger_id = ?2
                )
                ",
                params![id, self.ledger_id],
            )
            .map_err(|err| Error::other(format!("Failed to delete investment entry: {}", err)))?;

        if deleted == 0 {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Investment entry {} was not found.", id),
            ));
        }
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Account name cannot be empty.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Zero is a real valuation (you sold out) but never a real flow.
fn validate_amount(amount: Decimal, kind: InvestmentEntryKind) -> Result<Decimal> {
    if amount < Decimal::ZERO {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Amount cannot be negative. Use a withdrawal to take money out.",
        ));
    }
    if amount.is_zero() && kind != InvestmentEntryKind::Valuation {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("A {} needs an amount.", kind.as_str().to_lowercase()),
        ));
    }
    Ok(amount)
}

pub(crate) fn account_class(conn: &Connection, ledger_id: i64, id: i64) -> Result<AccountClass> {
    let label: String = conn
        .query_row(
            "SELECT class FROM accounts WHERE id = ?1 AND ledger_id = ?2",
            params![id, ledger_id],
            |row| row.get(0),
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Error::new(
                ErrorKind::NotFound,
                format!("Account {} was not found in this ledger.", id),
            ),
            other => Error::other(format!("Failed to read account: {}", other)),
        })?;
    AccountClass::from_label(&label).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Invalid account class '{}' in database.", label),
        )
    })
}

fn opening_balance(draft: &AccountDraft) -> Decimal {
    match draft.class {
        AccountClass::Investment => Decimal::ZERO,
        AccountClass::Cash | AccountClass::Credit => draft.opening_balance.normalize(),
    }
}

fn tracked_from(draft: &AccountDraft) -> Option<String> {
    draft
        .tracked_from
        .map(|date| date.format(DATE_FORMAT).to_string())
}

fn parse_class(index: usize, value: &str) -> rusqlite::Result<AccountClass> {
    AccountClass::from_label(value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid account class '{}' in database.", value),
            )),
        )
    })
}

fn name_conflict(err: rusqlite::Error, name: &str, action: &str) -> Error {
    match err {
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Error::new(
                ErrorKind::AlreadyExists,
                format!("An account named '{}' already exists.", name),
            )
        }
        other => Error::other(format!("Failed to {} account: {}", action, other)),
    }
}

fn parse_date(index: usize, value: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(value, DATE_FORMAT).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid date '{}' in investment database: {}", value, err),
            )),
        )
    })
}

fn parse_decimal(index: usize, value: &str) -> rusqlite::Result<Decimal> {
    Decimal::from_str(value.trim()).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid amount '{}' in investment database: {}", value, err),
            )),
        )
    })
}

fn parse_kind(index: usize, value: &str) -> rusqlite::Result<InvestmentEntryKind> {
    InvestmentEntryKind::from_label(value).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid entry kind '{}' in investment database.", value),
            )),
        )
    })
}
