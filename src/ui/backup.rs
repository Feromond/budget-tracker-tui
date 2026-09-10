use crate::app::state::App;
use crate::db::backup::{BackupEntry, backups_dir};
use crate::ui::helpers::clamp_table_scroll;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_backup_manager(f: &mut Frame, app: &mut App, area: Rect) {
    let dir = backups_dir(&app.database_path);
    let title = format!(" Backups ({}) ", dir.to_string_lossy());

    if app.backup_entries.is_empty() {
        let hint = if app.backups_enabled {
            "No backups yet. Press b to take one now."
        } else {
            "No backups yet, and automatic backups are off. Press b to take one now."
        };
        let empty = Paragraph::new(hint)
            .alignment(Alignment::Center)
            .block(Block::default().title(title).borders(Borders::ALL));
        f.render_widget(empty, area);
        return;
    }

    let header = Row::new(["Taken", "Kind", "Size", "Data", "Device"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .height(1);

    let rows = app.backup_entries.iter().map(|entry| {
        let device_style = if entry.is_this_device {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::LightYellow)
        };

        Row::new(vec![
            Cell::from(entry.taken_at.format("%Y-%m-%d %H:%M").to_string()),
            Cell::from(entry.kind.label()),
            Cell::from(format_size(entry.size_bytes)),
            Cell::from(
                entry
                    .schema_version
                    .map(|version| format!("v{}", version))
                    .unwrap_or_else(|| "?".to_string()),
            ),
            Cell::from(device_label(entry)).style(device_style),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(17),
            Constraint::Min(16),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(18),
        ],
    )
    .header(header)
    .block(Block::default().title(title).borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol("> ");

    clamp_table_scroll(&mut app.backup_table_state, app.backup_entries.len(), area);
    f.render_stateful_widget(table, area, &mut app.backup_table_state);
}

fn device_label(entry: &BackupEntry) -> String {
    if entry.is_this_device {
        "This device".to_string()
    } else {
        format!("Other ({})", entry.instance)
    }
}

fn format_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let bytes = bytes as f64;
    if bytes < KIB {
        format!("{} B", bytes as u64)
    } else if bytes < KIB * KIB {
        format!("{:.1} KB", bytes / KIB)
    } else {
        format!("{:.1} MB", bytes / (KIB * KIB))
    }
}
