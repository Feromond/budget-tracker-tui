//! CSV serialization: transaction import/export (used by the one-time migration and the
//! Import/Export actions) and parsing of the embedded category seed used to initialize the
//! database. The database itself is the persistence layer; this module only handles CSV.
use crate::model::{
    Accounts, CategoryInfo, DATE_FORMAT, RecurrenceFrequency, Transaction, TransactionType,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs::{File, create_dir_all};
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::result::Result as StdResult;

/// CSV rows use account names so exports work across ledgers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CsvTransaction {
    #[serde(deserialize_with = "deserialize_flexible_date")]
    #[serde(serialize_with = "serialize_date")]
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub subcategory: String,
    #[serde(default)]
    pub is_recurring: bool,
    #[serde(default)]
    pub recurrence_frequency: Option<RecurrenceFrequency>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_date")]
    #[serde(serialize_with = "serialize_optional_date")]
    pub recurrence_end_date: Option<NaiveDate>,
    #[serde(default)]
    pub is_generated_from_recurring: bool,
    /// Blank means the ledger's default account.
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub to_account: String,
}

impl Accounts {
    pub(crate) fn csv_row(&self, tx: &Transaction) -> CsvTransaction {
        CsvTransaction {
            date: tx.date,
            description: tx.description.clone(),
            amount: tx.amount,
            transaction_type: tx.transaction_type,
            category: tx.category.clone(),
            subcategory: tx.subcategory.clone(),
            is_recurring: tx.is_recurring,
            recurrence_frequency: tx.recurrence_frequency,
            recurrence_end_date: tx.recurrence_end_date,
            is_generated_from_recurring: tx.is_generated_from_recurring,
            account: self.name(tx.account_id).to_string(),
            to_account: tx
                .to_account_id
                .map(|id| self.name(id).to_string())
                .unwrap_or_default(),
        }
    }

    pub(crate) fn link_csv(&self, row: &CsvTransaction) -> StdResult<Transaction, String> {
        let resolve = |name: &str| {
            self.named(name).map(|account| account.id).ok_or_else(|| {
                format!(
                    "No account named '{}' in this ledger. Add it in Settings, then import again.",
                    name.trim()
                )
            })
        };
        let account_id = if row.account.trim().is_empty() {
            self.default_id()
                .ok_or("This ledger has no cash or credit account to import into.")?
        } else {
            resolve(&row.account)?
        };
        let to_account_id = match row.transaction_type {
            TransactionType::Transfer if row.to_account.trim().is_empty() => {
                return Err(format!(
                    "The transfer '{}' on {} has no to_account.",
                    row.description,
                    row.date.format(DATE_FORMAT)
                ));
            }
            TransactionType::Transfer => Some(resolve(&row.to_account)?),
            TransactionType::Income | TransactionType::Expense => None,
        };

        Ok(Transaction {
            date: row.date,
            description: row.description.clone(),
            amount: row.amount,
            transaction_type: row.transaction_type,
            category: row.category.clone(),
            subcategory: row.subcategory.clone(),
            is_recurring: row.is_recurring,
            recurrence_frequency: row.recurrence_frequency,
            recurrence_end_date: row.recurrence_end_date,
            is_generated_from_recurring: row.is_generated_from_recurring,
            id: None,
            parent_id: None,
            account_id,
            to_account_id,
        })
    }
}

fn deserialize_flexible_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Ok(date) = NaiveDate::parse_from_str(&s, DATE_FORMAT) {
        return Ok(date);
    }
    if let Ok(date) = NaiveDate::parse_from_str(&s, "%Y/%m/%d") {
        return Ok(date);
    }
    if let Ok(date) = NaiveDate::parse_from_str(&s, "%d/%m/%Y") {
        return Ok(date);
    }
    if let Ok(date) = NaiveDate::parse_from_str(&s, "%d-%m-%Y") {
        return Ok(date);
    }
    Err(SerdeError::custom(format!(
        "Invalid date format: '{}'. Expected YYYY-MM-DD, YYYY/MM/DD, DD/MM/YYYY, or DD-MM-YYYY.",
        s
    )))
}

fn default_category() -> String {
    "Uncategorized".to_string()
}

fn deserialize_optional_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            deserialize_flexible_date(serde::de::value::StrDeserializer::new(&s)).map(Some)
        }
        _ => Ok(None),
    }
}

fn serialize_optional_date<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match date {
        Some(d) => serializer.serialize_str(&d.format(DATE_FORMAT).to_string()),
        None => serializer.serialize_str(""),
    }
}

fn serialize_date<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&date.format(DATE_FORMAT).to_string())
}

pub(crate) fn load_transactions(data_path: &Path) -> StdResult<Vec<CsvTransaction>, Error> {
    if !data_path.exists() {
        if let Some(parent) = data_path.parent() {
            create_dir_all(parent)?;
        }
        return Ok(vec![]);
    }

    let file = File::open(data_path)?;
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(file);
    let mut transactions = Vec::new();
    let headers = rdr
        .headers()
        .map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Failed to read headers from {}: {}", data_path.display(), e),
            )
        })?
        .clone();

    for (index, result) in rdr.deserialize().enumerate() {
        let transaction: CsvTransaction = result.map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Failed to parse transaction at row {} in {}: {}. Headers: {:?}",
                    index + 2,
                    data_path.display(),
                    e,
                    headers
                ),
            )
        })?;
        transactions.push(transaction);
    }
    Ok(transactions)
}

pub(crate) fn save_transactions(
    transactions: &[CsvTransaction],
    data_path: &Path,
) -> StdResult<(), Error> {
    if let Some(parent) = data_path.parent() {
        create_dir_all(parent)?;
    }

    let file = File::create(data_path)?;
    let mut wtr = csv::Writer::from_writer(file);
    for transaction in transactions {
        wtr.serialize(transaction).map_err(|e| {
            Error::other(format!(
                "Failed to serialize transaction {:?} to {}: {}",
                transaction,
                data_path.display(),
                e
            ))
        })?;
    }
    wtr.flush()?;
    Ok(())
}

/// Parse the embedded category list used to seed the database on first run.
pub(crate) fn load_seed_categories() -> StdResult<Vec<CategoryInfo>, Error> {
    Ok(parse_seed_categories()?
        .into_iter()
        .map(|(category, _)| category)
        .collect())
}

pub(crate) fn seed_categories_added_after(version: u32) -> StdResult<Vec<CategoryInfo>, Error> {
    Ok(parse_seed_categories()?
        .into_iter()
        .filter(|(_, since)| *since > version)
        .map(|(category, _)| category)
        .collect())
}

/// A blank `Since` means version 1.
fn parse_seed_categories() -> StdResult<Vec<(CategoryInfo, u32)>, Error> {
    let embedded_csv_data = include_str!("../budget_categories.csv");

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(embedded_csv_data.as_bytes());
    let mut categories = Vec::new();
    let mut header_error = None;

    let headers = rdr.headers()?.clone();
    let type_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("Type"))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "Embedded category data missing required header: Type",
            )
        })?;
    let cat_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("Category"))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "Embedded category data missing required header: Category",
            )
        })?;
    let subcat_idx = headers
        .iter()
        .position(|h| h.eq_ignore_ascii_case("Subcategory"))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                "Embedded category data missing required header: Subcategory",
            )
        })?;
    let since_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("Since"));

    for (index, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Failed to read record at row {} from embedded category data: {}",
                    index + 1,
                    e
                ),
            )
        })?;

        let type_str = record
            .get(type_idx)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Missing Type at row {} in embedded data", index + 1),
                )
            })?
            .trim();
        let cat_str = record
            .get(cat_idx)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Missing Category at row {} in embedded data", index + 1),
                )
            })?
            .trim();
        let subcat_str = record
            .get(subcat_idx)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Missing Subcategory at row {} in embedded data", index + 1),
                )
            })?
            .trim();

        let transaction_type = match type_str {
            t if t.eq_ignore_ascii_case("Income") => TransactionType::Income,
            t if t.eq_ignore_ascii_case("Expense") => TransactionType::Expense,
            t if t.eq_ignore_ascii_case("Transfer") => TransactionType::Transfer,
            _ => {
                if header_error.is_none() {
                    header_error = Some(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Invalid Type '{}' at row {} in embedded data",
                            type_str,
                            index + 1
                        ),
                    ));
                }
                continue;
            }
        };

        if cat_str.is_empty() {
            continue;
        }

        let since = match since_idx.and_then(|idx| record.get(idx)).map(str::trim) {
            None | Some("") => 1,
            Some(value) => value.parse().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Invalid Since '{}' at row {} in embedded data",
                        value,
                        index + 1
                    ),
                )
            })?,
        };

        categories.push((
            CategoryInfo {
                transaction_type,
                category: cat_str.to_string(),
                subcategory: subcat_str.to_string(),
            },
            since,
        ));
    }

    if let Some(err) = header_error {
        Err(err)
    } else {
        Ok(categories)
    }
}
