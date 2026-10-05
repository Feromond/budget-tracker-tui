use crate::app::state::{App, AppMode};
use ratatui::prelude::*;
use ratatui::widgets::*;

struct Hint {
    key: &'static str,
    label: &'static str,
    short: &'static str,
    color: Color,
    /// 0 always shows. Higher numbers drop first.
    priority: u8,
}

impl Hint {
    const fn new(
        key: &'static str,
        label: &'static str,
        short: &'static str,
        color: Color,
        priority: u8,
    ) -> Self {
        Self {
            key,
            label,
            short,
            color,
            priority,
        }
    }
}

fn hint_spans(hints: &[&Hint], short: bool) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (index, hint) in hints.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" | "));
        }
        spans.push(Span::styled(
            hint.key,
            Style::default().fg(hint.color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(
            " {}",
            if short { hint.short } else { hint.label }
        )));
    }
    spans
}

fn fit_hints(hints: &[Hint], width: usize) -> Vec<Span<'static>> {
    let fits = |spans: &[Span]| spans.iter().map(Span::width).sum::<usize>() <= width;
    let mut shown: Vec<&Hint> = hints.iter().collect();

    let full = hint_spans(&shown, false);
    if fits(&full) {
        return full;
    }
    loop {
        let mut spans = hint_spans(&shown, true);
        let hidden = hints.len() - shown.len();
        if hidden > 0 {
            spans.push(Span::styled(
                format!(" | +{} more", hidden),
                Style::default().fg(Color::DarkGray),
            ));
        }
        let droppable = shown
            .iter()
            .enumerate()
            .filter(|(_, hint)| hint.priority > 0)
            .max_by_key(|(index, hint)| (hint.priority, *index))
            .map(|(index, _)| index);
        match droppable {
            Some(index) if !fits(&spans) => {
                shown.remove(index);
            }
            _ => return spans,
        }
    }
}

pub fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(2) as usize;
    let help_spans = match app.mode {
        AppMode::Normal => fit_hints(
            &[
                Hint::new("↑↓", "Nav", "Nav", Color::White, 6),
                Hint::new("a", "Add", "Add", Color::LightGreen, 1),
                Hint::new("e", "Edit", "Edit", Color::LightYellow, 1),
                Hint::new("d", "Delete", "Del", Color::LightRed, 1),
                Hint::new("r", "Recurring", "Rcr", Color::LightBlue, 5),
                Hint::new("f", "Filter", "Filt", Color::Cyan, 2),
                Hint::new("s", "Summary", "Mth", Color::LightMagenta, 3),
                Hint::new("c", "Cate", "Cate", Color::LightCyan, 4),
                Hint::new("b", "Budget", "Budg", Color::LightYellow, 3),
                Hint::new("i", "Investments", "Inv", Color::LightCyan, 3),
                Hint::new("D", "Debts", "Debt", Color::Yellow, 3),
                Hint::new("A", "Accounts", "Acct", Color::LightBlue, 4),
                Hint::new("q/Esc", "Quit", "Quit", Color::Magenta, 0),
                Hint::new("o", "⚙", "⚙", Color::Red, 0),
            ],
            width,
        ),
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
            Span::styled("Ctrl+N", Style::default().fg(Color::LightRed)),
            Span::raw(" Exclude | "),
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
        AppMode::SelectingCategory
        | AppMode::SelectingSubcategory
        | AppMode::SelectingConversionAccount
        | AppMode::SelectingAccountScope => vec![
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
            Span::styled(
                "1-7",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Sort | "),
            Span::styled("Enter", Style::default().fg(Color::Magenta)),
            Span::raw(" Drill Down | "),
            Span::styled("f", Style::default().fg(Color::LightYellow)),
            Span::raw(" Filter List | "),
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
        AppMode::CategoryCatalog => {
            let mut spans = vec![
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
            ];
            if app.suggests_conversion() {
                spans.push(Span::styled("t", Style::default().fg(Color::LightBlue)));
                spans.push(Span::raw(": To Transfers | "));
            }
            spans.extend([
                Span::styled("1-5", Style::default().fg(Color::LightBlue)),
                Span::raw(": Sort | "),
                Span::styled("q/Esc", Style::default().fg(Color::LightCyan)),
                Span::raw(": Back"),
            ]);
            spans
        }
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
        AppMode::ConfirmCategoryDelete | AppMode::ConfirmCategoryConversion => vec![
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
        AppMode::Debts => fit_hints(
            &[
                Hint::new("↑↓", "Nav", "Nav", Color::White, 6),
                Hint::new("←→", "Extra", "Extra", Color::LightGreen, 2),
                Hint::new("s", "Strategy", "Strat", Color::LightCyan, 2),
                Hint::new("Enter", "Detail", "Detail", Color::LightCyan, 3),
                Hint::new("a", "Add", "Add", Color::LightGreen, 1),
                Hint::new("e", "Edit", "Edit", Color::LightYellow, 1),
                Hint::new("p", "Pay", "Pay", Color::LightGreen, 1),
                Hint::new("r", "Reconcile", "Recon", Color::Yellow, 3),
                Hint::new("d", "Delete", "Del", Color::LightRed, 4),
                Hint::new("A", "Archived", "Arch", Color::LightBlue, 5),
                Hint::new("q/Esc", "Back", "Back", Color::Magenta, 0),
            ],
            width,
        ),
        AppMode::DebtDetail => fit_hints(
            &[
                Hint::new("↑↓", "Scroll", "Rows", Color::White, 6),
                Hint::new("Tab", "History/Schedule", "Panel", Color::LightBlue, 2),
                Hint::new("←→", "Extra", "Extra", Color::LightGreen, 2),
                Hint::new("s", "Strategy", "Strat", Color::LightCyan, 2),
                Hint::new("e", "Edit", "Edit", Color::LightYellow, 1),
                Hint::new("p", "Pay", "Pay", Color::LightGreen, 1),
                Hint::new("r", "Reconcile", "Recon", Color::Yellow, 3),
                Hint::new("q/Esc", "Back", "Back", Color::Magenta, 0),
            ],
            width,
        ),
        AppMode::DebtEditor | AppMode::DebtReconcile => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("←→ Adjust | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(" Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(" Cancel"),
        ],
        AppMode::ConfirmDebtDelete => vec![
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
        AppMode::AccountManager => vec![
            Span::raw("↑↓ Nav | "),
            Span::styled("a", Style::default().fg(Color::LightGreen)),
            Span::raw(": Add | "),
            Span::styled("e/Enter", Style::default().fg(Color::LightYellow)),
            Span::raw(": Edit | "),
            Span::styled("d", Style::default().fg(Color::LightRed)),
            Span::raw(": Delete | "),
            Span::styled("q/Esc", Style::default().fg(Color::LightCyan)),
            Span::raw(": Back"),
        ],
        AppMode::AccountEditor => vec![
            Span::raw("Tab/↑↓ Nav | "),
            Span::raw("←→ Toggle/Cursor | "),
            Span::styled("Enter", Style::default().fg(Color::LightGreen)),
            Span::raw(": Toggle/Save | "),
            Span::styled("Esc", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
        ],
        AppMode::ConfirmAccountDelete => vec![
            Span::styled("y", Style::default().fg(Color::LightGreen)),
            Span::raw(": Confirm | "),
            Span::styled("any other key", Style::default().fg(Color::LightRed)),
            Span::raw(": Cancel"),
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
