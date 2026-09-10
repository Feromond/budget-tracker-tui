use crate::app::state::{App, AppMode};
use crate::ui::ui;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;
use ratatui::prelude::Backend;
use std::result::Result as StdResult;
use std::time::Duration;

use super::{
    add_edit_mode, backup_mode, budget_mode, category_manager_mode, filter_mode, fuzzy_search_mode,
    help_mode, investments_mode, ledger_manager_mode, normal_mode, recurring_mode, selection_mode,
    settings_mode, summary_mode, transaction_io_mode,
};

pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> StdResult<(), Box<dyn std::error::Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    // A draw builds a row per filtered transaction, so idle repaints get expensive on big ledgers.
    let mut needs_redraw = true;

    while !app.should_quit {
        // Check for status expiry
        if let Some(expiry) = app.status_expiry
            && std::time::Instant::now() > expiry
        {
            app.clear_status_message();
            needs_redraw = true;
        }

        // Check for update in background channel
        if let Ok(Some(version)) = app.update_rx.try_recv() {
            app.update_available_version = Some(version);
            app.show_update_popup = true;
            needs_redraw = true;
        }

        if needs_redraw {
            terminal.draw(|f| ui(f, app))?;
            needs_redraw = false;
        }

        if event::poll(Duration::from_millis(250))? {
            needs_redraw = true;

            let event = match event::read()? {
                Event::Key(key) => Event::Key(normalize_key(key)),
                other => other,
            };
            match event {
                Event::Paste(text) => app.handle_paste(&text),
                Event::Key(key)
                    if key.kind == KeyEventKind::Press
                        && (app.show_update_popup
                                || key.modifiers == KeyModifiers::NONE
                                || (app.mode == AppMode::Settings && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('d') | KeyCode::Char('u') | KeyCode::Char('v')))
                                // Let Shift+Char pass through for typing capitals/symbols in settings path
                                || (app.mode == AppMode::Settings && key.modifiers == KeyModifiers::SHIFT && matches!(key.code, KeyCode::Char(_)))
                                // Import/Export path prompt: allow Shift+Char and Ctrl+D/U
                                || ((app.mode == AppMode::ImportTransactions || app.mode == AppMode::ExportTransactions) && key.modifiers == KeyModifiers::SHIFT && matches!(key.code, KeyCode::Char(_)))
                                || ((app.mode == AppMode::ImportTransactions || app.mode == AppMode::ExportTransactions) && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('d') | KeyCode::Char('u') | KeyCode::Char('v')))
                                // Allow Shift+Char in Adding, Editing and FuzzyFinding modes
                                || ((app.mode == AppMode::Adding || app.mode == AppMode::Editing || app.mode == AppMode::FuzzyFinding || app.mode == AppMode::CategoryEditor || app.mode == AppMode::CategoryCatalogFilter || app.mode == AppMode::LedgerEditor) && key.modifiers == KeyModifiers::SHIFT && matches!(key.code, KeyCode::Char(_)))
                                // Allow Shift+Arrow in date-like navigation modes
                                || ((app.mode == AppMode::Adding || app.mode == AppMode::Editing || app.mode == AppMode::AdvancedFiltering || app.mode == AppMode::RecurringSettings || app.mode == AppMode::Budget || app.mode == AppMode::InvestmentAccountEditor || app.mode == AppMode::InvestmentEntryEditor)
                                    && key.modifiers == KeyModifiers::SHIFT
                                    && matches!(key.code, KeyCode::Left | KeyCode::Right))
                                || ((app.mode == AppMode::InvestmentAccountEditor || app.mode == AppMode::InvestmentEntryEditor || app.mode == AppMode::Investments) && key.modifiers == KeyModifiers::SHIFT && matches!(key.code, KeyCode::Char(_)))
                                // Allow Ctrl+F/R in simple Filtering mode and Ctrl+R in AdvancedFiltering mode
                                || (app.mode == AppMode::Filtering && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('f') | KeyCode::Char('r')))
                                || (app.mode == AppMode::AdvancedFiltering && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('r')))
                                || (app.mode == AppMode::CategoryCatalogFilter && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Char('r')))
                                || ((app.mode == AppMode::Filtering || app.mode == AppMode::AdvancedFiltering) && key.modifiers == KeyModifiers::SHIFT && matches!(key.code, KeyCode::Char(_)))
                                // Allow Ctrl+Up/Down for jump navigation, Ctrl+C for copy, and Ctrl+F for advanced filter in Normal mode
                                || (app.mode == AppMode::Normal && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Char('c') | KeyCode::Char('f')))
                                // Allow Ctrl+C to copy the selected ledger
                                || (app.mode == AppMode::LedgerManager && key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c'))
                                // Allow Ctrl+Up/Down for jump navigation in the category catalog
                                || (app.mode == AppMode::CategoryCatalog && key.modifiers == KeyModifiers::CONTROL && matches!(key.code, KeyCode::Up | KeyCode::Down))
                                // Allow Ctrl+H for Help Toggle
                                || (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('h'))) =>
                {
                    if app.mode != AppMode::ConfirmDelete
                        && app.mode != AppMode::SelectingCategory
                        && app.mode != AppMode::SelectingSubcategory
                        && app.mode != AppMode::KeybindingsInfo
                    {
                        app.clear_status_message();
                    }
                    update(app, key);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

// Shift has no distinct meaning for these keys, but is often still held when
// pressing them while typing capitals; treat them as their unmodified forms.
fn normalize_key(mut key: KeyEvent) -> KeyEvent {
    if key.modifiers == KeyModifiers::SHIFT
        && matches!(
            key.code,
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Enter | KeyCode::BackTab
        )
    {
        key.modifiers = KeyModifiers::NONE;
    }
    key
}

// Main input handler, dispatching based on mode
fn update(app: &mut App, key_event: KeyEvent) {
    if app.show_update_popup {
        if key_event.kind == KeyEventKind::Press {
            match key_event.code {
                KeyCode::Enter | KeyCode::Char('o') | KeyCode::Char('O') => {
                    let opened = crate::app::util::open_url(
                        "https://github.com/Feromond/budget-tracker-tui/releases",
                    );
                    app.show_update_popup = false;
                    if opened {
                        // Close the app so the old and new versions are not run side by side.
                        app.quit();
                    } else {
                        app.set_status_message(
                            "Could not open browser. Visit github.com/Feromond/budget-tracker-tui/releases",
                            Some(chrono::Duration::seconds(5)),
                        );
                    }
                }
                _ => {
                    app.show_update_popup = false;
                }
            }
        }
        return;
    }

    // Global Toggle for Keybindings Help
    if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('h') {
        if app.mode == AppMode::KeybindingsInfo || app.mode == AppMode::KeybindingDetail {
            let prev = app.previous_mode.unwrap_or(AppMode::Normal);
            app.mode = prev;
            app.previous_mode = None;
        } else {
            app.previous_mode = Some(app.mode);
            app.mode = AppMode::KeybindingsInfo;
            app.help_table_state.select(Some(0));
            app.type_to_select.clear();
        }
        return;
    }

    match app.mode {
        AppMode::KeybindingsInfo | AppMode::KeybindingDetail => {
            help_mode::handle_help_mode(app, key_event)
        }
        AppMode::Normal => normal_mode::handle_normal_mode(app, key_event),
        AppMode::Adding | AppMode::Editing => add_edit_mode::handle_add_edit_mode(app, key_event),
        AppMode::ConfirmDelete => add_edit_mode::handle_confirm_delete(app, key_event),
        AppMode::Filtering | AppMode::AdvancedFiltering => {
            filter_mode::handle_filter_mode(app, key_event)
        }
        AppMode::FuzzyFinding => fuzzy_search_mode::handle_fuzzy_search_mode(app, key_event),
        AppMode::Summary | AppMode::CategorySummary => {
            summary_mode::handle_summary_mode(app, key_event)
        }
        AppMode::Budget | AppMode::BudgetCategoryEditor => {
            budget_mode::handle_budget_mode(app, key_event)
        }
        AppMode::SelectingCategory
        | AppMode::SelectingSubcategory
        | AppMode::SelectingFilterCategory
        | AppMode::SelectingFilterSubcategory
        | AppMode::SelectingRecurrenceFrequency => {
            selection_mode::handle_selection_mode(app, key_event)
        }
        AppMode::Settings => settings_mode::handle_settings_mode(app, key_event),
        AppMode::ImportTransactions | AppMode::ExportTransactions => {
            transaction_io_mode::handle_transaction_io_mode(app, key_event)
        }
        AppMode::RecurringSettings => recurring_mode::handle_recurring_mode(app, key_event),
        AppMode::CategoryCatalog
        | AppMode::CategoryCatalogFilter
        | AppMode::CategoryEditor
        | AppMode::ConfirmCategoryDelete => {
            category_manager_mode::handle_category_manager_mode(app, key_event)
        }
        AppMode::LedgerManager | AppMode::LedgerEditor | AppMode::ConfirmLedgerDelete => {
            ledger_manager_mode::handle_ledger_manager_mode(app, key_event)
        }
        AppMode::BackupManager | AppMode::ConfirmBackupRestore | AppMode::ConfirmBackupDelete => {
            backup_mode::handle_backup_mode(app, key_event)
        }
        AppMode::Investments
        | AppMode::InvestmentDetail
        | AppMode::InvestmentAccountEditor
        | AppMode::InvestmentEntryEditor
        | AppMode::ConfirmInvestmentDelete => {
            investments_mode::handle_investments_mode(app, key_event)
        }
    }
}
