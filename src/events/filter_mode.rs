use crate::app::fields::{AdvancedFilterField, FieldKey, FieldKind};
use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_filter_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::Filtering => handle_simple_filtering(app, key_event),
        AppMode::AdvancedFiltering => handle_advanced_filtering(app, key_event),
        _ => {}
    }
}

fn handle_simple_filtering(app: &mut App, key_event: KeyEvent) {
    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('f')) => app.start_advanced_filtering(),
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => app.reset_all_filters(),
        (KeyModifiers::NONE, KeyCode::Enter) => app.exit_filtering(),
        (KeyModifiers::NONE, KeyCode::Esc) => app.reset_all_filters(),
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            app.clear_advanced_filter_fields_only();
            app.insert_char_at_cursor(c);
            app.apply_filter();
        }
        (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.clear_advanced_filter_fields_only();
            app.insert_char_at_cursor(c);
            app.apply_filter();
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            app.clear_advanced_filter_fields_only();
            app.delete_char_before_cursor();
            app.apply_filter();
        }
        (KeyModifiers::NONE, KeyCode::Delete) => {
            app.clear_advanced_filter_fields_only();
            app.delete_char_after_cursor();
            app.apply_filter();
        }
        (KeyModifiers::NONE, KeyCode::Left) => app.move_cursor_left(),
        (KeyModifiers::NONE, KeyCode::Right) => app.move_cursor_right(),
        _ => {}
    }
}

fn handle_advanced_filtering(app: &mut App, key_event: KeyEvent) {
    match (key_event.modifiers, key_event.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => app.reset_all_filters(),
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => app.toggle_advanced_exclusion(),
        (KeyModifiers::NONE, KeyCode::Esc) => app.cancel_advanced_filtering(),
        (KeyModifiers::NONE, KeyCode::Enter) => match app.advanced_filter_fields.focused() {
            AdvancedFilterField::Category => app.start_advanced_category_selection(),
            AdvancedFilterField::Subcategory => app.start_advanced_subcategory_selection(),
            AdvancedFilterField::DateFrom
            | AdvancedFilterField::DateTo
            | AdvancedFilterField::Description
            | AdvancedFilterField::TransactionType
            | AdvancedFilterField::Recurring
            | AdvancedFilterField::AmountFrom
            | AdvancedFilterField::AmountTo => app.finish_advanced_filtering(),
        },
        (KeyModifiers::NONE, KeyCode::Tab) => app.next_advanced_filter_field(),
        (KeyModifiers::NONE, KeyCode::BackTab) => app.previous_advanced_filter_field(),
        (KeyModifiers::NONE, KeyCode::Up) => app.previous_advanced_filter_field(),
        (KeyModifiers::NONE, KeyCode::Down) => app.next_advanced_filter_field(),
        (KeyModifiers::NONE, KeyCode::Left) => match app.advanced_filter_fields.focused() {
            AdvancedFilterField::DateFrom | AdvancedFilterField::DateTo => {
                app.decrement_advanced_date()
            }
            AdvancedFilterField::TransactionType => app.toggle_advanced_transaction_type(),
            AdvancedFilterField::Recurring => app.toggle_advanced_recurring(),
            AdvancedFilterField::Description
            | AdvancedFilterField::Category
            | AdvancedFilterField::Subcategory
            | AdvancedFilterField::AmountFrom
            | AdvancedFilterField::AmountTo => app.move_cursor_left(),
        },
        (KeyModifiers::NONE, KeyCode::Right) => match app.advanced_filter_fields.focused() {
            AdvancedFilterField::DateFrom | AdvancedFilterField::DateTo => {
                app.increment_advanced_date()
            }
            AdvancedFilterField::TransactionType => app.toggle_advanced_transaction_type(),
            AdvancedFilterField::Recurring => app.toggle_advanced_recurring(),
            AdvancedFilterField::Description
            | AdvancedFilterField::Category
            | AdvancedFilterField::Subcategory
            | AdvancedFilterField::AmountFrom
            | AdvancedFilterField::AmountTo => app.move_cursor_right(),
        },
        (KeyModifiers::SHIFT, KeyCode::Left)
            if app.advanced_filter_fields.focused().kind() == FieldKind::Date =>
        {
            app.decrement_advanced_month()
        }
        (KeyModifiers::SHIFT, KeyCode::Right)
            if app.advanced_filter_fields.focused().kind() == FieldKind::Date =>
        {
            app.increment_advanced_month()
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) => {
            match app.advanced_filter_fields.focused().kind() {
                FieldKind::Date if c == '+' || c == '=' => app.increment_advanced_date(),
                FieldKind::Date if c == '-' => app.decrement_advanced_date(),
                _ => {
                    app.clear_simple_filter_field_only();
                    app.insert_char_at_cursor(c);
                }
            }
        }
        (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.clear_simple_filter_field_only();
            app.insert_char_at_cursor(c);
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            app.clear_simple_filter_field_only();
            app.delete_char_before_cursor();
        }
        (KeyModifiers::NONE, KeyCode::Delete) => {
            app.clear_simple_filter_field_only();
            app.delete_char_after_cursor();
        }
        _ => {}
    }
}
