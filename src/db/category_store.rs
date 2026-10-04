use crate::db::database::SqliteDatabase;
use crate::model::{CategoryDraft, CategoryInfo, CategoryRecord, TransactionType};
use rusqlite::{Connection, Row, params};
use std::io::{Error, ErrorKind, Result};

/// Keep in sync with the highest `Since` in `budget_categories.csv`.
pub(crate) const CATEGORY_SEED_VERSION: u32 = 2;

/// The category catalog is shared by every ledger, so updating or deleting a category re-points
/// the matching transactions across all of them, in one transaction with the catalog write.
pub trait CategoryStore {
    fn initialize(&self, seed_categories: &[CategoryInfo]) -> Result<()>;
    fn list(&self) -> Result<Vec<CategoryRecord>>;
    fn insert(&self, draft: &CategoryDraft) -> Result<CategoryRecord>;
    fn update(&self, id: i64, draft: &CategoryDraft) -> Result<()>;
    fn delete(&self, id: i64) -> Result<()>;
}

pub struct SqliteCategoryStore {
    database: SqliteDatabase,
}

impl SqliteCategoryStore {
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    fn ready_connection(&self) -> Result<Connection> {
        self.database.ready_connection("category")
    }

    /// On upgrade, add new defaults without restoring edited or deleted categories.
    fn seed(&self, conn: &mut Connection, seed_categories: &[CategoryInfo]) -> Result<()> {
        let seeded = self
            .database
            .metadata_value(conn, "category_seed_version")?;
        let new_rows: Vec<CategoryInfo> = match seeded {
            None => seed_categories.to_vec(),
            Some(value) => match value.trim().parse::<u32>() {
                Ok(version) if version < CATEGORY_SEED_VERSION => {
                    crate::csv_io::seed_categories_added_after(version)?
                }
                _ => return Ok(()),
            },
        };

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin category seed: {}", err)))?;
        {
            let mut stmt = tx
                .prepare(
                    "
                    INSERT INTO categories (transaction_type, category, subcategory, tag)
                    SELECT ?1, ?2, ?3, NULL
                    WHERE NOT EXISTS (
                        SELECT 1 FROM categories
                        WHERE transaction_type = ?1
                          AND LOWER(category) = LOWER(?2)
                          AND LOWER(subcategory) = LOWER(?3)
                    )
                    ",
                )
                .map_err(|err| {
                    Error::other(format!("Failed to prepare category seed insert: {}", err))
                })?;
            for category in new_rows {
                stmt.execute(params![
                    category.transaction_type.as_str(),
                    &category.category,
                    &category.subcategory
                ])
                .map_err(|err| Error::other(format!("Failed to seed categories: {}", err)))?;
            }
        }
        self.database.set_metadata_value(
            &tx,
            "category_seed_version",
            &CATEGORY_SEED_VERSION.to_string(),
        )?;
        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit category seed: {}", err)))
    }

    fn load_record_by_id(conn: &Connection, id: i64) -> Result<CategoryRecord> {
        conn.query_row(
            "
            SELECT id, transaction_type, category, subcategory, tag
            FROM categories
            WHERE id = ?1
            ",
            [id],
            Self::row_to_record,
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Error::new(
                ErrorKind::NotFound,
                format!("Category with id {} was not found.", id),
            ),
            other => Error::other(format!("Failed to load category {}: {}", id, other)),
        })
    }

    fn row_to_record(row: &Row<'_>) -> rusqlite::Result<CategoryRecord> {
        let transaction_type_str: String = row.get(1)?;

        let transaction_type =
            TransactionType::try_from(transaction_type_str.as_str()).map_err(|_| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Invalid transaction type '{}' in category database.",
                            transaction_type_str
                        ),
                    )),
                )
            })?;

        Ok(CategoryRecord {
            id: row.get(0)?,
            transaction_type,
            category: row.get(2)?,
            subcategory: row.get(3)?,
            tag: row.get(4)?,
        })
    }
}

impl CategoryStore for SqliteCategoryStore {
    fn initialize(&self, seed_categories: &[CategoryInfo]) -> Result<()> {
        let mut conn = self.ready_connection()?;
        self.seed(&mut conn, seed_categories)
    }

    fn list(&self) -> Result<Vec<CategoryRecord>> {
        let conn = self.ready_connection()?;

        let mut stmt = conn
            .prepare(
                "
                SELECT id, transaction_type, category, subcategory, tag
                FROM categories
                ORDER BY
                    CASE transaction_type
                        WHEN 'Income' THEN 0
                        ELSE 1
                    END,
                    LOWER(category),
                    LOWER(subcategory),
                    id
                ",
            )
            .map_err(|err| Error::other(format!("Failed to prepare category query: {}", err)))?;

        let rows = stmt
            .query_map([], Self::row_to_record)
            .map_err(|err| Error::other(format!("Failed to load categories: {}", err)))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| Error::other(format!("Failed to read categories: {}", err)))
    }

    fn insert(&self, draft: &CategoryDraft) -> Result<CategoryRecord> {
        let conn = self.ready_connection()?;

        conn.execute(
            "
            INSERT INTO categories (
                transaction_type,
                category,
                subcategory,
                tag
            ) VALUES (?1, ?2, ?3, ?4)
            ",
            params![
                draft.transaction_type.as_str(),
                &draft.category,
                &draft.subcategory,
                &draft.tag
            ],
        )
        .map_err(|err| Error::other(format!("Failed to insert category: {}", err)))?;

        Self::load_record_by_id(&conn, conn.last_insert_rowid())
    }

    fn update(&self, id: i64, draft: &CategoryDraft) -> Result<()> {
        let mut conn = self.ready_connection()?;
        let previous = Self::load_record_by_id(&conn, id)?;

        let crosses_transfer = previous.transaction_type != draft.transaction_type
            && (previous.transaction_type == TransactionType::Transfer
                || draft.transaction_type == TransactionType::Transfer);
        if crosses_transfer {
            let used: i64 = conn
                .query_row(
                    "
                    SELECT COUNT(*) FROM transactions
                    WHERE transaction_type = ?1
                      AND LOWER(category) = LOWER(?2)
                      AND LOWER(subcategory) = LOWER(?3)
                    ",
                    params![
                        previous.transaction_type.as_str(),
                        &previous.category,
                        &previous.subcategory
                    ],
                    |row| row.get(0),
                )
                .map_err(|err| Error::other(format!("Failed to check category use: {}", err)))?;
            if used > 0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "{} transaction{} use this category, so its type can't switch to or from Transfer. Convert them to transfers instead.",
                        used,
                        if used == 1 { "" } else { "s" }
                    ),
                ));
            }
        }

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin category update: {}", err)))?;

        tx.execute(
            "
            UPDATE categories
            SET
                transaction_type = ?1,
                category = ?2,
                subcategory = ?3,
                tag = ?4
            WHERE id = ?5
            ",
            params![
                draft.transaction_type.as_str(),
                &draft.category,
                &draft.subcategory,
                &draft.tag,
                id
            ],
        )
        .map_err(|err| Error::other(format!("Failed to update category: {}", err)))?;

        // Expense-only budgets, and the type is shared by every ledger, so drop them all.
        if draft.transaction_type != TransactionType::Expense {
            tx.execute("DELETE FROM budget_periods WHERE category_id = ?1", [id])
                .map_err(|err| {
                    Error::other(format!("Failed to clear category budgets: {}", err))
                })?;
        }

        tx.execute(
            "
            UPDATE transactions
            SET transaction_type = ?1, category = ?2, subcategory = ?3
            WHERE transaction_type = ?4
              AND LOWER(category) = LOWER(?5)
              AND LOWER(subcategory) = LOWER(?6)
            ",
            params![
                draft.transaction_type.as_str(),
                &draft.category,
                &draft.subcategory,
                previous.transaction_type.as_str(),
                &previous.category,
                &previous.subcategory,
            ],
        )
        .map_err(|err| {
            Error::other(format!(
                "Failed to update transactions for category: {}",
                err
            ))
        })?;

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit category update: {}", err)))
    }

    fn delete(&self, id: i64) -> Result<()> {
        let mut conn = self.ready_connection()?;
        let record = Self::load_record_by_id(&conn, id)?;

        // Deleting a top-level category resets its transactions to Uncategorized; deleting a
        // subcategory only clears the subcategory field.
        let set_clause = if record.subcategory.is_empty() {
            "category = 'Uncategorized', subcategory = ''"
        } else {
            "subcategory = ''"
        };

        let tx = conn
            .transaction()
            .map_err(|err| Error::other(format!("Failed to begin category delete: {}", err)))?;

        tx.execute("DELETE FROM categories WHERE id = ?1", [id])
            .map_err(|err| Error::other(format!("Failed to delete category: {}", err)))?;

        tx.execute(
            &format!(
                "
                UPDATE transactions
                SET {}
                WHERE transaction_type = ?1
                  AND LOWER(category) = LOWER(?2)
                  AND LOWER(subcategory) = LOWER(?3)
                ",
                set_clause
            ),
            params![
                record.transaction_type.as_str(),
                &record.category,
                &record.subcategory,
            ],
        )
        .map_err(|err| {
            Error::other(format!(
                "Failed to clear transactions for category: {}",
                err
            ))
        })?;

        tx.commit()
            .map_err(|err| Error::other(format!("Failed to commit category delete: {}", err)))
    }
}
