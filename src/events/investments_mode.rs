use crate::app::fields::{FieldKey, FieldKind, InvestmentAccountField, InvestmentEntryField};
use crate::app::state::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_investments_mode(app: &mut App, key_event: KeyEvent) {
    match app.mode {
        AppMode::Investments => handle_overview(app, key_event),
        AppMode::InvestmentDetail => handle_detail(app, key_event),
        AppMode::InvestmentAccountEditor => handle_account_editor(app, key_event),
        AppMode::InvestmentEntryEditor => handle_entry_editor(app, key_event),
        AppMode::ConfirmInvestmentDelete => handle_confirm_delete(app, key_event),
        _ => {}
    }
}

fn handle_overview(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => app.exit_investments_mode(),
        (KeyCode::Down, KeyModifiers::NONE) => app.next_investment_account(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_investment_account(),
        (KeyCode::Right, KeyModifiers::NONE) => app.cycle_investment_range(true),
        (KeyCode::Left, KeyModifiers::NONE) => app.cycle_investment_range(false),
        (KeyCode::Enter, KeyModifiers::NONE) => app.open_investment_detail(),
        (KeyCode::Char('a'), KeyModifiers::NONE) => app.start_adding_investment_account(),
        (KeyCode::Char('e'), KeyModifiers::NONE) => app.start_editing_investment_account(),
        (KeyCode::Char('d'), KeyModifiers::NONE) => app.prepare_delete_investment(),
        (KeyCode::Char('v'), KeyModifiers::NONE) => app.start_recording_valuation(),
        (KeyCode::Char('A'), _) => app.toggle_archived_investments(),
        _ => {}
    }
}

fn handle_detail(app: &mut App, key_event: KeyEvent) {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => app.exit_investment_detail(),
        (KeyCode::Down, KeyModifiers::NONE) => app.next_investment_entry(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_investment_entry(),
        (KeyCode::Right, KeyModifiers::NONE) => app.cycle_investment_range(true),
        (KeyCode::Left, KeyModifiers::NONE) => app.cycle_investment_range(false),
        // a/e/d follow the rows on screen, which here are entries.
        (KeyCode::Char('a'), KeyModifiers::NONE) => app.start_adding_investment_entry(),
        (KeyCode::Char('e'), KeyModifiers::NONE) | (KeyCode::Enter, KeyModifiers::NONE) => {
            app.start_editing_investment_entry()
        }
        (KeyCode::Char('d'), KeyModifiers::NONE) => app.prepare_delete_investment(),
        (KeyCode::Char('v'), KeyModifiers::NONE) => app.start_recording_valuation(),
        _ => {}
    }
}

fn handle_account_editor(app: &mut App, key_event: KeyEvent) {
    let focused = app.investment_account_fields.focused();
    let inert = !app.investment_opening_fields_active()
        && matches!(
            focused,
            InvestmentAccountField::OpeningValue
                | InvestmentAccountField::OpeningInvested
                | InvestmentAccountField::OpeningDate
        );

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.cancel_investment_account_editor(),
        (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.next_investment_account_field()
        }
        (KeyCode::BackTab, KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.previous_investment_account_field()
        }
        (KeyCode::Enter, KeyModifiers::NONE) => match focused {
            InvestmentAccountField::Status => app.toggle_investment_status(),
            _ => app.save_investment_account(),
        },
        (KeyCode::Left, KeyModifiers::NONE) => match focused {
            InvestmentAccountField::Status => app.toggle_investment_status(),
            InvestmentAccountField::OpeningDate if !inert => app.decrement_date(),
            _ => app.move_cursor_left(),
        },
        (KeyCode::Right, KeyModifiers::NONE) => match focused {
            InvestmentAccountField::Status => app.toggle_investment_status(),
            InvestmentAccountField::OpeningDate if !inert => app.increment_date(),
            _ => app.move_cursor_right(),
        },
        (KeyCode::Left, KeyModifiers::SHIFT)
            if focused == InvestmentAccountField::OpeningDate && !inert =>
        {
            app.decrement_month()
        }
        (KeyCode::Right, KeyModifiers::SHIFT)
            if focused == InvestmentAccountField::OpeningDate && !inert =>
        {
            app.increment_month()
        }
        (KeyCode::Char(_), _) | (KeyCode::Backspace, _) | (KeyCode::Delete, _) if inert => {}
        (KeyCode::Char(c), KeyModifiers::NONE) => match focused.kind() {
            FieldKind::Date if c == '+' || c == '=' => app.increment_date(),
            FieldKind::Date if c == '-' => app.decrement_date(),
            FieldKind::Date if c.is_ascii_digit() => app.insert_char_at_cursor(c),
            FieldKind::Date => {}
            kind if !kind.is_editable() => {}
            _ => app.insert_char_at_cursor(c),
        },
        (KeyCode::Char(c), KeyModifiers::SHIFT) if focused.kind() == FieldKind::Text => {
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

fn handle_entry_editor(app: &mut App, key_event: KeyEvent) {
    let focused = app.investment_entry_fields.focused();

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Esc, KeyModifiers::NONE) => app.cancel_investment_entry_editor(),
        (KeyCode::Tab, KeyModifiers::NONE) | (KeyCode::Down, KeyModifiers::NONE) => {
            app.next_investment_entry_field()
        }
        (KeyCode::BackTab, KeyModifiers::NONE) | (KeyCode::Up, KeyModifiers::NONE) => {
            app.previous_investment_entry_field()
        }
        (KeyCode::Enter, KeyModifiers::NONE) => match focused {
            InvestmentEntryField::EntryKind => app.cycle_investment_entry_kind(true),
            _ => app.save_investment_entry(),
        },
        (KeyCode::Left, KeyModifiers::NONE) => match focused {
            InvestmentEntryField::EntryKind => app.cycle_investment_entry_kind(false),
            InvestmentEntryField::Date => app.decrement_date(),
            _ => app.move_cursor_left(),
        },
        (KeyCode::Right, KeyModifiers::NONE) => match focused {
            InvestmentEntryField::EntryKind => app.cycle_investment_entry_kind(true),
            InvestmentEntryField::Date => app.increment_date(),
            _ => app.move_cursor_right(),
        },
        (KeyCode::Left, KeyModifiers::SHIFT) if focused == InvestmentEntryField::Date => {
            app.decrement_month()
        }
        (KeyCode::Right, KeyModifiers::SHIFT) if focused == InvestmentEntryField::Date => {
            app.increment_month()
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => match focused.kind() {
            FieldKind::Date if c == '+' || c == '=' => app.increment_date(),
            FieldKind::Date if c == '-' => app.decrement_date(),
            FieldKind::Date if c.is_ascii_digit() => app.insert_char_at_cursor(c),
            FieldKind::Date => {}
            kind if !kind.is_editable() => {}
            _ => app.insert_char_at_cursor(c),
        },
        (KeyCode::Char(c), KeyModifiers::SHIFT) if focused.kind() == FieldKind::Text => {
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

fn handle_confirm_delete(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete_investment(),
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_delete_investment(),
        _ => {}
    }
}
