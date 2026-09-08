use chrono::{Datelike, Duration, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt;

use serde::Deserializer;
use serde::de::Error as SerdeError;

pub(crate) const DATE_FORMAT: &str = "%Y-%m-%d";

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub enum TransactionType {
    Income,
    Expense,
}

impl TransactionType {
    pub fn as_str(self) -> &'static str {
        match self {
            TransactionType::Income => "Income",
            TransactionType::Expense => "Expense",
        }
    }
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for TransactionType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "income" => Ok(TransactionType::Income),
            "expense" => Ok(TransactionType::Expense),
            t if t.starts_with('i') => Ok(TransactionType::Income),
            t if t.starts_with('e') => Ok(TransactionType::Expense),
            _ => Err(()),
        }
    }
}

impl<'de> Deserialize<'de> for TransactionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TransactionType::try_from(s.as_str()).map_err(|_| {
            SerdeError::custom(format!(
                "Invalid transaction type: '{}'. Expected 'Income', 'Expense', 'i', or 'e'.",
                s
            ))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    BiWeekly,
    SemiMonthly,
    SemiMonthlyWorkday,
    Monthly,
    Quarterly,
    Yearly,
}

impl RecurrenceFrequency {
    pub fn to_string(self) -> &'static str {
        match self {
            RecurrenceFrequency::Daily => "Daily",
            RecurrenceFrequency::Weekly => "Weekly",
            RecurrenceFrequency::BiWeekly => "Bi-Weekly",
            RecurrenceFrequency::SemiMonthly => "Semi-Monthly",
            RecurrenceFrequency::SemiMonthlyWorkday => "Semi-Monthly (Weekday Adjusted)",
            RecurrenceFrequency::Monthly => "Monthly",
            RecurrenceFrequency::Quarterly => "Quarterly",
            RecurrenceFrequency::Yearly => "Yearly",
        }
    }

    /// Parse a frequency from its display label (e.g. "Bi-Weekly"). Used for both the
    /// recurring-settings form and database round-tripping.
    pub fn from_label(label: &str) -> Option<RecurrenceFrequency> {
        match label {
            "Daily" => Some(RecurrenceFrequency::Daily),
            "Weekly" => Some(RecurrenceFrequency::Weekly),
            "Bi-Weekly" => Some(RecurrenceFrequency::BiWeekly),
            "Semi-Monthly" => Some(RecurrenceFrequency::SemiMonthly),
            "Semi-Monthly (Weekday Adjusted)" => Some(RecurrenceFrequency::SemiMonthlyWorkday),
            "Monthly" => Some(RecurrenceFrequency::Monthly),
            "Quarterly" => Some(RecurrenceFrequency::Quarterly),
            "Yearly" => Some(RecurrenceFrequency::Yearly),
            _ => None,
        }
    }

    pub fn all() -> Vec<RecurrenceFrequency> {
        vec![
            RecurrenceFrequency::Daily,
            RecurrenceFrequency::Weekly,
            RecurrenceFrequency::BiWeekly,
            RecurrenceFrequency::SemiMonthly,
            RecurrenceFrequency::SemiMonthlyWorkday,
            RecurrenceFrequency::Monthly,
            RecurrenceFrequency::Quarterly,
            RecurrenceFrequency::Yearly,
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    // Apply the custom deserializer for reading dates so it can work on excel edits
    #[serde(deserialize_with = "deserialize_flexible_date")]
    #[serde(serialize_with = "date_format::serialize")]
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub subcategory: String,
    // Recurring transaction fields
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
    // Database identity. Excluded from CSV (import/export stay byte-compatible).
    // `id` is set for persisted (real) rows and None for in-memory-only generated rows.
    #[serde(skip)]
    pub id: Option<i64>,
    // In-memory only: the source row's id, stamped onto generated occurrences so we can
    // jump back to the source without fragile attribute matching. Never a DB column.
    #[serde(skip)]
    pub parent_id: Option<i64>,
}

impl Transaction {
    /// Build a database draft (the real-row fields stored in the `transactions` table) from a
    /// transaction. Drops `id`, the generated flag, and the in-memory `parent_id`.
    pub fn to_draft(&self) -> TransactionDraft {
        TransactionDraft {
            date: self.date,
            description: self.description.clone(),
            amount: self.amount,
            transaction_type: self.transaction_type,
            category: self.category.clone(),
            subcategory: self.subcategory.clone(),
            is_recurring: self.is_recurring,
            recurrence_frequency: self.recurrence_frequency,
            recurrence_end_date: self.recurrence_end_date,
        }
    }
}

/// Fields persisted for a real transaction row (regular transactions + recurring sources).
/// Generated occurrences are never stored, so there is no generated flag or parent link here.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionDraft {
    pub date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub category: String,
    pub subcategory: String,
    pub is_recurring: bool,
    pub recurrence_frequency: Option<RecurrenceFrequency>,
    pub recurrence_end_date: Option<NaiveDate>,
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

pub mod date_format {
    use chrono::NaiveDate;
    use serde::{self, Serializer};

    const FORMAT: &str = super::DATE_FORMAT;

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum SortColumn {
    Date,
    Description,
    Amount,
    Type,
    Category,
    Subcategory,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// `amount: None` clears the budget from that month on, which is not the same as having
/// no period. `category_id: None` is the ledger's monthly budget, not a category budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetPeriod {
    pub id: i64,
    pub category_id: Option<i64>,
    pub start: BudgetMonth,
    pub amount: Option<Decimal>,
}

/// Field order matters: it is what makes the earliest start sort first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BudgetMonth {
    pub year: i32,
    pub month: u32,
}

impl BudgetMonth {
    /// Sorts before every real month, so a period seeded here covers all history.
    pub const BEGINNING: Self = Self { year: 0, month: 1 };

    pub fn new(year: i32, month: u32) -> Self {
        Self { year, month }
    }

    pub fn next(self) -> Self {
        if self.month >= 12 {
            Self::new(self.year + 1, 1)
        } else {
            Self::new(self.year, self.month + 1)
        }
    }
}

/// How far a budget edit reaches. `RemoveChange` is only offered when a period starts
/// exactly at the edited month, since that is the only thing there is to undo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetEditScope {
    FromThisMonth,
    ThisMonthOnly,
    ReplaceAllMonths,
    RemoveChange,
}

/// A store operation an edit resolves to. Keeping the decision separate from the writing
/// is what lets the scope rules be tested without a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetWrite {
    Set(BudgetMonth, Option<Decimal>),
    Remove(BudgetMonth),
    RemoveAll,
}

/// Owns the rule for reading a value out of the periods, so the UI never walks them.
#[derive(Debug, Default, Clone)]
pub struct BudgetSchedule {
    periods: Vec<BudgetPeriod>,
}

impl BudgetSchedule {
    pub fn new(mut periods: Vec<BudgetPeriod>) -> Self {
        periods.sort_by_key(|period| period.start);
        Self { periods }
    }

    /// The latest period starting on or before `month`. `None` means no budget, whether
    /// none was ever set or one cleared it.
    pub fn amount_for(&self, category_id: Option<i64>, month: BudgetMonth) -> Option<Decimal> {
        self.periods
            .iter()
            .rfind(|period| period.category_id == category_id && period.start <= month)
            .and_then(|period| period.amount)
    }

    pub fn monthly_budget(&self, month: BudgetMonth) -> Option<Decimal> {
        self.amount_for(None, month)
    }

    pub fn category_budget(&self, category_id: i64, month: BudgetMonth) -> Option<Decimal> {
        self.amount_for(Some(category_id), month)
    }

    /// Resolve an edit into the writes that carry it out. Pure, so the ordering rule that
    /// makes `ThisMonthOnly` work is checked by tests rather than by inspection.
    pub fn plan_edit(
        &self,
        category_id: Option<i64>,
        start: BudgetMonth,
        amount: Option<Decimal>,
        scope: BudgetEditScope,
    ) -> Vec<BudgetWrite> {
        match scope {
            BudgetEditScope::FromThisMonth => vec![BudgetWrite::Set(start, amount)],
            BudgetEditScope::ThisMonthOnly => {
                // Read what the next month inherits now; after the first write it would
                // just report the value being set.
                let inherited = self.amount_for(category_id, start.next());
                vec![
                    BudgetWrite::Set(start, amount),
                    BudgetWrite::Set(start.next(), inherited),
                ]
            }
            BudgetEditScope::ReplaceAllMonths => {
                vec![
                    BudgetWrite::RemoveAll,
                    BudgetWrite::Set(BudgetMonth::BEGINNING, amount),
                ]
            }
            BudgetEditScope::RemoveChange => vec![BudgetWrite::Remove(start)],
        }
    }

    pub fn years(&self) -> Vec<i32> {
        let mut years: Vec<i32> = self
            .periods
            .iter()
            .map(|period| period.start.year)
            .filter(|year| *year > BudgetMonth::BEGINNING.year)
            .collect();
        years.sort_unstable();
        years.dedup();
        years
    }

    /// Lets a destructive edit report how much it replaced.
    pub fn period_count(&self, category_id: Option<i64>) -> usize {
        self.periods
            .iter()
            .filter(|period| period.category_id == category_id)
            .count()
    }

    /// Only a period starting exactly here can be removed.
    pub fn starts_at(&self, category_id: Option<i64>, month: BudgetMonth) -> bool {
        self.periods
            .iter()
            .any(|period| period.category_id == category_id && period.start == month)
    }

    pub fn budgeted_categories(&self, month: BudgetMonth) -> Vec<(i64, Decimal)> {
        let mut ids: Vec<i64> = self
            .periods
            .iter()
            .filter_map(|period| period.category_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|id| self.category_budget(id, month).map(|amount| (id, amount)))
            .collect()
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum CategorySortColumn {
    Type,
    Category,
    Subcategory,
    Tag,
    TargetBudget,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MonthlySummary {
    pub income: Decimal,
    pub expense: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryInfo {
    pub transaction_type: TransactionType,
    pub category: String,
    pub subcategory: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryDraft {
    pub transaction_type: TransactionType,
    pub category: String,
    pub subcategory: String,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRecord {
    pub id: i64,
    pub transaction_type: TransactionType,
    pub category: String,
    pub subcategory: String,
    pub tag: Option<String>,
}

impl CategoryRecord {
    pub fn to_category_info(&self) -> CategoryInfo {
        CategoryInfo {
            transaction_type: self.transaction_type,
            category: self.category.clone(),
            subcategory: self.subcategory.clone(),
        }
    }

    pub fn to_draft(&self) -> CategoryDraft {
        CategoryDraft {
            transaction_type: self.transaction_type,
            category: self.category.clone(),
            subcategory: self.subcategory.clone(),
            tag: self.tag.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvestmentEntryKind {
    Valuation,
    Contribution,
    Withdrawal,
}

impl InvestmentEntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            InvestmentEntryKind::Valuation => "Valuation",
            InvestmentEntryKind::Contribution => "Contribution",
            InvestmentEntryKind::Withdrawal => "Withdrawal",
        }
    }

    pub fn from_label(label: &str) -> Option<InvestmentEntryKind> {
        match label {
            "Valuation" => Some(InvestmentEntryKind::Valuation),
            "Contribution" => Some(InvestmentEntryKind::Contribution),
            "Withdrawal" => Some(InvestmentEntryKind::Withdrawal),
            _ => None,
        }
    }

    pub fn all() -> [InvestmentEntryKind; 3] {
        [
            InvestmentEntryKind::Valuation,
            InvestmentEntryKind::Contribution,
            InvestmentEntryKind::Withdrawal,
        ]
    }

    pub fn flow_sign(self) -> Decimal {
        match self {
            InvestmentEntryKind::Valuation => Decimal::ZERO,
            InvestmentEntryKind::Contribution => Decimal::ONE,
            InvestmentEntryKind::Withdrawal => Decimal::NEGATIVE_ONE,
        }
    }
}

impl fmt::Display for InvestmentEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvestmentAccount {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub position: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvestmentAccountDraft {
    pub name: String,
    pub kind: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvestmentEntry {
    pub id: i64,
    pub account_id: i64,
    pub date: NaiveDate,
    pub entry_kind: InvestmentEntryKind,
    pub amount: Decimal,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvestmentEntryDraft {
    pub account_id: i64,
    pub date: NaiveDate,
    pub entry_kind: InvestmentEntryKind,
    pub amount: Decimal,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvestmentRange {
    Ytd,
    OneYear,
    ThreeYears,
    FiveYears,
    All,
}

impl InvestmentRange {
    pub fn label(self) -> &'static str {
        match self {
            InvestmentRange::Ytd => "YTD",
            InvestmentRange::OneYear => "1Y",
            InvestmentRange::ThreeYears => "3Y",
            InvestmentRange::FiveYears => "5Y",
            InvestmentRange::All => "All",
        }
    }

    pub fn all() -> [InvestmentRange; 5] {
        [
            InvestmentRange::Ytd,
            InvestmentRange::OneYear,
            InvestmentRange::ThreeYears,
            InvestmentRange::FiveYears,
            InvestmentRange::All,
        ]
    }

    pub fn requested_start(self, today: NaiveDate) -> Option<NaiveDate> {
        match self {
            // Last New Year's Eve, not Jan 1, or a move on Jan 1 falls outside the year.
            InvestmentRange::Ytd => NaiveDate::from_ymd_opt(today.year() - 1, 12, 31),
            InvestmentRange::OneYear => Some(crate::validation::add_months(today, -12)),
            InvestmentRange::ThreeYears => Some(crate::validation::add_months(today, -36)),
            InvestmentRange::FiveYears => Some(crate::validation::add_months(today, -60)),
            InvestmentRange::All => None,
        }
    }

    pub fn start(self, today: NaiveDate, earliest: Option<NaiveDate>) -> NaiveDate {
        let floor = earliest.unwrap_or(today);
        match self.requested_start(today) {
            Some(requested) => requested.max(floor),
            None => floor,
        }
    }

    pub fn crops(self, today: NaiveDate, earliest: Option<NaiveDate>) -> bool {
        match (self.requested_start(today), earliest) {
            (Some(requested), Some(first)) => requested > first,
            _ => false,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Portfolio {
    accounts: Vec<InvestmentAccount>,
    entries: Vec<InvestmentEntry>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InvestmentPosition {
    pub value: Decimal,
    pub invested: Decimal,
    pub as_of: Option<NaiveDate>,
}

impl InvestmentPosition {
    pub fn gain(&self) -> Decimal {
        self.value - self.invested
    }

    pub fn roi(&self) -> Option<Decimal> {
        (self.invested > Decimal::ZERO).then(|| (self.gain() / self.invested) * Decimal::from(100))
    }
}

impl Portfolio {
    pub fn new(mut accounts: Vec<InvestmentAccount>, mut entries: Vec<InvestmentEntry>) -> Self {
        accounts.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
        entries.sort_by(|a, b| {
            a.account_id
                .cmp(&b.account_id)
                .then_with(|| a.date.cmp(&b.date))
        });
        Self { accounts, entries }
    }

    pub fn visible_accounts(&self, show_archived: bool) -> Vec<&InvestmentAccount> {
        self.accounts
            .iter()
            .filter(|account| show_archived || !account.archived)
            .collect()
    }

    pub fn account(&self, id: i64) -> Option<&InvestmentAccount> {
        self.accounts.iter().find(|account| account.id == id)
    }

    pub fn entries_for(&self, account_id: i64) -> impl Iterator<Item = &InvestmentEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.account_id == account_id)
    }

    pub fn earliest_date(&self, account_id: Option<i64>, show_archived: bool) -> Option<NaiveDate> {
        let ids = self.target_ids(account_id, show_archived);
        self.entries
            .iter()
            .filter(|entry| ids.contains(&entry.account_id))
            .map(|entry| entry.date)
            .min()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    fn last_valuation(&self, account_id: i64, date: NaiveDate) -> Option<&InvestmentEntry> {
        self.entries.iter().rfind(|entry| {
            entry.account_id == account_id
                && entry.entry_kind == InvestmentEntryKind::Valuation
                && entry.date <= date
        })
    }

    pub fn last_valuation_date(&self, account_id: i64) -> Option<NaiveDate> {
        self.entries
            .iter()
            .rfind(|entry| {
                entry.account_id == account_id && entry.entry_kind == InvestmentEntryKind::Valuation
            })
            .map(|entry| entry.date)
    }

    /// `after` is exclusive so a flow never lands in two adjacent periods.
    fn net_flow(&self, account_id: i64, after: Option<NaiveDate>, through: NaiveDate) -> Decimal {
        self.entries
            .iter()
            .filter(|entry| entry.account_id == account_id && entry.date <= through)
            .filter(|entry| after.is_none_or(|bound| entry.date > bound))
            .map(|entry| entry.amount * entry.entry_kind.flow_sign())
            .sum()
    }

    /// Valuations read as end of day, so a same-day contribution is already inside one.
    /// That's what stops it counting twice or looking like growth.
    pub fn value_on(&self, account_id: i64, date: NaiveDate) -> Decimal {
        match self.last_valuation(account_id, date) {
            Some(mark) => mark.amount + self.net_flow(account_id, Some(mark.date), date),
            None => self.net_flow(account_id, None, date),
        }
    }

    pub fn position(&self, account_id: i64, on: NaiveDate) -> InvestmentPosition {
        InvestmentPosition {
            value: self.value_on(account_id, on),
            invested: self.net_flow(account_id, None, on),
            as_of: self.last_valuation_date(account_id),
        }
    }

    /// Oldest mark wins: a total is only as current as its stalest part.
    pub fn total_position(&self, on: NaiveDate, show_archived: bool) -> InvestmentPosition {
        let accounts = self.visible_accounts(show_archived);
        InvestmentPosition {
            value: accounts
                .iter()
                .map(|account| self.value_on(account.id, on))
                .sum(),
            invested: accounts
                .iter()
                .map(|account| self.net_flow(account.id, None, on))
                .sum(),
            as_of: accounts
                .iter()
                .filter_map(|account| self.last_valuation_date(account.id))
                .min(),
        }
    }

    fn first_entry(&self, account_id: i64) -> Option<NaiveDate> {
        self.entries
            .iter()
            .find(|entry| entry.account_id == account_id)
            .map(|entry| entry.date)
    }

    /// An account counts only from its own first entry, never from before you tracked it.
    fn effective_start(&self, account_id: i64, start: NaiveDate) -> NaiveDate {
        match self.first_entry(account_id) {
            Some(first) => start.max(first),
            None => start,
        }
    }

    fn window_starts(
        &self,
        account_id: Option<i64>,
        start: NaiveDate,
        show_archived: bool,
    ) -> Vec<(i64, NaiveDate)> {
        self.target_ids(account_id, show_archived)
            .into_iter()
            .map(|id| (id, self.effective_start(id, start)))
            .collect()
    }

    pub fn gain_between(
        &self,
        account_id: Option<i64>,
        start: NaiveDate,
        end: NaiveDate,
        show_archived: bool,
    ) -> Decimal {
        self.window_starts(account_id, start, show_archived)
            .into_iter()
            .map(|(id, from)| {
                self.value_on(id, end)
                    - self.value_on(id, from)
                    - self.net_flow(id, Some(from), end)
            })
            .sum()
    }

    pub fn invested_between(
        &self,
        account_id: Option<i64>,
        start: NaiveDate,
        end: NaiveDate,
        show_archived: bool,
    ) -> Decimal {
        self.window_starts(account_id, start, show_archived)
            .into_iter()
            .map(|(id, from)| self.net_flow(id, Some(from), end))
            .sum()
    }

    fn target_ids(&self, account_id: Option<i64>, show_archived: bool) -> Vec<i64> {
        match account_id {
            Some(id) => vec![id],
            None => self
                .visible_accounts(show_archived)
                .iter()
                .map(|account| account.id)
                .collect(),
        }
    }

    /// Weights each flow by how long it was actually invested.
    pub fn modified_dietz(
        &self,
        account_id: Option<i64>,
        start: NaiveDate,
        end: NaiveDate,
        show_archived: bool,
    ) -> Option<Decimal> {
        let days = (end - start).num_days();
        if days <= 0 {
            return None;
        }

        let starts = self.window_starts(account_id, start, show_archived);
        let opening: Decimal = starts
            .iter()
            .map(|(id, from)| self.value_on(*id, *from))
            .sum();
        let gain = self.gain_between(account_id, start, end, show_archived);

        let weighted: Decimal = self
            .entries
            .iter()
            .filter(|entry| {
                starts
                    .iter()
                    .any(|(id, from)| *id == entry.account_id && entry.date > *from)
            })
            .filter(|entry| entry.date <= end)
            .map(|entry| {
                let remaining = (end - entry.date).num_days();
                let weight = Decimal::from(remaining) / Decimal::from(days);
                entry.amount * entry.entry_kind.flow_sign() * weight
            })
            .sum();

        let base = opening + weighted;
        (base > Decimal::ZERO).then(|| (gain / base) * Decimal::from(100))
    }

    pub fn annualized_return(
        &self,
        account_id: Option<i64>,
        start: NaiveDate,
        end: NaiveDate,
        show_archived: bool,
    ) -> Option<f64> {
        let years = (end - start).num_days() as f64 / 365.25;
        if years <= 0.0 {
            return None;
        }

        // Periods butt up at each year end, or a move on Jan 1 falls through the gap.
        let mut compounded = 1.0f64;
        let mut measured = false;
        let mut period_start = start;
        for year in start.year()..=end.year() {
            let period_end = NaiveDate::from_ymd_opt(year, 12, 31)
                .unwrap_or(end)
                .min(end);
            if period_end <= period_start {
                continue;
            }
            if let Some(period) =
                self.modified_dietz(account_id, period_start, period_end, show_archived)
            {
                compounded *= 1.0 + (period.to_f64().unwrap_or(0.0) / 100.0);
                measured = true;
            }
            period_start = period_end;
        }

        // Zero is a real wipeout that both formulas below report as -100%.
        if !measured || compounded < 0.0 {
            return None;
        }
        // Annualizing a few weeks out to a year reads as a wild claim.
        if years < 1.0 {
            return Some((compounded - 1.0) * 100.0);
        }
        Some((compounded.powf(1.0 / years) - 1.0) * 100.0)
    }

    pub fn series(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        points: usize,
        account_id: Option<i64>,
        show_archived: bool,
    ) -> Vec<(NaiveDate, Decimal, Decimal)> {
        let points = points.max(2);
        let span = (end - start).num_days().max(1);
        let ids = self.target_ids(account_id, show_archived);

        (0..points)
            .map(|step| {
                let offset = span * step as i64 / (points as i64 - 1);
                let date = start + Duration::days(offset);
                let value = ids.iter().map(|id| self.value_on(*id, date)).sum();
                let invested = ids.iter().map(|id| self.net_flow(*id, None, date)).sum();
                (date, value, invested)
            })
            .collect()
    }

    pub fn yearly_breakdown(
        &self,
        account_id: Option<i64>,
        show_archived: bool,
    ) -> Vec<(i32, Decimal, Decimal, Option<Decimal>)> {
        let ids = self.target_ids(account_id, show_archived);
        let Some(first) = self
            .entries
            .iter()
            .filter(|entry| ids.contains(&entry.account_id))
            .map(|entry| entry.date)
            .min()
        else {
            return Vec::new();
        };
        let today = chrono::Local::now().date_naive();

        (first.year()..=today.year())
            .map(|year| {
                // First year opens at the first entry, or pre-tracking growth lands in it.
                let start = NaiveDate::from_ymd_opt(year - 1, 12, 31)
                    .unwrap_or(first)
                    .max(first);
                let end = NaiveDate::from_ymd_opt(year, 12, 31)
                    .unwrap_or(today)
                    .min(today);
                (
                    year,
                    self.invested_between(account_id, start, end, show_archived),
                    self.gain_between(account_id, start, end, show_archived),
                    self.modified_dietz(account_id, start, end, show_archived),
                )
            })
            .collect()
    }
}
