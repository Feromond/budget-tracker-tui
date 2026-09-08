use crate::app::fields::CategoryEditField;
use crate::app::state::App;
use crate::model::{CategorySortColumn, SortOrder};
use crate::ui::form::render_field_form;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_category_catalog(f: &mut Frame, app: &mut App, area: Rect) {
    let title = if app.is_category_filter_active() {
        format!(
            " Category Catalog ({}/{} match \"{}\") ",
            app.filtered_category_indices.len(),
            app.category_records.len(),
            app.category_filter_query
        )
    } else {
        format!(
            " Category Catalog ({}) ",
            app.database_path.to_string_lossy()
        )
    };

    if app.category_records.is_empty() {
        let empty = Paragraph::new("No categories found. Press 'a' to add one.")
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    if app.filtered_category_indices.is_empty() {
        let empty = Paragraph::new("No categories match the filter. Press Esc to clear it.")
            .block(Block::default().title(title).borders(Borders::ALL))
            .alignment(Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    let sort_columns = [
        CategorySortColumn::Type,
        CategorySortColumn::Category,
        CategorySortColumn::Subcategory,
        CategorySortColumn::Tag,
        CategorySortColumn::TargetBudget,
    ];
    let header_style = if app.is_category_filter_active() {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Cyan).bold()
    };
    let header_cells = ["Type", "Category", "Subcategory", "Tag", "Budget"]
        .iter()
        .zip(sort_columns.iter())
        .map(|(title, column)| {
            let symbol = if app.category_sort_by == *column {
                match app.category_sort_order {
                    SortOrder::Ascending => " ▲",
                    SortOrder::Descending => " ▼",
                }
            } else {
                ""
            };
            let content = format!("{}{}", title, symbol);
            if *column == CategorySortColumn::TargetBudget {
                Cell::from(Line::from(content).alignment(Alignment::Right)).style(header_style)
            } else {
                Cell::from(content).style(header_style)
            }
        });
    let header = Row::new(header_cells).height(1);

    let budget_month = Some(crate::app::state::App::current_budget_key());
    let records = &app.category_records;
    let rows = app
        .filtered_category_indices
        .iter()
        .filter_map(|&index| records.get(index))
        .map(|record| {
            let budget = budget_month
                .and_then(|month| app.budget_schedule.category_budget(record.id, month))
                .map(|value| {
                    Cell::from(Line::from(format!("{value:.2}")).alignment(Alignment::Right)).style(
                        Style::default()
                            .fg(Color::LightCyan)
                            .bg(Color::Rgb(20, 20, 20))
                            .add_modifier(Modifier::BOLD),
                    )
                });

            Row::new(vec![
                Cell::from(record.transaction_type.to_string()),
                Cell::from(record.category.clone()),
                Cell::from(if record.subcategory.is_empty() {
                    "(None)".to_string()
                } else {
                    record.subcategory.clone()
                }),
                Cell::from(record.tag.clone().unwrap_or_default()),
                budget.unwrap_or_else(|| Cell::from(Line::from("").alignment(Alignment::Right))),
            ])
        });

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(30),
            Constraint::Percentage(26),
            Constraint::Percentage(18),
            Constraint::Percentage(16),
        ],
    )
    .header(header)
    .block(Block::default().title(title).borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.category_table_state);
}

pub fn render_category_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let input = Paragraph::new(app.category_filter_query.as_str())
        .style(Style::default().fg(Color::LightYellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter Categories (type, category, subcategory, tag)"),
        );
    f.render_widget(input, area);
    // Cursor setting is handled in the main `ui` function
}

pub fn render_category_editor(f: &mut Frame, app: &App, area: Rect) {
    let income_category =
        app.category_edit_fields[CategoryEditField::TransactionType].eq_ignore_ascii_case("income");
    let title = if app.editing_category_id.is_some() {
        "Edit Category"
    } else {
        "Add Category"
    };

    render_field_form(
        f,
        &app.category_edit_fields,
        app.category_edit_cursor,
        area,
        title,
        Some(" [Esc] Cancel, [Enter] Toggle/Save "),
        // Budgets are keyed on expense categories, so there is nothing to set here.
        if income_category {
            |field| (field == CategoryEditField::Budget).then_some("Expense categories only")
        } else {
            |_| None
        },
    );
}
