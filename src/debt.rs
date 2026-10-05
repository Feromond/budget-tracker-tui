use crate::model::{Account, Transaction, TransactionType};
use chrono::{Datelike, NaiveDate};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::{Decimal, RoundingStrategy};

pub const MAX_MONTHS: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebtTerms {
    pub account_id: i64,
    /// Percent, so 19.99 means 19.99%.
    pub apr: Option<Decimal>,
    pub payment: Option<Decimal>,
    pub in_plan: bool,
}

impl DebtTerms {
    pub fn projectable(&self) -> Option<(Decimal, Decimal)> {
        Some((self.apr?, self.payment?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayoffStrategy {
    Minimums,
    #[default]
    Avalanche,
    Snowball,
}

impl PayoffStrategy {
    pub fn label(self) -> &'static str {
        match self {
            PayoffStrategy::Minimums => "Minimums only",
            PayoffStrategy::Avalanche => "Avalanche",
            PayoffStrategy::Snowball => "Snowball",
        }
    }

    pub fn all() -> [PayoffStrategy; 3] {
        [
            PayoffStrategy::Avalanche,
            PayoffStrategy::Snowball,
            PayoffStrategy::Minimums,
        ]
    }

    pub fn uses_extra(self) -> bool {
        self != PayoffStrategy::Minimums
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebtInput {
    pub id: i64,
    pub owed: Decimal,
    pub apr: Decimal,
    pub payment: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleRow {
    pub month: usize,
    pub payment: Decimal,
    pub interest: Decimal,
    pub balance: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtOutcome {
    pub id: i64,
    pub paid_off_month: Option<usize>,
    pub interest: Decimal,
    pub schedule: Vec<ScheduleRow>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PayoffPlan {
    pub outcomes: Vec<DebtOutcome>,
    /// Index 0 is today; the rest are monthly totals.
    pub totals: Vec<Decimal>,
}

impl PayoffPlan {
    pub fn months(&self) -> Option<usize> {
        self.outcomes
            .iter()
            .map(|outcome| outcome.paid_off_month)
            .try_fold(0, |longest, month| month.map(|m| longest.max(m)))
    }

    pub fn total_interest(&self) -> Decimal {
        self.outcomes.iter().map(|outcome| outcome.interest).sum()
    }

    pub fn outcome(&self, id: i64) -> Option<&DebtOutcome> {
        self.outcomes.iter().find(|outcome| outcome.id == id)
    }

    pub fn sequence(&self, id: i64) -> Option<usize> {
        let target = self.outcome(id).filter(|o| o.paid_off_month != Some(0))?;
        let key =
            |outcome: &DebtOutcome| (outcome.paid_off_month.unwrap_or(usize::MAX), outcome.id);
        Some(
            self.outcomes
                .iter()
                .filter(|other| other.paid_off_month != Some(0) && key(other) < key(target))
                .count()
                + 1,
        )
    }
}

// Caps balances to prevent overflow during long projections.
fn ceiling() -> Decimal {
    Decimal::from(1_000_000_000_000_000_i64)
}

fn monthly_interest(owed: Decimal, apr: Decimal) -> Decimal {
    (owed.saturating_mul(apr) / Decimal::from(1200)).round_dp(2)
}

pub fn simulate(debts: &[DebtInput], strategy: PayoffStrategy, extra: Decimal) -> PayoffPlan {
    let mut owed: Vec<Decimal> = debts
        .iter()
        .map(|debt| debt.owed.clamp(Decimal::ZERO, ceiling()))
        .collect();
    let mut outcomes: Vec<DebtOutcome> = debts
        .iter()
        .zip(&owed)
        .map(|(debt, owed)| DebtOutcome {
            id: debt.id,
            paid_off_month: owed.is_zero().then_some(0),
            interest: Decimal::ZERO,
            schedule: Vec::new(),
        })
        .collect();

    let mut order: Vec<usize> = (0..debts.len()).collect();
    match strategy {
        PayoffStrategy::Avalanche => {
            order.sort_by(|&a, &b| debts[b].apr.cmp(&debts[a].apr).then(owed[a].cmp(&owed[b])))
        }
        PayoffStrategy::Snowball => {
            order.sort_by(|&a, &b| owed[a].cmp(&owed[b]).then(debts[b].apr.cmp(&debts[a].apr)))
        }
        PayoffStrategy::Minimums => {}
    }
    let budget: Decimal =
        debts.iter().map(|debt| debt.payment).sum::<Decimal>() + extra.max(Decimal::ZERO);

    let mut totals = vec![owed.iter().copied().sum::<Decimal>()];
    for month in 1..=MAX_MONTHS {
        if owed.iter().all(|amount| amount.is_zero()) {
            break;
        }
        let active: Vec<bool> = owed.iter().map(|amount| *amount > Decimal::ZERO).collect();
        let mut interest = vec![Decimal::ZERO; debts.len()];
        let mut paid = vec![Decimal::ZERO; debts.len()];

        for (i, debt) in debts.iter().enumerate().filter(|(i, _)| active[*i]) {
            interest[i] = monthly_interest(owed[i], debt.apr);
            owed[i] = owed[i].saturating_add(interest[i]).min(ceiling());
            paid[i] = debt.payment.max(Decimal::ZERO).min(owed[i]);
            owed[i] -= paid[i];
        }

        if strategy.uses_extra() {
            let mut left = budget - paid.iter().copied().sum::<Decimal>();
            for &i in &order {
                if left <= Decimal::ZERO {
                    break;
                }
                let amount = left.min(owed[i]);
                owed[i] -= amount;
                paid[i] += amount;
                left -= amount;
            }
        }

        for (i, outcome) in outcomes.iter_mut().enumerate().filter(|(i, _)| active[*i]) {
            outcome.interest += interest[i];
            outcome.schedule.push(ScheduleRow {
                month,
                payment: paid[i],
                interest: interest[i],
                balance: owed[i],
            });
            if owed[i].is_zero() {
                outcome.paid_off_month = Some(month);
            }
        }
        totals.push(owed.iter().copied().sum());
    }

    PayoffPlan { outcomes, totals }
}

pub fn payment_for_term(owed: Decimal, apr: Decimal, months: u32) -> Option<Decimal> {
    let mut payment = closed_form_payment(owed, apr, months)?;
    if months as usize > MAX_MONTHS {
        return Some(payment);
    }
    // Add cents to cover monthly interest rounding.
    for _ in 0..1000 {
        let input = DebtInput {
            id: 0,
            owed,
            apr,
            payment,
        };
        let plan = simulate(&[input], PayoffStrategy::Minimums, Decimal::ZERO);
        if plan.months().is_some_and(|m| m <= months as usize) {
            return Some(payment);
        }
        payment += Decimal::new(1, 2);
    }
    Some(payment)
}

fn closed_form_payment(owed: Decimal, apr: Decimal, months: u32) -> Option<Decimal> {
    if months == 0 || owed <= Decimal::ZERO {
        return None;
    }
    let principal = owed.to_f64()?;
    let rate = apr.to_f64()? / 1200.0;
    let n = months as f64;
    let payment = if rate == 0.0 {
        principal / n
    } else {
        principal * rate / (1.0 - (1.0 + rate).powf(-n))
    };
    Decimal::from_f64(payment).map(|p| p.round_dp_with_strategy(2, RoundingStrategy::AwayFromZero))
}

fn balance_change(account_id: i64, tx: &Transaction) -> Decimal {
    match tx.transaction_type {
        TransactionType::Income if tx.account_id == account_id => tx.amount,
        TransactionType::Expense if tx.account_id == account_id => -tx.amount,
        TransactionType::Transfer if tx.to_account_id == Some(account_id) => tx.amount,
        TransactionType::Transfer if tx.account_id == account_id => -tx.amount,
        _ => Decimal::ZERO,
    }
}

fn counted<'a>(
    account: &'a Account,
    transactions: &'a [Transaction],
    on: NaiveDate,
) -> impl Iterator<Item = (&'a Transaction, Decimal)> + 'a {
    transactions
        .iter()
        .filter(move |tx| tx.date <= on && account.tracks(tx.date))
        .map(move |tx| (tx, balance_change(account.id, tx)))
        .filter(|(_, change)| !change.is_zero())
}

pub fn signed_owed_on(account: &Account, transactions: &[Transaction], on: NaiveDate) -> Decimal {
    let balance: Decimal = account.opening_balance
        + counted(account, transactions, on)
            .map(|(_, change)| change)
            .sum::<Decimal>();
    -balance
}

pub fn owed_on(account: &Account, transactions: &[Transaction], on: NaiveDate) -> Decimal {
    signed_owed_on(account, transactions, on).max(Decimal::ZERO)
}

/// `dates` must be ascending.
pub fn owed_series(
    account: &Account,
    transactions: &[Transaction],
    dates: &[NaiveDate],
) -> Vec<Decimal> {
    let Some(last) = dates.last() else {
        return Vec::new();
    };
    let mut flows: Vec<(NaiveDate, Decimal)> = counted(account, transactions, *last)
        .map(|(tx, change)| (tx.date, change))
        .collect();
    flows.sort_by_key(|(date, _)| *date);

    let mut balance = account.opening_balance;
    let mut next = 0;
    dates
        .iter()
        .map(|date| {
            while next < flows.len() && flows[next].0 <= *date {
                balance += flows[next].1;
                next += 1;
            }
            (-balance).max(Decimal::ZERO)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebtActivity {
    /// Includes the starting balance.
    pub charged: Decimal,
    pub paid: Decimal,
    /// Transfers only; refunds don't count.
    pub paid_this_month: Decimal,
    pub interest_last_year: Decimal,
}

impl DebtActivity {
    pub fn progress(&self) -> Option<Decimal> {
        (self.charged > Decimal::ZERO)
            .then(|| (self.paid / self.charged).clamp(Decimal::ZERO, Decimal::ONE))
    }
}

pub fn activity(account: &Account, transactions: &[Transaction], today: NaiveDate) -> DebtActivity {
    let year_ago = crate::validation::add_months(today, -12);
    let mut activity = DebtActivity {
        charged: (-account.opening_balance).max(Decimal::ZERO),
        ..Default::default()
    };
    for (_, change) in counted(account, transactions, today) {
        if change > Decimal::ZERO {
            activity.paid += change;
        } else {
            activity.charged -= change;
        }
    }
    let recorded = transactions
        .iter()
        .filter(|tx| tx.date <= today && !balance_change(account.id, tx).is_zero());
    for tx in recorded {
        let payment = tx.transaction_type == TransactionType::Transfer
            && tx.to_account_id == Some(account.id);
        if payment && tx.date.year() == today.year() && tx.date.month() == today.month() {
            activity.paid_this_month += tx.amount;
        }
        if tx.transaction_type == TransactionType::Expense
            && tx.date > year_ago
            && is_interest_or_fee(tx)
        {
            activity.interest_last_year += tx.amount;
        }
    }
    activity
}

pub const DEBT_CATEGORY: &str = "Debt Payments";
const INTEREST_SUBCATEGORY: &str = "Interest Charges";
const FEES_SUBCATEGORY: &str = "Fees & Penalties";
const MORTGAGE_PAYMENT: &str = "Mortgage Principal";

fn is_interest_or_fee(tx: &Transaction) -> bool {
    let fee = tx.category.eq_ignore_ascii_case(DEBT_CATEGORY)
        && tx.subcategory.eq_ignore_ascii_case(FEES_SUBCATEGORY);
    fee || [&tx.category, &tx.subcategory]
        .iter()
        .any(|text| text.to_lowercase().contains("interest"))
}

pub fn interest_category(account: &Account) -> [(&'static str, &'static str); 2] {
    let fallback = (DEBT_CATEGORY, INTEREST_SUBCATEGORY);
    if payment_subcategory(account) == Some(MORTGAGE_PAYMENT) {
        [("Housing", "Mortgage Interest"), fallback]
    } else {
        [fallback, fallback]
    }
}

pub fn payment_subcategory(account: &Account) -> Option<&'static str> {
    let text = format!("{} {}", account.name, account.kind).to_lowercase();
    let tokens: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();
    let has = |words: &[&str]| {
        words.iter().any(|word| {
            if word.contains(' ') {
                text.contains(word)
            } else {
                tokens.contains(word)
            }
        })
    };
    if has(&["line of credit", "loc", "heloc"]) {
        Some("Line of Credit Payments")
    } else if has(&["mortgage"]) {
        Some(MORTGAGE_PAYMENT)
    } else if has(&["student", "osap"]) {
        Some("Student Loan")
    } else if has(&["car", "auto", "vehicle", "truck"]) {
        Some("Car Loan")
    } else if has(&["card", "visa", "mastercard", "amex", "credit"]) {
        Some("Credit Card Payments")
    } else if has(&["loan"]) {
        Some("Personal Loan")
    } else {
        None
    }
}
