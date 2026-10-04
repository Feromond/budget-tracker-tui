use crate::app::fields::{CategoryEditField, FieldKey, FieldKind};
use crate::app::state::{App, AppMode};
use crate::model::CategorySortColumn;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_category_manager_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::CategoryCatalog => handle_category_catalog(app, key_event),
        AppMode::CategoryCatalogFilter => handle_category_filter(app, key_event),
        AppMode::CategoryEditor => handle_category_editor(app, key_event),
        AppMode::ConfirmCategoryDelete => handle_confirm_category_delete(app, key_event),
        AppMode::SelectingConversionAccount => handle_conversion_account(app, key_event),
        AppMode::ConfirmCategoryConversion => handle_confirm_category_conversion(app, key_event),
        _ => {}
    }
}

fn handle_category_catalog(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('q'), KeyModifiers::NONE) => {
            if app.is_category_filter_active() {
                app.reset_category_filter();
            } else {
                app.exit_category_catalog();
            }
        }
        (KeyCode::Down, KeyModifiers::NONE) => app.next_category_record(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_category_record(),
        (KeyCode::Up, KeyModifiers::CONTROL) => app.jump_to_first_category(),
        (KeyCode::Down, KeyModifiers::CONTROL) => app.jump_to_last_category(),
        (KeyCode::PageUp, KeyModifiers::NONE) => app.page_up_category(),
        (KeyCode::PageDown, KeyModifiers::NONE) => app.page_down_category(),
        (KeyCode::Char('f'), KeyModifiers::NONE) => app.start_category_filtering(),
        (KeyCode::Char('a'), KeyModifiers::NONE) => app.start_adding_category(),
        (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, KeyModifiers::NONE) => {
            app.start_editing_category()
        }
        (KeyCode::Char('d'), KeyModifiers::NONE) => app.prepare_delete_category(),
        (KeyCode::Char('b'), KeyModifiers::NONE) => app.start_editing_catalog_budget(),
        (KeyCode::Char('t'), KeyModifiers::NONE) => app.start_category_conversion(),
        (KeyCode::Char('1'), _) | (KeyCode::F(1), _) => {
            app.set_category_sort_column(CategorySortColumn::Type)
        }
        (KeyCode::Char('2'), _) | (KeyCode::F(2), _) => {
            app.set_category_sort_column(CategorySortColumn::Category)
        }
        (KeyCode::Char('3'), _) | (KeyCode::F(3), _) => {
            app.set_category_sort_column(CategorySortColumn::Subcategory)
        }
        (KeyCode::Char('4'), _) | (KeyCode::F(4), _) => {
            app.set_category_sort_column(CategorySortColumn::Tag)
        }
        (KeyCode::Char('5'), _) | (KeyCode::F(5), _) => {
            app.set_category_sort_column(CategorySortColumn::TargetBudget)
        }
        _ => {}
    }
}

fn handle_category_filter(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.reset_category_filter(),
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => app.reset_category_filter(),
        (KeyCode::Enter, KeyModifiers::NONE) => app.finish_category_filtering(),
        (KeyCode::Down, KeyModifiers::NONE) => app.next_category_record(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_category_record(),
        (KeyCode::Left, KeyModifiers::NONE) => app.move_cursor_left(),
        (KeyCode::Right, KeyModifiers::NONE) => app.move_cursor_right(),
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            app.insert_char_at_cursor(c);
            app.apply_category_filter();
        }
        (KeyCode::Backspace, KeyModifiers::NONE) => {
            app.delete_char_before_cursor();
            app.apply_category_filter();
        }
        (KeyCode::Delete, KeyModifiers::NONE) => {
            app.delete_char_after_cursor();
            app.apply_category_filter();
        }
        _ => {}
    }
}

fn handle_category_editor(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.exit_category_editor(true),
        (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.next_category_field()
        }
        (KeyCode::BackTab, KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.previous_category_field()
        }
        (KeyCode::Enter, KeyModifiers::NONE) => {
            match app.category_edit_fields.focused() {
                CategoryEditField::TransactionType => app.toggle_category_transaction_type(),
                // The budget is a separate record keyed on the category id, so the category
                // has to exist before the popup has anything to write against.
                CategoryEditField::Budget => app.save_category_then_edit_budget(),
                _ => {
                    app.save_category();
                }
            }
        }
        (KeyCode::Left, KeyModifiers::NONE) => {
            if app.category_edit_fields.focused() == CategoryEditField::TransactionType {
                app.toggle_category_transaction_type();
            } else {
                app.move_cursor_left();
            }
        }
        (KeyCode::Right, KeyModifiers::NONE) => {
            if app.category_edit_fields.focused() == CategoryEditField::TransactionType {
                app.toggle_category_transaction_type();
            } else {
                app.move_cursor_right();
            }
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => match app.category_edit_fields.focused().kind() {
            FieldKind::Amount if c.is_ascii_digit() || c == '.' => app.insert_char_at_cursor(c),
            FieldKind::Amount => {}
            FieldKind::Text => app.insert_char_at_cursor(c),
            _ => {}
        },
        (KeyCode::Char(c), KeyModifiers::SHIFT)
            if app.category_edit_fields.focused().kind() == FieldKind::Text =>
        {
            app.insert_char_at_cursor(c);
        }
        (KeyCode::Backspace, KeyModifiers::NONE)
            if app.category_edit_fields.focused().kind().is_editable() =>
        {
            app.delete_char_before_cursor();
        }
        (KeyCode::Delete, KeyModifiers::NONE)
            if app.category_edit_fields.focused().kind().is_editable() =>
        {
            app.delete_char_after_cursor();
        }
        _ => {}
    }
}

fn handle_confirm_category_delete(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_category(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete_category(),
        _ => {}
    }
}

fn handle_conversion_account(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => app.cancel_category_conversion(),
        KeyCode::Enter => app.choose_conversion_account(),
        KeyCode::Down => app.select_next_list_item(),
        KeyCode::Up => app.select_previous_list_item(),
        KeyCode::Char(c) => app.handle_type_to_select(c),
        _ => {}
    }
}

fn handle_confirm_category_conversion(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_category_conversion(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_category_conversion(),
        _ => {}
    }
}
