pub mod backup;
pub mod budget;
pub mod category_manager;
pub mod category_summary;
pub mod dialog;
pub mod filter;
pub mod form;
pub mod fuzzy_search;
pub mod help;
pub mod help_popup;
pub mod helpers;
pub mod investments;
pub mod ledger_manager;
pub mod recurring;
pub mod settings;
pub mod status;
pub mod summary;
pub mod transaction_form;
pub mod transaction_io;
pub mod transaction_table;
pub mod update_popup;

use crate::app::state::{App, AppMode};
use ratatui::Frame;
use ratatui::layout::Rect;

/// Whichever investments screen a popup was opened over.
fn render_investment_background(f: &mut Frame, app: &mut App, area: Rect) {
    if app.investment_detail_id.is_some() {
        investments::render_investment_detail(f, app, area);
    } else {
        investments::render_investments_view(f, app, area);
    }
}

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
    // Determine the effective mode for rendering the background (if in help mode)
    let render_mode =
        if app.mode == AppMode::KeybindingsInfo || app.mode == AppMode::KeybindingDetail {
            app.previous_mode.unwrap_or(AppMode::Normal)
        } else {
            app.mode
        };

    let filter_bar_height = if matches!(
        render_mode,
        AppMode::Filtering | AppMode::CategoryCatalogFilter
    ) {
        3
    } else {
        0
    };
    let status_bar_height = if app.status_message.is_some() { 3 } else { 0 };
    let summary_bar_height = if matches!(
        render_mode,
        AppMode::CategorySummary
            | AppMode::Budget
            | AppMode::BudgetCategoryEditor
            | AppMode::Settings
            | AppMode::CategoryCatalog
            | AppMode::CategoryCatalogFilter
            | AppMode::CategoryEditor
            | AppMode::ConfirmCategoryDelete
            | AppMode::Adding
            | AppMode::Editing
            | AppMode::FuzzyFinding
            | AppMode::RecurringSettings
            | AppMode::SelectingRecurrenceFrequency
            | AppMode::SelectingCategory
            | AppMode::SelectingSubcategory
            | AppMode::KeybindingsInfo
            | AppMode::KeybindingDetail
            | AppMode::ImportTransactions
            | AppMode::ExportTransactions
            | AppMode::LedgerManager
            | AppMode::LedgerEditor
            | AppMode::ConfirmLedgerDelete
            | AppMode::BackupManager
            | AppMode::ConfirmBackupRestore
            | AppMode::ConfirmBackupDelete
            | AppMode::Investments
            | AppMode::InvestmentDetail
            | AppMode::InvestmentAccountEditor
            | AppMode::InvestmentEntryEditor
            | AppMode::ConfirmInvestmentDelete
    ) {
        0
    } else {
        3
    };
    let help_bar_height = if app.hide_help_bar { 0 } else { 3 };

    let main_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(filter_bar_height),
            ratatui::layout::Constraint::Length(summary_bar_height),
            ratatui::layout::Constraint::Length(status_bar_height),
            ratatui::layout::Constraint::Length(help_bar_height),
        ])
        .split(f.area());

    let main_area = main_chunks[0];
    let filter_area = main_chunks[1];
    let summary_area = main_chunks[2];
    let status_area = main_chunks[3];
    let help_area = main_chunks[4];

    match render_mode {
        AppMode::Normal | AppMode::Filtering => {
            transaction_table::render_transaction_table(f, app, main_area);
        }
        AppMode::AdvancedFiltering => {
            filter::render_advanced_filter_form(f, app, main_area);
        }
        AppMode::SelectingFilterCategory | AppMode::SelectingFilterSubcategory => {
            filter::render_advanced_filter_form(f, app, main_area);
            dialog::render_selection_popup(f, app, main_area);
        }
        AppMode::Adding | AppMode::Editing => {
            transaction_form::render_transaction_form(f, app, main_area);
        }
        AppMode::ConfirmDelete => {
            transaction_table::render_transaction_table(f, app, main_area);
            dialog::render_confirmation_dialog(f, "Confirm Delete? (y/n)", main_area);
        }
        AppMode::Summary => {
            summary::render_summary_view(f, app, main_area);
        }
        AppMode::SelectingCategory | AppMode::SelectingSubcategory => {
            transaction_form::render_transaction_form(f, app, main_area);
            dialog::render_selection_popup(f, app, main_area);
        }
        AppMode::FuzzyFinding => {
            transaction_form::render_transaction_form(f, app, main_area);
            fuzzy_search::render_fuzzy_search(f, app, main_area);
        }
        AppMode::CategorySummary => {
            category_summary::render_category_summary_view(f, app, main_area);
        }
        AppMode::Budget => {
            budget::render_budget_view(f, app, main_area);
        }
        AppMode::BudgetCategoryEditor => {
            if app.budget_edit_origin == AppMode::CategoryCatalog {
                category_manager::render_category_catalog(f, app, main_area);
            } else {
                budget::render_budget_view(f, app, main_area);
            }
            budget::render_budget_target_editor(f, app, main_area);
        }
        AppMode::CategoryCatalog | AppMode::CategoryCatalogFilter => {
            category_manager::render_category_catalog(f, app, main_area);
        }
        AppMode::CategoryEditor => {
            category_manager::render_category_editor(f, app, main_area);
        }
        AppMode::ConfirmCategoryDelete => {
            category_manager::render_category_catalog(f, app, main_area);
            dialog::render_confirmation_dialog(f, "Delete selected category? (y/n)", main_area);
        }
        AppMode::Settings => {
            transaction_table::render_transaction_table(f, app, main_area);
            settings::render_settings_form(f, app, main_area);
        }
        AppMode::ImportTransactions | AppMode::ExportTransactions => {
            transaction_table::render_transaction_table(f, app, main_area);
            settings::render_settings_form(f, app, main_area);
            transaction_io::render_io_prompt(f, app, main_area);
        }
        AppMode::LedgerManager => {
            ledger_manager::render_ledger_manager(f, app, main_area);
        }
        AppMode::LedgerEditor => {
            ledger_manager::render_ledger_manager(f, app, main_area);
            ledger_manager::render_ledger_editor(f, app, main_area);
        }
        AppMode::ConfirmLedgerDelete => {
            ledger_manager::render_ledger_manager(f, app, main_area);
            dialog::render_confirmation_dialog(f, &app.ledger_delete_prompt, main_area);
        }
        AppMode::BackupManager => {
            backup::render_backup_manager(f, app, main_area);
        }
        AppMode::ConfirmBackupRestore | AppMode::ConfirmBackupDelete => {
            backup::render_backup_manager(f, app, main_area);
            let prompt = app.backup_confirm_prompt.clone();
            dialog::render_confirmation_dialog(f, &prompt, main_area);
        }
        AppMode::Investments | AppMode::InvestmentDetail => {
            render_investment_background(f, app, main_area);
        }
        AppMode::InvestmentAccountEditor => {
            render_investment_background(f, app, main_area);
            investments::render_investment_account_editor(f, app, main_area);
        }
        AppMode::InvestmentEntryEditor => {
            render_investment_background(f, app, main_area);
            investments::render_investment_entry_editor(f, app, main_area);
        }
        AppMode::ConfirmInvestmentDelete => {
            render_investment_background(f, app, main_area);
            dialog::render_confirmation_dialog(f, &app.investment_delete_prompt, main_area);
        }
        AppMode::RecurringSettings => {
            recurring::render_recurring_settings(f, app, main_area);
        }
        AppMode::SelectingRecurrenceFrequency => {
            recurring::render_recurring_settings(f, app, main_area);
            dialog::render_selection_popup(f, app, main_area);
        }
        _ => {}
    }

    if render_mode == AppMode::Filtering {
        filter::render_filter_input(f, app, filter_area);
    }

    if render_mode == AppMode::CategoryCatalogFilter {
        category_manager::render_category_filter_input(f, app, filter_area);
    }

    // Determine year filter based on current mode
    let year_filter = match render_mode {
        AppMode::Summary => app
            .summary_years
            .get(app.selected_summary_year_index)
            .copied(),
        AppMode::CategorySummary => app
            .category_summary_years
            .get(app.category_summary_year_index)
            .copied(),
        AppMode::Budget | AppMode::BudgetCategoryEditor => app.selected_budget_year(),
        _ => None, // No year filter for other modes - show all transactions as before
    };

    if let Some(msg) = &app.status_message {
        status::render_status_bar(f, msg, status_area);
    }

    summary::render_summary_bar(f, app, summary_area, year_filter);

    if !app.hide_help_bar {
        help::render_help_bar(f, app, help_area);
    }

    if app.mode == AppMode::KeybindingsInfo || app.mode == AppMode::KeybindingDetail {
        help_popup::render_keybindings_popup(f, app, f.area());
    }

    if app.show_update_popup {
        update_popup::render_update_popup(f, app, f.area());
    }

    if render_mode == AppMode::Filtering {
        let cursor_x = app.simple_filter_content[..app.simple_filter_cursor]
            .chars()
            .count() as u16;
        f.set_cursor_position(ratatui::layout::Position::new(
            filter_area.x + cursor_x + 1,
            filter_area.y + 1,
        ));
    }

    if render_mode == AppMode::CategoryCatalogFilter {
        let cursor_x = app.category_filter_query[..app.category_filter_cursor]
            .chars()
            .count() as u16;
        f.set_cursor_position(ratatui::layout::Position::new(
            filter_area.x + cursor_x + 1,
            filter_area.y + 1,
        ));
    }

    if app.mode == AppMode::FuzzyFinding {
        let popup_area = helpers::centered_rect(60, 60, main_area);
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(popup_area);

        let search_area = chunks[0];
        let cursor_x = app.search_query.chars().count() as u16;
        // Ensure cursor doesn't exceed width
        let max_width = search_area.width.saturating_sub(2);
        let displayed_cursor = cursor_x.min(max_width);

        f.set_cursor_position(ratatui::layout::Position::new(
            search_area.x + displayed_cursor + 1,
            search_area.y + 1,
        ));
    }
}
