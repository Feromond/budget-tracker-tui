use crate::app::state::{App, BudgetCategoryComparison, BudgetEditTarget};
use crate::model::{BudgetEditScope, BudgetMonth};
use crate::ui::helpers::{format_amount, format_signed_amount, month_to_color, month_to_short_str};
use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

const PANEL_CHROME_COLOR: Color = Color::LightBlue;
const SELECTED_MONTH_BAR_COLOR: Color = Color::Rgb(255, 165, 0);
const BUDGET_GUIDE_SYMBOL: &str = "━";
const WARNING_COLOR: Color = Color::Rgb(255, 165, 0); // orange

fn budget_panel_block(title: Line<'static>, borders: Borders) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(borders)
        .border_style(Style::default().fg(PANEL_CHROME_COLOR))
}

fn usage_color(actual: Decimal, target: Decimal) -> Color {
    if target <= Decimal::ZERO {
        return Color::DarkGray;
    }
    let ratio = (actual / target).to_f64().unwrap_or(0.0);
    if ratio > 1.0 {
        Color::LightRed
    } else if ratio >= 0.85 {
        WARNING_COLOR
    } else if ratio >= 0.60 {
        Color::LightYellow
    } else {
        Color::LightGreen
    }
}

fn usage_percent(actual: Decimal, target: Decimal) -> String {
    if target <= Decimal::ZERO {
        return "N/A".to_string();
    }

    let percent = ((actual / target) * Decimal::from(100))
        .round_dp(1)
        .to_f64()
        .unwrap_or(0.0);
    format!("{percent:.1}%")
}

fn average_monthly_expense(monthly: &[(u32, Decimal)]) -> Decimal {
    if monthly.is_empty() {
        return Decimal::ZERO;
    }

    let total: Decimal = monthly.iter().map(|(_, expense)| *expense).sum();
    total / Decimal::from(monthly.len() as u32)
}

fn comparison_row(comparison: &BudgetCategoryComparison) -> Row<'static> {
    let remaining = comparison.budget - comparison.actual_expense;
    let spent_style = if comparison.actual_expense > comparison.budget {
        Style::default().fg(Color::LightRed)
    } else {
        Style::default().fg(Color::LightGreen)
    };
    let remaining_style = if remaining >= Decimal::ZERO {
        Style::default().fg(Color::LightGreen)
    } else {
        Style::default().fg(Color::LightRed)
    };

    let subcategory = if comparison.subcategory.is_empty() {
        "-".to_string()
    } else {
        comparison.subcategory.clone()
    };

    Row::new(vec![
        Cell::from(comparison.category.clone()),
        Cell::from(subcategory),
        Cell::from(Line::from(format_amount(&comparison.budget)).alignment(Alignment::Right))
            .style(Style::default().fg(Color::LightBlue)),
        Cell::from(
            Line::from(format_amount(&comparison.actual_expense)).alignment(Alignment::Right),
        )
        .style(spent_style),
        Cell::from(Line::from(format_signed_amount(&remaining)).alignment(Alignment::Right))
            .style(remaining_style),
        Cell::from(
            Line::from(usage_percent(comparison.actual_expense, comparison.budget))
                .alignment(Alignment::Right),
        )
        .style(Style::default().fg(usage_color(comparison.actual_expense, comparison.budget))),
    ])
}

fn title_with_month(
    prefix: &str,
    month: Option<u32>,
    year_label: &str,
    suffix: Option<&str>,
    is_filtered: bool,
) -> Line<'static> {
    let mut spans = vec![];
    if is_filtered {
        spans.push(Span::styled(
            "(Filtered) ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(
        prefix.to_string(),
        Style::default()
            .fg(PANEL_CHROME_COLOR)
            .add_modifier(Modifier::BOLD),
    ));
    if let Some(month) = month {
        spans.push(Span::styled(
            month_to_short_str(month).to_string(),
            Style::default()
                .fg(month_to_color(month))
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(
        year_label.to_string(),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));
    if let Some(suffix) = suffix {
        spans.push(Span::styled(
            suffix.to_string(),
            Style::default()
                .fg(PANEL_CHROME_COLOR)
                .add_modifier(Modifier::BOLD),
        ));
    }
    Line::from(spans)
}

fn compact_selected_budget_title(
    comparison: Option<&BudgetCategoryComparison>,
    width: u16,
) -> Line<'static> {
    match comparison {
        Some(comparison) if width >= 34 => Line::from(vec![
            Span::styled(
                "Selected Budget".to_string(),
                Style::default()
                    .fg(PANEL_CHROME_COLOR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)),
            Span::styled(
                comparison.category.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        _ => Line::from(vec![Span::styled(
            "Selected Budget".to_string(),
            Style::default()
                .fg(PANEL_CHROME_COLOR)
                .add_modifier(Modifier::BOLD),
        )]),
    }
}

fn compact_yearly_pattern_title(
    comparison: Option<&BudgetCategoryComparison>,
    selected_month: Option<u32>,
    width: u16,
) -> Line<'static> {
    match comparison {
        Some(comparison) if width >= 42 => {
            let mut spans = vec![Span::styled(
                "Yearly Pattern".to_string(),
                Style::default()
                    .fg(PANEL_CHROME_COLOR)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.push(Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)));
            spans.push(Span::styled(
                format_amount(&comparison.budget),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ));
            if let Some(month) = selected_month {
                spans.push(Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)));
                spans.push(Span::styled(
                    month_to_short_str(month).to_string(),
                    Style::default()
                        .fg(month_to_color(month))
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(spans)
        }
        Some(comparison) if width >= 28 => Line::from(vec![
            Span::styled(
                "Yearly Pattern".to_string(),
                Style::default()
                    .fg(PANEL_CHROME_COLOR)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(PANEL_CHROME_COLOR)),
            Span::styled(
                format_amount(&comparison.budget),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        _ => Line::from(vec![Span::styled(
            "Yearly Pattern".to_string(),
            Style::default()
                .fg(PANEL_CHROME_COLOR)
                .add_modifier(Modifier::BOLD),
        )]),
    }
}

pub fn render_budget_target_editor(f: &mut Frame, app: &App, area: Rect) {
    let scopes = app.budget_edit_scopes();
    // Fixed size: a percentage of a short terminal clips the input box.
    let width = area.width.saturating_sub(4).clamp(24, 52).min(area.width);
    let height = (7 + scopes.len() as u16).min(area.height);
    let popup_area = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    };
    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(scopes.len() as u16),
            Constraint::Min(0),
        ])
        .split(popup_area);

    let title = match app.budget_edit_target {
        Some(BudgetEditTarget::MonthlyBudget) => " Monthly Budget ",
        _ => " Category Budget ",
    };
    let block = Block::default()
        .title(title)
        .title_bottom(" [Enter] Save, [Up/Down] Scope, [Esc] Cancel ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PANEL_CHROME_COLOR));
    f.render_widget(block, popup_area);

    let label = app.budget_edit_label().unwrap_or_default();
    let heading = Paragraph::new(Line::from(Span::styled(
        label,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);
    f.render_widget(heading, chunks[0]);

    let input = Paragraph::new(app.budget_edit_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Amount (empty clears)")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(input, chunks[1]);

    // The month the write is anchored to, which is not always the budget view's selection.
    let month_label = app
        .budget_edit_month
        .map(|key| format!("{} {}", month_to_short_str(key.month), key.year))
        .unwrap_or_default();
    let selected_scope = app.budget_edit_scope();
    let scope_lines: Vec<Line> = scopes
        .iter()
        .map(|scope| {
            let text = match scope {
                BudgetEditScope::FromThisMonth => format!("From {} on", month_label),
                BudgetEditScope::ThisMonthOnly => format!("{} only", month_label),
                BudgetEditScope::ReplaceAllMonths => "Replace all months".to_string(),
                BudgetEditScope::RemoveChange => format!("Remove {} change", month_label),
            };
            let chosen = *scope == selected_scope;
            Line::from(vec![
                Span::styled(
                    if chosen { " (o) " } else { " ( ) " },
                    Style::default().fg(PANEL_CHROME_COLOR),
                ),
                Span::styled(
                    text,
                    if chosen {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(scope_lines), chunks[2]);

    let cursor_byte_idx = app.budget_edit_cursor.min(app.budget_edit_input.len());
    let visual_cursor = app.budget_edit_input[..cursor_byte_idx].chars().count() as u16;
    f.set_cursor_position(Position::new(
        chunks[1].x + visual_cursor + 1,
        chunks[1].y + 1,
    ));
}

pub fn render_budget_view(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(11), Constraint::Min(10)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[0]);

    let current_year = app.selected_budget_year();
    let selected_month = app.selected_budget_month;
    let is_filtered = app.filtered_indices.len() != app.transactions.len();

    let year_label = current_year.map_or_else(|| "N/A".to_string(), |year| year.to_string());
    let month_label = selected_month
        .map(month_to_short_str)
        .unwrap_or("No Data")
        .to_string();
    let month_color = selected_month.map(month_to_color).unwrap_or(Color::White);
    let year_progress = format!(
        "({}/{})",
        app.budget_year_index + 1,
        app.budget_years.len().max(1)
    );

    let actual_expense = match (current_year, selected_month) {
        (Some(year), Some(month)) => app.budget_month_expense(year, month),
        _ => Decimal::ZERO,
    };
    let monthly_budget = match (current_year, selected_month) {
        (Some(year), Some(month)) => app
            .budget_schedule
            .monthly_budget(BudgetMonth::new(year, month)),
        _ => None,
    };
    let remaining_budget = monthly_budget.map(|target| target - actual_expense);
    let allocated_budget = match (current_year, selected_month) {
        (Some(year), Some(month)) => app.total_allocated_budget(year, month),
        _ => Decimal::ZERO,
    };
    let unallocated_budget = monthly_budget.map(|target| target - allocated_budget);
    // Overspending outranks over-allocating, so the worse news is the one on show.
    let budget_status = match (monthly_budget, remaining_budget, unallocated_budget) {
        (None, _, _) => ("No Budget Set", Color::LightYellow),
        (Some(_), Some(left), _) if left < Decimal::ZERO => ("Over Budget", Color::LightRed),
        (Some(_), _, Some(spare)) if spare < Decimal::ZERO => ("Over Allocated", WARNING_COLOR),
        _ => ("On Track", Color::LightGreen),
    };
    let usage_value = match monthly_budget {
        Some(target) => usage_percent(actual_expense, target),
        None => "N/A".to_string(),
    };
    let mut allocated_spans = vec![
        Span::styled("Total:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format_amount(&allocated_budget),
            Style::default().fg(Color::LightBlue),
        ),
    ];
    if let Some(target) = monthly_budget {
        allocated_spans.push(Span::raw(" "));
        allocated_spans.push(Span::styled(
            format!("({})", usage_percent(allocated_budget, target)),
            Style::default().fg(usage_color(allocated_budget, target)),
        ));
    }
    let allocated_line = Line::from(allocated_spans);

    let allocation_divider = Line::from(Span::styled(
        format!(
            "{:─<width$}",
            "─ Category Budgets ",
            width = top_chunks[0].width as usize
        ),
        Style::default().fg(Color::DarkGray),
    ));

    let unallocated_line = Line::from(vec![
        Span::styled("Spare:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            match unallocated_budget {
                Some(value) => format_signed_amount(&value),
                None => "N/A".to_string(),
            },
            match unallocated_budget {
                Some(value) if value < Decimal::ZERO => Style::default().fg(Color::LightRed),
                Some(_) => Style::default().fg(Color::LightGreen),
                None => Style::default().fg(Color::White),
            },
        ),
    ]);

    let status_lines = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                budget_status.0,
                Style::default()
                    .fg(budget_status.1)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Month:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                month_label.clone(),
                Style::default()
                    .fg(month_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Year: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                year_label.clone(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                year_progress.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Budget: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                monthly_budget
                    .map(|value| format_amount(&value))
                    .unwrap_or_else(|| "Not set".to_string()),
                Style::default().fg(Color::LightBlue),
            ),
        ]),
        Line::from(vec![
            Span::styled("Spent:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format_amount(&actual_expense),
                if monthly_budget.is_some() && remaining_budget.unwrap_or_default() < Decimal::ZERO
                {
                    Style::default().fg(Color::LightRed)
                } else {
                    Style::default().fg(Color::LightGreen)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Left:   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                match remaining_budget {
                    Some(value) => format_signed_amount(&value),
                    None => "N/A".to_string(),
                },
                match remaining_budget {
                    Some(value) if value < Decimal::ZERO => Style::default().fg(Color::LightRed),
                    Some(_) => Style::default().fg(Color::LightGreen),
                    None => Style::default().fg(Color::White),
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Usage:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(usage_value, Style::default().fg(Color::LightYellow)),
        ]),
        allocation_divider,
        allocated_line,
        unallocated_line,
    ];

    let summary = Paragraph::new(status_lines)
        .block(budget_panel_block(
            title_with_month(
                "Budget Status - ",
                selected_month,
                &year_label,
                None,
                is_filtered,
            ),
            Borders::TOP,
        ))
        .wrap(Wrap { trim: true });
    f.render_widget(summary, top_chunks[0]);

    let mut bars: Vec<Bar> = Vec::new();
    let mut max_expense = monthly_budget
        .unwrap_or(Decimal::ZERO)
        .round()
        .to_u64()
        .unwrap_or(0);
    if let Some(year) = current_year {
        for month in 1..=12 {
            let expense = app.budget_month_expense(year, month);
            if expense.is_zero() && Some(month) != selected_month {
                continue;
            }
            let value = expense.round().to_u64().unwrap_or(0);
            max_expense = max_expense.max(value);
            let month_target = app
                .budget_schedule
                .monthly_budget(BudgetMonth::new(year, month));
            let style = if Some(month) == selected_month {
                let base = Style::default()
                    .fg(SELECTED_MONTH_BAR_COLOR)
                    .add_modifier(Modifier::BOLD);
                if let Some(target) = month_target {
                    if expense > target {
                        base.bg(Color::Rgb(45, 10, 10))
                    } else {
                        base
                    }
                } else {
                    base
                }
            } else if let Some(target) = month_target {
                if expense > target {
                    Style::default().fg(Color::LightRed)
                } else {
                    Style::default().fg(Color::LightGreen)
                }
            } else {
                Style::default().fg(Color::LightBlue)
            };

            bars.push(
                Bar::default()
                    .label(month_to_short_str(month))
                    .value(value)
                    .style(style),
            );
        }
    } else {
        bars.push(Bar::default().label("N/A").value(0));
    }

    let usable_width = top_chunks[1].width.saturating_sub(2);
    let width_per_bar_and_gap = (usable_width / (bars.len() as u16).max(1)).max(1);
    let bar_gap = if width_per_bar_and_gap > 1 { 1 } else { 0 };
    let bar_width = width_per_bar_and_gap.saturating_sub(bar_gap).max(1);
    let chart = BarChart::default()
        .block(budget_panel_block(
            title_with_month(
                "Monthly Spending - ",
                selected_month,
                &year_label,
                None,
                is_filtered,
            ),
            Borders::TOP | Borders::LEFT,
        ))
        .data(BarGroup::default().bars(&bars))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .group_gap(0)
        .label_style(Style::default().fg(Color::White))
        .max(max_expense.max(10));
    f.render_widget(chart, top_chunks[1]);

    let comparisons = app.current_budget_category_comparisons();
    let rows = if comparisons.is_empty() {
        vec![Row::new(vec![
            Cell::from(if selected_month.is_some() {
                "No budgeted categories"
            } else {
                "No month selected"
            }),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])]
    } else {
        comparisons.iter().map(comparison_row).collect()
    };

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(chunks[1]);

    let header = Row::new(vec![
        Cell::from("Category").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Subcategory").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from(Line::from("Budget").alignment(Alignment::Right))
            .style(Style::default().fg(Color::LightBlue).bold()),
        Cell::from(Line::from("Spent").alignment(Alignment::Right))
            .style(Style::default().fg(Color::LightRed).bold()),
        Cell::from(Line::from("Left").alignment(Alignment::Right))
            .style(Style::default().fg(Color::LightGreen).bold()),
        Cell::from(Line::from("Usage").alignment(Alignment::Right))
            .style(Style::default().fg(Color::LightYellow).bold()),
    ])
    .style(Style::default().bg(Color::DarkGray));

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(24),
            Constraint::Percentage(24),
            Constraint::Percentage(13),
            Constraint::Percentage(13),
            Constraint::Percentage(13),
            Constraint::Percentage(13),
        ],
    )
    .header(header)
    .block(budget_panel_block(
        title_with_month(
            "Budgeted Categories - ",
            selected_month,
            &year_label,
            Some(" (rows)"),
            is_filtered,
        ),
        Borders::TOP,
    ))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(" > ");
    f.render_stateful_widget(table, bottom_chunks[0], &mut app.budget_table_state);

    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(bottom_chunks[1]);

    let selected_comparison = app.selected_budget_category_comparison();
    let monthly_pattern = match (current_year, selected_comparison.as_ref()) {
        (Some(year), Some(comparison)) => app.budget_category_monthly_expenses(year, comparison),
        _ => Vec::new(),
    };
    let average_expense = average_monthly_expense(&monthly_pattern);
    let detail_lines = if let Some(comparison) = &selected_comparison {
        let remaining = comparison.budget - comparison.actual_expense;
        let subcategory_label = if comparison.subcategory.is_empty() {
            "None".to_string()
        } else {
            comparison.subcategory.clone()
        };
        vec![
            Line::from(vec![
                Span::styled("Category: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    comparison.category.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Subcat:   ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(subcategory_label),
            ]),
            Line::from(vec![
                Span::styled("Budget:   ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format_amount(&comparison.budget),
                    Style::default().fg(Color::LightBlue),
                ),
            ]),
            Line::from(vec![
                Span::styled("Spent:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format_amount(&comparison.actual_expense),
                    if comparison.actual_expense > comparison.budget {
                        Style::default().fg(Color::LightRed)
                    } else {
                        Style::default().fg(Color::LightGreen)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Left:     ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format_signed_amount(&remaining),
                    if remaining < Decimal::ZERO {
                        Style::default().fg(Color::LightRed)
                    } else {
                        Style::default().fg(Color::LightGreen)
                    },
                ),
            ]),
            Line::from(vec![
                Span::styled("Usage:    ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    usage_percent(comparison.actual_expense, comparison.budget),
                    Style::default().fg(Color::LightYellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("Avg/mo:   ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format_amount(&average_expense),
                    Style::default().fg(Color::LightCyan),
                ),
            ]),
        ]
    } else {
        vec![
            Line::from("Select a budget row to inspect it."),
            Line::from(""),
            Line::from("Budget health, yearly trend,"),
            Line::from("and average context."),
        ]
    };

    let detail_title =
        compact_selected_budget_title(selected_comparison.as_ref(), detail_chunks[0].width);
    let details = Paragraph::new(detail_lines)
        .block(budget_panel_block(
            detail_title,
            Borders::TOP | Borders::LEFT,
        ))
        .wrap(Wrap { trim: true });
    f.render_widget(details, detail_chunks[0]);

    let detail_chart_title = compact_yearly_pattern_title(
        selected_comparison.as_ref(),
        selected_month,
        detail_chunks[1].width,
    );
    let mut selected_bars: Vec<Bar> = Vec::new();
    let mut selected_max = 10u64;
    if let Some(comparison) = selected_comparison.as_ref() {
        for (month, expense) in &monthly_pattern {
            let value = expense.round().to_u64().unwrap_or(0);
            selected_max = selected_max.max(value);
            let month_budget = current_year.and_then(|year| {
                app.budget_schedule
                    .category_budget(comparison.id, BudgetMonth::new(year, *month))
            });
            if let Some(budget) = month_budget {
                selected_max = selected_max.max(budget.round().to_u64().unwrap_or(0));
            }
            let style = if Some(*month) == selected_month {
                Style::default()
                    .fg(SELECTED_MONTH_BAR_COLOR)
                    .add_modifier(Modifier::BOLD)
            } else if month_budget.is_some_and(|budget| *expense > budget) {
                Style::default().fg(Color::LightRed)
            } else if expense.is_zero() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::LightGreen)
            };
            selected_bars.push(
                Bar::default()
                    .label(month_to_short_str(*month))
                    .value(value)
                    .style(style),
            );
        }
        selected_max = selected_max.max(comparison.budget.round().to_u64().unwrap_or(0));
        selected_max = selected_max.max(average_expense.round().to_u64().unwrap_or(0));
    } else {
        selected_bars.push(Bar::default().label("N/A").value(0));
    }
    let usable_width = detail_chunks[1].width.saturating_sub(2);
    let width_per_bar_and_gap = (usable_width / (selected_bars.len() as u16).max(1)).max(1);
    let bar_gap = if width_per_bar_and_gap > 1 { 1 } else { 0 };
    let bar_width = width_per_bar_and_gap.saturating_sub(bar_gap).max(1);
    let detail_block = budget_panel_block(detail_chart_title, Borders::TOP | Borders::LEFT);
    let bars_area = detail_block.inner(detail_chunks[1]);
    let chart_max = selected_max.max(10);
    let detail_chart = BarChart::default()
        .block(detail_block)
        .data(BarGroup::default().bars(&selected_bars))
        .bar_width(bar_width)
        .bar_gap(bar_gap)
        .group_gap(0)
        .label_style(Style::default().fg(Color::White))
        .max(chart_max);
    f.render_widget(detail_chart, detail_chunks[1]);

    if let (Some(comparison), Some(year)) = (selected_comparison.as_ref(), current_year) {
        let budgets: Vec<Option<Decimal>> = monthly_pattern
            .iter()
            .map(|(month, _)| {
                app.budget_schedule
                    .category_budget(comparison.id, BudgetMonth::new(year, *month))
            })
            .collect();
        render_budget_guide(f, bars_area, &budgets, chart_max, bar_width, bar_gap);
    }
}

/// Bars fill in eighths of a cell, so the guide sits where a bar of that amount would stop.
fn render_budget_guide(
    f: &mut Frame,
    area: Rect,
    budgets: &[Option<Decimal>],
    max: u64,
    bar_width: u16,
    bar_gap: u16,
) {
    // BarChart gives the bottom row to labels.
    let rows = area.height.saturating_sub(1);
    let step = bar_width + bar_gap;
    if rows == 0 || area.width == 0 || max == 0 || step == 0 {
        return;
    }

    let drawn = budgets.len().min(((area.width + bar_gap) / step) as usize);
    let buf = f.buffer_mut();
    for (index, budget) in budgets.iter().take(drawn).enumerate() {
        let Some(budget) = budget else { continue };
        let ticks = budget
            .round()
            .to_u64()
            .unwrap_or(0)
            .saturating_mul(u64::from(rows))
            .saturating_mul(8)
            / max;
        let cell = (ticks / 8) as u16;
        if cell >= rows {
            continue;
        }

        let x = area.left() + index as u16 * step;
        // The gap is spanned too, so a steady budget reads as one continuous line.
        let span = if index + 1 == drawn { bar_width } else { step };
        let y = area.top() + rows - 1 - cell;
        for offset in 0..span.min(area.right().saturating_sub(x)) {
            buf[(x + offset, y)]
                .set_symbol(BUDGET_GUIDE_SYMBOL)
                .set_style(
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD),
                );
        }
    }
}
