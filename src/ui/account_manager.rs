use crate::app::fields::AccountField;
use crate::app::state::App;
use crate::model::{Account, AccountClass};
use crate::ui::form::{FieldOverride, render_field_form};
use crate::ui::helpers::{centered_rect, clamp_table_scroll, format_amount};
use ratatui::prelude::*;
use ratatui::widgets::*;
use rust_decimal::Decimal;

pub fn balance_label(account: &Account, balance: Decimal) -> &'static str {
    match account.class {
        AccountClass::Credit if balance <= Decimal::ZERO => "Owed",
        AccountClass::Credit => "Credit",
        AccountClass::Investment => "Value",
        AccountClass::Cash => "Balance",
    }
}

pub fn balance_amount(account: &Account, balance: Decimal) -> String {
    match account.class {
        AccountClass::Credit => format_amount(&balance.abs()),
        AccountClass::Cash | AccountClass::Investment => format_amount(&balance),
    }
}

pub fn render_account_manager(f: &mut Frame, app: &mut App, area: Rect) {
    let title = format!(" Accounts: {} ", app.active_ledger_name());

    let header = Row::new(vec![
        Cell::from("Account"),
        Cell::from("Class"),
        Cell::from("Type"),
        Cell::from(Line::from("Balance").alignment(Alignment::Right)),
        Cell::from(Line::from("Transactions").alignment(Alignment::Right)),
        Cell::from("Status"),
    ])
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows: Vec<Row> = app
        .accounts
        .all()
        .iter()
        .map(|account| {
            let style = if account.archived {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            let class_color = match account.class {
                AccountClass::Cash => Color::LightGreen,
                AccountClass::Credit => Color::LightRed,
                AccountClass::Investment => Color::LightMagenta,
            };
            Row::new(vec![
                Cell::from(account.name.clone()),
                Cell::from(account.class.as_str()).style(Style::default().fg(class_color)),
                Cell::from(account.kind.clone()),
                {
                    let balance = app.account_balance(account);
                    let amount = balance_amount(account, balance);
                    let owing = account.class == AccountClass::Credit && balance < Decimal::ZERO;
                    let text = if owing {
                        format!("{} owed", amount)
                    } else {
                        amount
                    };
                    Cell::from(Line::from(text).alignment(Alignment::Right))
                },
                Cell::from(
                    Line::from(app.account_transaction_count(account.id).to_string())
                        .alignment(Alignment::Right),
                ),
                Cell::from(if account.archived {
                    "Archived"
                } else {
                    "Active"
                }),
            ])
            .style(style)
        })
        .collect();

    let row_count = rows.len();
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(28),
            Constraint::Length(12),
            Constraint::Percentage(20),
            Constraint::Length(18),
            Constraint::Length(14),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(Block::default().title(title).borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol("> ");

    clamp_table_scroll(&mut app.account_table_state, row_count, area);
    f.render_stateful_widget(table, area, &mut app.account_table_state);
}

pub fn render_account_editor(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let title = if app.editing_account_id.is_some() {
        " Edit Account "
    } else {
        " Add Account "
    };

    render_field_form(
        f,
        &app.account_fields,
        app.account_cursor,
        popup_area,
        title,
        Some(" [Esc] Cancel, [Enter] Toggle/Save "),
        match AccountClass::from_label(&app.account_fields[AccountField::Class]) {
            Some(AccountClass::Credit) => |field| {
                (field == AccountField::OpeningBalance)
                    .then_some(FieldOverride::Label("Starting Amount Owed"))
            },
            Some(AccountClass::Investment) => |field| {
                matches!(
                    field,
                    AccountField::OpeningBalance | AccountField::BalanceDate
                )
                .then_some(FieldOverride::Placeholder(
                    "Set by valuations in the investments view",
                ))
            },
            _ => |_| None,
        },
    );
}
