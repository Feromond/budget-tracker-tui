use crate::app::state::App;
use crate::ui::helpers::{centered_rect, clamp_list_scroll};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_confirmation_dialog(f: &mut Frame, message: &str, area: Rect) {
    let dialog_area = centered_rect(60, 20, area);

    let dialog_block = Block::default()
        .title("Confirmation")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let dialog_text = Paragraph::new(message)
        .block(dialog_block)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, dialog_area); // Clear the area behind the dialog
    f.render_widget(dialog_text, dialog_area);
}

pub fn render_selection_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_title = match app.mode {
        crate::app::state::AppMode::SelectingCategory => "Select Category (Enter/Esc)",
        crate::app::state::AppMode::SelectingSubcategory => "Select Subcategory (Enter/Esc)",
        crate::app::state::AppMode::SelectingRecurrenceFrequency => "Select Frequency (Enter/Esc)",
        _ => "Select Option",
    };

    let items: Vec<ListItem> = app
        .current_selection_list
        .iter()
        .map(|i| ListItem::new(i.as_str()).style(Style::default().fg(Color::White)))
        .collect();

    let item_count = items.len();
    let list = List::new(items)
        .block(Block::default().title(popup_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

    let popup_area = centered_rect(60, 50, area);

    f.render_widget(Clear, popup_area);
    clamp_list_scroll(&mut app.selection_list_state, item_count, popup_area);
    f.render_stateful_widget(list, popup_area, &mut app.selection_list_state);
}
