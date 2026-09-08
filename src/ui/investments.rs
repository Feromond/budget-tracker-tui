use crate::app::investments::STALE_VALUATION_DAYS;
use crate::app::state::App;
use crate::model::{InvestmentEntryKind, InvestmentRange};
use crate::ui::form::render_field_form;
use crate::ui::helpers::{centered_rect, format_amount, format_signed_amount};
use chrono::NaiveDate;
use ratatui::prelude::*;
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Clear, Dataset, GraphType, Paragraph, Row, Table, Wrap,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

const PANEL_CHROME_COLOR: Color = Color::LightBlue;
const VALUE_COLOR: Color = Color::LightCyan;
const INVESTED_COLOR: Color = Color::Magenta;
const STALE_COLOR: Color = Color::Rgb(255, 165, 0);

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

fn gain_style(amount: Decimal) -> Style {
    if amount < Decimal::ZERO {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default().fg(Color::LightGreen)
    }
}

fn format_percent(value: Option<Decimal>) -> String {
    match value {
        Some(percent) => format!(
            "{}{:.1}%",
            if percent >= Decimal::ZERO { "+" } else { "" },
            percent.to_f64().unwrap_or(0.0)
        ),
        None => "N/A".to_string(),
    }
}

fn format_rate(value: Option<f64>) -> String {
    match value {
        Some(percent) => format!("{}{:.1}%", if percent >= 0.0 { "+" } else { "" }, percent),
        None => "N/A".to_string(),
    }
}

pub fn render_investments_view(f: &mut Frame, app: &mut App, area: Rect) {
    if app.portfolio.is_empty() {
        render_empty_state(f, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(accounts_table_height(app)),
        ])
        .split(area);

    render_stats_panel(f, app, chunks[0]);
    render_value_chart(f, app, chunks[1]);
    render_accounts_table(f, app, chunks[2]);
}

pub fn render_investment_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Percentage(45),
        ])
        .split(area);

    render_stats_panel(f, app, chunks[0]);
    render_value_chart(f, app, chunks[1]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(chunks[2]);

    render_entries_table(f, app, bottom[0]);
    render_yearly_table(f, app, bottom[1]);
}

fn accounts_table_height(app: &App) -> u16 {
    // rule, header, spacer, totals
    let rows = app.visible_investment_ids().len() as u16 + 4;
    rows.clamp(5, 14)
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    let body = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "No investment accounts yet.",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Press 'a' to add one. You can give it a starting value and how much of"),
        Line::from("that you contributed, so growth is measured from the right place."),
        Line::from(""),
        Line::from(Span::styled(
            "Then record a valuation ('v') whenever you check on it, and a contribution",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "or withdrawal ('n') whenever money moves. Growth is worked out from those.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .block(panel(Line::from(heading("Investments")), Borders::TOP));

    f.render_widget(body, area);
}

fn render_stats_panel(f: &mut Frame, app: &App, area: Rect) {
    let position = app.investment_position();
    let (start, end) = app.investment_window();
    let account_id = app.investment_detail_id;
    let archived = app.show_archived_investments;

    let period_gain = app.portfolio.gain_between(account_id, start, end, archived);
    let annualized = app
        .portfolio
        .annualized_return(account_id, start, end, archived);

    let title = match account_id.and_then(|id| app.portfolio.account(id)) {
        Some(account) => {
            let mut spans = vec![heading(&account.name)];
            if !account.kind.trim().is_empty() {
                spans.push(Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)));
                spans.push(Span::styled(
                    account.kind.clone(),
                    Style::default().fg(Color::Cyan),
                ));
            }
            if account.archived {
                spans.push(Span::styled(
                    " (archived)",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            Line::from(spans)
        }
        None => Line::from(vec![
            heading("Investments"),
            Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)),
            Span::styled(
                app.active_ledger_name().to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    };

    let block = panel(title, Borders::TOP);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(inner);

    let value_lines = vec![
        stat_line(
            "Value",
            format_amount(&position.value),
            Style::default().fg(VALUE_COLOR),
        ),
        stat_line(
            "Invested",
            format_amount(&position.invested),
            Style::default().fg(INVESTED_COLOR),
        ),
    ];
    let gain_lines = vec![
        stat_line(
            "Gain",
            format_signed_amount(&position.gain()),
            gain_style(position.gain()),
        ),
        stat_line(
            "ROI",
            format_percent(position.roi()),
            gain_style(position.gain()),
        ),
    ];
    // Over All this would contradict Gain, so show freshness instead.
    let period_lines = vec![
        match app.investment_range {
            InvestmentRange::All => stat_line(
                "As of",
                position
                    .as_of
                    .map(|date| date.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string()),
                Style::default().fg(Color::DarkGray),
            ),
            range => stat_line(
                range.label(),
                format_signed_amount(&period_gain),
                gain_style(period_gain),
            ),
        },
        stat_line(
            "Annual",
            format_rate(annualized),
            Style::default().fg(Color::White),
        ),
    ];

    f.render_widget(Paragraph::new(value_lines), columns[0]);
    f.render_widget(Paragraph::new(gain_lines), columns[1]);
    f.render_widget(Paragraph::new(period_lines), columns[2]);
}

fn stat_line(label: &str, value: String, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<9}", label),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, style),
    ])
}

/// The gap between the two lines is the growth.
fn render_value_chart(f: &mut Frame, app: &App, area: Rect) {
    let (start, end) = app.investment_window();
    let points = (area.width.saturating_sub(12)).max(2) as usize;
    let series = app.portfolio.series(
        start,
        end,
        points,
        app.investment_detail_id,
        app.show_archived_investments,
    );

    let value_points: Vec<(f64, f64)> = series
        .iter()
        .enumerate()
        .map(|(index, (_, value, _))| (index as f64, value.to_f64().unwrap_or(0.0)))
        .collect();
    let invested_points: Vec<(f64, f64)> = series
        .iter()
        .enumerate()
        .map(|(index, (_, _, invested))| (index as f64, invested.to_f64().unwrap_or(0.0)))
        .collect();

    let peak = value_points
        .iter()
        .chain(invested_points.iter())
        .map(|(_, y)| *y)
        .fold(0.0f64, f64::max);
    let y_max = if peak <= 0.0 { 1.0 } else { peak * 1.1 };
    let x_max = (series.len().max(2) - 1) as f64;

    let datasets = vec![
        Dataset::default()
            .name("Invested")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(INVESTED_COLOR))
            .data(&invested_points),
        Dataset::default()
            .name("Value")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(VALUE_COLOR))
            .data(&value_points),
    ];

    let title = Line::from(vec![
        heading("Growth"),
        Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)),
        Span::styled(
            app.investment_range.label().to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" (◀/▶) ", Style::default().fg(Color::DarkGray)),
    ]);

    let chart = Chart::new(datasets)
        .block(panel(title, Borders::TOP))
        .x_axis(
            Axis::default()
                .bounds([0.0, x_max])
                .labels(axis_dates(start, end)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max])
                .labels(axis_amounts(y_max)),
        );

    f.render_widget(chart, area);
}

fn axis_dates(start: NaiveDate, end: NaiveDate) -> Vec<Span<'static>> {
    let span = (end - start).num_days();
    // Three labels all reading "Sep 2026" is worse than no labels at all.
    let format = match span {
        0..=92 => "%d %b",
        93..=730 => "%b %Y",
        _ => "%Y",
    };
    let label = |date: NaiveDate| Span::raw(date.format(format).to_string());
    if span == 0 {
        return vec![label(end)];
    }
    vec![label(start), label(start + (end - start) / 2), label(end)]
}

fn axis_amounts(y_max: f64) -> Vec<Span<'static>> {
    [0.0, y_max * 0.5, y_max]
        .iter()
        .map(|value| Span::raw(compact_amount(*value)))
        .collect()
}

fn compact_amount(value: f64) -> String {
    match value.abs() {
        v if v >= 1_000_000.0 => format!("{:.1}M", value / 1_000_000.0),
        v if v >= 1_000.0 => format!("{:.0}k", value / 1_000.0),
        _ => format!("{:.0}", value),
    }
}

fn render_accounts_table(f: &mut Frame, app: &mut App, area: Rect) {
    let today = app.today();
    let archived = app.show_archived_investments;
    let show_kind = area.width >= 88;
    let stale_ids: Vec<i64> = app
        .visible_investment_ids()
        .into_iter()
        .filter(|id| app.is_valuation_stale(*id))
        .collect();

    let mut rows: Vec<Row> = app
        .portfolio
        .visible_accounts(archived)
        .iter()
        .map(|account| {
            let position = app.portfolio.position(account.id, today);
            let stale = stale_ids.contains(&account.id);

            let (as_of_text, as_of_style) = match position.as_of {
                Some(date) if stale => (
                    format!("{} !", date.format("%Y-%m-%d")),
                    Style::default().fg(STALE_COLOR),
                ),
                Some(date) => (
                    date.format("%Y-%m-%d").to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
                None => (
                    "no value yet".to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            };

            let name = if show_kind || account.kind.trim().is_empty() {
                account.name.clone()
            } else {
                format!("{} ({})", account.name, account.kind.trim())
            };
            let name_style = if account.archived {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let mut cells = vec![Cell::from(name).style(name_style)];
            if show_kind {
                cells
                    .push(Cell::from(account.kind.clone()).style(Style::default().fg(Color::Cyan)));
            }
            cells.extend([
                Cell::from(right(format_amount(&position.value)))
                    .style(Style::default().fg(VALUE_COLOR)),
                Cell::from(as_of_text).style(as_of_style),
                Cell::from(right(format_amount(&position.invested)))
                    .style(Style::default().fg(INVESTED_COLOR)),
                Cell::from(right(format_signed_amount(&position.gain())))
                    .style(gain_style(position.gain())),
                Cell::from(right(format_percent(position.roi())))
                    .style(gain_style(position.gain())),
            ]);
            Row::new(cells)
        })
        .collect();

    let total = app.portfolio.total_position(today, archived);
    let mut total_cells =
        vec![Cell::from("Total").style(Style::default().add_modifier(Modifier::BOLD))];
    if show_kind {
        total_cells.push(Cell::from(""));
    }
    total_cells.extend([
        Cell::from(right(format_amount(&total.value))).style(
            Style::default()
                .fg(VALUE_COLOR)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from(""),
        Cell::from(right(format_amount(&total.invested)))
            .style(Style::default().fg(INVESTED_COLOR)),
        Cell::from(right(format_signed_amount(&total.gain())))
            .style(gain_style(total.gain()).add_modifier(Modifier::BOLD)),
        Cell::from(right(format_percent(total.roi()))).style(gain_style(total.gain())),
    ]);
    // Spacer without a row to navigate past.
    rows.push(Row::new(total_cells).top_margin(1));

    let mut header_cells = vec![Cell::from("Account")];
    if show_kind {
        header_cells.push(Cell::from("Type"));
    }
    header_cells.extend([
        Cell::from(right("Value".to_string())),
        Cell::from("As Of"),
        Cell::from(right("Invested".to_string())),
        Cell::from(right("Gain".to_string())),
        Cell::from(right("ROI".to_string())),
    ]);
    let header = Row::new(header_cells).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let mut widths = vec![Constraint::Min(10)];
    if show_kind {
        widths.push(Constraint::Length(12));
    }
    widths.extend([
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(7),
    ]);

    let mut title = vec![heading("Accounts")];
    if archived {
        title.push(Span::styled(
            " (incl. archived)",
            Style::default().fg(Color::DarkGray),
        ));
    } else if app.visible_investment_ids().is_empty() {
        title.push(Span::styled(
            " | all archived, press A to show",
            Style::default().fg(Color::DarkGray),
        ));
    }
    if !stale_ids.is_empty() {
        title.push(Span::styled(
            format!(
                " | {} stale (over {} days)",
                stale_ids.len(),
                STALE_VALUATION_DAYS
            ),
            Style::default().fg(STALE_COLOR),
        ));
    }

    let table = Table::new(rows, widths)
        .header(header)
        .block(panel(Line::from(title), Borders::TOP))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.investment_table_state);
}

fn render_entries_table(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(account_id) = app.investment_detail_id else {
        return;
    };

    let entries: Vec<_> = {
        let mut collected: Vec<_> = app.portfolio.entries_for(account_id).collect();
        collected.reverse();
        collected
    };

    let rows: Vec<Row> = entries
        .iter()
        .map(|entry| {
            let (label, style) = match entry.entry_kind {
                InvestmentEntryKind::Valuation => ("Valuation", Style::default().fg(VALUE_COLOR)),
                InvestmentEntryKind::Contribution => {
                    ("Contribution", Style::default().fg(Color::LightGreen))
                }
                InvestmentEntryKind::Withdrawal => {
                    ("Withdrawal", Style::default().fg(Color::LightRed))
                }
            };

            Row::new(vec![
                Cell::from(entry.date.format("%Y-%m-%d").to_string()),
                Cell::from(label).style(style),
                Cell::from(right(format_amount(&entry.amount))).style(style),
                Cell::from(entry.note.clone()).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Date"),
        Cell::from("Entry"),
        Cell::from(right("Amount".to_string())),
        Cell::from("Note"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let widths = [
        Constraint::Length(11),
        Constraint::Length(13),
        Constraint::Length(12),
        Constraint::Min(3),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(panel(Line::from(heading("History")), Borders::TOP))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.investment_entry_table_state);
}

fn render_yearly_table(f: &mut Frame, app: &App, area: Rect) {
    let breakdown = app
        .portfolio
        .yearly_breakdown(app.investment_detail_id, app.show_archived_investments);

    let rows: Vec<Row> = breakdown
        .iter()
        .rev()
        .map(|(year, invested, growth, ret)| {
            Row::new(vec![
                Cell::from(year.to_string()).style(Style::default().fg(Color::Magenta)),
                Cell::from(right(format_amount(invested)))
                    .style(Style::default().fg(INVESTED_COLOR)),
                Cell::from(right(format_signed_amount(growth))).style(gain_style(*growth)),
                Cell::from(right(format_percent(*ret))).style(gain_style(*growth)),
            ])
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Year"),
        Cell::from(right("Invested".to_string())),
        Cell::from(right("Growth".to_string())),
        Cell::from(right("Return".to_string())),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let widths = [
        Constraint::Length(6),
        Constraint::Min(8),
        Constraint::Min(8),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths).header(header).block(panel(
        Line::from(heading("By Year")),
        Borders::TOP | Borders::LEFT,
    ));

    f.render_widget(table, area);
}

pub fn render_investment_account_editor(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 80, area);
    f.render_widget(Clear, popup_area);

    let title = if app.editing_investment_account_id.is_some() {
        " Edit Investment Account "
    } else {
        " Add Investment Account "
    };

    render_field_form(
        f,
        &app.investment_account_fields,
        app.investment_account_cursor,
        popup_area,
        title,
        Some(" [Esc] Cancel, [Enter] Save "),
        if app.investment_opening_fields_active() {
            |_| None
        } else {
            |field| {
                matches!(
                    field,
                    crate::app::fields::InvestmentAccountField::OpeningValue
                        | crate::app::fields::InvestmentAccountField::OpeningInvested
                        | crate::app::fields::InvestmentAccountField::OpeningDate
                )
                .then_some("Edit the account's entries instead")
            }
        },
    );
}

pub fn render_investment_entry_editor(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let account = app
        .active_investment_account_id()
        .and_then(|id| app.portfolio.account(id))
        .map(|account| account.name.as_str())
        .unwrap_or("Investment");
    let title = if app.editing_investment_entry_id.is_some() {
        format!(" Edit Entry | {} ", account)
    } else {
        format!(" Record Entry | {} ", account)
    };

    render_field_form(
        f,
        &app.investment_entry_fields,
        app.investment_entry_cursor,
        popup_area,
        &title,
        Some(" [Esc] Cancel, [Enter] Toggle/Save "),
        |_| None,
    );
}

fn right(text: String) -> Line<'static> {
    Line::from(text).alignment(Alignment::Right)
}
