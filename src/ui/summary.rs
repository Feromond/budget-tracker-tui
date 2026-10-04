use crate::app::state::App;
use crate::model::BudgetMonth;
use crate::ui::helpers::{TRANSFER_COLOR, format_amount, format_hours, month_to_short_str};
use crate::validation::days_in_month;
use chrono::Datelike;
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{
    Axis, Bar, BarChart, BarGroup, Block, Borders, Chart, Dataset, GraphType, Paragraph,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

pub fn render_summary_view(f: &mut Frame, app: &mut App, area: Rect) {
    let summary_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let line_chart_area = summary_chunks[0];
    let bar_chart_area = summary_chunks[1];

    let current_year = app
        .summary_years
        .get(app.selected_summary_year_index)
        .copied();
    let year_str = current_year.map_or_else(|| "N/A".to_string(), |y| y.to_string());
    let year_progress = format!(
        "({}/{})",
        app.selected_summary_year_index + 1,
        app.summary_years.len().max(1)
    );

    // --- Line Chart: Daily Expenses for Selected Month or All Months (Multi Mode) ---
    let mut months: Vec<u32> = vec![];
    if let Some(year) = current_year {
        months = app.sorted_months_for_year(year);
    }
    let color_palette = [
        Color::LightRed,
        Color::LightGreen,
        Color::LightBlue,
        Color::LightYellow,
        Color::LightMagenta,
        Color::LightCyan,
        Color::Red,
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
    ];
    let mut all_line_data: Vec<Vec<(f64, f64)>> = vec![];
    let mut legend_labels = vec![];
    let mut max_expense = Decimal::ZERO;
    let mut datasets = vec![];
    if app.summary_multi_month_mode {
        for &month in &months {
            let year = current_year.unwrap_or(0);
            let num_days = days_in_month(year, month) as usize;
            let mut daily_expenses = vec![Decimal::ZERO; num_days];
            for &idx in &app.filtered_indices {
                let tx = &app.transactions[idx];
                if tx.date.year() == year
                    && tx.date.month() == month
                    && let crate::model::TransactionType::Expense = tx.transaction_type
                {
                    let day = tx.date.day() as usize;
                    if day > 0 && day <= num_days {
                        daily_expenses[day - 1] += tx.amount;
                    }
                }
            }
            let line_data: Vec<(f64, f64)> = if app.summary_cumulative_mode {
                let mut cum = Decimal::ZERO;
                daily_expenses
                    .iter()
                    .enumerate()
                    .map(|(i, &amt)| {
                        cum += amt;
                        if cum > max_expense {
                            max_expense = cum;
                        }
                        ((i + 1) as f64, cum.to_f64().unwrap_or(0.0))
                    })
                    .collect()
            } else {
                daily_expenses
                    .iter()
                    .enumerate()
                    .map(|(i, &amt)| {
                        if amt > max_expense {
                            max_expense = amt;
                        }
                        ((i + 1) as f64, amt.to_f64().unwrap_or(0.0))
                    })
                    .collect()
            };
            all_line_data.push(line_data);
        }
        for (i, (&month, line_data)) in months.iter().zip(all_line_data.iter()).enumerate() {
            datasets.push(
                Dataset::default()
                    .name(month_to_short_str(month))
                    .marker(ratatui::symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(color_palette[i % color_palette.len()]))
                    .data(line_data),
            );
            legend_labels.push(Span::styled(
                month_to_short_str(month),
                Style::default().fg(color_palette[i % color_palette.len()]),
            ));
        }
    } else {
        all_line_data.clear();
        if let (Some(year), Some(month)) = (current_year, app.selected_summary_month) {
            let num_days = days_in_month(year, month) as usize;
            let mut daily_expenses = vec![Decimal::ZERO; num_days];
            for &idx in &app.filtered_indices {
                let tx = &app.transactions[idx];
                if tx.date.year() == year
                    && tx.date.month() == month
                    && let crate::model::TransactionType::Expense = tx.transaction_type
                {
                    let day = tx.date.day() as usize;
                    if day > 0 && day <= num_days {
                        daily_expenses[day - 1] += tx.amount;
                    }
                }
            }
            let line_data: Vec<(f64, f64)> = if app.summary_cumulative_mode {
                let mut cum = Decimal::ZERO;
                daily_expenses
                    .iter()
                    .enumerate()
                    .map(|(i, &amt)| {
                        cum += amt;
                        if cum > max_expense {
                            max_expense = cum;
                        }
                        ((i + 1) as f64, cum.to_f64().unwrap_or(0.0))
                    })
                    .collect()
            } else {
                daily_expenses
                    .iter()
                    .enumerate()
                    .map(|(i, &amt)| {
                        if amt > max_expense {
                            max_expense = amt;
                        }
                        ((i + 1) as f64, amt.to_f64().unwrap_or(0.0))
                    })
                    .collect()
            };
            all_line_data.push(line_data);
            let data_ref = all_line_data.last().unwrap();
            // Determine color for this month (same as in title)
            let month_color = app
                .selected_summary_month
                .and_then(|m| months.iter().position(|&x| x == m))
                .map(|idx| color_palette[idx % color_palette.len()])
                .unwrap_or(Color::White);
            datasets.push(
                Dataset::default()
                    .name(month_to_short_str(month))
                    .marker(ratatui::symbols::Marker::Braille)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(month_color))
                    .data(data_ref),
            );
            legend_labels.push(Span::styled(
                month_to_short_str(month),
                Style::default().fg(month_color),
            ));
        }
    }
    let month_index =
        if let (Some(m), false) = (app.selected_summary_month, app.summary_multi_month_mode) {
            months
                .iter()
                .position(|&x| x == m)
                .map(|i| i + 1)
                .unwrap_or(1)
        } else {
            1
        };
    let month_count = months.len().max(1);
    let year_str_owned = year_str.clone();
    let year_progress_owned = year_progress.clone();
    let is_filtered = app.filtered_indices.len() != app.transactions.len();
    let chart_title = if app.summary_multi_month_mode {
        let y = year_str_owned.clone();
        let yp = year_progress_owned.clone();
        let mut title_spans = vec![];
        if is_filtered {
            title_spans.push(Span::styled(
                "(Filtered) ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        title_spans.push(Span::styled(
            "Daily Spending",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        if app.summary_cumulative_mode {
            title_spans.push(Span::styled(
                " (Cumulative)",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        title_spans.push(Span::styled(
            " (All Months)",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            " - ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            y,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            " ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            yp,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        Line::from(title_spans)
    } else {
        let month_color = app
            .selected_summary_month
            .and_then(|m| months.iter().position(|&x| x == m))
            .map(|idx| color_palette[idx % color_palette.len()])
            .unwrap_or(Color::White);
        let month_str = app
            .selected_summary_month
            .map(month_to_short_str)
            .unwrap_or("-")
            .to_string();
        let y = year_str_owned.clone();
        let mut title_spans = vec![];
        if is_filtered {
            title_spans.push(Span::styled(
                "(Filtered) ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        title_spans.push(Span::styled(
            "Daily Spending",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        if app.summary_cumulative_mode {
            title_spans.push(Span::styled(
                " (Cumulative)",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        title_spans.push(Span::styled(
            " - ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            y,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            " ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            month_str,
            Style::default()
                .fg(month_color)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            format!(" ({}/{})", month_index, month_count),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        Line::from(title_spans)
    };
    let year = current_year.unwrap_or(0);
    let max_days = months
        .iter()
        .map(|&m| days_in_month(year, m))
        .max()
        .unwrap_or(31);
    let x_max = max_days as f64;
    // Generate x-axis labels at key points: start, 1/4, 1/2, 3/4, end
    let mut x_labels = vec![];
    let n_ticks = 5;
    for i in 0..n_ticks {
        let pos: f64 = 1.0 + ((x_max - 1.0) * (i as f64) / (n_ticks as f64 - 1.0));
        x_labels.push(Span::raw(format!("{:.0}", pos.round())));
    }
    // Y-axis ticks: 0, 1/4 max, 1/2 max, 3/4 max, max
    let y_max = max_expense.to_f64().unwrap_or(0.0).max(10.0);
    let y_labels = vec![
        Span::raw("0"),
        Span::raw(format!("{:.0}", y_max * 0.25)),
        Span::raw(format!("{:.0}", y_max * 0.5)),
        Span::raw(format!("{:.0}", y_max * 0.75)),
        Span::raw(format!("{:.0}", y_max)),
    ];

    // Add cumulative budget line if in cumulative mode and budget is set
    let mut cumulative_budget_line: Option<Vec<(f64, f64)>> = None;
    if app.summary_cumulative_mode
        && !app.summary_multi_month_mode
        && let (Some(year), Some(month)) = (current_year, app.selected_summary_month)
        && let Some(budget) = app
            .budget_schedule
            .monthly_budget(BudgetMonth::new(year, month))
    {
        let num_days = days_in_month(year, month) as usize;
        if num_days > 0 {
            let daily_budget = budget / Decimal::from(num_days);
            let budget_line: Vec<(f64, f64)> = (1..=num_days)
                .map(|d| {
                    (
                        d as f64,
                        (daily_budget * Decimal::from(d)).to_f64().unwrap_or(0.0),
                    )
                })
                .collect();
            cumulative_budget_line = Some(budget_line);
            legend_labels.push(Span::styled(
                "CumuBudget",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::DIM),
            ));
        }
    }
    if let Some(ref budget_line) = cumulative_budget_line {
        datasets.push(
            Dataset::default()
                .name("CumuBudget")
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::DIM),
                )
                .data(budget_line),
        );
    }
    let chart = Chart::new(datasets)
        .block(Block::default().title(chart_title).borders(Borders::TOP))
        .x_axis(
            Axis::default()
                .title("Day")
                .bounds([1.0, x_max])
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Spending")
                .bounds([0.0, y_max])
                .labels(y_labels),
        );
    f.render_widget(chart, line_chart_area);
    // Show legend if needed (single or multi month)
    if (app.summary_multi_month_mode && !legend_labels.is_empty())
        || (!app.summary_multi_month_mode
            && (!legend_labels.is_empty() || cumulative_budget_line.is_some()))
    {
        let legend_line = Line::from(legend_labels);
        let legend_area = Rect {
            x: line_chart_area.x + 2,
            y: line_chart_area.y + 2, // below the title
            width: line_chart_area.width.saturating_sub(4),
            height: 1,
        };
        f.render_widget(legend_line, legend_area);
    }

    // --- Bar Chart: Monthly Net Balance ---
    let table_title = {
        let y = year_str_owned.clone();
        let yp = year_progress_owned.clone();
        let mut title_spans = vec![];
        if is_filtered {
            title_spans.push(Span::styled(
                "(Filtered) ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        title_spans.push(Span::styled(
            "Monthly Net Balance - ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            y,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            " ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            yp,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        Line::from(title_spans)
    };
    let mut chart_data_styled: Vec<Bar> = Vec::with_capacity(12);
    let mut max_abs_chart_value = 0i64;
    if let Some(year) = current_year {
        for month in 1..=12 {
            let summary = app
                .monthly_summaries
                .get(&(year, month))
                .cloned()
                .unwrap_or_default();
            let net = summary.income - summary.expense;
            let net_i64 = net.round().to_i64().unwrap_or(0);
            chart_data_styled.push(
                Bar::default()
                    .label(month_to_short_str(month))
                    .value(net_i64.unsigned_abs())
                    .style(if net >= Decimal::ZERO {
                        Style::default().fg(Color::LightGreen)
                    } else {
                        Style::default().fg(Color::LightRed)
                    }),
            );
            max_abs_chart_value = max_abs_chart_value.max(net_i64.abs());
        }
    } else {
        chart_data_styled.push(Bar::default().label("N/A").value(0));
    }
    let num_bars = chart_data_styled.len() as u16;
    let usable_width = bar_chart_area.width.saturating_sub(2);
    let width_per_bar_and_gap = (usable_width / num_bars.max(1)).max(1);
    let bar_gap = if width_per_bar_and_gap > 1 {
        1u16
    } else {
        0u16
    };
    let bar_width = width_per_bar_and_gap.saturating_sub(bar_gap).max(1);
    let bar_chart = BarChart::default()
        .block(Block::default().title(table_title).borders(Borders::TOP))
        .data(BarGroup::default().bars(&chart_data_styled))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .group_gap(0)
        .label_style(Style::default().fg(Color::White))
        .max(max_abs_chart_value.max(10) as u64);
    f.render_widget(bar_chart, bar_chart_area);
}

pub fn render_summary_bar(f: &mut Frame, app: &App, area: Rect, year_filter: Option<i32>) {
    let totals = crate::app::util::calculate_totals(app, year_filter);
    let (total_income, total_expense) = (totals.income, totals.expense);
    let net_balance = total_income - total_expense;

    let income_str = if app.show_hours {
        format_hours(&total_income, app.hourly_rate)
    } else {
        format_amount(&total_income)
    };
    let expense_str = if app.show_hours {
        format_hours(&total_expense, app.hourly_rate)
    } else {
        format_amount(&total_expense)
    };
    let net_str_val = if app.show_hours {
        format_hours(&net_balance, app.hourly_rate)
    } else {
        format_amount(&net_balance)
    };

    let income_span = Span::styled(
        format!("Income: {}", income_str),
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD),
    );
    let expense_span = Span::styled(
        format!("Expenses: {}", expense_str),
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD),
    );
    let net_style = if net_balance >= Decimal::ZERO {
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD)
    };
    let net_str = if net_balance >= Decimal::ZERO {
        format!("+{}", net_str_val)
    } else {
        net_str_val
    };
    let net_span = Span::styled(format!("Net: {}", net_str), net_style);

    let mut spans = vec![
        income_span,
        Span::raw(" | "),
        expense_span,
        Span::raw(" | "),
        net_span,
    ];
    if !totals.transferred.is_zero() {
        let transferred_str = if app.show_hours {
            format_hours(&totals.transferred, app.hourly_rate)
        } else {
            format_amount(&totals.transferred)
        };
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            format!("Transferred: {}", transferred_str),
            Style::default()
                .fg(TRANSFER_COLOR)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(account) = app.account_scope.and_then(|id| app.accounts.get(id)) {
        let balance = app.account_balance(account);
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(
            format!(
                "{}: {}",
                crate::ui::account_manager::balance_label(account, balance),
                crate::ui::account_manager::balance_amount(account, balance)
            ),
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ));
    }
    let summary_line = Line::from(spans).alignment(Alignment::Center);

    let is_filtered = app.filtered_indices.len() != app.transactions.len();
    let mut title = match (year_filter, is_filtered) {
        (Some(year), true) => format!("Grand Total - {} (Filtered)", year),
        (Some(year), false) => format!("Grand Total - {}", year),
        (None, true) => "Grand Total (Filtered)".to_string(),
        (None, false) => "Grand Total (All Transactions)".to_string(),
    };
    if app.account_scope.is_some() {
        title.push_str(&format!(" · {}", app.account_scope_label()));
    }

    let summary_paragraph =
        Paragraph::new(summary_line).block(Block::default().borders(Borders::ALL).title(
            Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        ));

    f.render_widget(summary_paragraph, area);
}
