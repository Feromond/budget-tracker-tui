use crate::app::file_browser::BrowseTarget;
use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_transaction_io_mode(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.cancel_transaction_io(),
        (KeyCode::Enter, KeyModifiers::NONE) => match app.mode {
            AppMode::ImportTransactions => app.import_transactions(),
            AppMode::ExportTransactions => app.export_transactions(),
            _ => {}
        },
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => app.reset_transaction_io_path(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.clear_transaction_io_path(),
        (KeyCode::Char('o'), KeyModifiers::CONTROL) => match app.mode {
            AppMode::ImportTransactions => app.open_file_browser(BrowseTarget::Import),
            AppMode::ExportTransactions => app.open_file_browser(BrowseTarget::Export),
            _ => {}
        },
        (KeyCode::Left, KeyModifiers::NONE) => app.move_cursor_left(),
        (KeyCode::Right, KeyModifiers::NONE) => app.move_cursor_right(),
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            app.insert_char_at_cursor(c)
        }
        (KeyCode::Backspace, KeyModifiers::NONE) => app.delete_char_before_cursor(),
        (KeyCode::Delete, KeyModifiers::NONE) => app.delete_char_after_cursor(),
        _ => {}
    }
}
