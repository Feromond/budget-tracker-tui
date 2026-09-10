use crate::app::state::App;
use crate::ui::helpers::{centered_rect, clamp_list_scroll};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_fuzzy_search(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(60, 60, area);

    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(0),    // List
        ])
        .split(popup_area);

    // Search Bar
    let search_block = Block::default()
        .title("Search Category (Type to filter)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let search_text = Paragraph::new(app.search_query.as_str())
        .block(search_block)
        .style(Style::default().fg(Color::White));

    f.render_widget(search_text, chunks[0]);

    // List
    let items: Vec<ListItem> = app
        .current_selection_list
        .iter()
        .map(|i| ListItem::new(i.as_str()).style(Style::default().fg(Color::White)))
        .collect();

    let item_count = items.len();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Results (Up/Down/Enter)"),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    clamp_list_scroll(&mut app.selection_list_state, item_count, chunks[1]);
    f.render_stateful_widget(list, chunks[1], &mut app.selection_list_state);
}
