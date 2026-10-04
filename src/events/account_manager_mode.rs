use crate::app::fields::{AccountField, FieldKey};
use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_account_manager_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::AccountManager => handle_account_list(app, key_event),
        AppMode::AccountEditor => handle_account_editor(app, key_event),
        AppMode::ConfirmAccountDelete => handle_confirm_account_delete(app, key_event),
        _ => {}
    }
}

fn handle_account_list(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
            app.exit_account_manager()
        }
        (KeyCode::Down, KeyModifiers::NONE) => app.next_account(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_account(),
        (KeyCode::Char('a'), KeyModifiers::NONE) => app.start_adding_account(),
        (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, KeyModifiers::NONE) => {
            app.start_editing_account()
        }
        (KeyCode::Char('d'), KeyModifiers::NONE) => app.prepare_delete_account(),
        _ => {}
    }
}

fn handle_account_editor(app: &mut App, key_event: KeyEvent) {
    let focused = app.account_fields.focused();
    let toggle = matches!(focused, AccountField::Class | AccountField::Status);
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.cancel_account_editor(),
        (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.next_account_field()
        }
        (KeyCode::BackTab, KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.previous_account_field()
        }
        (KeyCode::Enter, KeyModifiers::NONE) if toggle => app.cycle_account_toggle(true),
        (KeyCode::Enter, KeyModifiers::NONE) => app.save_account(),
        (KeyCode::Left, KeyModifiers::NONE) if toggle => app.cycle_account_toggle(false),
        (KeyCode::Right, KeyModifiers::NONE) if toggle => app.cycle_account_toggle(true),
        (KeyCode::Left, KeyModifiers::NONE) => app.move_cursor_left(),
        (KeyCode::Right, KeyModifiers::NONE) => app.move_cursor_right(),
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT)
            if focused.kind().is_editable() =>
        {
            app.insert_char_at_cursor(c)
        }
        (KeyCode::Backspace, KeyModifiers::NONE) if focused.kind().is_editable() => {
            app.delete_char_before_cursor()
        }
        (KeyCode::Delete, KeyModifiers::NONE) if focused.kind().is_editable() => {
            app.delete_char_after_cursor()
        }
        _ => {}
    }
}

fn handle_confirm_account_delete(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_account(),
        _ => app.cancel_delete_account(),
    }
}
