use crate::app::settings_types::{SettingItem, SettingKey, SettingType, SettingsState};
use crate::app::state::App;
use crate::ui::helpers::{
    cursor_column, keep_end, keep_start, key_chip, scroll_to_show, tilde_path,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use std::ops::Range;
use std::path::Path;

const ACCENT: Color = Color::Yellow;
const HEADING: Color = Color::LightCyan;
const SECTION_COLORS: [Color; 4] = [
    Color::LightCyan,
    Color::LightMagenta,
    Color::LightBlue,
    Color::LightYellow,
];
const ON: Color = Color::LightGreen;
const MUTED: Color = Color::Gray;
const FAINT: Color = Color::DarkGray;
const ROW: Color = Color::DarkGray;
const PANE_MAX_WIDTH: u16 = 96;
const SIDEBAR_WIDTH: u16 = 22;
const SIDEBAR_MIN_WIDTH: u16 = 64;
const LABEL_WIDTH: usize = 28;

struct Section<'a> {
    title: &'a str,
    subtitle: &'a str,
    rows: Range<usize>,
    color: Color,
}

#[derive(Clone, Copy)]
struct RowLook {
    label_width: usize,
    color: Color,
}

fn sections(state: &SettingsState) -> Vec<Section<'_>> {
    state
        .sections()
        .into_iter()
        .enumerate()
        .map(|(index, (header, rows))| Section {
            title: &state.items[header].label,
            subtitle: &state.items[header].help,
            rows,
            color: SECTION_COLORS[index % SECTION_COLORS.len()],
        })
        .collect()
}

fn footer_hints() -> Line<'static> {
    let key = |color: Color| Style::default().fg(color).add_modifier(Modifier::BOLD);
    let label = Style::default().fg(Color::White);
    let gap = Span::styled("  ·  ", Style::default().fg(FAINT));
    Line::from(vec![
        Span::raw(" "),
        Span::styled("Enter", key(Color::LightGreen)),
        Span::styled(" Save", label),
        gap.clone(),
        Span::styled("Esc", key(Color::LightRed)),
        Span::styled(" Cancel", label),
        gap,
        Span::styled("Tab", key(Color::LightYellow)),
        Span::styled(" Sections", label),
        Span::raw(" "),
    ])
}

pub fn render_settings_form(f: &mut Frame, app: &App, area: Rect) {
    render_settings(
        f,
        &app.settings_state,
        app.file_browser.is_none(),
        app.hide_help_bar,
        area,
    );
}

fn render_settings(
    f: &mut Frame,
    state: &SettingsState,
    show_cursor: bool,
    show_footer: bool,
    area: Rect,
) {
    let sections = sections(state);
    if sections.is_empty() {
        return;
    }
    let selected = state.selected_index;
    let current = sections
        .iter()
        .position(|section| section.rows.contains(&selected))
        .unwrap_or(0);

    f.render_widget(Clear, area);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(
            " Settings ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title(
            Line::from(vec![
                Span::styled(" Budget Tracker ", Style::default().fg(HEADING)),
                Span::styled(
                    concat!("v", env!("CARGO_PKG_VERSION"), " "),
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ])
            .right_aligned(),
        );
    if show_footer {
        block = block.title_bottom(footer_hints().centered());
    }
    let inner = block.inner(area).inner(Margin::new(2, 1));
    f.render_widget(block, area);

    if inner.width >= SIDEBAR_MIN_WIDTH {
        let [sidebar, pane] =
            Layout::horizontal([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(20)])
                .spacing(3)
                .areas(inner);
        render_sidebar(f, &sections, current, sidebar);
        let pane = Rect {
            width: pane.width.min(PANE_MAX_WIDTH),
            ..pane
        };
        render_section(f, state, show_cursor, &sections[current], None, pane);
    } else {
        render_section(
            f,
            state,
            show_cursor,
            &sections[current],
            Some((current, sections.len())),
            inner,
        );
    }
}

fn render_sidebar(f: &mut Frame, sections: &[Section], current: usize, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(FAINT));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let width = inner.width.saturating_sub(1) as usize;
    let mut lines = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        let title = format!(
            " {:<width$}",
            section.title,
            width = width.saturating_sub(2)
        );
        lines.push(if index == current {
            Line::from(vec![
                Span::styled("▌", Style::default().fg(ACCENT).bg(ROW)),
                Span::styled(
                    title,
                    Style::default()
                        .fg(section.color)
                        .bg(ROW)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(vec![
                Span::raw(" "),
                Span::styled(title, Style::default().fg(section.color)),
            ])
        });
        lines.push(Line::default());
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_section(
    f: &mut Frame,
    state: &SettingsState,
    show_cursor: bool,
    section: &Section,
    position: Option<(usize, usize)>,
    area: Rect,
) {
    let items = &state.items;
    let selected = state.selected_index;
    let [head, body, about] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(4),
    ])
    .areas(area);

    let mut title = vec![Span::styled(
        section.title,
        Style::default()
            .fg(section.color)
            .add_modifier(Modifier::BOLD),
    )];
    if let Some((index, count)) = position {
        title.push(Span::styled(
            format!("  {} of {} · Tab for more", index + 1, count),
            Style::default().fg(FAINT),
        ));
    }
    f.render_widget(
        Paragraph::new(vec![
            Line::from(title),
            Line::styled(
                section.subtitle,
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            ),
        ]),
        head,
    );

    let count = section.rows.len();
    let step = if count * 2 <= body.height as usize + 1 {
        2
    } else {
        1
    };
    let visible = (body.height as usize).div_ceil(step).max(1);
    let focus = selected.saturating_sub(section.rows.start);
    let offset = (focus + 1)
        .saturating_sub(visible)
        .min(count.saturating_sub(visible));
    let look = RowLook {
        label_width: LABEL_WIDTH.min(body.width as usize / 2),
        color: section.color,
    };
    for (slot, index) in section.rows.clone().skip(offset).take(visible).enumerate() {
        let row = Rect::new(body.x, body.y + (slot * step) as u16, body.width, 1);
        render_row(
            f,
            state,
            show_cursor,
            &items[index],
            index == selected,
            look,
            row,
        );
    }
    let edge = body.right().saturating_sub(1);
    let marker = Style::default().fg(MUTED);
    if offset > 0 {
        let top = Rect::new(edge, body.y, 1, 1);
        f.render_widget(Paragraph::new("▲").style(marker), top);
    }
    if offset + visible < count {
        let bottom = Rect::new(edge, body.bottom().saturating_sub(1), 1, 1);
        f.render_widget(Paragraph::new("▼").style(marker), bottom);
    }

    let about_text = items
        .get(selected)
        .map(|item| {
            Line::from(vec![
                Span::styled(
                    format!("{}  ", item.label),
                    Style::default()
                        .fg(section.color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(item.help.as_str(), Style::default().fg(MUTED)),
            ])
        })
        .unwrap_or_default();
    f.render_widget(
        Paragraph::new(about_text).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(FAINT)),
        ),
        about,
    );
}

fn render_row(
    f: &mut Frame,
    state: &SettingsState,
    show_cursor: bool,
    item: &SettingItem,
    focused: bool,
    look: RowLook,
    area: Rect,
) {
    let label_width = look.label_width;
    let editable = matches!(
        item.setting_type,
        SettingType::Path | SettingType::Number | SettingType::Integer
    );
    let row_bg = if focused {
        Style::default().bg(ROW)
    } else {
        Style::default()
    };
    f.render_widget(Block::default().style(row_bg), area);

    let label_style = if focused {
        Style::default().fg(look.color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(look.color)
    };
    let marker = if focused { "▌" } else { " " };
    let mut spans = vec![
        Span::styled(marker, Style::default().fg(ACCENT)),
        Span::styled(
            format!(
                " {:<width$.fit$}",
                item.label,
                width = label_width.saturating_sub(1),
                fit = label_width.saturating_sub(2)
            ),
            label_style,
        ),
    ];

    let value_x = area.x + 1 + label_width as u16;
    let chip = match item.setting_type {
        SettingType::Action => Some(key_chip("Enter", "Open", focused)),
        _ if item.key == SettingKey::DatabasePath => Some(key_chip("Ctrl+O", "Browse", focused)),
        _ => None,
    }
    .filter(|chip| area.right() >= value_x + chip.width() as u16 + 14);
    let chip_width = chip.as_ref().map_or(0, |chip| chip.width() as u16 + 2);
    let value_width = area.right().saturating_sub(value_x + 2 + chip_width);
    let value_style = if focused {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(MUTED)
    };
    let prefix = if item.key == SettingKey::HourlyRate && (focused || !item.value.is_empty()) {
        "$"
    } else {
        ""
    };

    let mut field = None;
    match item.setting_type {
        SettingType::Toggle => {
            let arrow = if focused {
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FAINT)
            };
            if item.is_on() {
                spans.push(Span::styled("◀ ", arrow));
                spans.push(Span::styled(
                    "● On",
                    Style::default().fg(ON).add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled("  ○ Off ", Style::default().fg(MUTED)));
                spans.push(Span::styled("▶", arrow));
            }
        }
        SettingType::Action => {
            spans.push(Span::styled(
                keep_start(&item.value, value_width as usize),
                value_style,
            ));
        }
        _ if focused && editable => {
            spans.push(Span::styled(prefix, Style::default().fg(MUTED)));
            let x = value_x + prefix.len() as u16;
            field = Some(Rect::new(
                x,
                area.y,
                area.right().saturating_sub(x + 2 + chip_width),
                1,
            ));
        }
        _ if item.value.trim().is_empty() => {
            spans.push(Span::styled(
                "Not set",
                Style::default().fg(FAINT).add_modifier(Modifier::ITALIC),
            ));
        }
        SettingType::Path => {
            let path = tilde_path(Path::new(&item.value));
            spans.push(Span::styled(
                keep_end(&path, value_width as usize),
                value_style,
            ));
        }
        _ => {
            spans.push(Span::styled(prefix, value_style));
            spans.push(Span::styled(item.value.clone(), value_style));
        }
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);

    if let Some(field) = field {
        let cursor = cursor_column(&item.value, state.edit_cursor);
        let scroll = scroll_to_show(cursor, field.width);
        f.render_widget(
            Paragraph::new(item.value.as_str())
                .style(value_style.add_modifier(Modifier::UNDERLINED))
                .scroll((0, scroll)),
            field,
        );
        if show_cursor {
            f.set_cursor_position(Position::new(
                field.x + cursor.saturating_sub(scroll),
                area.y,
            ));
        }
    }

    if let Some(chip) = chip {
        let width = chip.width() as u16;
        let at = Rect::new(area.right().saturating_sub(width + 1), area.y, width, 1);
        f.render_widget(Paragraph::new(chip), at);
    } else if item.setting_type == SettingType::Action {
        let chevron = if focused {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FAINT)
        };
        let end = Rect::new(area.right().saturating_sub(2), area.y, 1, 1);
        f.render_widget(Paragraph::new("›").style(chevron), end);
    }
}
