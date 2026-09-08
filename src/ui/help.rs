use crate::app::state::{App, AppMode};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let help_spans = match app.mode {
        AppMode::Normal => vec![
            Span::styled("↑↓ Nav | ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                "a",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Add | "),
            Span::styled(
                "e",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Edit | "),
            Span::styled(
                "d",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Del | "),
            Span::styled(
                "r",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Rcr | "),
            Span::styled(
                "f",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Filt | "),
            Span::styled(
                "s",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Mth | "),
            Span::styled(
                "c",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cate | "),
            Span::styled(
                "b",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Budg | "),
            Span::styled(
                "i",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Inv | "),
            Span::styled(
                "1-6",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Srt | "),
            Span::styled(
                "q/Esc",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit | "),
            Span::styled("o", Style::default().fg(Color::Red)).add_modifier(Modifier::BOLD),
            Span::raw(" ⚙"),
        ],
        AppMode::Adding | AppMode::Editing => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("←→ Toggle | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Save/Select | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Cancel"),
        ],
        AppMode::ConfirmDelete => vec![
            Span::styled("y", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("n/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::Filtering => vec![
            Span::raw("← → Cursor | "),
            Span::raw("Bksp/Del Edit | "),
            Span::styled("Ctrl+F", Style::default().fg(Color::Red)),
            Span::raw(" Adv Filt | "),
            Span::styled("Ctrl+R", Style::default().fg(Color::LightYellow)),
            Span::raw(" Clear | "),
            Span::styled("Enter/Esc", Style::default().fg(Color::LightGreen)),
            Span::raw(" Apply/Exit"),
        ],
        AppMode::AdvancedFiltering => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("← → Adjust | "),
            Span::styled("Ctrl+R", Style::default().fg(Color::LightYellow)),
            Span::raw(" Clear | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Cancel"),
        ],
        AppMode::SelectingFilterCategory | AppMode::SelectingFilterSubcategory => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::SelectingCategory | AppMode::SelectingSubcategory => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::Summary => vec![
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Month | "),
            Span::styled(
                "←→",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("/"),
            Span::styled(
                "[]",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Year | "),
            Span::styled("m", Style::default().fg(Color::LightBlue)),
            Span::raw(" Multi | "),
            Span::styled("c", Style::default().fg(Color::LightYellow)),
            Span::raw(" Cumu | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Back"),
        ],
        AppMode::CategorySummary => vec![
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Nav | "),
            Span::styled(
                "←→",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("/"),
            Span::styled(
                "[]",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Year | "),
            Span::styled(
                "PgUp/PgDn",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Month Jump | "),
            Span::styled("Enter", Style::default().fg(Color::Magenta)),
            Span::raw(" ▶ Expand ▼ Collapse | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Back"),
        ],
        AppMode::Budget => vec![
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Rows | "),
            Span::styled(
                "←→",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Month | "),
            Span::styled(
                "Shift+←→",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Year | "),
            Span::styled("e", Style::default().fg(Color::LightYellow)),
            Span::raw(" Edit Budget | "),
            Span::styled("t", Style::default().fg(Color::LightMagenta)),
            Span::raw(" Edit Monthly | "),
            Span::styled("c", Style::default().fg(Color::LightGreen)),
            Span::raw(" Edit Categories | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Back"),
        ],
        AppMode::BudgetCategoryEditor => vec![
            Span::raw("Type amount | "),
            Span::raw("←→ Cursor | "),
            Span::styled("↑↓", Style::default().fg(Color::LightBlue)),
            Span::raw(" Scope | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Cancel"),
        ],
        AppMode::Settings => vec![
            Span::styled(
                "Tab/↑↓",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(": Nav | "),
            Span::styled(
                "←/→",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(": Cursor | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel | "),
            Span::styled("Ctrl+D", Style::default().fg(Color::LightMagenta)),
            Span::raw(": Reset | "),
            Span::styled("Ctrl+U", Style::default().fg(Color::LightMagenta)),
            Span::raw(": Clear"),
        ],
        AppMode::CategoryCatalog => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("f", Style::default().fg(Color::Cyan)),
            Span::raw(": Filter | "),
            Span::styled("a", Style::default().fg(Color::LightGreen)),
            Span::raw(": Add | "),
            Span::styled("e/Enter", Style::default().fg(Color::LightYellow)),
            Span::raw(": Edit | "),
            Span::styled("d", Style::default().fg(Color::LightRed)),
            Span::raw(": Delete | "),
            Span::styled("b", Style::default().fg(Color::LightMagenta)),
            Span::raw(": Budget | "),
            Span::styled("1-5", Style::default().fg(Color::LightBlue)),
            Span::raw(": Sort | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightCyan)),
            Span::raw(": Back"),
        ],
        AppMode::CategoryCatalogFilter => vec![
            Span::raw("Type to Filter | "),
            Span::raw("↑↓ Nav | "),
            Span::styled("Ctrl+R", Style::default().fg(Color::LightYellow)),
            Span::raw(" Clear | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Apply | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Clear & Back"),
        ],
        AppMode::CategoryEditor => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("←→ Type/Cursor | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Toggle/Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::ConfirmCategoryDelete => vec![
            Span::styled("y", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("n/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::Investments => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("←→", Style::default().fg(Color::Magenta)),
            Span::raw(" Range | "),
            Span::styled("Enter", Style::default().fg(Color::LightCyan)),
            Span::raw(" Detail | "),
            Span::styled("v", Style::default().fg(Color::LightCyan)),
            Span::raw(" Value | "),
            Span::styled("a", Style::default().fg(Color::LightGreen)),
            Span::raw(" Add | "),
            Span::styled("e", Style::default().fg(Color::LightYellow)),
            Span::raw(" Edit | "),
            Span::styled("d", Style::default().fg(Color::LightRed)),
            Span::raw(" Del | "),
            Span::styled("q/Esc", Style::default().fg(Color::Magenta)),
            Span::raw(" Back"),
        ],
        AppMode::InvestmentDetail => vec![
            Span::raw("↑↓ Entries | "),
            Span::styled("←→", Style::default().fg(Color::Magenta)),
            Span::raw(" Range | "),
            Span::styled("v", Style::default().fg(Color::LightCyan)),
            Span::raw(" Value | "),
            Span::styled("a", Style::default().fg(Color::LightGreen)),
            Span::raw(" Add | "),
            Span::styled("e/Enter", Style::default().fg(Color::LightYellow)),
            Span::raw(" Edit | "),
            Span::styled("d", Style::default().fg(Color::LightRed)),
            Span::raw(" Del | "),
            Span::styled("q/Esc", Style::default().fg(Color::Magenta)),
            Span::raw(" Back"),
        ],
        AppMode::InvestmentAccountEditor | AppMode::InvestmentEntryEditor => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("←→ Adjust | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Toggle/Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Cancel"),
        ],
        AppMode::ConfirmInvestmentDelete => vec![
            Span::styled("y", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("n/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::LedgerManager => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Switch | "),
            Span::styled("a", Style::default().fg(Color::LightGreen)),
            Span::raw(": Add | "),
            Span::styled("e", Style::default().fg(Color::LightYellow)),
            Span::raw(": Rename | "),
            Span::styled("Ctrl+C", Style::default().fg(Color::LightBlue)),
            Span::raw(": Copy | "),
            Span::styled("d", Style::default().fg(Color::LightRed)),
            Span::raw(": Delete | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightCyan)),
            Span::raw(": Back"),
        ],
        AppMode::LedgerEditor => vec![
            Span::raw("Type a name | "),
            Span::raw("←→ Cursor | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::BackupManager => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Restore | "),
            Span::styled("b", Style::default().fg(Color::LightBlue)),
            Span::raw(": Back up now | "),
            Span::styled("d", Style::default().fg(Color::LightRed)),
            Span::raw(": Delete | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightCyan)),
            Span::raw(": Back"),
        ],
        AppMode::ConfirmBackupRestore | AppMode::ConfirmBackupDelete => vec![
            Span::styled("y", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("n/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::ConfirmLedgerDelete => vec![
            Span::styled("y", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("n/Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::RecurringSettings => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("←→ Toggle/Date | "),
            Span::raw("Shift+←→ Month | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Select/Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Cancel"),
        ],
        AppMode::SelectingRecurrenceFrequency => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::KeybindingsInfo | AppMode::KeybindingDetail => vec![
            Span::styled("Esc/q/Ctrl+H", Style::default().fg(Color::LightRed)),
            Span::raw(": Close Help | "),
            Span::raw("↑↓/PgUp/PgDn: Scroll/Select | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Details"),
        ],
        AppMode::FuzzyFinding => vec![
            Span::raw("Type to Search | "),
            Span::raw("↑↓ Nav | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Select | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::ImportTransactions | AppMode::ExportTransactions => vec![
            Span::raw("Type path | "),
            Span::raw("←→ Cursor | "),
            Span::styled("Ctrl+U", Style::default().fg(Color::LightMagenta)),
            Span::raw(" Clear | "),
            Span::styled("Ctrl+D", Style::default().fg(Color::LightMagenta)),
            Span::raw(" Default | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Confirm | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Back"),
        ],
    };

    let help_paragraph = Paragraph::new(Line::from(help_spans))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from(vec![
                    Span::styled("Help - ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        "Ctrl+H",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " For More Info",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ])),
        );
    f.render_widget(help_paragraph, area);
}
