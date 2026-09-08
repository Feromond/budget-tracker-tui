use crate::app::state::{App, AppMode};
use crate::ui::form::render_field_form;
use ratatui::prelude::*;

pub fn render_transaction_form(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.mode == AppMode::Editing {
        "Edit Transaction"
    } else {
        "Add New Transaction"
    };

    render_field_form(
        f,
        &app.add_edit_fields,
        app.add_edit_cursor,
        area,
        title,
        None,
        |_| None,
    );
}
