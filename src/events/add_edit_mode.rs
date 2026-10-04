use crate::app::fields::{AddEditField, FieldKey, FieldKind};
use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_add_edit_mode(app: &mut App, key_event: KeyEvent) {
    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            if app.mode == AppMode::Adding {
                app.exit_adding(true);
            } else {
                app.exit_editing(true);
            }
        }
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.next_add_edit_field();
        }
        (KeyModifiers::NONE, KeyCode::BackTab) => {
            app.previous_add_edit_field();
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            // Toggle Type, trigger selection popups, or save transaction
            match app.add_edit_fields.focused() {
                AddEditField::TransactionType => app.cycle_transaction_type(true),
                AddEditField::Category => app.start_category_selection(),
                AddEditField::Subcategory => app.start_subcategory_selection(),
                field @ (AddEditField::Account | AddEditField::ToAccount) => {
                    app.start_account_selection(field)
                }
                AddEditField::Date | AddEditField::Description | AddEditField::Amount => {
                    if app.mode == AppMode::Adding {
                        app.add_transaction();
                    } else {
                        app.update_transaction();
                    }
                }
            }
        }
        (KeyModifiers::NONE, KeyCode::Up) => app.previous_add_edit_field(),
        (KeyModifiers::NONE, KeyCode::Down) => app.next_add_edit_field(),
        (KeyModifiers::NONE, KeyCode::Left) => match app.add_edit_fields.focused() {
            AddEditField::Date => app.decrement_date(),
            AddEditField::TransactionType => app.cycle_transaction_type(false),
            AddEditField::Description
            | AddEditField::Amount
            | AddEditField::Category
            | AddEditField::Subcategory
            | AddEditField::Account
            | AddEditField::ToAccount => app.move_cursor_left(),
        },
        (KeyModifiers::NONE, KeyCode::Right) => match app.add_edit_fields.focused() {
            AddEditField::Date => app.increment_date(),
            AddEditField::TransactionType => app.cycle_transaction_type(true),
            AddEditField::Description
            | AddEditField::Amount
            | AddEditField::Category
            | AddEditField::Subcategory
            | AddEditField::Account
            | AddEditField::ToAccount => app.move_cursor_right(),
        },
        (KeyModifiers::SHIFT, KeyCode::Left)
            if app.add_edit_fields.focused() == AddEditField::Date =>
        {
            app.decrement_month()
        }
        (KeyModifiers::SHIFT, KeyCode::Right)
            if app.add_edit_fields.focused() == AddEditField::Date =>
        {
            app.increment_month()
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            let field = app.add_edit_fields.focused();
            match field.kind() {
                FieldKind::Date if c == '+' || c == '=' => app.increment_date(),
                FieldKind::Date if c == '-' => app.decrement_date(),
                FieldKind::Date if c.is_ascii_digit() => app.insert_char_at_cursor(c),
                FieldKind::Date => {}
                kind if !kind.is_editable() => {}
                _ => app.insert_char_at_cursor(c),
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char(c))
            if app.add_edit_fields.focused() == AddEditField::Description =>
        {
            app.insert_char_at_cursor(c);
        }
        (KeyModifiers::NONE, KeyCode::Backspace)
            if app.add_edit_fields.focused().kind().is_editable() =>
        {
            app.delete_char_before_cursor();
        }
        (KeyModifiers::NONE, KeyCode::Delete)
            if app.add_edit_fields.focused().kind().is_editable() =>
        {
            app.delete_char_after_cursor();
        }
        _ => {}
    }
}

pub fn handle_confirm_delete(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete(),
        _ => {}
    }
}
