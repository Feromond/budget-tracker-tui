use crate::app::state::App;
use crate::model::SortColumn;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_normal_mode(app: &mut App, key_event: KeyEvent) {
    let key_code = key_event.code;
    let modifiers = key_event.modifiers;

    match (key_code, modifiers) {
        // Navigation with modifiers
        (KeyCode::Up, KeyModifiers::CONTROL) => app.jump_to_first(),
        (KeyCode::Down, KeyModifiers::CONTROL) => app.jump_to_last(),
        (KeyCode::PageUp, KeyModifiers::NONE) => app.page_up(),
        (KeyCode::PageDown, KeyModifiers::NONE) => app.page_down(),
        // Regular navigation
        (KeyCode::Down, KeyModifiers::NONE) => app.next_item(),
        (KeyCode::Up, KeyModifiers::NONE) => app.previous_item(),
        // Application commands
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
            if app.is_filter_active() {
                app.reset_all_filters();
            } else {
                app.quit();
            }
        }
        (KeyCode::Char('a'), _) => app.start_adding(),
        (KeyCode::Char('d'), _) => app.prepare_for_delete(),
        (KeyCode::Char('e'), _) => app.start_editing(),
        (KeyCode::Char('f'), KeyModifiers::CONTROL) => app.start_advanced_filtering(),
        (KeyCode::Char('f'), _) => app.start_filtering(),
        (KeyCode::Char('r'), _) => app.start_recurring_settings(),
        (KeyCode::Char('s'), _) => app.enter_summary_mode(),
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.copy_transaction(),
        (KeyCode::Char('c'), _) => app.enter_category_summary_mode(),
        (KeyCode::Char('b'), _) => app.enter_budget_mode(),
        (KeyCode::Char('i'), _) => app.enter_investments_mode(),
        (KeyCode::Char('o'), _) => app.enter_settings_mode(),
        // Sorting
        (KeyCode::Char('1'), _) | (KeyCode::F(1), _) => app.set_sort_column(SortColumn::Date),
        (KeyCode::Char('2'), _) | (KeyCode::F(2), _) => {
            app.set_sort_column(SortColumn::Description)
        }
        (KeyCode::Char('3'), _) | (KeyCode::F(3), _) => app.set_sort_column(SortColumn::Category),
        (KeyCode::Char('4'), _) | (KeyCode::F(4), _) => {
            app.set_sort_column(SortColumn::Subcategory)
        }
        (KeyCode::Char('5'), _) | (KeyCode::F(5), _) => app.set_sort_column(SortColumn::Type),
        (KeyCode::Char('6'), _) | (KeyCode::F(6), _) => app.set_sort_column(SortColumn::Amount),
        _ => {}
    }
}
