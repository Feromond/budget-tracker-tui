use crate::app::fields::{FieldKey, FieldKind};
use crate::app::state::App;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let input = Paragraph::new(app.simple_filter_content.as_str())
        .style(Style::default().fg(Color::LightYellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter (Description, ! to exclude)"),
        );
    f.render_widget(input, area);
    // Cursor setting is handled in the main `ui` function
}

pub fn render_advanced_filter_form(f: &mut Frame, app: &App, area: Rect) {
    let focused_field = app.advanced_filter_fields.focused();
    let widgets: Vec<_> = app
        .advanced_filter_fields
        .iter()
        .map(|(field, text)| {
            let excluded = app.is_advanced_field_excluded(field);
            let exclude_style = Style::default().fg(Color::LightRed).bold();
            let mut label = vec![Span::raw(field.label())];
            if excluded {
                label.push(Span::styled(" EXCLUDING", exclude_style));
            }
            if !field.hint().is_empty() {
                label.push(Span::raw(format!(" {}", field.hint())));
            }
            let content = if field.kind() == FieldKind::Toggle {
                Span::styled(format!(" < {} > ", text), Style::default().bold())
            } else {
                Span::raw(text.as_str())
            };
            Paragraph::new(content)
                .style(Style::default().fg(if excluded {
                    Color::LightRed
                } else {
                    Color::White
                }))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(Line::from(label))
                        .border_style(if field == focused_field {
                            Style::default().fg(Color::Yellow)
                        } else if excluded {
                            Style::default().fg(Color::LightRed)
                        } else {
                            Style::default()
                        }),
                )
        })
        .collect();
    let margin = 1;
    let fh = 3;
    let total = widgets.len();
    let avail = area.height.saturating_sub(margin * 2);
    let maxv = ((avail / fh) as usize).max(1).min(total);
    let offset = focused_field
        .index()
        .saturating_sub(maxv - 1)
        .min(total - maxv);
    let mut cons = vec![Constraint::Length(fh); maxv];
    cons.push(Constraint::Min(0));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints(cons)
        .split(area);
    for (idx, w) in widgets.iter().enumerate().skip(offset).take(maxv) {
        f.render_widget(w.clone(), chunks[idx - offset]);
    }
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Advanced Filters"),
        area,
    );
    if focused_field.kind().is_editable() {
        let field_idx = focused_field.index();
        let text = &app.advanced_filter_fields[focused_field];
        let cursor_pos = app.advanced_filter_cursor.min(text.len());
        // Calculate visual cursor position (accounting for potential multi-byte chars)
        let visual_cursor = text[..cursor_pos].chars().count() as u16;

        if field_idx >= offset && field_idx < offset + maxv {
            let vis = field_idx - offset;
            let ch = chunks[vis];
            f.set_cursor_position(Position::new(ch.x + visual_cursor + 1, ch.y + 1));
        }
    }
}
