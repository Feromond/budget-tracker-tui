use crate::app::fields::{FieldKey, FieldKind, FieldSet};
use ratatui::prelude::*;
use ratatui::widgets::*;

/// A field that doesn't apply right now, dimmed in place of its value.
pub type Placeholder<K> = fn(K) -> Option<&'static str>;

pub fn render_field_form<K: FieldKey, const N: usize>(
    f: &mut Frame,
    fields: &FieldSet<K, N>,
    cursor: usize,
    area: Rect,
    title: &str,
    bottom_hint: Option<&str>,
    placeholder: Placeholder<K>,
) {
    let focused_field = fields.focused();
    let input_widgets: Vec<_> = fields
        .iter()
        .map(|(field, text)| {
            let box_title = format!("{} {}", field.label(), field.hint())
                .trim_end()
                .to_string();
            let content = match placeholder(field) {
                Some(hint) => Span::styled(hint, Style::default().fg(Color::DarkGray)),
                None if field.kind() == FieldKind::Toggle => Span::styled(
                    format!(" < {} > ", text),
                    Style::default().fg(Color::White).bold(),
                ),
                None => Span::raw(text.as_str()),
            };

            Paragraph::new(content)
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(box_title)
                        .border_style(if field == focused_field {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                )
        })
        .collect();

    let margin = 1;
    let field_height = 3;
    let total_fields = input_widgets.len();
    let available_height = area.height.saturating_sub(margin * 2);
    let max_visible_fields = ((available_height / field_height) as usize)
        .max(1)
        .min(total_fields);
    let scroll_offset = focused_field
        .index()
        .saturating_sub(max_visible_fields - 1)
        .min(total_fields - max_visible_fields);

    let mut constraints = Vec::with_capacity(max_visible_fields + 1);
    for _ in 0..max_visible_fields {
        constraints.push(Constraint::Length(field_height));
    }
    constraints.push(Constraint::Min(0));

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints(constraints)
        .split(area);

    for (index, widget) in input_widgets
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_visible_fields)
    {
        f.render_widget(widget.clone(), form_chunks[index - scroll_offset]);
    }

    let mut form_block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL);
    if let Some(hint) = bottom_hint {
        form_block = form_block.title_bottom(hint.to_string());
    }
    f.render_widget(form_block, area);

    if focused_field.kind().is_editable() && placeholder(focused_field).is_none() {
        let field_index = focused_field.index();
        let text = &fields[focused_field];
        let cursor_byte_idx = cursor.min(text.len());
        let visual_cursor = text[..cursor_byte_idx].chars().count() as u16;

        if field_index >= scroll_offset
            && field_index < scroll_offset + max_visible_fields
            && let Some(chunk) = form_chunks.get(field_index - scroll_offset)
        {
            f.set_cursor_position(Position::new(chunk.x + visual_cursor + 1, chunk.y + 1));
        }
    }
}
