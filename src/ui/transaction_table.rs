use crate::app::state::App;
use crate::model::{DATE_FORMAT, SortColumn, SortOrder, TransactionType};
use crate::ui::helpers::{clamp_table_scroll, format_amount, format_hours};
use ratatui::prelude::*;
use ratatui::widgets::*;
pub fn render_transaction_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_titles = [
        "Date",
        "Description",
        "Category",
        "Subcategory",
        "Type",
        if app.show_hours && app.hourly_rate.is_some() {
            "Hours"
        } else {
            "Amount"
        },
    ];
    let sort_columns = [
        SortColumn::Date,
        SortColumn::Description,
        SortColumn::Category,
        SortColumn::Subcategory,
        SortColumn::Type,
        SortColumn::Amount,
    ];

    let is_filtered = app.filtered_indices.len() != app.transactions.len();
    let header_cells = header_titles
        .iter()
        .zip(sort_columns.iter())
        .map(|(title, column)| {
            let style = if is_filtered {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default().fg(Color::Cyan).bold()
            };
            let symbol = if app.sort_by == *column {
                match app.sort_order {
                    SortOrder::Ascending => " ▲",
                    SortOrder::Descending => " ▼",
                }
            } else {
                ""
            };
            let content = format!("{}{}", title, symbol);
            if *column == SortColumn::Amount {
                Cell::from(Line::from(content).alignment(Alignment::Center)).style(style)
            } else {
                Cell::from(content).style(style)
            }
        });

    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1)
        .bottom_margin(1);

    // Forecast occurrences are dimmed so they read as projections, not settled transactions.
    let today = chrono::Local::now().date_naive();
    let rows = app.filtered_indices.iter().map(|&original_index| {
        if original_index >= app.transactions.len() {
            return Row::new(vec![Cell::from("Error: Invalid Index").fg(Color::Red)])
                .height(1)
                .bottom_margin(0);
        }
        let tx = &app.transactions[original_index];
        let amount_style = match tx.transaction_type {
            TransactionType::Income => Style::default().fg(Color::LightGreen),
            TransactionType::Expense => Style::default().fg(Color::LightRed),
        };

        // Add visual indicators for recurring transactions
        let description_text = if tx.is_recurring {
            if tx.is_generated_from_recurring {
                format!("⟲ {}", tx.description) // Generated from recurring
            } else {
                format!("⟲* {}", tx.description) // Original recurring transaction
            }
        } else {
            tx.description.clone()
        };

        let amount_cell_text = if app.show_hours {
            format_hours(&tx.amount, app.hourly_rate)
        } else {
            format_amount(&tx.amount)
        };

        let cells = vec![
            Cell::from(tx.date.format(DATE_FORMAT).to_string()),
            Cell::from(description_text),
            Cell::from(tx.category.as_str()),
            Cell::from(tx.subcategory.as_str()),
            Cell::from(format!("{:?}", tx.transaction_type)),
            Cell::from(Line::from(amount_cell_text).alignment(Alignment::Right))
                .style(amount_style),
        ];
        let row = Row::new(cells).height(1).bottom_margin(0);
        if tx.is_generated_from_recurring && tx.date > today {
            row.style(Style::default().add_modifier(Modifier::DIM))
        } else {
            row
        }
    });

    let is_filtered = app.filtered_indices.len() != app.transactions.len();
    let table_title = {
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
            "Transactions",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(": {}", app.active_ledger_name()),
            Style::default().fg(Color::LightGreen),
        ));
        if app.recurring_forecast_months > 0 {
            spans.push(Span::styled(
                format!(" (Forecast +{}mo)", app.recurring_forecast_months),
                Style::default().fg(Color::Cyan),
            ));
        }
        Line::from(spans)
    };
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(18),
            Constraint::Percentage(18),
            Constraint::Percentage(7),
            Constraint::Percentage(12),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(table_title))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(" > ");

    clamp_table_scroll(&mut app.table_state, app.filtered_indices.len(), area);
    f.render_stateful_widget(table, area, &mut app.table_state);
}
