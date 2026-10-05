use crate::app::state::App;
use crate::debt::{self, DebtOutcome, MAX_MONTHS, PayoffPlan, PayoffStrategy};
use crate::model::{Account, DATE_FORMAT};
use crate::ui::form::render_field_form;
use crate::ui::helpers::{
    TRANSFER_COLOR, axis_amounts, centered_rect, clamp_table_scroll, format_amount, right,
};
use crate::validation::add_months;
use chrono::{Duration, NaiveDate};
use ratatui::prelude::*;
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Clear, Dataset, GraphType, LegendPosition, Paragraph, Row,
    Table, Wrap,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

const PANEL_CHROME_COLOR: Color = Color::LightMagenta;
const OWED_COLOR: Color = Color::LightRed;
const PLAN_COLOR: Color = Color::LightCyan;
const BASELINE_COLOR: Color = Color::DarkGray;
const PAID_COLOR: Color = Color::LightGreen;
const HISTORY_MONTHS: i32 = 12;
const NEVER_HORIZON: usize = 360;

fn panel(title: Line<'static>, borders: Borders) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(borders)
        .border_style(Style::default().fg(PANEL_CHROME_COLOR))
}

fn heading(text: &str) -> Span<'static> {
    Span::styled(
        text.to_string(),
        Style::default()
            .fg(PANEL_CHROME_COLOR)
            .add_modifier(Modifier::BOLD),
    )
}

fn separator() -> Span<'static> {
    Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR))
}

fn dim(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(Color::DarkGray))
}

fn stat_line(label: &str, value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = vec![Span::styled(
        format!("{:<10}", label),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    spans.extend(value);
    Line::from(spans)
}

fn month_label(today: NaiveDate, months: usize) -> String {
    add_months(today, months as i32).format("%b %Y").to_string()
}

fn payoff_text(today: NaiveDate, months: Option<usize>) -> (String, Style) {
    match months {
        Some(0) => ("Paid off".to_string(), Style::default().fg(PAID_COLOR)),
        Some(months) => (
            format!("{} ({} mo)", month_label(today, months), months),
            Style::default().fg(PLAN_COLOR),
        ),
        None => ("Never".to_string(), Style::default().fg(OWED_COLOR)),
    }
}

fn percent(value: Option<Decimal>) -> String {
    value
        .map(|ratio| {
            format!(
                "{:.0}%",
                (ratio * Decimal::from(100)).to_f64().unwrap_or(0.0)
            )
        })
        .unwrap_or_else(|| "-".to_string())
}

fn header(cells: Vec<Cell<'static>>) -> Row<'static> {
    Row::new(cells).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
}

fn plan_title(app: &App) -> Vec<Span<'static>> {
    let mut spans = vec![
        heading("Plan: "),
        Span::styled(
            app.debt_strategy.label().to_string(),
            Style::default().fg(PLAN_COLOR).add_modifier(Modifier::BOLD),
        ),
        dim(" (s)"),
    ];
    if app.debt_strategy.uses_extra() {
        spans.extend([
            separator(),
            Span::raw("Extra "),
            Span::styled(
                format!("{}/mo", format_amount(&app.debt_extra)),
                Style::default().fg(PAID_COLOR).add_modifier(Modifier::BOLD),
            ),
            dim(" (◀/▶) "),
        ]);
    }
    spans
}

struct Plans {
    current: PayoffPlan,
    baseline: Option<PayoffPlan>,
}

impl Plans {
    fn new(app: &App) -> Self {
        let current = app.debt_plan(app.debt_strategy);
        let baseline = app
            .debt_strategy
            .uses_extra()
            .then(|| app.debt_plan(PayoffStrategy::Minimums));
        Self { current, baseline }
    }
}

/// The bool marks a shared-plan projection.
fn projection_for(app: &App, plans: &Plans, account: &Account) -> Option<(DebtOutcome, bool)> {
    if app.debt_in_plan(account.id) {
        return plans
            .current
            .outcome(account.id)
            .cloned()
            .map(|o| (o, true));
    }
    let own = app.standalone_debt_plan(account)?;
    own.outcomes.into_iter().next().map(|o| (o, false))
}

fn payoff_cell(
    today: NaiveDate,
    projection: Option<&(DebtOutcome, bool)>,
    track_only: bool,
) -> (String, Style) {
    let style = |in_plan: bool, color: Color| {
        Style::default().fg(if in_plan { color } else { Color::DarkGray })
    };
    match projection {
        Some((outcome, in_plan)) => match outcome.paid_off_month {
            Some(0) => ("Paid off".to_string(), Style::default().fg(PAID_COLOR)),
            Some(months) => (month_label(today, months), style(*in_plan, PLAN_COLOR)),
            None => ("Never".to_string(), style(*in_plan, OWED_COLOR)),
        },
        None if track_only => (
            "Track only".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        None => ("-".to_string(), Style::default().fg(Color::DarkGray)),
    }
}

pub fn render_debts_view(f: &mut Frame, app: &mut App, area: Rect) {
    if app.debt_accounts().is_empty() {
        render_empty_state(f, app, area);
        return;
    }
    let plans = Plans::new(app);

    let rows = app.debt_accounts().len() as u16 + 4;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(rows.clamp(5, 14)),
        ])
        .split(area);

    render_overview_stats(f, app, &plans, chunks[0]);
    render_overview_chart(f, app, &plans, chunks[1]);
    render_debts_table(f, app, &plans, chunks[2]);
}

pub fn render_debt_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(account) = app.active_debt().cloned() else {
        return;
    };
    let plans = Plans::new(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Percentage(45),
        ])
        .split(area);

    render_detail_stats(f, app, &plans, &account, chunks[0]);
    render_detail_chart(f, app, &plans, &account, chunks[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(chunks[2]);
    render_history_table(f, app, bottom[0]);
    render_schedule_table(f, app, &plans, &account, bottom[1]);
}

fn render_empty_state(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "No debts yet.",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Press 'a' to add a card or loan with what you owe, the rate, and your"),
        Line::from("payment. You'll see when it's paid off and what the interest costs."),
        Line::from(""),
        Line::from(dim(
            "Then 'p' records a payment and 'r' matches a statement.",
        )),
    ];
    if app.show_archived_debts {
        lines.insert(1, Line::from(dim("(including archived)")));
    } else if app
        .accounts
        .all()
        .iter()
        .any(|a| a.class == crate::model::AccountClass::Credit)
    {
        lines.insert(
            1,
            Line::from(dim("All debts are archived. Press A to show them.")),
        );
    }
    let body = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(panel(Line::from(heading("Debts")), Borders::TOP));
    f.render_widget(body, area);
}

fn three_columns(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area)
}

fn savings_line(plans: &Plans) -> Line<'static> {
    let Some(baseline) = &plans.baseline else {
        return stat_line("Saves", vec![dim("pick a strategy (s)")]);
    };
    let saved = baseline.total_interest() - plans.current.total_interest();
    let sooner = match (baseline.months(), plans.current.months()) {
        (Some(slow), Some(fast)) if slow > fast => format!(", {} mo sooner", slow - fast),
        (None, Some(_)) => ", and it ends".to_string(),
        _ => String::new(),
    };
    if saved <= Decimal::ZERO && sooner.is_empty() {
        return stat_line("Saves", vec![dim("nothing yet, add extra (▶)")]);
    }
    stat_line(
        "Saves",
        vec![Span::styled(
            format!("{}{}", format_amount(&saved), sooner),
            Style::default().fg(PAID_COLOR),
        )],
    )
}

fn render_overview_stats(f: &mut Frame, app: &App, plans: &Plans, area: Rect) {
    let today = app.today();
    let debts = app.debt_accounts();
    let owed: Decimal = debts.iter().map(|account| app.debt_owed(account)).sum();
    let payments = app.debt_monthly_total();
    let paid_this_month: Decimal = debts
        .iter()
        .map(|account| app.debt_activity(account).paid_this_month)
        .sum();

    let title = Line::from(vec![
        heading("Debts"),
        separator(),
        Span::styled(
            app.active_ledger_name().to_string(),
            Style::default().fg(Color::Cyan),
        ),
    ]);
    let block = panel(title, Borders::TOP);
    let columns = three_columns(block.inner(area));
    f.render_widget(block, area);

    let (debt_free, debt_free_style) = if plans.current.outcomes.is_empty() {
        ("-".to_string(), Style::default().fg(Color::DarkGray))
    } else {
        payoff_text(today, plans.current.months())
    };

    f.render_widget(
        Paragraph::new(vec![
            stat_line(
                "Owed",
                vec![Span::styled(
                    format_amount(&owed),
                    Style::default().fg(OWED_COLOR),
                )],
            ),
            stat_line(
                "Payments",
                vec![Span::raw(format!("{}/mo", format_amount(&payments)))],
            ),
        ]),
        columns[0],
    );
    f.render_widget(
        Paragraph::new(vec![
            stat_line("Debt-free", vec![Span::styled(debt_free, debt_free_style)]),
            stat_line(
                "Interest",
                vec![Span::styled(
                    format_amount(&plans.current.total_interest()),
                    Style::default().fg(OWED_COLOR),
                )],
            ),
        ]),
        columns[1],
    );
    f.render_widget(
        Paragraph::new(vec![
            stat_line(
                "This month",
                vec![Span::styled(
                    format!("{} paid", format_amount(&paid_this_month)),
                    Style::default().fg(PAID_COLOR),
                )],
            ),
            savings_line(plans),
        ]),
        columns[2],
    );
}

fn render_detail_stats(f: &mut Frame, app: &App, plans: &Plans, account: &Account, area: Rect) {
    let today = app.today();
    let owed = app.debt_owed(account);
    let activity = app.debt_activity(account);
    let terms = app.debt_terms.get(&account.id);
    let projection = projection_for(app, plans, account);

    let mut title = vec![heading(&account.name)];
    if !account.kind.trim().is_empty() {
        title.push(separator());
        title.push(Span::styled(
            account.kind.clone(),
            Style::default().fg(Color::Cyan),
        ));
    }
    if let Some(apr) = terms.and_then(|t| t.apr) {
        title.push(separator());
        title.push(Span::styled(
            format!("{}% APR", apr.normalize()),
            Style::default().fg(Color::Yellow),
        ));
    }
    if terms.is_some_and(|t| !t.in_plan) {
        title.push(separator());
        title.push(dim("track only, not in the payoff plan"));
    }
    if account.archived {
        title.push(dim(" (archived)"));
    }
    let block = panel(Line::from(title), Borders::TOP);
    let columns = three_columns(block.inner(area));
    f.render_widget(block, area);

    let payment = match (terms, terms.and_then(|t| t.payment)) {
        (_, Some(payment)) => Span::raw(format!("{}/mo", format_amount(&payment))),
        (Some(_), None) => dim("-"),
        (None, _) => dim("no terms yet (e)"),
    };
    let (payoff, payoff_style) = match (owed.is_zero(), &projection) {
        (true, _) => ("Paid off".to_string(), Style::default().fg(PAID_COLOR)),
        (false, Some((outcome, _))) => payoff_text(today, outcome.paid_off_month),
        (false, None) if terms.is_some() => (
            "Track only".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        (false, None) => ("-".to_string(), Style::default().fg(Color::DarkGray)),
    };
    let interest = projection.map(|(o, _)| o.interest).unwrap_or_default();

    f.render_widget(
        Paragraph::new(vec![
            stat_line(
                "Owed",
                vec![Span::styled(
                    format_amount(&owed),
                    Style::default().fg(OWED_COLOR),
                )],
            ),
            stat_line("Payment", vec![payment]),
        ]),
        columns[0],
    );
    f.render_widget(
        Paragraph::new(vec![
            stat_line("Payoff", vec![Span::styled(payoff, payoff_style)]),
            stat_line(
                "Interest",
                vec![Span::styled(
                    format_amount(&interest),
                    Style::default().fg(OWED_COLOR),
                )],
            ),
        ]),
        columns[1],
    );
    f.render_widget(
        Paragraph::new(vec![
            stat_line(
                "Paid down",
                vec![Span::styled(
                    format!(
                        "{} of {}",
                        percent(activity.progress()),
                        format_amount(&activity.charged)
                    ),
                    Style::default().fg(PAID_COLOR),
                )],
            ),
            stat_line(
                "Interest",
                vec![
                    Span::raw(format_amount(&activity.interest_last_year)),
                    dim(" recorded, last 12 mo"),
                ],
            ),
        ]),
        columns[2],
    );
}

// Start where every debt has history beyond its opening snapshot.
fn history_start(app: &App, accounts: &[&Account], today: NaiveDate) -> NaiveDate {
    let floor = add_months(today, -HISTORY_MONTHS);
    let earliest = accounts
        .iter()
        .filter_map(|account| {
            account.tracked_from.or_else(|| {
                app.transactions
                    .iter()
                    .filter(|tx| {
                        tx.account_id == account.id || tx.to_account_id == Some(account.id)
                    })
                    .map(|tx| tx.date)
                    .min()
            })
        })
        .min();
    let latest_snapshot = accounts.iter().filter_map(|a| a.tracked_from).max();
    earliest
        .into_iter()
        .chain(latest_snapshot)
        .max()
        .map_or(today, |start| start.max(floor))
}

fn projection_points(
    today: NaiveDate,
    balances: &[Decimal],
    horizon: usize,
) -> Vec<(NaiveDate, f64)> {
    balances
        .iter()
        .take(horizon + 1)
        .enumerate()
        .map(|(month, balance)| {
            (
                add_months(today, month as i32),
                balance.to_f64().unwrap_or(0.0),
            )
        })
        .collect()
}

fn horizon(plans: &Plans, months: impl Fn(&PayoffPlan) -> Option<usize>) -> usize {
    let span = |plan: &PayoffPlan| months(plan).unwrap_or(NEVER_HORIZON);
    let longest = plans
        .baseline
        .as_ref()
        .map_or(0, span)
        .max(span(&plans.current));
    longest.clamp(1, MAX_MONTHS)
}

fn render_overview_chart(f: &mut Frame, app: &App, plans: &Plans, area: Rect) {
    let today = app.today();
    let all = app.debt_accounts();
    let untermed = all
        .iter()
        .filter(|account| {
            !app.debt_terms.contains_key(&account.id) && !app.debt_owed(account).is_zero()
        })
        .count();
    let mut title = vec![heading("Balance"), dim(" (debts in the plan)")];
    if untermed > 0 {
        title.push(separator());
        title.push(Span::styled(
            format!(
                "{} debt{} without terms (e)",
                untermed,
                if untermed == 1 { "" } else { "s" }
            ),
            Style::default().fg(Color::Yellow),
        ));
    }

    let debts: Vec<&Account> = all
        .into_iter()
        .filter(|account| app.debt_in_plan(account.id))
        .collect();
    if debts.is_empty() {
        let body = Paragraph::new(vec![
            Line::from(""),
            Line::from(dim(
                "Nothing in the plan yet. Give a debt a rate and payment (e).",
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(panel(Line::from(title), Borders::TOP));
        f.render_widget(body, area);
        return;
    }
    let start = history_start(app, &debts, today);
    let samples = history_dates(start, today, area.width / 3);
    let mut owed = vec![Decimal::ZERO; samples.len()];
    for account in &debts {
        for (total, value) in
            owed.iter_mut()
                .zip(debt::owed_series(account, &app.transactions, &samples))
        {
            *total += value;
        }
    }
    let history: Vec<(NaiveDate, f64)> = samples
        .into_iter()
        .zip(owed.iter().map(|value| value.to_f64().unwrap_or(0.0)))
        .collect();

    let horizon = horizon(plans, PayoffPlan::months);
    let plan = projection_points(today, &plans.current.totals, horizon);
    let baseline = plans
        .baseline
        .as_ref()
        .map(|baseline| projection_points(today, &baseline.totals, horizon));

    render_chart(f, area, Line::from(title), start, history, plan, baseline);
}

fn render_detail_chart(f: &mut Frame, app: &App, plans: &Plans, account: &Account, area: Rect) {
    let today = app.today();
    let start = history_start(app, &[account], today);
    let samples = history_dates(start, today, area.width / 3);
    let history: Vec<(NaiveDate, f64)> = debt::owed_series(account, &app.transactions, &samples)
        .into_iter()
        .zip(samples)
        .map(|(value, date)| (date, value.to_f64().unwrap_or(0.0)))
        .collect();

    let owed = app.debt_owed(account);
    let balances = |outcome: &DebtOutcome| -> Vec<Decimal> {
        std::iter::once(owed)
            .chain(outcome.schedule.iter().map(|row| row.balance))
            .collect()
    };
    let projection = projection_for(app, plans, account);
    let baseline_outcome = projection
        .as_ref()
        .filter(|(_, in_plan)| *in_plan)
        .and_then(|_| plans.baseline.as_ref()?.outcome(account.id));
    let span = |outcome: Option<&DebtOutcome>| match outcome {
        Some(outcome) => outcome.paid_off_month.unwrap_or(NEVER_HORIZON),
        None => 0,
    };
    let horizon = span(projection.as_ref().map(|(o, _)| o))
        .max(span(baseline_outcome))
        .clamp(1, MAX_MONTHS);
    let plan = projection
        .as_ref()
        .map(|(outcome, _)| projection_points(today, &balances(outcome), horizon))
        .unwrap_or_default();
    let baseline =
        baseline_outcome.map(|outcome| projection_points(today, &balances(outcome), horizon));

    render_chart(
        f,
        area,
        Line::from(heading("Balance")),
        start,
        history,
        plan,
        baseline,
    );
}

fn history_dates(start: NaiveDate, end: NaiveDate, points: u16) -> Vec<NaiveDate> {
    let points = points.max(2) as i64;
    let span = (end - start).num_days();
    if span <= 0 {
        return vec![end];
    }
    (0..points)
        .map(|step| start + Duration::days(span * step / (points - 1)))
        .collect()
}

fn render_chart(
    f: &mut Frame,
    area: Rect,
    title: Line<'static>,
    start: NaiveDate,
    history: Vec<(NaiveDate, f64)>,
    plan: Vec<(NaiveDate, f64)>,
    baseline: Option<Vec<(NaiveDate, f64)>>,
) {
    let x = |date: NaiveDate| (date - start).num_days() as f64;
    let points = |series: &[(NaiveDate, f64)]| -> Vec<(f64, f64)> {
        series
            .iter()
            .map(|(date, value)| (x(*date), *value))
            .collect()
    };
    let history_points = points(&history);
    let plan_points = points(&plan);
    let baseline_points = baseline.as_deref().map(points).unwrap_or_default();

    let end = plan
        .last()
        .into_iter()
        .chain(baseline.as_ref().and_then(|b| b.last()))
        .chain(history.last())
        .map(|(date, _)| *date)
        .max()
        .unwrap_or(start);
    let peak = history_points
        .iter()
        .chain(&plan_points)
        .chain(&baseline_points)
        .map(|(_, y)| *y)
        .fold(0.0f64, f64::max);
    let y_max = if peak <= 0.0 { 1.0 } else { peak * 1.15 };
    let x_max = x(end).max(1.0);

    let mut datasets = Vec::new();
    if !baseline_points.is_empty() {
        datasets.push(
            Dataset::default()
                .name("Minimums only")
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(BASELINE_COLOR))
                .data(&baseline_points),
        );
    }
    datasets.push(
        Dataset::default()
            .name("Plan")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(PLAN_COLOR))
            .data(&plan_points),
    );
    datasets.push(
        Dataset::default()
            .name("Owed")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(OWED_COLOR))
            .data(&history_points),
    );

    let format = if (end - start).num_days() > 365 * 4 {
        "%Y"
    } else {
        "%b %Y"
    };
    let label = |date: NaiveDate| Span::raw(date.format(format).to_string());
    let labels = vec![label(start), label(start + (end - start) / 2), label(end)];

    let chart = Chart::new(datasets)
        .block(panel(title, Borders::TOP))
        .x_axis(Axis::default().bounds([0.0, x_max]).labels(labels))
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(axis_amounts(y_max)),
        )
        .legend_position(Some(LegendPosition::TopRight));
    f.render_widget(chart, area);
}

fn render_debts_table(f: &mut Frame, app: &mut App, plans: &Plans, area: Rect) {
    let today = app.today();
    let show_kind = area.width >= 100;
    let debts: Vec<Account> = app.debt_accounts().into_iter().cloned().collect();

    let mut total_owed = Decimal::ZERO;
    let mut rows: Vec<Row> = debts
        .iter()
        .map(|account| {
            let owed = app.debt_owed(account);
            let activity = app.debt_activity(account);
            let terms = app.debt_terms.get(&account.id).copied();
            let in_plan = app.debt_in_plan(account.id);
            let projection = projection_for(app, plans, account);
            let payment = terms.and_then(|t| t.payment);
            total_owed += owed;

            let name_style = if account.archived {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            let order = plans
                .current
                .sequence(account.id)
                .map(|n| n.to_string())
                .unwrap_or_default();

            let mut cells = vec![
                Cell::from(order).style(Style::default().fg(PLAN_COLOR)),
                Cell::from(account.name.clone()).style(name_style),
            ];
            if show_kind {
                cells
                    .push(Cell::from(account.kind.clone()).style(Style::default().fg(Color::Cyan)));
            }
            if owed.is_zero() {
                cells.push(
                    Cell::from(right("Paid off".to_string()))
                        .style(Style::default().fg(PAID_COLOR)),
                );
            } else {
                cells.push(
                    Cell::from(right(format_amount(&owed))).style(Style::default().fg(OWED_COLOR)),
                );
            }
            let blank =
                || Cell::from(right("-".to_string())).style(Style::default().fg(Color::DarkGray));
            cells.push(match terms.and_then(|t| t.apr) {
                Some(apr) => Cell::from(right(format!("{}%", apr.normalize())))
                    .style(Style::default().fg(Color::Yellow)),
                None => blank(),
            });
            let planned = projection
                .as_ref()
                .filter(|(_, in_plan)| *in_plan)
                .and_then(|(outcome, _)| outcome.schedule.first())
                .map(|row| row.payment);
            cells.push(match (terms, payment) {
                (_, Some(payment)) => match planned {
                    Some(planned) if planned > payment => {
                        Cell::from(right(format_amount(&planned)))
                            .style(Style::default().fg(PAID_COLOR).add_modifier(Modifier::BOLD))
                    }
                    _ => Cell::from(right(format_amount(&payment))),
                },
                (Some(_), None) => blank(),
                (None, _) => Cell::from(right("set (e)".to_string()))
                    .style(Style::default().fg(Color::DarkGray)),
            });

            let paid = activity.paid_this_month;
            cells.push(match payment {
                Some(payment) if paid >= payment => {
                    Cell::from(right(format!("✓ {}", format_amount(&paid))))
                        .style(Style::default().fg(PAID_COLOR))
                }
                Some(_) if paid > Decimal::ZERO => Cell::from(right(format_amount(&paid)))
                    .style(Style::default().fg(Color::Yellow)),
                None if paid > Decimal::ZERO => {
                    Cell::from(right(format_amount(&paid))).style(Style::default().fg(PAID_COLOR))
                }
                _ => blank(),
            });

            let (payoff, payoff_style) = payoff_cell(
                today,
                projection.as_ref(),
                terms.is_some_and(|t| !t.in_plan),
            );
            let interest_style =
                Style::default().fg(if in_plan { OWED_COLOR } else { Color::DarkGray });
            cells.extend([
                Cell::from(payoff).style(payoff_style),
                Cell::from(right(
                    projection
                        .as_ref()
                        .map(|(o, _)| format_amount(&o.interest))
                        .unwrap_or_else(|| "-".to_string()),
                ))
                .style(interest_style),
                Cell::from(right(percent(activity.progress())))
                    .style(Style::default().fg(PAID_COLOR)),
            ]);
            Row::new(cells)
        })
        .collect();

    let mut total_cells = vec![
        Cell::from(""),
        Cell::from("Total").style(Style::default().add_modifier(Modifier::BOLD)),
    ];
    if show_kind {
        total_cells.push(Cell::from(""));
    }
    total_cells.extend([
        Cell::from(right(format_amount(&total_owed)))
            .style(Style::default().fg(OWED_COLOR).add_modifier(Modifier::BOLD)),
        Cell::from(""),
        Cell::from(right(format_amount(&app.debt_monthly_total()))),
        Cell::from(""),
        Cell::from(""),
        Cell::from(right(format_amount(&plans.current.total_interest())))
            .style(Style::default().fg(OWED_COLOR)),
        Cell::from(""),
    ]);
    rows.push(Row::new(total_cells).top_margin(1));

    let mut header_cells = vec![Cell::from("#"), Cell::from("Debt")];
    if show_kind {
        header_cells.push(Cell::from("Type"));
    }
    header_cells.extend([
        Cell::from(right("Owed".to_string())),
        Cell::from(right("APR".to_string())),
        Cell::from(right("Payment".to_string())),
        Cell::from(right("This Month".to_string())),
        Cell::from("Payoff"),
        Cell::from(right("Interest".to_string())),
        Cell::from(right("Paid".to_string())),
    ]);

    let mut widths = vec![Constraint::Length(2), Constraint::Min(10)];
    if show_kind {
        widths.push(Constraint::Length(12));
    }
    widths.extend([
        Constraint::Length(12),
        Constraint::Length(7),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(11),
        Constraint::Length(5),
    ]);

    let mut title = plan_title(app);
    if app.show_archived_debts {
        title.push(dim(" (incl. archived)"));
    }

    let row_count = rows.len();
    let table = Table::new(rows, widths)
        .header(header(header_cells))
        .block(panel(Line::from(title), Borders::TOP))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    clamp_table_scroll(&mut app.debt_table_state, row_count, area);
    f.render_stateful_widget(table, area, &mut app.debt_table_state);
}

fn focus_hint(focused: bool) -> Span<'static> {
    if focused {
        Span::styled(" ↑↓", Style::default().fg(Color::Yellow))
    } else {
        dim(" (Tab)")
    }
}

fn highlight(focused: bool) -> Style {
    if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}

fn render_history_table(f: &mut Frame, app: &mut App, area: Rect) {
    let rows: Vec<Row> = app
        .debt_history()
        .into_iter()
        .map(|(tx, change)| {
            let change_style = if change > Decimal::ZERO {
                Style::default().fg(OWED_COLOR)
            } else {
                Style::default().fg(PAID_COLOR)
            };
            let change_text = if change > Decimal::ZERO {
                format!("+{}", format_amount(&change))
            } else {
                format!("-{}", format_amount(&change.abs()))
            };
            let category = if tx.subcategory.is_empty() {
                tx.category.clone()
            } else {
                format!("{} > {}", tx.category, tx.subcategory)
            };
            let description_style = if tx.to_account_id.is_some() {
                Style::default().fg(TRANSFER_COLOR)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(tx.date.format(DATE_FORMAT).to_string()),
                Cell::from(crate::ui::helpers::marked_description(tx)).style(description_style),
                Cell::from(right(change_text)).style(change_style),
                Cell::from(category).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(11),
        Constraint::Min(10),
        Constraint::Length(12),
        Constraint::Min(8),
    ];
    let focused = !app.debt_schedule_focused;
    let title = Line::from(vec![
        heading("History"),
        focus_hint(focused),
        dim(" (+ owed more, - paid down)"),
    ]);
    let row_count = rows.len();
    let table = Table::new(rows, widths)
        .header(header(vec![
            Cell::from("Date"),
            Cell::from("Description"),
            Cell::from(right("Change".to_string())),
            Cell::from("Category"),
        ]))
        .block(panel(title, Borders::TOP))
        .row_highlight_style(highlight(focused))
        .highlight_symbol(if focused { "> " } else { "  " });

    clamp_table_scroll(&mut app.debt_history_table_state, row_count, area);
    f.render_stateful_widget(table, area, &mut app.debt_history_table_state);
}

fn render_schedule_table(
    f: &mut Frame,
    app: &mut App,
    plans: &Plans,
    account: &Account,
    area: Rect,
) {
    let projection = projection_for(app, plans, account);
    let focused = app.debt_schedule_focused;
    let mut title = match &projection {
        Some((_, false)) => vec![
            heading("Schedule"),
            dim(" (its own payment, not in the plan)"),
        ],
        _ => plan_title(app),
    };
    title.insert(1, focus_hint(focused));
    let title = Line::from(title);
    let block = panel(title, Borders::TOP | Borders::LEFT);
    let Some((outcome, _)) = projection else {
        let terms = app.debt_terms.get(&account.id);
        let message = if app.debt_owed(account).is_zero() {
            "Nothing owed, nothing to plan."
        } else if terms.is_some_and(|t| !t.in_plan) {
            "Track only. Add a rate and payment (e) to see its own schedule."
        } else {
            "Set an interest rate and payment (e) to see how this gets paid off."
        };
        f.render_widget(
            Paragraph::new(dim(message))
                .wrap(Wrap { trim: true })
                .block(block),
            area,
        );
        return;
    };

    let today = app.today();
    let rows: Vec<Row> = outcome
        .schedule
        .iter()
        .map(|row| {
            Row::new(vec![
                Cell::from(month_label(today, row.month))
                    .style(Style::default().fg(Color::Magenta)),
                Cell::from(right(format_amount(&row.payment)))
                    .style(Style::default().fg(PAID_COLOR)),
                Cell::from(right(format_amount(&row.interest)))
                    .style(Style::default().fg(OWED_COLOR)),
                Cell::from(right(format_amount(&row.balance))),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(9),
        Constraint::Min(8),
        Constraint::Min(8),
        Constraint::Min(8),
    ];
    let table = Table::new(rows, widths)
        .header(header(vec![
            Cell::from("Month"),
            Cell::from(right("Payment".to_string())),
            Cell::from(right("Interest".to_string())),
            Cell::from(right("Balance".to_string())),
        ]))
        .block(block)
        .row_highlight_style(highlight(focused))
        .highlight_symbol(if focused { "> " } else { "  " });

    let row_count = outcome.schedule.len();
    let state = &mut app.debt_schedule_table_state;
    if let Some(selected) = state.selected()
        && selected >= row_count
    {
        state.select(row_count.checked_sub(1));
    }
    clamp_table_scroll(state, row_count, area);
    f.render_stateful_widget(table, area, state);
}

pub fn render_debt_editor(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 85, area);
    f.render_widget(Clear, popup_area);
    let title = if app.editing_debt_id.is_some() {
        " Edit Debt "
    } else {
        " Add Debt "
    };
    render_field_form(
        f,
        &app.debt_fields,
        app.debt_cursor,
        popup_area,
        title,
        Some(" [Esc] Cancel, [Enter] Save "),
        |_| None,
    );
}

pub fn render_reconcile_popup(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(55, 40, area);
    f.render_widget(Clear, popup_area);
    let (name, recorded) = app
        .active_debt()
        .map(|account| (account.name.clone(), app.debt_owed(account)))
        .unwrap_or_default();
    let title = format!(" Reconcile | {} ", name);
    let hint = format!(
        " Recorded today: {} | [Enter] Check, [Esc] Cancel ",
        format_amount(&recorded)
    );
    render_field_form(
        f,
        &app.reconcile_fields,
        app.reconcile_cursor,
        popup_area,
        &title,
        Some(&hint),
        |_| None,
    );
}
