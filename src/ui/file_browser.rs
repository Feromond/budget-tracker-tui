use crate::app::file_browser::{BrowserEntry, EntryKind, FileBrowser, Pane};
use crate::ui::helpers::{
    clamp_list_scroll, clamp_table_scroll, format_size, home_relative, keep_end, tilde_path,
};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, HighlightSpacing, List, ListItem, Padding, Paragraph,
    Row, Table,
};
use std::path::{Component, Path};

const ACCENT: Color = Color::Yellow;
const FOLDER: Color = Color::LightBlue;
const ACTION: Color = Color::LightGreen;
const HEADING: Color = Color::LightCyan;
const MUTED: Color = Color::Gray;
const FAINT: Color = Color::DarkGray;
const MAX_WIDTH: u16 = 116;
const MAX_HEIGHT: u16 = 32;
const PLACES_WIDTH: u16 = 26;
const SIDEBAR_MIN_WIDTH: u16 = 72;
const DETAILS_MIN_WIDTH: u16 = 60;

pub fn render_file_browser(f: &mut Frame, browser: &mut FileBrowser, show_hints: bool, area: Rect) {
    let width = area.width.saturating_sub(4).min(MAX_WIDTH);
    let height = area.height.saturating_sub(2).min(MAX_HEIGHT);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    f.render_widget(Clear, popup);

    let mut outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(Color::Black))
        .title(Span::styled(
            browser.target.title(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
    if show_hints {
        let hints = crate::ui::help::file_browser_hints(popup.width.saturating_sub(4) as usize);
        outer = outer.title_bottom(Line::from(padded(hints)).centered());
    }
    let inner = outer.inner(popup).inner(Margin::new(1, 0));
    f.render_widget(outer, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);

    render_location(f, browser, rows[0]);

    let body = rows[2];
    if body.width >= SIDEBAR_MIN_WIDTH {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(PLACES_WIDTH), Constraint::Min(20)])
            .spacing(1)
            .split(body);
        render_places(f, browser, panes[0]);
        render_files(f, browser, panes[1]);
    } else if browser.focus == Pane::Places {
        render_places(f, browser, body);
    } else {
        render_files(f, browser, body);
    }

    render_footer(f, browser, rows[3]);
}

// Set text colours explicitly so hints don't inherit the border colour.
fn padded(spans: Vec<Span<'static>>) -> Vec<Span<'static>> {
    let mut padded = vec![Span::raw(" ")];
    padded.extend(spans.into_iter().map(|span| {
        if span.content == " | " {
            span.style(Style::default().fg(FAINT))
        } else if span.style.fg.is_none() {
            span.style(Style::default().fg(Color::White))
        } else {
            span
        }
    }));
    padded.push(Span::raw(" "));
    padded
}

fn selected_row() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

fn pane_block(title: &str, focused: bool) -> Block<'static> {
    let (border, title_style) = if focused {
        (
            Style::default().fg(ACCENT),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )
    } else {
        (Style::default().fg(FAINT), Style::default().fg(MUTED))
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border)
        .padding(Padding::horizontal(1))
        .title(Span::styled(format!(" {title} "), title_style))
}

fn badge(text: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {text} "),
        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

fn render_location(f: &mut Frame, browser: &FileBrowser, area: Rect) {
    let mut badges = vec![badge(&browser.target.extension_list(), Color::Cyan)];
    if browser.show_hidden {
        badges.push(Span::raw(" "));
        badges.push(badge("hidden shown", Color::Magenta));
    }
    let badge_width: u16 = badges.iter().map(|span| span.width() as u16).sum();
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(badge_width)])
        .spacing(1)
        .split(area);

    let segments = path_segments(&browser.dir);
    let separator = || Span::styled(" › ", Style::default().fg(FAINT));
    let width_of = |segments: &[String]| {
        segments.iter().map(|s| s.chars().count()).sum::<usize>()
            + segments.len().saturating_sub(1) * 3
    };
    let mut start = 0;
    while start + 1 < segments.len() && width_of(&segments[start..]) + 4 > split[0].width as usize {
        start += 1;
    }

    let mut spans = Vec::new();
    if start > 0 {
        spans.push(Span::styled("…", Style::default().fg(FAINT)));
        spans.push(separator());
    }
    let last = segments.len().saturating_sub(1);
    for (index, segment) in segments.iter().enumerate().skip(start) {
        if index > start {
            spans.push(separator());
        }
        let style = if index == last {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FOLDER)
        };
        spans.push(Span::styled(segment.clone(), style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), split[0]);
    f.render_widget(
        Paragraph::new(Line::from(badges)).alignment(Alignment::Right),
        split[1],
    );
}

fn path_segments(dir: &Path) -> Vec<String> {
    let (mut segments, rest) = match home_relative(dir) {
        Some(rest) => (vec!["~".to_string()], rest),
        None => (Vec::new(), dir.to_path_buf()),
    };
    for component in rest.components() {
        match component {
            Component::Prefix(prefix) => {
                segments.push(prefix.as_os_str().to_string_lossy().to_string())
            }
            Component::RootDir => match segments.last_mut() {
                Some(drive) => drive.push('\\'),
                None => segments.push("/".to_string()),
            },
            Component::Normal(name) => segments.push(name.to_string_lossy().to_string()),
            Component::CurDir | Component::ParentDir => {}
        }
    }
    segments
}

fn render_places(f: &mut Frame, browser: &mut FileBrowser, area: Rect) {
    let focused = browser.focus == Pane::Places;
    let here = browser.current_place();
    let items: Vec<ListItem> = browser
        .places
        .iter()
        .enumerate()
        .map(|(index, place)| {
            if place.path.is_none() {
                return ListItem::new(Span::styled(
                    place.label.to_uppercase(),
                    Style::default().fg(HEADING).add_modifier(Modifier::BOLD),
                ));
            }
            if Some(index) == here {
                ListItem::new(Line::from(vec![
                    Span::styled("● ", Style::default().fg(ACTION)),
                    Span::styled(
                        place.label.clone(),
                        Style::default().fg(ACTION).add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(place.label.clone(), Style::default().fg(Color::White)),
                ]))
            }
        })
        .collect();

    let block = pane_block("Places", focused);
    let inner = block.inner(area);
    let list = List::new(items)
        .block(block)
        .highlight_style(selected_row())
        .highlight_symbol(Span::styled("› ", Style::default().fg(ACCENT)))
        .highlight_spacing(HighlightSpacing::Always);
    let count = browser.places.len();
    clamp_list_scroll(&mut browser.places_state, count, inner);
    if focused {
        f.render_stateful_widget(list, area, &mut browser.places_state);
    } else {
        // Keep the scroll position without showing a selection.
        let mut unselected = browser.places_state;
        unselected.select(None);
        f.render_stateful_widget(list, area, &mut unselected);
    }
}

fn render_files(f: &mut Frame, browser: &mut FileBrowser, area: Rect) {
    let focused = browser.focus == Pane::Files;
    let block = pane_block("Files", focused);
    let inner = block.inner(area);

    let rows: Vec<Row> = browser
        .entries
        .iter()
        .map(|entry| file_row(browser, entry))
        .collect();
    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from(Line::from("Size").alignment(Alignment::Right)),
        Cell::from(Line::from("Modified").alignment(Alignment::Right)),
    ])
    .style(Style::default().fg(HEADING).add_modifier(Modifier::BOLD));
    let widths = if inner.width >= DETAILS_MIN_WIDTH {
        vec![
            Constraint::Min(12),
            Constraint::Length(9),
            Constraint::Length(10),
        ]
    } else {
        vec![
            Constraint::Min(12),
            Constraint::Length(0),
            Constraint::Length(0),
        ]
    };

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(2)
        .block(block)
        .highlight_symbol(Span::styled("› ", Style::default().fg(ACCENT)))
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(if focused {
            selected_row()
        } else {
            Style::default().add_modifier(Modifier::UNDERLINED)
        });
    let count = browser.entries.len();
    clamp_table_scroll(&mut browser.table_state, count, inner);
    f.render_stateful_widget(table, area, &mut browser.table_state);

    let has_content = browser
        .entries
        .iter()
        .any(|entry| matches!(entry.kind, EntryKind::Folder | EntryKind::File));
    let below_rows = count as u16 + 2;
    if !has_content && inner.height > below_rows {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "No folders or {} files here",
                    browser.target.extension_list()
                ),
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            ))
            .alignment(Alignment::Center),
            Rect::new(inner.x, inner.y + below_rows, inner.width, 1),
        );
    }
}

fn file_row<'a>(browser: &FileBrowser, entry: &'a BrowserEntry) -> Row<'a> {
    let name = match entry.kind {
        EntryKind::Parent => Line::from(Span::styled("↑ ..", Style::default().fg(MUTED))),
        EntryKind::UseFolder => Line::from(vec![
            Span::styled(
                "+ ",
                Style::default().fg(ACTION).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                browser.target.use_folder_label(),
                Style::default().fg(ACTION),
            ),
            Span::styled(
                entry.name.as_str(),
                Style::default().fg(ACTION).add_modifier(Modifier::BOLD),
            ),
        ]),
        EntryKind::Folder => Line::from(vec![
            Span::styled("▸ ", Style::default().fg(FOLDER)),
            Span::styled(
                entry.name.as_str(),
                Style::default().fg(FOLDER).add_modifier(Modifier::BOLD),
            ),
            Span::styled("/", Style::default().fg(FAINT)),
        ]),
        EntryKind::File if browser.is_current(entry) => Line::from(vec![
            Span::styled("● ", Style::default().fg(ACTION)),
            Span::styled(
                entry.name.as_str(),
                Style::default().fg(ACTION).add_modifier(Modifier::BOLD),
            ),
        ]),
        EntryKind::File => Line::from(vec![
            Span::raw("  "),
            Span::styled(entry.name.as_str(), Style::default().fg(Color::White)),
        ]),
    };
    let detail = |text: Option<String>| {
        Cell::from(
            Line::from(Span::styled(
                text.unwrap_or_default(),
                Style::default().fg(MUTED),
            ))
            .alignment(Alignment::Right),
        )
    };
    Row::new(vec![
        Cell::from(name),
        detail(entry.size.map(format_size)),
        detail(
            entry
                .modified
                .map(|date| date.format("%Y-%m-%d").to_string()),
        ),
    ])
}

fn render_footer(f: &mut Frame, browser: &FileBrowser, area: Rect) {
    let width = area.width as usize;
    let line = if let Some(error) = &browser.error {
        Line::from(vec![
            Span::styled(
                "✗ ",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                keep_end(error, width.saturating_sub(2)),
                Style::default().fg(Color::LightRed),
            ),
        ])
    } else {
        let (verb, target) = enter_action(browser);
        let mut spans = vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" → ", Style::default().fg(FAINT)),
            Span::styled(
                verb,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        let room = width.saturating_sub(spans.iter().map(Span::width).sum());
        spans.push(Span::styled(
            keep_end(&target, room),
            Style::default().fg(FOLDER),
        ));
        Line::from(spans)
    };
    f.render_widget(Paragraph::new(line), area);
}

fn enter_action(browser: &FileBrowser) -> (&'static str, String) {
    if browser.focus == Pane::Places {
        let label = browser
            .places_state
            .selected()
            .and_then(|index| browser.places.get(index))
            .map(|place| place.label.clone())
            .unwrap_or_default();
        return ("Go to ", label);
    }
    match browser.selected().map(|entry| (&entry.kind, entry)) {
        Some((EntryKind::Parent, _)) => (
            "Up to ",
            browser.dir.parent().map(tilde_path).unwrap_or_default(),
        ),
        Some((EntryKind::Folder, entry)) => ("Open ", format!("{}/", entry.name)),
        Some((EntryKind::UseFolder | EntryKind::File, entry)) => {
            ("Choose ", tilde_path(&browser.dir.join(&entry.name)))
        }
        None => ("", String::new()),
    }
}
