use crate::app::state::App;
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

pub fn render_settings_form(f: &mut Frame, app: &App, area: Rect) {
    let items = &app.settings_state.items;
    if items.is_empty() {
        return;
    }

    let margin = 2;
    let field_height = 3;

    let total_fields = items.len();
    let available_height = area.height.saturating_sub(margin * 2);
    let max_visible_fields = ((available_height / field_height) as usize)
        .max(1)
        .min(total_fields);

    let current_idx = app.settings_state.selected_index;
    // Ensure scroll offset keeps selected item in view
    let scroll_offset = if total_fields <= max_visible_fields {
        0
    } else {
        // Simple scrolling logic: try to center or keep visible
        let mut offset = 0;

        // Attempt to keep current item somewhat centered if possible
        let center_pos = max_visible_fields / 2;
        if current_idx > center_pos {
            offset = current_idx - center_pos;
        }

        // Clamp to max possible offset so we don't scroll past the end
        let max_offset = total_fields.saturating_sub(max_visible_fields);
        offset.min(max_offset)
    };

    // Main Block
    f.render_widget(Clear, area);

    // Get help text for current item
    let help_text = if let Some(item) = items.get(current_idx) {
        if item.setting_type == crate::app::settings_types::SettingType::SectionHeader {
            String::new()
        } else {
            format!(" {} ", item.help)
        }
    } else {
        String::new()
    };

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .style(Style::default().bg(Color::Black))
        .title(" Settings - [Esc] Cancel, [Enter] Save/Action ")
        .title(
            Line::from(vec![
                Span::styled(" Budget Tracker ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    concat!("v", env!("CARGO_PKG_VERSION"), " "),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ])
            .right_aligned(),
        )
        .title_bottom(Line::from(help_text).centered());

    f.render_widget(popup_block, area);

    // Scrollbar
    if total_fields > max_visible_fields {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state =
            ScrollbarState::new(total_fields.saturating_sub(max_visible_fields))
                .position(scroll_offset);

        // The scrollbar area needs to overlap the right border of the popup block
        let scrollbar_area = area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    let inner_area = Rect::new(
        area.x + margin,
        area.y + margin,
        area.width.saturating_sub(margin * 2),
        area.height.saturating_sub(margin * 2),
    );

    let mut constraints = Vec::with_capacity(max_visible_fields);
    for _ in 0..max_visible_fields {
        constraints.push(Constraint::Length(field_height));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    let input_width = inner_area.width.saturating_sub(2).max(10);

    for (i, chunk) in chunks.iter().enumerate() {
        let item_idx = scroll_offset + i;
        if item_idx >= items.len() {
            break;
        }

        let item = &items[item_idx];
        let is_focused = item_idx == current_idx;

        if item.setting_type == crate::app::settings_types::SettingType::SectionHeader {
            // Render Header
            let header = Paragraph::new(Line::from(vec![Span::styled(
                &item.label,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .padding(Padding::new(0, 0, 1, 0)),
            ); // Center vertically roughly

            f.render_widget(header, *chunk);
        } else {
            // Render Input Field
            let is_action = item.setting_type == crate::app::settings_types::SettingType::Action;
            let border_style = if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            let title = format!(" {} ", item.label);
            let display_value = if is_action {
                format!("  [ {} ]  ", item.value)
            } else {
                item.value.clone()
            };

            // Horizontal scrolling for long values
            let mut scroll_x = 0;
            if is_focused {
                let cursor = app.settings_state.edit_cursor as u16;
                if cursor >= input_width {
                    scroll_x = cursor - input_width + 1;
                }
            }

            let p = if is_action {
                Paragraph::new(Line::from(vec![Span::styled(
                    display_value,
                    if is_focused {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    },
                )]))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(border_style),
                )
            } else {
                Paragraph::new(display_value).scroll((0, scroll_x)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(border_style),
                )
            };

            f.render_widget(p, *chunk);

            // Cursor
            if is_focused
                && item.setting_type != crate::app::settings_types::SettingType::Toggle
                && item.setting_type != crate::app::settings_types::SettingType::Action
            {
                let cursor = app.settings_state.edit_cursor as u16;
                let visible_cursor = cursor.saturating_sub(scroll_x);
                f.set_cursor_position(Position::new(chunk.x + visible_cursor + 1, chunk.y + 1));
            }
        }
    }
}
