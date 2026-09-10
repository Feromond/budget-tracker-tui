use crate::app::help::get_help_for_mode;
use crate::app::state::{App, AppMode};
use crate::ui::helpers::clamp_table_scroll;
use ratatui::{prelude::*, widgets::*};

pub fn render_keybindings_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(area, 60, 80);

    f.render_widget(Clear, popup_area);

    let mode = app.previous_mode.unwrap_or(AppMode::Normal);
    let bindings = get_help_for_mode(mode);

    let rows: Vec<Row> = bindings
        .iter()
        .map(|b| {
            let description_cell = if b.extended_description.is_some() {
                Line::from(vec![
                    Span::raw(b.description),
                    Span::styled(" [+]", Style::default().fg(Color::LightGreen)),
                ])
            } else {
                Line::from(b.description)
            };

            Row::new(vec![
                Cell::from(Span::styled(
                    b.key,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(description_cell),
                Cell::from(Span::styled(b.group, Style::default().fg(Color::DarkGray))),
            ])
        })
        .collect();

    let total_rows = bindings.len();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Key", "Action", "Group"])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .title(format!(
                " Keybindings for {:?} (Scroll: ↑/↓, Select: Enter) ",
                mode
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue)),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    clamp_table_scroll(&mut app.help_table_state, total_rows, popup_area);
    // Render the stateful widget
    f.render_stateful_widget(table, popup_area, &mut app.help_table_state);

    // Render scrollbar based on state
    let selected_index = app.help_table_state.selected().unwrap_or(0);
    let mut scrollbar_state = ScrollbarState::new(total_rows).position(selected_index);

    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));

    f.render_stateful_widget(
        scrollbar,
        popup_area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    // If in detail mode, render the detail popup on top
    if app.mode == AppMode::KeybindingDetail {
        // Ensure we get the same bindings list to index into
        if let Some(binding) = bindings.get(selected_index)
            && let Some(desc) = binding.extended_description
        {
            render_extended_help_popup(f, binding.key, desc, popup_area);
        }
    }
}

fn render_extended_help_popup(f: &mut Frame, title: &str, description: &str, parent_area: Rect) {
    let area = centered_rect(parent_area, 80, 40);
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Details: {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let paragraph = Paragraph::new(description)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
