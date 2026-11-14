use crate::app::state::{App, AppMode};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render_io_prompt(f: &mut Frame, app: &App, area: Rect) {
    let is_import = app.mode == AppMode::ImportTransactions;
    let title = if is_import {
        " Import Transactions (CSV) "
    } else {
        " Export Transactions (CSV) "
    };
    let action_hint = if is_import {
        "[Enter] Import"
    } else {
        "[Enter] Export"
    };
    let label = if is_import {
        "CSV file to import:"
    } else {
        "Destination CSV path:"
    };

    let width = area.width.saturating_sub(8).clamp(20, 90);
    let height = 5u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let popup = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black))
        .title(title)
        .title_bottom(
            Line::from(format!(
                " {}  [Esc] Cancel  [Ctrl+U] Clear  [Ctrl+D] Default ",
                action_hint
            ))
            .centered(),
        );
    f.render_widget(block, popup);

    let label_area = Rect::new(popup.x + 2, popup.y + 1, popup.width.saturating_sub(4), 1);
    f.render_widget(
        Paragraph::new(Span::styled(label, Style::default().fg(Color::Gray))),
        label_area,
    );
    let browse = crate::ui::helpers::key_chip("Ctrl+O", "Browse files", true);
    if label_area.width as usize >= label.len() + browse.width() + 2 {
        f.render_widget(Paragraph::new(browse).right_aligned(), label_area);
    }

    let input_area = Rect::new(popup.x + 2, popup.y + 2, popup.width.saturating_sub(4), 1);
    let input_width = input_area.width.max(1);
    let cursor = crate::ui::helpers::cursor_column(&app.io_path_input, app.io_path_cursor);
    let scroll_x = crate::ui::helpers::scroll_to_show(cursor, input_width);
    f.render_widget(
        Paragraph::new(app.io_path_input.as_str()).scroll((0, scroll_x)),
        input_area,
    );

    if app.file_browser.is_none() {
        let visible_cursor = cursor.saturating_sub(scroll_x);
        f.set_cursor_position(Position::new(input_area.x + visible_cursor, input_area.y));
    }
}
