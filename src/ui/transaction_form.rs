use crate::app::fields::AddEditField;
use crate::app::state::{App, AppMode};
use crate::model::TransactionType;
use crate::ui::form::{FieldOverride, render_field_form};
use ratatui::prelude::*;

pub fn render_transaction_form(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.mode == AppMode::Editing {
        "Edit Transaction"
    } else {
        "Add New Transaction"
    };

    let transfer = app.add_edit_type() == TransactionType::Transfer;
    render_field_form(
        f,
        &app.add_edit_fields,
        app.add_edit_cursor,
        area,
        title,
        None,
        if transfer {
            |field| (field == AddEditField::Account).then_some(FieldOverride::Label("From Account"))
        } else {
            |field| {
                (field == AddEditField::ToAccount)
                    .then_some(FieldOverride::Placeholder("Transfers only"))
            }
        },
    );
}
