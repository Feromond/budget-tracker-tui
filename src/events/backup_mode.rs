use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_backup_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::BackupManager => handle_backup_list(app, key_event),
        AppMode::ConfirmBackupRestore => match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_restore_backup(),
            _ => app.cancel_backup_confirm(),
        },
        AppMode::ConfirmBackupDelete => match key_event.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_backup(),
            _ => app.cancel_backup_confirm(),
        },
        _ => {}
    }
}

fn handle_backup_list(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
            app.exit_backup_manager()
        }
        (KeyCode::Down, KeyModifiers::NONE) => app.next_backup(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_backup(),
        (KeyCode::Enter, KeyModifiers::NONE) => app.prepare_restore_backup(),
        (KeyCode::Char('b'), KeyModifiers::NONE) => app.create_backup_now(),
        (KeyCode::Char('d'), KeyModifiers::NONE) => app.prepare_delete_backup(),
        _ => {}
    }
}
