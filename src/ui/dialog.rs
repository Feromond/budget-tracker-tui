use crate::app::state::App;
use crate::ui::helpers::{centered_rect, clamp_list_scroll};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_confirmation_dialog(f: &mut Frame, message: &str, area: Rect) {
    let base = centered_rect(60, 20, area);
    let lines = wrapped_line_count(message, base.width.saturating_sub(2));
    let height = (lines + 2).max(base.height).min(area.height);
    let dialog_area = Rect {
        y: area.y + (area.height - height) / 2,
        height,
        ..base
    };

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

fn wrapped_line_count(text: &str, width: u16) -> u16 {
    let width = usize::from(width.max(1));
    let mut lines = 0u16;
    for paragraph in text.lines() {
        let mut used = 0;
        lines += 1;
        for word in paragraph.split_whitespace() {
            let len = word.chars().count();
            if used == 0 {
                used = len;
            } else if used + 1 + len <= width {
                used += 1 + len;
            } else {
                lines += 1;
                used = len;
            }
            while used > width {
                lines += 1;
                used -= width;
            }
        }
    }
    lines.max(1)
}

pub fn render_selection_popup(f: &mut Frame, app: &mut App, area: Rect) {
    use crate::app::fields::{AddEditField, SelectingField};
    let picking_account = matches!(
        app.selecting_field,
        Some(SelectingField::AddEdit(
            AddEditField::Account | AddEditField::ToAccount
        ))
    );
    let popup_title = match app.mode {
        _ if picking_account => "Select Account (Enter/Esc)",
        crate::app::state::AppMode::SelectingCategory => "Select Category (Enter/Esc)",
        crate::app::state::AppMode::SelectingAccountScope => "Show Account (Enter/Esc)",
        crate::app::state::AppMode::SelectingConversionAccount => {
            "Pick the Other Account (Enter/Esc)"
        }
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
