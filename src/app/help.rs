use crate::app::state::AppMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBindingInfo {
    pub key: &'static str,
    pub description: &'static str,
    pub group: &'static str,
    pub extended_description: Option<&'static str>,
}

impl KeyBindingInfo {
    pub const fn new(
        key: &'static str,
        description: &'static str,
        group: &'static str,
        extended_description: Option<&'static str>,
    ) -> Self {
        Self {
            key,
            description,
            group,
            extended_description,
        }
    }
}

pub fn get_help_for_mode(mode: AppMode) -> Vec<KeyBindingInfo> {
    match mode {
        AppMode::Normal => vec![
            KeyBindingInfo::new("↑/↓", "Navigate transactions", "Navigation", None),
            KeyBindingInfo::new("PgUp/PgDn", "Scroll page up/down", "Navigation", None),
            KeyBindingInfo::new("Ctrl+Up/Down", "Jump to First/Last", "Navigation", None),
            KeyBindingInfo::new(
                "a",
                "Add new transaction",
                "Actions",
                Some(
                    "Opens the 'Add Transaction' form where you can input details like date, amount, category, etc.",
                ),
            ),
            KeyBindingInfo::new(
                "e",
                "Edit selected transaction",
                "Actions",
                Some("Opens the edit mode for the currently selected transaction."),
            ),
            KeyBindingInfo::new(
                "d",
                "Delete selected transaction",
                "Actions",
                Some("Prompts for confirmation to delete the selected transaction."),
            ),
            KeyBindingInfo::new(
                "Ctrl+C",
                "Copy selected transaction",
                "Actions",
                Some(
                    "Creates a duplicate of the selected transaction with today's date, keeping the same description, category, subcategory, amount, and type.",
                ),
            ),
            KeyBindingInfo::new(
                "r",
                "Manage recurring transactions",
                "Actions",
                Some("Opens the recurring transactions manager."),
            ),
            KeyBindingInfo::new(
                "f",
                "Filter transactions",
                "Actions",
                Some("Enables simple filtering mode. Type to filter by any field."),
            ),
            KeyBindingInfo::new(
                "Ctrl+F",
                "Advanced Filter",
                "Actions",
                Some(
                    "Opens the advanced filter form directly, for filtering by date range, category, type, amount, and more.",
                ),
            ),
            KeyBindingInfo::new(
                "s",
                "Monthly Summary",
                "Actions",
                Some("View a monthly breakdown of income vs expenses."),
            ),
            KeyBindingInfo::new(
                "c",
                "Category Summary",
                "Actions",
                Some("View expenses broken down by category and subcategory."),
            ),
            KeyBindingInfo::new(
                "b",
                "Budget View",
                "Actions",
                Some(
                    "View monthly budget progress and budgeted category spending for the selected month.",
                ),
            ),
            KeyBindingInfo::new(
                "i",
                "Investments",
                "Actions",
                Some(
                    "Track what your investments are worth over time. You record valuations and contributions separately, so growth is always derived rather than typed in.",
                ),
            ),
            KeyBindingInfo::new(
                "o",
                "Settings",
                "Actions",
                Some(
                    "Open application settings: database location, category management, CSV import/export, budget and display preferences.",
                ),
            ),
            KeyBindingInfo::new("1/F1", "Sort by Date", "Sorting", None),
            KeyBindingInfo::new("2/F2", "Sort by Description", "Sorting", None),
            KeyBindingInfo::new("3/F3", "Sort by Category", "Sorting", None),
            KeyBindingInfo::new("4/F4", "Sort by Subcategory", "Sorting", None),
            KeyBindingInfo::new("5/F5", "Sort by Type", "Sorting", None),
            KeyBindingInfo::new("6/F6", "Sort by Amount", "Sorting", None),
            KeyBindingInfo::new(
                "q/Esc",
                "Quit / Clear Filters",
                "System",
                Some("If filters are active, clears them. Otherwise, quits the application."),
            ),
            KeyBindingInfo::new(
                "Ctrl+H",
                "Show Keybindings Help",
                "System",
                Some("Shows this help menu."),
            ),
        ],
        AppMode::Adding | AppMode::Editing => vec![
            KeyBindingInfo::new("Tab/Shift+Tab/↑↓", "Navigate fields", "Navigation", None),
            KeyBindingInfo::new(
                "Date Field",
                "Transaction Date (YYYY-MM-DD)",
                "Fields",
                Some(
                    "Enter the date of the transaction (YYYY-MM-DD). Use Arrow keys to adjust day by day. Hold Shift + Arrow keys to jump by month. Accurate dating helps with monthly filtering.",
                ),
            ),
            KeyBindingInfo::new(
                "Amount Field",
                "Transaction Amount",
                "Fields",
                Some(
                    "Numerical value of the transaction. Positive numbers are standard. The 'Type' field determines if it's income or expense. For expenses, enter the positive cost.",
                ),
            ),
            KeyBindingInfo::new(
                "Category",
                "Main Category",
                "Fields",
                Some(
                    "Primary classification (e.g., 'Food', 'Housing'). Press Enter to open a selection list of categories from the current catalog.",
                ),
            ),
            KeyBindingInfo::new(
                "Subcategory",
                "Sub Category",
                "Fields",
                Some(
                    "More specific classification (e.g., 'Groceries', 'Rent'). Useful for detailed breakdown in the Category Summary view.",
                ),
            ),
            KeyBindingInfo::new(
                "Type",
                "Expense / Income",
                "Fields",
                Some(
                    "Classifies the transaction as 'Expense' or 'Income'. This affects how totals are calculated in summaries. Use Left/Right arrows to toggle.",
                ),
            ),
            KeyBindingInfo::new(
                "Description",
                "Notes / Details",
                "Fields",
                Some(
                    "Optional details about the transaction. Add context that doesn't fit in categories.",
                ),
            ),
            KeyBindingInfo::new(
                "←/→",
                "Toggle type / Adjust date / Move cursor",
                "Input",
                Some(
                    "On the Type field toggles Income/Expense; on the Date field moves the date by one day; on text fields moves the cursor.",
                ),
            ),
            KeyBindingInfo::new(
                "Shift+←/→",
                "Jump month (Date field)",
                "Input",
                Some("In date fields, moves date by one month instead of one day."),
            ),
            KeyBindingInfo::new(
                "+/-",
                "Adjust date",
                "Input",
                Some(
                    "On the Date field, '+' or '=' moves the date forward a day and '-' moves it back.",
                ),
            ),
            KeyBindingInfo::new(
                "Enter",
                "Save / Toggle type / Open selection",
                "Actions",
                Some(
                    "On the Type field toggles Income/Expense; on Category/Subcategory opens a selection list; on any other field saves the transaction.",
                ),
            ),
            KeyBindingInfo::new("Esc", "Cancel", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::ConfirmDelete => vec![
            KeyBindingInfo::new("y", "Confirm deletion", "Actions", None),
            KeyBindingInfo::new("n/Esc", "Cancel deletion", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::Filtering => vec![
            KeyBindingInfo::new("Any Char", "Type filter text", "Input", None),
            KeyBindingInfo::new("Bksp/Del", "Delete character", "Input", None),
            KeyBindingInfo::new("←/→", "Move cursor", "Navigation", None),
            KeyBindingInfo::new(
                "Ctrl+F",
                "Switch to Advanced Filter",
                "Actions",
                Some("Switch to advanced filtering mode for more specific criteria."),
            ),
            KeyBindingInfo::new("Ctrl+R", "Clear filter", "Actions", None),
            KeyBindingInfo::new("Enter", "Apply filter", "Actions", None),
            KeyBindingInfo::new("Esc", "Clear filter", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::AdvancedFiltering => vec![
            KeyBindingInfo::new("Tab/Shift+Tab/↑↓", "Navigate fields", "Navigation", None),
            KeyBindingInfo::new(
                "Start Date",
                "Filter From Date",
                "Fields",
                Some("Include transactions on or after this date."),
            ),
            KeyBindingInfo::new(
                "End Date",
                "Filter To Date",
                "Fields",
                Some("Include transactions on or before this date."),
            ),
            KeyBindingInfo::new(
                "Min Amount",
                "Minimum Amount",
                "Fields",
                Some("Filter transactions with amount greater than or equal to this."),
            ),
            KeyBindingInfo::new(
                "Max Amount",
                "Maximum Amount",
                "Fields",
                Some("Filter transactions with amount less than or equal to this."),
            ),
            KeyBindingInfo::new(
                "Category",
                "Filter Category",
                "Fields",
                Some("Filter by specific category."),
            ),
            KeyBindingInfo::new(
                "Subcategory",
                "Filter Subcategory",
                "Fields",
                Some("Filter by specific subcategory."),
            ),
            KeyBindingInfo::new(
                "Type",
                "Filter Type",
                "Fields",
                Some("Filter by Expense or Income."),
            ),
            KeyBindingInfo::new(
                "Desc",
                "Filter Description",
                "Fields",
                Some("Filter by text in description."),
            ),
            KeyBindingInfo::new(
                "Recurring",
                "Filter Recurring",
                "Fields",
                Some("Show only recurring transactions, or only one-time ones."),
            ),
            KeyBindingInfo::new(
                "←/→",
                "Adjust date / Toggle type / Move cursor",
                "Input",
                Some(
                    "On the Start/End Date fields adjusts by a day; on the Type and Recurring fields toggles the value; on text fields moves the cursor.",
                ),
            ),
            KeyBindingInfo::new("Shift+←/→", "Jump month (date fields)", "Input", None),
            KeyBindingInfo::new("Ctrl+R", "Clear all filters", "Actions", None),
            KeyBindingInfo::new(
                "Enter",
                "Save & Apply / Open selection",
                "Actions",
                Some(
                    "On the Category/Subcategory fields opens a selection list; on any other field saves and applies the filters.",
                ),
            ),
            KeyBindingInfo::new("Esc", "Cancel / Back to Normal Mode", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::Summary => vec![
            KeyBindingInfo::new("↑/↓", "Change Month", "Navigation", None),
            KeyBindingInfo::new("←/→ / [/] / PgUp/PgDn", "Change Year", "Navigation", None),
            KeyBindingInfo::new(
                "m",
                "Toggle Multi-Month View",
                "View",
                Some("Switch between single month view and multi-month view."),
            ),
            KeyBindingInfo::new(
                "c",
                "Toggle Cumulative View",
                "View",
                Some("Toggle cumulative (running total) view."),
            ),
            KeyBindingInfo::new("q/Esc", "Back to Transactions", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::CategorySummary => vec![
            KeyBindingInfo::new("↑/↓", "Select Category/Subcategory", "Navigation", None),
            KeyBindingInfo::new("←/→ / [/]", "Change Year", "Navigation", None),
            KeyBindingInfo::new("PgUp/PgDn", "Jump Selected Month", "Navigation", None),
            KeyBindingInfo::new(
                "Enter",
                "Drill Down One Level",
                "Actions",
                Some(
                    "Expand or collapse a month or subcategory. Transactions appear oldest first. Press Enter on a transaction to open it in the main list with your filters unchanged.",
                ),
            ),
            KeyBindingInfo::new(
                "f",
                "Filter Transaction List",
                "Actions",
                Some(
                    "Open the main transaction list with new filters based on this row. Month rows filter by month; other rows also use category and subcategory. Names match partial text, ignoring case. Blank subcategories and Uncategorized categories leave those fields unrestricted. Summaries use these filters too. Press q in the main list to clear them.",
                ),
            ),
            KeyBindingInfo::new("q/Esc", "Back to Transactions", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::Budget => vec![
            KeyBindingInfo::new("↑/↓", "Select Budget Row", "Navigation", None),
            KeyBindingInfo::new("←/→", "Change Month", "Navigation", None),
            KeyBindingInfo::new("Shift+←/→", "Change Year", "Navigation", None),
            KeyBindingInfo::new(
                "e",
                "Edit Category Budget",
                "Actions",
                Some(
                    "Change the budget of the selected category from the selected month on. Leaving the amount empty clears it from that month, which also drops the category from this table.",
                ),
            ),
            KeyBindingInfo::new(
                "t",
                "Edit Monthly Budget",
                "Actions",
                Some(
                    "Set the whole month's spending limit from the selected month on. Earlier months keep the budget they had, so past history is not rewritten.",
                ),
            ),
            KeyBindingInfo::new(
                "c",
                "Manage Categories",
                "Actions",
                Some(
                    "Open the category catalog to add or edit category budgets without leaving the budget view. Esc returns here.",
                ),
            ),
            KeyBindingInfo::new("q/Esc", "Back to Transactions", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::BudgetCategoryEditor => vec![
            KeyBindingInfo::new("0-9/.", "Type the amount", "Input", None),
            KeyBindingInfo::new("←/→", "Move cursor", "Navigation", None),
            KeyBindingInfo::new("Bksp/Del", "Delete character", "Input", None),
            KeyBindingInfo::new(
                "Tab/Shift+Tab/↑↓",
                "Choose how far the change reaches",
                "Actions",
                Some(
                    "From this month on carries forward until something else changes it. This month only restores the old amount next month. Replace all months wipes the whole history for this budget and applies one amount everywhere, which is the way to undo a mistake rather than date around it. Remove change appears when a change starts in this month, and deletes it so the month inherits the previous amount again.",
                ),
            ),
            KeyBindingInfo::new(
                "Enter",
                "Save budget",
                "Actions",
                Some("An empty amount clears the budget from the selected month on."),
            ),
            KeyBindingInfo::new("Esc", "Cancel", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::Settings => vec![
            KeyBindingInfo::new("Tab/Shift+Tab/↑↓", "Navigate settings", "Navigation", None),
            KeyBindingInfo::new(
                "Database Path",
                "SQLite Database Location",
                "Fields",
                Some(
                    "Path to the SQLite database that stores your transactions and category catalog. Point this at a new file to create a fresh database, or an existing file to load different data. Tip: place it in iCloud Drive, Google Drive, or Dropbox to sync across devices.",
                ),
            ),
            KeyBindingInfo::new(
                "Ledger",
                "Switch or Manage Ledgers",
                "Fields",
                Some(
                    "Shows the ledger currently open. Press Enter to switch to another ledger, or to add, rename, and delete them. Ledgers are independent sets of transactions inside one database, useful for separate accounts or for forecasting without touching your real data. They all share the same category catalog.",
                ),
            ),
            KeyBindingInfo::new(
                "Manage Categories",
                "Open Category Catalog",
                "Fields",
                Some("Select this action and press Enter to open the category catalog manager."),
            ),
            KeyBindingInfo::new(
                "Import Transactions",
                "Import from CSV",
                "Fields",
                Some(
                    "Press Enter to open a path prompt and import a CSV. New rows are added and exact duplicates are skipped; recurring occurrences are regenerated automatically.",
                ),
            ),
            KeyBindingInfo::new(
                "Export Transactions",
                "Export to CSV",
                "Fields",
                Some(
                    "Press Enter to open a path prompt and export all transactions to a CSV file you can back up or share.",
                ),
            ),
            KeyBindingInfo::new(
                "Hourly Rate",
                "Hourly Earning Rate",
                "Fields",
                Some(
                    "Optional. Enter your hourly rate to enable viewing transaction amounts as equivalent hours worked.",
                ),
            ),
            KeyBindingInfo::new(
                "Show Hours",
                "Toggle Hours View",
                "Fields",
                Some(
                    "Enable to display transaction amounts in equivalent hours based on your hourly rate.",
                ),
            ),
            KeyBindingInfo::new(
                "Fuzzy Search",
                "Toggle Fuzzy Search",
                "Fields",
                Some(
                    "Toggle to enable fuzzy searching for categories/subcategories. When enabled, selecting 'Category' opens a search bar to filter both category and subcategory at once.",
                ),
            ),
            KeyBindingInfo::new(
                "Hide Help Bar",
                "Toggle Help Bar",
                "Fields",
                Some(
                    "Toggle to hide the bottom help bar for a cleaner interface. Ctrl+H will still work. NOT RECOMMENDED unless you know the keybindings well.",
                ),
            ),
            KeyBindingInfo::new(
                "←/→",
                "Toggle Options / Move cursor",
                "Navigation",
                Some(
                    "On toggle settings changes the value (e.g. Yes/No); on path/number fields moves the text cursor.",
                ),
            ),
            KeyBindingInfo::new(
                "Ctrl+D",
                "Reset Database Path",
                "Actions",
                Some(
                    "On the Database Path field, resets it to the default location for the current data directory.",
                ),
            ),
            KeyBindingInfo::new("Ctrl+U", "Clear Field", "Actions", None),
            KeyBindingInfo::new(
                "Enter",
                "Save Settings / Activate Action",
                "Actions",
                Some(
                    "On editable fields saves your settings; on action rows (Manage Categories, Import, Export) opens that action.",
                ),
            ),
            KeyBindingInfo::new("Esc", "Cancel / Back", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::CategoryCatalog => vec![
            KeyBindingInfo::new("↑/↓", "Navigate categories", "Navigation", None),
            KeyBindingInfo::new("PgUp/PgDn", "Scroll page up/down", "Navigation", None),
            KeyBindingInfo::new("Ctrl+Up/Down", "Jump to First/Last", "Navigation", None),
            KeyBindingInfo::new(
                "f",
                "Filter categories",
                "Actions",
                Some(
                    "Opens a filter bar. Type to narrow the catalog by type, category, subcategory, or tag as you type.",
                ),
            ),
            KeyBindingInfo::new("a", "Add category", "Actions", None),
            KeyBindingInfo::new("e/Enter", "Edit selected category", "Actions", None),
            KeyBindingInfo::new("d", "Delete selected category", "Actions", None),
            KeyBindingInfo::new(
                "b",
                "Edit budget",
                "Actions",
                Some(
                    "Opens the same budget popup the budget view uses, dated from the current month. Expense categories only.",
                ),
            ),
            KeyBindingInfo::new("1/F1", "Sort by Type", "Sorting", None),
            KeyBindingInfo::new("2/F2", "Sort by Category", "Sorting", None),
            KeyBindingInfo::new("3/F3", "Sort by Subcategory", "Sorting", None),
            KeyBindingInfo::new(
                "4/F4",
                "Sort by Tag",
                "Sorting",
                Some("Categories without a tag are listed last in both directions."),
            ),
            KeyBindingInfo::new(
                "5/F5",
                "Sort by Budget",
                "Sorting",
                Some(
                    "Groups the categories that have a budget set at the top; categories without one are listed last in both directions.",
                ),
            ),
            KeyBindingInfo::new(
                "q/Esc",
                "Back to Previous View / Clear Filter",
                "Actions",
                Some("If a filter is active, clears it. Otherwise, returns to the previous view."),
            ),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::CategoryCatalogFilter => vec![
            KeyBindingInfo::new("Any Char", "Type filter text", "Input", None),
            KeyBindingInfo::new("Bksp/Del", "Delete character", "Input", None),
            KeyBindingInfo::new("←/→", "Move cursor", "Navigation", None),
            KeyBindingInfo::new(
                "↑/↓",
                "Navigate categories",
                "Navigation",
                Some("Move through the filtered list without leaving the filter bar."),
            ),
            KeyBindingInfo::new("Ctrl+R", "Clear filter", "Actions", None),
            KeyBindingInfo::new("Enter", "Apply filter", "Actions", None),
            KeyBindingInfo::new("Esc", "Clear filter", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::CategoryEditor => vec![
            KeyBindingInfo::new("Tab/Shift+Tab/↑↓", "Navigate fields", "Navigation", None),
            KeyBindingInfo::new("←/→", "Toggle type / move cursor", "Navigation", None),
            KeyBindingInfo::new("Enter", "Toggle type or save", "Actions", None),
            KeyBindingInfo::new(
                "Budget",
                "Save, then open the budget popup",
                "Fields",
                Some(
                    "A budget is stored against the category rather than inside it, so pressing Enter here saves the category first and then opens the same popup the budget view uses. Expense categories only.",
                ),
            ),
            KeyBindingInfo::new("Esc", "Cancel editor", "Actions", None),
            KeyBindingInfo::new(
                "Tag",
                "Optional label",
                "Fields",
                Some("Optional. Store an extra short label for the category row."),
            ),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::ConfirmCategoryDelete => vec![
            KeyBindingInfo::new("y", "Confirm delete", "Actions", None),
            KeyBindingInfo::new("n/Esc", "Cancel delete", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::Investments => vec![
            KeyBindingInfo::new("↑/↓", "Navigate accounts", "Navigation", None),
            KeyBindingInfo::new(
                "←/→",
                "Change time range",
                "Navigation",
                Some(
                    "Cycles the window every figure is measured over: YTD, 1Y, 3Y, 5Y, All. The chart, the period gain, and the annualized return all follow it.",
                ),
            ),
            KeyBindingInfo::new(
                "Enter",
                "Open account detail",
                "Actions",
                Some(
                    "Shows the account's full entry history, its own growth chart, and a year-by-year breakdown of what you contributed versus what it earned.",
                ),
            ),
            KeyBindingInfo::new(
                "v",
                "Record a valuation",
                "Actions",
                Some(
                    "Records what the account is worth today. This is the one to press whenever you check on your investments. Growth is worked out from the change since your last valuation, minus anything you paid in. To record money in or out instead, open the account with Enter and press 'a'.",
                ),
            ),
            KeyBindingInfo::new(
                "a",
                "Add investment account",
                "Actions",
                Some(
                    "Creates an account. If it already exists in real life, give it a starting value and how much of that you contributed, so growth is measured from the right place rather than counting your whole balance as a gain.",
                ),
            ),
            KeyBindingInfo::new("e", "Edit selected account", "Actions", None),
            KeyBindingInfo::new(
                "d",
                "Delete selected account",
                "Actions",
                Some("Permanently deletes the account and its entire history, after confirmation."),
            ),
            KeyBindingInfo::new(
                "A",
                "Show/hide archived accounts",
                "Actions",
                Some(
                    "Archived accounts keep their history so past performance stays correct, but stay out of the table and the totals until you show them.",
                ),
            ),
            KeyBindingInfo::new("q/Esc", "Back to transactions", "System", None),
        ],
        AppMode::InvestmentDetail => vec![
            KeyBindingInfo::new("↑/↓", "Navigate entries", "Navigation", None),
            KeyBindingInfo::new("←/→", "Change time range", "Navigation", None),
            KeyBindingInfo::new(
                "v",
                "Record a valuation",
                "Actions",
                Some("Shortcut for adding an entry with the type already set to Valuation."),
            ),
            KeyBindingInfo::new(
                "a",
                "Add an entry",
                "Actions",
                Some(
                    "Opens the entry form, set to Contribution. Use the Entry field to switch it to a withdrawal or a valuation.",
                ),
            ),
            KeyBindingInfo::new("e/Enter", "Edit selected entry", "Actions", None),
            KeyBindingInfo::new("d", "Delete selected entry", "Actions", None),
            KeyBindingInfo::new("q/Esc", "Back to the accounts list", "System", None),
        ],
        AppMode::InvestmentAccountEditor => vec![
            KeyBindingInfo::new(
                "Tab/Shift+Tab/↑↓",
                "Move between fields",
                "Navigation",
                None,
            ),
            KeyBindingInfo::new("←/→", "Move cursor, or toggle status", "Navigation", None),
            KeyBindingInfo::new(
                "Starting Value",
                "What the account is worth now",
                "Fields",
                Some(
                    "Optional, and only offered when creating an account. Writes an opening valuation so an account you already held does not report its whole balance as a gain.",
                ),
            ),
            KeyBindingInfo::new(
                "Contributed So Far",
                "Your cost basis",
                "Fields",
                Some(
                    "Optional, defaults to the starting value. Set it lower when part of the balance is already growth you earned before you started tracking.",
                ),
            ),
            KeyBindingInfo::new("Enter", "Save account", "Actions", None),
            KeyBindingInfo::new("Esc", "Cancel", "System", None),
        ],
        AppMode::InvestmentEntryEditor => vec![
            KeyBindingInfo::new(
                "Tab/Shift+Tab/↑↓",
                "Move between fields",
                "Navigation",
                None,
            ),
            KeyBindingInfo::new(
                "←/→",
                "Move cursor, step the date, or toggle the entry",
                "Navigation",
                None,
            ),
            KeyBindingInfo::new("Shift+←/→", "Step the date by month", "Navigation", None),
            KeyBindingInfo::new(
                "Entry",
                "Valuation, Contribution or Withdrawal",
                "Fields",
                Some(
                    "A valuation records what the account is worth. A contribution or withdrawal records money crossing the boundary. Keeping them apart is what lets growth be derived rather than guessed.",
                ),
            ),
            KeyBindingInfo::new("Enter", "Save entry", "Actions", None),
            KeyBindingInfo::new("Esc", "Cancel", "System", None),
        ],
        AppMode::ConfirmInvestmentDelete => vec![
            KeyBindingInfo::new("y", "Confirm deletion", "Actions", None),
            KeyBindingInfo::new("n/Esc", "Cancel deletion", "Actions", None),
        ],
        AppMode::LedgerManager => vec![
            KeyBindingInfo::new("↑/↓", "Navigate ledgers", "Navigation", None),
            KeyBindingInfo::new(
                "Enter",
                "Switch to selected ledger",
                "Actions",
                Some(
                    "Opens the selected ledger. The transaction table, summaries, and budget view all follow the ledger that is open; the category catalog is shared by every ledger.",
                ),
            ),
            KeyBindingInfo::new(
                "a",
                "Add ledger",
                "Actions",
                Some("Creates a new, empty ledger in the same database file."),
            ),
            KeyBindingInfo::new("e", "Rename selected ledger", "Actions", None),
            KeyBindingInfo::new(
                "Ctrl+C",
                "Copy selected ledger",
                "Actions",
                Some(
                    "Creates a new ledger holding a copy of every transaction in the selected one, so you can experiment or forecast without touching the original.",
                ),
            ),
            KeyBindingInfo::new(
                "d",
                "Delete selected ledger",
                "Actions",
                Some(
                    "Permanently deletes the ledger and every transaction in it, after confirmation. The last remaining ledger cannot be deleted.",
                ),
            ),
            KeyBindingInfo::new("q/Esc", "Back to Settings", "System", None),
        ],
        AppMode::LedgerEditor => vec![
            KeyBindingInfo::new("←/→", "Move cursor", "Navigation", None),
            KeyBindingInfo::new("Enter", "Save ledger name", "Actions", None),
            KeyBindingInfo::new("Esc", "Cancel", "System", None),
        ],
        AppMode::ConfirmLedgerDelete => vec![
            KeyBindingInfo::new("y", "Confirm deletion", "Actions", None),
            KeyBindingInfo::new("n/Esc", "Cancel", "Actions", None),
        ],
        AppMode::BackupManager => vec![
            KeyBindingInfo::new("↑/↓", "Navigate backups", "Navigation", None),
            KeyBindingInfo::new(
                "Enter",
                "Restore the selected backup",
                "Actions",
                Some(
                    "Asks for confirmation, then saves a 'Before restore' backup of the current database before replacing it.",
                ),
            ),
            KeyBindingInfo::new(
                "b",
                "Back up now",
                "Actions",
                Some("Creates a manual snapshot, kept until you delete it."),
            ),
            KeyBindingInfo::new(
                "d",
                "Delete the selected backup",
                "Actions",
                Some("Asks for confirmation, then permanently deletes the snapshot file."),
            ),
            KeyBindingInfo::new(
                "Kinds",
                "Automatic, Manual, Before upgrade, Before restore",
                "Reference",
                Some(
                    "When enabled, startup takes a daily snapshot, or a pre-upgrade snapshot if needed. Older daily snapshots are cleaned up using the limit in Settings. Other kinds stay until you delete them.",
                ),
            ),
            KeyBindingInfo::new(
                "Device",
                "Which install wrote the backup",
                "Reference",
                Some(
                    "Backups are stored beside the database and appear here if synced from another machine. The Device column matches IDs from config.json, not the hardware. Automatic cleanup only removes snapshots with the current ID; copying config.json shares that ID.",
                ),
            ),
            KeyBindingInfo::new("q/Esc", "Back to Settings", "System", None),
        ],
        AppMode::ConfirmBackupRestore | AppMode::ConfirmBackupDelete => vec![
            KeyBindingInfo::new("y", "Confirm", "Actions", None),
            KeyBindingInfo::new("n/Esc", "Cancel", "Actions", None),
        ],
        AppMode::RecurringSettings => vec![
            KeyBindingInfo::new("Tab/Shift+Tab/↑↓", "Navigate fields", "Navigation", None),
            KeyBindingInfo::new(
                "Active",
                "Enable/Disable",
                "Fields",
                Some("Toggle if this transaction will be recurring."),
            ),
            KeyBindingInfo::new(
                "Frequency",
                "Recurrence Interval",
                "Fields",
                Some(
                    "Determines the interval for the transaction. Options include 'Daily', 'Weekly', 'Bi-Weekly', 'Semi-Monthly' (15th and last day), 'Semi-Monthly (Weekday Adjusted)' (15th and last day, moved earlier to the nearest weekday when either falls on a weekend), 'Monthly', 'Quarterly', and 'Yearly'. The app will automatically generate these transactions up to the current date when you open it.",
                ),
            ),
            KeyBindingInfo::new(
                "End Date",
                "Stop Date",
                "Fields",
                Some(
                    "Optional. If set, the recurring transaction will stop being generated after this date. Leave empty for indefinite recurrence.",
                ),
            ),
            KeyBindingInfo::new(
                "←/→",
                "Toggle active / Adjust end date",
                "Input",
                Some(
                    "On the Active field toggles recurrence on/off; on the End Date field moves the date by one day.",
                ),
            ),
            KeyBindingInfo::new("Shift+←/→", "Jump month (End Date)", "Input", None),
            KeyBindingInfo::new(
                "Enter",
                "Select frequency / Save",
                "Actions",
                Some(
                    "On the Frequency field opens the frequency picker; on any other field saves the recurring settings.",
                ),
            ),
            KeyBindingInfo::new("Esc", "Cancel", "Actions", None),
            KeyBindingInfo::new(
                "Tip!",
                "Your recurring transaction changes",
                "Info",
                Some(
                    "The recurring transactions generate from the initial transaction up to the current date. If a recurring transaction needs to be changed, set an end date and then create a new transaction with the updated details. That can then be set as recurring again to continue.",
                ),
            ),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::SelectingCategory
        | AppMode::SelectingSubcategory
        | AppMode::SelectingFilterCategory
        | AppMode::SelectingFilterSubcategory
        | AppMode::SelectingRecurrenceFrequency => vec![
            KeyBindingInfo::new("↑/↓", "Navigate options", "Navigation", None),
            KeyBindingInfo::new("Enter", "Confirm Selection", "Actions", None),
            KeyBindingInfo::new("Esc", "Cancel Selection", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::FuzzyFinding => vec![
            KeyBindingInfo::new(
                "Any Char",
                "Type to search",
                "Input",
                Some("Type to filter the list of categories and subcategories."),
            ),
            KeyBindingInfo::new("↑/↓", "Navigate results", "Navigation", None),
            KeyBindingInfo::new(
                "Enter",
                "Select Category",
                "Actions",
                Some("Confirm selection. Auto-fills both Category and Subcategory fields."),
            ),
            KeyBindingInfo::new("Esc", "Cancel", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        AppMode::ImportTransactions | AppMode::ExportTransactions => vec![
            KeyBindingInfo::new(
                "Any Char",
                "Type the file path",
                "Input",
                Some("Type or paste an absolute path to the CSV file."),
            ),
            KeyBindingInfo::new("←/→", "Move cursor", "Navigation", None),
            KeyBindingInfo::new("Ctrl+U", "Clear path", "Actions", None),
            KeyBindingInfo::new("Ctrl+D", "Reset to default location", "Actions", None),
            KeyBindingInfo::new("Enter", "Confirm import/export", "Actions", None),
            KeyBindingInfo::new("Esc", "Cancel / back to Settings", "Actions", None),
            KeyBindingInfo::new("Ctrl+H", "Show Keybindings Help", "System", None),
        ],
        _ => vec![
            KeyBindingInfo::new("Ctrl+H", "Close Help", "System", None),
            KeyBindingInfo::new("Esc", "Close Help", "System", None),
        ],
    }
}
