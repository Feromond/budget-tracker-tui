use crate::app::state::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_file_browser(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => return app.close_file_browser(),
        KeyCode::Enter => return app.choose_file_browser_entry(),
        _ => {}
    }
    let Some(browser) = app.file_browser.as_mut() else {
        return;
    };
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Tab | KeyCode::BackTab, _) => browser.toggle_focus(),
        (KeyCode::Up, _) => browser.move_by(-1),
        (KeyCode::Down, _) => browser.move_by(1),
        (KeyCode::PageUp, _) => browser.move_by(-10),
        (KeyCode::PageDown, _) => browser.move_by(10),
        (KeyCode::Home, _) => browser.select_first(),
        (KeyCode::End, _) => browser.select_last(),
        (KeyCode::Right, _) => browser.enter_selected(),
        (KeyCode::Left | KeyCode::Backspace, _) => browser.go_up(),
        (KeyCode::Char('.'), KeyModifiers::NONE) if !browser.is_typing() => browser.toggle_hidden(),
        (KeyCode::Char('~'), KeyModifiers::NONE) if !browser.is_typing() => browser.go_home(),
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => browser.type_char(c),
        _ => {}
    }
}
