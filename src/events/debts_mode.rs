use crate::app::debts::{EXTRA_BIG_STEP, EXTRA_STEP};
use crate::app::fields::{DebtField, FieldKey, FieldKind, ReconcileField};
use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_debts_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::Debts => handle_overview(app, key_event),
        AppMode::DebtDetail => handle_detail(app, key_event),
        AppMode::DebtEditor => handle_editor(app, key_event),
        AppMode::DebtReconcile => handle_reconcile(app, key_event),
        AppMode::ConfirmDebtDelete => handle_confirm_delete(app, key_event),
        _ => {}
    }
}

fn handle_planner(app: &mut App, key_event: KeyEvent) -> bool {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Right, KeyModifiers::NONE) => app.adjust_debt_extra(EXTRA_STEP),
        (KeyCode::Left, KeyModifiers::NONE) => app.adjust_debt_extra(-EXTRA_STEP),
        (KeyCode::Right, KeyModifiers::SHIFT) => app.adjust_debt_extra(EXTRA_BIG_STEP),
        (KeyCode::Left, KeyModifiers::SHIFT) => app.adjust_debt_extra(-EXTRA_BIG_STEP),
        (KeyCode::Char('s'), KeyModifiers::NONE) => app.cycle_debt_strategy(),
        (KeyCode::Char('e'), KeyModifiers::NONE) => app.start_editing_debt(),
        (KeyCode::Char('p'), KeyModifiers::NONE) => app.start_debt_payment(),
        (KeyCode::Char('r'), KeyModifiers::NONE) => app.start_reconcile(),
        _ => return false,
    }
    true
}

fn handle_overview(app: &mut App, key_event: KeyEvent) {
    if handle_planner(app, key_event) {
        return;
    }
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => app.exit_debts_mode(),
        (KeyCode::Down, KeyModifiers::NONE) => app.step_debt(true),
        (KeyCode::Up, KeyModifiers::NONE) => app.step_debt(false),
        (KeyCode::Enter, KeyModifiers::NONE) => app.open_debt_detail(),
        (KeyCode::Char('a'), KeyModifiers::NONE) => app.start_adding_debt(),
        (KeyCode::Char('d'), KeyModifiers::NONE) => app.prepare_delete_debt(),
        (KeyCode::Char('A'), _) => app.toggle_archived_debts(),
        _ => {}
    }
}

fn handle_detail(app: &mut App, key_event: KeyEvent) {
    if handle_planner(app, key_event) {
        return;
    }
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => app.exit_debt_detail(),
        (KeyCode::Down, KeyModifiers::NONE) => app.step_debt_detail_row(true),
        (KeyCode::Up, KeyModifiers::NONE) => app.step_debt_detail_row(false),
        (KeyCode::Tab, KeyModifiers::NONE) => app.toggle_debt_detail_focus(),
        _ => {}
    }
}

fn type_into<K: FieldKey>(app: &mut App, field: K, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char(c), KeyModifiers::NONE) => match field.kind() {
            FieldKind::Date if c == '+' || c == '=' => app.increment_date(),
            FieldKind::Date if c == '-' => app.decrement_date(),
            FieldKind::Date if c.is_ascii_digit() => app.insert_char_at_cursor(c),
            FieldKind::Date => {}
            kind if !kind.is_editable() => {}
            _ => app.insert_char_at_cursor(c),
        },
        (KeyCode::Char(c), KeyModifiers::SHIFT) if field.kind() == FieldKind::Text => {
            app.insert_char_at_cursor(c)
        }
        (KeyCode::Backspace, KeyModifiers::NONE) if field.kind().is_editable() => {
            app.delete_char_before_cursor()
        }
        (KeyCode::Delete, KeyModifiers::NONE) if field.kind().is_editable() => {
            app.delete_char_after_cursor()
        }
        (KeyCode::Left, KeyModifiers::NONE) if field.kind() == FieldKind::Date => {
            app.decrement_date()
        }
        (KeyCode::Right, KeyModifiers::NONE) if field.kind() == FieldKind::Date => {
            app.increment_date()
        }
        (KeyCode::Left, KeyModifiers::SHIFT) if field.kind() == FieldKind::Date => {
            app.decrement_month()
        }
        (KeyCode::Right, KeyModifiers::SHIFT) if field.kind() == FieldKind::Date => {
            app.increment_month()
        }
        (KeyCode::Left, KeyModifiers::NONE) => app.move_cursor_left(),
        (KeyCode::Right, KeyModifiers::NONE) => app.move_cursor_right(),
        _ => {}
    }
}

fn handle_editor(app: &mut App, key_event: KeyEvent) {
    let focused = app.debt_fields.focused();
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.cancel_debt_editor(),
        (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.next_debt_field()
        }
        (KeyCode::BackTab, KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.previous_debt_field()
        }
        (KeyCode::Enter, KeyModifiers::NONE) if focused == DebtField::Status => {
            app.toggle_debt_status()
        }
        (KeyCode::Enter | KeyCode::Left | KeyCode::Right, KeyModifiers::NONE)
            if focused == DebtField::Plan =>
        {
            app.toggle_debt_plan()
        }
        (KeyCode::Enter, KeyModifiers::NONE) => app.save_debt(),
        (KeyCode::Left | KeyCode::Right, KeyModifiers::NONE) if focused == DebtField::Status => {
            app.toggle_debt_status()
        }
        _ => type_into(app, focused, key_event),
    }
}

fn handle_reconcile(app: &mut App, key_event: KeyEvent) {
    let focused: ReconcileField = app.reconcile_fields.focused();
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.cancel_reconcile(),
        (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.next_reconcile_field()
        }
        (KeyCode::BackTab, KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.previous_reconcile_field()
        }
        (KeyCode::Enter, KeyModifiers::NONE) => app.save_reconcile(),
        _ => type_into(app, focused, key_event),
    }
}

fn handle_confirm_delete(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_debt(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete_debt(),
        _ => {}
    }
}
