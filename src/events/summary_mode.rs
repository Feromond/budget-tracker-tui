use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_summary_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::Summary => handle_regular_summary(app, key_event),
        AppMode::CategorySummary => handle_category_summary(app, key_event),
        _ => {}
    }
}

fn handle_regular_summary(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('q') | KeyCode::Esc => app.exit_summary_mode(),
        KeyCode::Down => app.next_summary_month(),
        KeyCode::Up => app.previous_summary_month(),
        KeyCode::Char(']') | KeyCode::PageDown | KeyCode::Right => app.next_summary_year(),
        KeyCode::Char('[') | KeyCode::PageUp | KeyCode::Left => app.previous_summary_year(),
        KeyCode::Char('m') => app.summary_multi_month_mode = !app.summary_multi_month_mode,
        KeyCode::Char('c') => app.summary_cumulative_mode = !app.summary_cumulative_mode,
        _ => {}
    }
}

fn handle_category_summary(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('q') | KeyCode::Esc => app.exit_category_summary_mode(),
        KeyCode::Down => app.next_category_summary_item(),
        KeyCode::Up => app.previous_category_summary_item(),
        // PageUp/PageDown for month jumping
        KeyCode::PageUp => app.previous_category_summary_month(),
        KeyCode::PageDown => app.next_category_summary_month(),
        // Brackets and Left/Right for year navigation
        KeyCode::Char(']') | KeyCode::Right => app.next_category_summary_year(),
        KeyCode::Char('[') | KeyCode::Left => app.previous_category_summary_year(),
        KeyCode::Enter => app.toggle_category_summary_row(),
        KeyCode::Char('f') => app.filter_transactions_from_category_summary(),
        _ => {}
    }
}
