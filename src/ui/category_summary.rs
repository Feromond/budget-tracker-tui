use crate::app::state::{App, CategorySummaryItem};
use crate::model::{MonthlySummary, TransactionType};
use crate::ui::helpers::{format_amount, month_to_short_str};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::collections::HashMap;

// Dim these amounts because they are already included in the total above.
fn cell_detail_amount(amount: Decimal, income: bool) -> Cell<'static> {
    let color = if income {
        Color::LightGreen
    } else {
        Color::LightRed
    };
    Cell::from(Line::from(format_amount(&amount)).alignment(Alignment::Right))
        .style(Style::default().fg(color).add_modifier(Modifier::DIM))
}

fn next_sibling(items: &[CategorySummaryItem], from: usize) -> Option<&CategorySummaryItem> {
    items[from + 1..]
        .iter()
        .find(|item| !matches!(item, CategorySummaryItem::Transaction(_, _)))
}

fn cell_income(amount: Decimal, bold: bool) -> Cell<'static> {
    if amount.round_dp(2).is_zero() {
        return Cell::from("");
    }
    let mut style = Style::default().fg(Color::LightGreen);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Cell::from(Line::from(format_amount(&amount)).alignment(Alignment::Right)).style(style)
}
fn cell_expense(amount: Decimal, bold: bool) -> Cell<'static> {
    if amount.round_dp(2).is_zero() {
        return Cell::from("");
    }
    let mut style = Style::default().fg(Color::LightRed);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Cell::from(Line::from(format_amount(&amount)).alignment(Alignment::Right)).style(style)
}
fn cell_net(net: Decimal, bold: bool) -> Cell<'static> {
    let s = if net >= Decimal::ZERO {
        format!("+{}", format_amount(&net))
    } else {
        format_amount(&net)
    };
    let mut style = if net >= Decimal::ZERO {
        Style::default().fg(Color::LightGreen)
    } else {
        Style::default().fg(Color::LightRed)
    };
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    Cell::from(Line::from(s).alignment(Alignment::Right)).style(style)
}

pub fn render_category_summary_view(f: &mut Frame, app: &mut App, area: Rect) {
    let summary_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let table_area = summary_chunks[0];
    let main_table_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(table_area);
    let list_area = main_table_chunks[0];
    let footer_area = main_table_chunks[1];

    let chart_area = summary_chunks[1];

    // Table Setup
    let header_titles = [
        "Month",
        "Category",
        "Subcategory",
        "Income",
        "Expense",
        "Net",
    ];
    let header_cells = header_titles.iter().enumerate().map(|(i, h)| {
        let (content, style) = if i >= 3 {
            let s = match i {
                3 => Style::default().fg(Color::LightGreen).bold(), // Income
                4 => Style::default().fg(Color::LightRed).bold(),   // Expense
                5 => Style::default().fg(Color::LightBlue).bold(),  // Net
                _ => Style::default().fg(Color::Cyan).bold(),
            };
            (Line::from(*h).alignment(Alignment::Right), s)
        } else {
            (Line::from(*h), Style::default().fg(Color::Cyan).bold())
        };
        Cell::from(content).style(style)
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    // Data for Table (hierarchical)
    let current_year = app
        .category_summary_years
        .get(app.category_summary_year_index)
        .copied();
    let year_str = current_year.map_or_else(|| "N/A".to_string(), |y| y.to_string());
    let items = &app.cached_visible_category_items;
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
    // Build sorted months for the current year for color mapping
    let mut months: Vec<u32> = vec![];
    if let Some(year) = current_year {
        months = app.sorted_category_months_for_year(year);
    }

    let (total_income, total_expense) = crate::app::util::calculate_totals(app, current_year);

    let selected_row = app.category_summary_table_state.selected();
    let today = chrono::Local::now().date_naive();
    let mut last_expanded_month: Option<u32> = None;
    let mut parent_subcategory_is_last = true;
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(i, item)| match item {
            CategorySummaryItem::Month(month, summary) => {
                let symbol = if app.expanded_category_summary_months.contains(month) {
                    "▼"
                } else {
                    "▶"
                };
                let month_idx = months.iter().position(|&m| m == *month).unwrap_or(0);
                let arrow_color = color_palette[month_idx % color_palette.len()];
                let month_cell = Cell::from(Line::from(vec![
                    Span::styled(symbol, Style::default().fg(arrow_color)),
                    Span::raw(" "),
                    Span::raw(month_to_short_str(*month)),
                ]));
                if app.expanded_category_summary_months.contains(month) {
                    last_expanded_month = Some(*month);
                } else {
                    last_expanded_month = None;
                }
                let inc_cell = cell_income(summary.income, true);
                let exp_cell = cell_expense(summary.expense, true);
                let net_cell = cell_net(summary.income - summary.expense, true);
                Row::new(vec![
                    month_cell,
                    Cell::from(""),
                    Cell::from(""),
                    inc_cell,
                    exp_cell,
                    net_cell,
                ])
                .height(1)
                .bottom_margin(0)
                .style(Style::default().bg(Color::Rgb(20, 20, 20))) // Dark gray background on month rows
            }
            CategorySummaryItem::Subcategory(month, category, sub, summary) => {
                let child_count = items[i + 1..]
                    .iter()
                    .take_while(|item| matches!(item, CategorySummaryItem::Transaction(_, _)))
                    .count();
                let is_last_child = match next_sibling(items, i) {
                    Some(CategorySummaryItem::Subcategory(next_m, _, _, _)) => *next_m != *month,
                    _ => true,
                };
                parent_subcategory_is_last = is_last_child;

                let mut first_cell = Cell::from("");
                if let Some(expanded_month) = last_expanded_month
                    && expanded_month == *month
                {
                    let month_idx = months.iter().position(|&m| m == *month).unwrap_or(0);
                    let arrow_color = color_palette[month_idx % color_palette.len()];
                    let tree_symbol = if is_last_child { "└─" } else { "├─" };

                    first_cell = Cell::from(Line::from(vec![
                        Span::styled(tree_symbol, Style::default().fg(arrow_color)),
                        Span::raw(" "),
                    ]));
                }
                let hint = if child_count > 0 {
                    format!(" ▾ ({})", child_count)
                } else if selected_row == Some(i) {
                    " ▸".to_string()
                } else {
                    String::new()
                };
                let display_sub = if sub.is_empty() { "Uncategorized" } else { sub };
                let inc_cell = cell_income(summary.income, false);
                let exp_cell = cell_expense(summary.expense, false);
                let net_cell = cell_net(summary.income - summary.expense, false);
                Row::new(vec![
                    first_cell,
                    Cell::from(category.clone()),
                    Cell::from(format!("{}{}", display_sub, hint)),
                    inc_cell,
                    exp_cell,
                    net_cell,
                ])
                .height(1)
                .bottom_margin(0)
            }
            CategorySummaryItem::Transaction(month, index) => {
                let Some(tx) = app.transactions.get(*index) else {
                    return Row::new(vec![Cell::from("Error: Invalid Index").fg(Color::Red)])
                        .height(1)
                        .bottom_margin(0);
                };
                let month_idx = months.iter().position(|&m| m == *month).unwrap_or(0);
                let arrow_color = color_palette[month_idx % color_palette.len()];
                let month_guide = if parent_subcategory_is_last {
                    "  "
                } else {
                    "│ "
                };
                let is_last_transaction = !matches!(
                    items.get(i + 1),
                    Some(CategorySummaryItem::Transaction(_, _))
                );
                let branch = if is_last_transaction {
                    "└─ "
                } else {
                    "├─ "
                };

                let description = if tx.is_recurring {
                    if tx.is_generated_from_recurring {
                        format!("⟲ {}", tx.description)
                    } else {
                        format!("⟲* {}", tx.description)
                    }
                } else {
                    tx.description.clone()
                };

                let (inc_cell, exp_cell) = match tx.transaction_type {
                    TransactionType::Income => {
                        (cell_detail_amount(tx.amount, true), Cell::from(""))
                    }
                    TransactionType::Expense => {
                        (Cell::from(""), cell_detail_amount(tx.amount, false))
                    }
                };

                let row = Row::new(vec![
                    Cell::from(Span::styled(month_guide, Style::default().fg(arrow_color))),
                    Cell::from(Line::from(vec![
                        Span::styled(
                            format!("  {}", branch),
                            Style::default().fg(arrow_color).add_modifier(Modifier::DIM),
                        ),
                        Span::styled(
                            tx.date.format("%b %d (%a)").to_string(),
                            Style::default().fg(Color::Gray),
                        ),
                    ])),
                    Cell::from(description),
                    inc_cell,
                    exp_cell,
                    Cell::from(""),
                ])
                .height(1)
                .bottom_margin(0);
                if tx.is_generated_from_recurring && tx.date > today {
                    row.style(Style::default().add_modifier(Modifier::DIM))
                } else {
                    row
                }
            }
        })
        .collect();

    // Prepend Yearly Total Row
    let total_net = total_income - total_expense;
    let total_inc_cell = cell_income(total_income, true);
    let total_exp_cell = cell_expense(total_expense, true);
    let total_net_cell = cell_net(total_net, true);

    let total_row = Row::new(vec![
        Cell::from(Span::styled(
            "Grand Total",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        )),
        Cell::from(Span::styled(
            &year_str,
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        )),
        Cell::from(""),
        total_inc_cell,
        total_exp_cell,
        total_net_cell,
    ])
    .height(1)
    .bottom_margin(0)
    .style(Style::default().bg(Color::Rgb(10, 10, 10)));

    let is_filtered = app.filtered_indices.len() != app.transactions.len();
    let table_title = {
        let y = year_str.clone();
        let idx = app.category_summary_year_index + 1;
        let total = app.category_summary_years.len().max(1);
        let idx_total = format!(" ({}/{})", idx, total);
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
            "Category/Subcategory Summary - ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            y,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            idx_total,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        Line::from(title_spans)
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(11),
        ],
    )
    .header(header)
    .block(Block::default().title(table_title).borders(Borders::TOP))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(" > ");

    f.render_stateful_widget(table, list_area, &mut app.category_summary_table_state);

    let footer_table = Table::new(
        vec![total_row],
        [
            Constraint::Length(12),
            Constraint::Percentage(23),
            Constraint::Percentage(30),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(11),
        ],
    );

    f.render_widget(footer_table, footer_area);

    // Chart Setup
    let mut chart_title = Line::from(vec![Span::styled(
        "Category Net Balance",
        Style::default().add_modifier(Modifier::BOLD),
    )]);
    let mut bars: Vec<Bar> = Vec::new();
    let mut max_abs_chart_value: u64 = 10;

    if let Some(selected_index) = app.category_summary_table_state.selected()
        && let Some(item) = items.get(selected_index)
        && let Some(year) = current_year
    {
        let selected_month = match item {
            CategorySummaryItem::Month(m, _) => *m,
            CategorySummaryItem::Subcategory(m, _, _, _) => *m,
            CategorySummaryItem::Transaction(m, _) => *m,
        };
        let mut category_totals: HashMap<String, MonthlySummary> = HashMap::new();
        if let Some(month_map) = app.category_summaries.get(&(year, selected_month)) {
            for ((category, _), summary) in month_map.iter() {
                let cat_summary = category_totals.entry(category.clone()).or_default();
                cat_summary.income += summary.income;
                cat_summary.expense += summary.expense;
            }
        }

        let mut category_data_for_chart: Vec<(String, Decimal)> = category_totals
            .into_iter()
            .map(|(cat, summary)| {
                let net_balance = summary.income - summary.expense;
                (cat, net_balance)
            })
            .collect();

        category_data_for_chart.sort_by(|(c1, _), (c2, _)| c1.cmp(c2));

        let mut current_max: i64 = 0;
        bars = category_data_for_chart
            .iter()
            .map(|(cat, net)| {
                let net_val = *net;
                let net_i64 = net_val.round().to_i64().unwrap_or(0);
                let net_style = if net_val >= Decimal::ZERO {
                    Style::default().fg(Color::LightGreen)
                } else {
                    Style::default().fg(Color::LightRed)
                };
                current_max = current_max.max(net_i64.abs());
                Bar::default()
                    .label(cat.chars().take(15).collect::<String>())
                    .value(net_i64.unsigned_abs())
                    .style(net_style)
            })
            .collect();
        max_abs_chart_value = (current_max as u64).max(10);
        let month_idx = months
            .iter()
            .position(|&m| m == selected_month)
            .unwrap_or(0);
        let month_color = color_palette[month_idx % color_palette.len()];
        let y = year_str.clone();
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
            "── Category Net Balance ─ ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            month_to_short_str(selected_month),
            Style::default()
                .fg(month_color)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            " ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            y,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        chart_title = Line::from(title_spans);
    }

    if bars.is_empty() {
        bars.push(Bar::default().label("No Data").value(0));
        let y = year_str.clone();
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
            "Category Net Balance - ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            y,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        title_spans.push(Span::styled(
            " (Select Row)",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        chart_title = Line::from(title_spans);
    }

    let num_bars = bars.len() as u16;
    let usable_width = chart_area.width.saturating_sub(2);
    let width_per_bar_and_gap = (usable_width / num_bars.max(1)).max(1);
    let bar_gap = if width_per_bar_and_gap > 1 {
        1u16
    } else {
        0u16
    };
    let bar_width = width_per_bar_and_gap.saturating_sub(bar_gap).max(1);

    let bar_chart = BarChart::default()
        .block(Block::default().title(chart_title).borders(Borders::TOP))
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .group_gap(0)
        .label_style(Style::default().fg(Color::White))
        .max(max_abs_chart_value);

    f.render_widget(bar_chart, chart_area);
}
