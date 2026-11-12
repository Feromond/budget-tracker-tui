use crate::app::state::{App, UpdatePhase};
use crate::app::updater::Action;
use crate::ui::helpers::centered_rect;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
};

pub fn render_update_popup(f: &mut Frame, app: &App, area: Rect) {
    if !app.show_update_popup {
        return;
    }
    let Some(update) = &app.update else {
        return;
    };
    let version = &update.version;
    let popup_area = centered_rect(50, 40, area);

    let block = Block::default()
        .title("Update Available")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    let inner_area = block.inner(popup_area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);

    let text = match &app.update_phase {
        UpdatePhase::Offer => {
            let how = match &update.action {
                Action::Replace { .. } | Action::Installer { .. } => {
                    "Press 'u' to update now.".to_string()
                }
                Action::Brew { .. } | Action::Cargo { .. } => format!(
                    "Press 'u' to close Budget Tracker and run\n{}\nIt reopens when that's done.",
                    update.action.command().unwrap_or_default()
                ),
                Action::Browser => "Visit GitHub releases to download.".to_string(),
            };
            format!(
                "\nVersion {version} is available!\n\n{how}\n\nPress <Enter> or 'o' to open the releases page and exit.\nPress any other key to close."
            )
        }
        UpdatePhase::Downloading { .. } => {
            format!("\nDownloading version {version}...\n\n\n\nPress Esc to cancel.")
        }
        UpdatePhase::Installed(_) => format!(
            "\nUpdated to version {version}.\n\nPress 'r' to restart into it now.\nPress any other key to keep working; the update applies next time you start."
        ),
        UpdatePhase::Failed(error) => format!(
            "\nThe update didn't work: {error}.\n\nPress <Enter> or 'o' to open the releases page and exit.\nPress any other key to close."
        ),
    };

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner_area);

    // Drawn after the text so the paragraph's style doesn't repaint it.
    if let UpdatePhase::Downloading { received, total } = app.update_phase {
        render_progress(f, inner_area, received, total);
    }
}

fn render_progress(f: &mut Frame, area: Rect, received: u64, total: u64) {
    let [_, gauge_area, _] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);
    let gauge_area = gauge_area.inner(ratatui::layout::Margin::new(2, 0));
    let ratio = if total == 0 {
        0.0
    } else {
        (received as f64 / total as f64).clamp(0.0, 1.0)
    };
    let label = format!(
        "{:.1} / {:.1} MB",
        received as f64 / 1_048_576.0,
        total as f64 / 1_048_576.0
    );
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .ratio(ratio)
            .label(label),
        gauge_area,
    );
}
