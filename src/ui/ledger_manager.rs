use crate::app::state::App;
use crate::ui::helpers::clamp_table_scroll;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_ledger_manager(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" Ledgers ({}) ", app.database_path.to_string_lossy());

    let header = Row::new(["", "Ledger"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

    let active_id = app.active_ledger_id;
    let rows = app.ledgers.iter().map(|ledger| {
        let is_active = ledger.id == active_id;
        let marker = if is_active { "●" } else { "" };
        let name_style = if is_active {
            Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Row::new(vec![
            Cell::from(marker).style(Style::default().fg(Color::LightGreen)),
            Cell::from(ledger.name.clone()).style(name_style),
        ])
    });

    let table = Table::new(rows, [Constraint::Length(3), Constraint::Min(10)])
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    clamp_table_scroll(&mut app.ledger_table_state, app.ledgers.len(), area);
    f.render_stateful_widget(table, area, &mut app.ledger_table_state);
}

pub fn render_ledger_editor(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = crate::ui::helpers::centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let title = if app.editing_ledger_id.is_some() {
        " Rename Ledger "
    } else if app.ledger_copy_source_id.is_some() {
        " Copy Ledger "
    } else {
        " Add Ledger "
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(popup_area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    f.render_widget(block, popup_area);

    let input = Paragraph::new(app.ledger_name_input.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Name")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(input, chunks[0]);

    let cursor_byte_idx = app.ledger_name_cursor.min(app.ledger_name_input.len());
    let visual_cursor = app.ledger_name_input[..cursor_byte_idx].chars().count() as u16;
    f.set_cursor_position(Position::new(
        chunks[0].x + visual_cursor + 1,
        chunks[0].y + 1,
    ));
}
