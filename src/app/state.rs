use crate::app::fields::{
    AddEditField, AdvancedFilterField, CategoryEditField, FieldSet, InvestmentAccountField,
    InvestmentEntryField, RecurringField, SelectingField,
};
use crate::app::update_checker;
use crate::config::{AppSettings, load_settings, save_settings};
use crate::csv_io::{load_seed_categories, load_transactions};
use crate::db::budget_store::{BudgetStore, SqliteBudgetStore};
use crate::db::category_store::{CategoryStore, SqliteCategoryStore};
use crate::db::database::{SCHEMA_VERSION, SqliteDatabase};
use crate::db::investment_store::{InvestmentStore, SqliteInvestmentStore};
use crate::db::ledger_store::{DEFAULT_LEDGER_ID, LedgerRecord, LedgerStore, SqliteLedgerStore};
use crate::db::transaction_store::{SqliteTransactionStore, TransactionStore};
use crate::model::*;
use chrono::{Datelike, Duration, NaiveDate};
use ratatui::widgets::{ListState, TableState};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::fs::{copy, create_dir_all};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

pub(crate) enum DateUnit {
    Day,
    Month,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum AppMode {
    Normal,
    Adding,
    Editing,
    ConfirmDelete,
    Filtering,
    AdvancedFiltering,
    SelectingFilterCategory,
    SelectingFilterSubcategory,
    Summary,
    SelectingCategory,
    SelectingSubcategory,
    CategorySummary,
    Budget,
    BudgetCategoryEditor,
    Settings,
    RecurringSettings,
    SelectingRecurrenceFrequency,
    KeybindingsInfo,
    KeybindingDetail,
    FuzzyFinding,
    CategoryCatalog,
    CategoryCatalogFilter,
    CategoryEditor,
    ConfirmCategoryDelete,
    ImportTransactions,
    ExportTransactions,
    LedgerManager,
    LedgerEditor,
    ConfirmLedgerDelete,
    Investments,
    InvestmentDetail,
    InvestmentAccountEditor,
    InvestmentEntryEditor,
    ConfirmInvestmentDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvestmentDeleteTarget {
    Account(i64),
    Entry(i64),
}

#[derive(Debug)]
pub enum CategorySummaryItem {
    Month(u32, MonthlySummary),
    Subcategory(u32, String, String, MonthlySummary),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetEditTarget {
    MonthlyBudget,
    Category(i64),
}

impl BudgetEditTarget {
    /// The `category_id` this maps to in `budget_periods`, where NULL is the monthly budget.
    pub fn category_id(self) -> Option<i64> {
        match self {
            BudgetEditTarget::MonthlyBudget => None,
            BudgetEditTarget::Category(id) => Some(id),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BudgetCategoryComparison {
    pub id: i64,
    pub category: String,
    pub subcategory: String,
    pub budget: Decimal,
    pub actual_expense: Decimal,
}

pub struct App {
    pub(crate) transactions: Vec<Transaction>,
    pub(crate) filtered_indices: Vec<usize>,
    pub(crate) categories: Vec<CategoryInfo>,
    pub(crate) category_records: Vec<CategoryRecord>,
    pub(crate) ledgers: Vec<LedgerRecord>,
    pub(crate) active_ledger_id: i64,
    pub(crate) data_file_path: PathBuf,
    pub(crate) database_path: PathBuf,
    pub(crate) should_quit: bool,
    pub(crate) table_state: TableState,
    pub(crate) mode: AppMode,
    pub(crate) simple_filter_content: String,
    pub(crate) simple_filter_cursor: usize,
    pub(crate) add_edit_fields: FieldSet<AddEditField, 6>,
    pub(crate) add_edit_cursor: usize,
    pub(crate) advanced_filter_fields: FieldSet<AdvancedFilterField, 9>,
    pub(crate) advanced_filter_cursor: usize,
    pub(crate) delete_index: Option<usize>,
    pub(crate) editing_index: Option<usize>,
    pub(crate) status_message: Option<String>,
    pub(crate) status_expiry: Option<std::time::Instant>,
    pub(crate) sort_by: SortColumn,
    pub(crate) sort_order: SortOrder,
    // Monthly Summary State
    pub(crate) monthly_summaries: HashMap<(i32, u32), MonthlySummary>,
    pub(crate) summary_years: Vec<i32>,
    pub(crate) selected_summary_year_index: usize,
    pub(crate) selected_summary_month: Option<u32>,
    pub(crate) summary_multi_month_mode: bool,
    pub(crate) summary_cumulative_mode: bool,
    // Category/Subcategory Selection Popup State
    pub(crate) selecting_field: Option<SelectingField>,
    pub(crate) current_selection_list: Vec<String>,
    pub(crate) selection_list_state: ListState,
    pub(crate) type_to_select: crate::app::util::TypeToSelect,
    // Category Summary State
    pub(crate) category_summary_table_state: TableState,
    pub(crate) category_summaries: HashMap<(i32, u32), HashMap<(String, String), MonthlySummary>>,
    pub(crate) category_summary_years: Vec<i32>,
    pub(crate) category_summary_year_index: usize,
    // Expansion state for hierarchical category summary
    pub(crate) expanded_category_summary_months: HashSet<u32>,
    // Flattened list of visible items for rendering and navigation
    pub(crate) cached_visible_category_items: Vec<CategorySummaryItem>,
    // Budget view state
    pub(crate) budget_years: Vec<i32>,
    pub(crate) budget_year_index: usize,
    pub(crate) selected_budget_month: Option<u32>,
    pub(crate) budget_table_state: TableState,
    pub(crate) budget_schedule: BudgetSchedule,
    pub(crate) budget_edit_target: Option<BudgetEditTarget>,
    pub(crate) budget_edit_month: Option<BudgetMonth>,
    pub(crate) budget_edit_origin: AppMode,
    pub(crate) budget_edit_scope_choice: BudgetEditScope,
    pub(crate) budget_edit_input: String,
    pub(crate) budget_edit_cursor: usize,
    // Settings form state
    pub(crate) settings_state: crate::app::settings_types::SettingsState,
    // Category catalog state
    pub(crate) category_table_state: TableState,
    pub(crate) filtered_category_indices: Vec<usize>,
    pub(crate) category_filter_query: String,
    pub(crate) category_filter_cursor: usize,
    pub(crate) category_sort_by: CategorySortColumn,
    pub(crate) category_sort_order: SortOrder,
    pub(crate) category_edit_fields: FieldSet<CategoryEditField, 5>,
    pub(crate) category_edit_cursor: usize,
    pub(crate) editing_category_id: Option<i64>,
    pub(crate) category_delete_id: Option<i64>,
    // Mode to return to when leaving the category catalog (Settings or Budget)
    pub(crate) category_catalog_origin: AppMode,
    // Ledger manager state
    pub(crate) ledger_table_state: TableState,
    pub(crate) ledger_name_input: String,
    pub(crate) ledger_name_cursor: usize,
    pub(crate) editing_ledger_id: Option<i64>,
    pub(crate) ledger_delete_id: Option<i64>,
    pub(crate) ledger_delete_prompt: String,
    pub(crate) ledger_copy_source_id: Option<i64>,
    // Investments
    pub(crate) portfolio: Portfolio,
    pub(crate) investment_table_state: TableState,
    pub(crate) investment_entry_table_state: TableState,
    pub(crate) investment_range: InvestmentRange,
    pub(crate) show_archived_investments: bool,
    pub(crate) investment_detail_id: Option<i64>,
    pub(crate) investment_account_fields: FieldSet<InvestmentAccountField, 6>,
    pub(crate) investment_account_cursor: usize,
    pub(crate) editing_investment_account_id: Option<i64>,
    pub(crate) investment_entry_fields: FieldSet<InvestmentEntryField, 4>,
    pub(crate) investment_entry_cursor: usize,
    pub(crate) editing_investment_entry_id: Option<i64>,
    pub(crate) investment_delete_target: Option<InvestmentDeleteTarget>,
    pub(crate) investment_delete_prompt: String,
    // Budget
    pub(crate) hourly_rate: Option<Decimal>,
    pub(crate) show_hours: bool,
    pub(crate) fuzzy_search_mode: bool,
    pub(crate) search_query: String,
    // Recurring transaction state
    pub(crate) recurring_settings_fields: FieldSet<RecurringField, 3>,
    pub(crate) recurring_transaction_index: Option<usize>,
    // Months past today that recurring occurrences are projected; 0 stops at today.
    pub(crate) recurring_forecast_months: u32,
    // Import/Export path prompt state (shared by ImportTransactions/ExportTransactions modes)
    pub(crate) io_path_input: String,
    pub(crate) io_path_cursor: usize,
    // Help/Keybindings
    pub(crate) previous_mode: Option<AppMode>,
    pub(crate) help_table_state: TableState,
    pub(crate) hide_help_bar: bool,
    // Update Check
    pub(crate) update_available_version: Option<String>,
    pub(crate) show_update_popup: bool,
    pub(crate) update_rx: mpsc::Receiver<Option<String>>,
}

impl App {
    pub fn new() -> Self {
        // --- Start Update Check ---
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = update_checker::check_for_updates();
            let _ = tx.send(result);
        });

        // --- Load Settings ---
        let (loaded_settings, load_settings_error_msg) = match load_settings() {
            Ok(settings) => (settings, None),
            Err(e) => (
                AppSettings::default(),
                Some(format!("Config Load Error: {}. Using defaults.", e)),
            ),
        };

        let (initial_data_file_path, data_path_error_msg) = Self::resolve_configured_path(
            loaded_settings.data_file_path.clone(),
            Self::get_default_data_file_path,
            "transactions.csv",
            "Data file",
        );
        let (initial_database_path, database_path_error_msg) =
            match loaded_settings.database_path.clone() {
                Some(path) => Self::resolve_configured_path(
                    Some(path),
                    Self::get_default_database_file_path,
                    "budget.db",
                    "Database",
                ),
                None => Self::resolve_default_database_path(&initial_data_file_path),
            };

        // --- Resolve the ledger to open before anything reads the transactions table ---
        let (ledgers, active_ledger_id, ledger_error_msg) =
            match Self::ledger_store_for_path(&initial_database_path).initialize() {
                Ok(selection) => (selection.ledgers, selection.active_id, None),
                Err(e) => (
                    Vec::new(),
                    DEFAULT_LEDGER_ID,
                    Some(format!(
                        "Ledger Error [{}]: {}",
                        initial_database_path.display(),
                        e
                    )),
                ),
            };

        // --- Migrate legacy CSV into the database (one time), then load from the database ---
        let migration_msg = match Self::run_one_time_csv_migration(
            &initial_database_path,
            &initial_data_file_path,
            active_ledger_id,
        ) {
            Ok(msg) => msg,
            Err(e) => Some(format!("Transaction migration error: {}", e)),
        };
        let (mut transactions, load_tx_specific_error_msg) =
            match Self::transaction_store_for_path(&initial_database_path, active_ledger_id).list()
            {
                Ok(txs) => (txs, None),
                Err(e) => (
                    vec![],
                    Some(format!(
                        "Load TX Error [{}]: {}",
                        initial_database_path.display(),
                        e
                    )),
                ),
            };

        let (seed_categories, load_seed_error_msg) = match load_seed_categories() {
            Ok(cats) => (cats, None),
            Err(e) => (vec![], Some(format!("Embedded Category Seed Error: {}", e))),
        };
        let (category_records, load_cat_error_msg) =
            match Self::load_category_records(&initial_database_path, &seed_categories) {
                Ok(records) => (records, None),
                Err(e) => (
                    vec![],
                    Some(format!(
                        "Category DB Error [{}]: {}",
                        initial_database_path.display(),
                        e
                    )),
                ),
            };
        let categories = if category_records.is_empty() {
            seed_categories.clone()
        } else {
            Self::project_categories(&category_records)
        };

        // Combine potential errors (settings, paths, tx load, category load)
        let load_tx_error_msg = [
            load_settings_error_msg,
            data_path_error_msg,
            load_tx_specific_error_msg,
            database_path_error_msg,
            load_seed_error_msg,
            migration_msg,
            ledger_error_msg,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" | ");
        let load_tx_error_msg = if load_tx_error_msg.is_empty() {
            None
        } else {
            Some(load_tx_error_msg)
        };

        // --- Combine All Load Errors ---
        let load_error_msg = match (load_tx_error_msg, load_cat_error_msg) {
            (Some(tx_err), Some(cat_err)) => Some(format!("{} | {}", tx_err, cat_err)),
            (Some(tx_err), None) => Some(tx_err),
            (None, Some(cat_err)) => Some(cat_err),
            (None, None) => None,
        };
        let initial_sort_by = SortColumn::Date;
        let initial_sort_order = SortOrder::Descending;
        crate::app::util::sort_transactions_impl(
            &mut transactions,
            initial_sort_by,
            initial_sort_order,
        );
        let initial_filtered_indices = (0..transactions.len()).collect();
        let initial_filtered_category_indices = (0..category_records.len()).collect();
        let mut app = Self {
            transactions,
            filtered_indices: initial_filtered_indices,
            categories,
            category_records,
            ledgers,
            active_ledger_id,
            data_file_path: initial_data_file_path,
            database_path: initial_database_path,
            should_quit: false,
            table_state: TableState::default(),
            mode: AppMode::Normal,
            simple_filter_content: String::new(),
            simple_filter_cursor: 0,
            add_edit_fields: Default::default(),
            add_edit_cursor: 0,
            advanced_filter_fields: Default::default(),
            advanced_filter_cursor: 0,
            delete_index: None,
            editing_index: None,
            status_message: load_error_msg,
            status_expiry: None,
            sort_by: initial_sort_by,
            sort_order: initial_sort_order,
            monthly_summaries: HashMap::new(),
            summary_years: Vec::new(),
            selected_summary_year_index: 0,
            selected_summary_month: None,
            summary_multi_month_mode: false,
            summary_cumulative_mode: false,
            selecting_field: None,
            current_selection_list: Vec::new(),
            selection_list_state: ListState::default(),
            type_to_select: crate::app::util::TypeToSelect::new(),
            category_summaries: HashMap::new(),
            category_summary_years: Vec::new(),
            category_summary_year_index: 0,
            category_summary_table_state: TableState::default(),
            expanded_category_summary_months: HashSet::new(),
            cached_visible_category_items: Vec::new(),
            budget_years: Vec::new(),
            budget_year_index: 0,
            selected_budget_month: None,
            budget_table_state: TableState::default(),
            budget_schedule: BudgetSchedule::default(),
            budget_edit_target: None,
            budget_edit_month: None,
            budget_edit_origin: AppMode::Budget,
            budget_edit_scope_choice: BudgetEditScope::FromThisMonth,
            budget_edit_input: String::new(),
            budget_edit_cursor: 0,
            settings_state: crate::app::settings_types::SettingsState::default(),
            category_table_state: TableState::default(),
            filtered_category_indices: initial_filtered_category_indices,
            category_filter_query: String::new(),
            category_filter_cursor: 0,
            category_sort_by: CategorySortColumn::Type,
            category_sort_order: SortOrder::Ascending,
            category_edit_fields: Default::default(),
            category_edit_cursor: 0,
            editing_category_id: None,
            category_delete_id: None,
            category_catalog_origin: AppMode::Settings,
            ledger_table_state: TableState::default(),
            ledger_name_input: String::new(),
            ledger_name_cursor: 0,
            editing_ledger_id: None,
            ledger_delete_id: None,
            ledger_delete_prompt: String::new(),
            ledger_copy_source_id: None,
            portfolio: Portfolio::default(),
            investment_table_state: TableState::default(),
            investment_entry_table_state: TableState::default(),
            investment_range: InvestmentRange::All,
            show_archived_investments: false,
            investment_detail_id: None,
            investment_account_fields: Default::default(),
            investment_account_cursor: 0,
            editing_investment_account_id: None,
            investment_entry_fields: Default::default(),
            investment_entry_cursor: 0,
            editing_investment_entry_id: None,
            investment_delete_target: None,
            investment_delete_prompt: String::new(),
            hourly_rate: loaded_settings.hourly_rate,
            show_hours: loaded_settings.show_hours.unwrap_or(false),
            fuzzy_search_mode: loaded_settings.fuzzy_search_mode.unwrap_or(false),
            search_query: String::new(),
            recurring_settings_fields: Default::default(),
            recurring_transaction_index: None,
            recurring_forecast_months: loaded_settings
                .recurring_forecast_months
                .unwrap_or(0)
                .min(crate::recurring::MAX_FORECAST_MONTHS),
            io_path_input: String::new(),
            io_path_cursor: 0,
            previous_mode: None,
            help_table_state: TableState::default(),
            hide_help_bar: loaded_settings.hide_help_bar.unwrap_or(false),
            update_available_version: None,
            show_update_popup: false,
            update_rx: rx,
        };
        if let Some(version) = Self::newer_schema_version(&app.database_path) {
            app.status_message = Some(format!(
                "Cannot open '{}': it was created by a newer version of Budget Tracker \
                 (data version {}; this build supports up to {}). Update Budget Tracker to open it.",
                app.database_path.display(),
                version,
                SCHEMA_VERSION
            ));
            app.should_quit = true;
            return app;
        }

        if let Err(err) = app
            .reload_budget_schedule()
            .and_then(|_| app.migrate_legacy_target_budget())
        {
            app.status_message = Some(format!("Budget load error: {}", err));
        }
        if let Err(err) = app.reload_portfolio() {
            app.status_message = Some(format!("Investment load error: {}", err));
        }
        app.calculate_monthly_summaries();
        app.calculate_category_summaries();
        app.refresh_budget_years();
        if !app.summary_years.is_empty() {
            app.selected_summary_year_index = app.summary_years.len() - 1;
        }
        if !app.transactions.is_empty() {
            app.table_state.select(Some(0));
        }

        // Generate recurring transactions up to today (or the configured forecast horizon)
        app.generate_recurring_transactions();

        app
    }

    fn resolve_configured_path(
        configured_path: Option<String>,
        default_path_fn: fn() -> Result<PathBuf, Error>,
        fallback_name: &str,
        label: &str,
    ) -> (PathBuf, Option<String>) {
        match configured_path {
            Some(path_str) => {
                let path = PathBuf::from(path_str);
                if let Some(parent) = path.parent()
                    && let Err(err) = create_dir_all(parent)
                {
                    let fallback =
                        default_path_fn().unwrap_or_else(|_| PathBuf::from(fallback_name));
                    return (
                        fallback,
                        Some(format!(
                            "{} path error: Could not create parent dir for {}: {}. Using default.",
                            label,
                            path.display(),
                            err
                        )),
                    );
                }

                (path, None)
            }
            None => match default_path_fn() {
                Ok(path) => (path, None),
                Err(err) => (
                    PathBuf::from(fallback_name),
                    Some(format!(
                        "{} default path error: {}. Using local '{}'.",
                        label, err, fallback_name
                    )),
                ),
            },
        }
    }

    fn project_categories(records: &[CategoryRecord]) -> Vec<CategoryInfo> {
        records
            .iter()
            .map(CategoryRecord::to_category_info)
            .collect()
    }

    fn resolve_default_database_path(data_file_path: &Path) -> (PathBuf, Option<String>) {
        let default_path = Self::default_database_path_for_data_path(data_file_path);

        if let Some(parent) = default_path.parent()
            && let Err(err) = create_dir_all(parent)
        {
            return (
                PathBuf::from("budget.db"),
                Some(format!(
                    "Database default path error: Could not create parent dir for {}: {}. Using local 'budget.db'.",
                    default_path.display(),
                    err
                )),
            );
        }

        (default_path, None)
    }

    fn category_store_for_path(database_path: &Path) -> SqliteCategoryStore {
        SqliteCategoryStore::new(SqliteDatabase::new(database_path))
    }

    pub(crate) fn category_store(&self) -> SqliteCategoryStore {
        Self::category_store_for_path(&self.database_path)
    }

    pub(crate) fn budget_store(&self) -> SqliteBudgetStore {
        SqliteBudgetStore::new(SqliteDatabase::new(&self.database_path))
    }

    pub(crate) fn investment_store(&self) -> SqliteInvestmentStore {
        SqliteInvestmentStore::new(
            SqliteDatabase::new(&self.database_path),
            self.active_ledger_id,
        )
    }

    pub(crate) fn reload_portfolio(&mut self) -> Result<(), Error> {
        let store = self.investment_store();
        self.portfolio = Portfolio::new(store.list_accounts()?, store.list_entries()?);
        Ok(())
    }

    /// One-shot move of the old global target out of config.json into per-ledger history,
    /// starting before any real month so past months keep the figure they already showed.
    fn migrate_legacy_target_budget(&mut self) -> Result<(), Error> {
        let Ok(mut settings) = load_settings() else {
            return Ok(());
        };
        let Some(amount) = settings.target_budget else {
            return Ok(());
        };

        let store = self.budget_store();
        for ledger in &self.ledgers {
            let already_set = store
                .list(ledger.id)?
                .iter()
                .any(|period| period.category_id.is_none());
            if !already_set {
                store.set(ledger.id, None, BudgetMonth::BEGINNING, Some(amount))?;
            }
        }

        // Clear it from the config file so it cannot come back and overwrite the history.
        settings.target_budget = None;
        let _ = save_settings(&settings);
        self.reload_budget_schedule()
    }

    pub(crate) fn reload_budget_schedule(&mut self) -> Result<(), Error> {
        let periods = self.budget_store().list(self.active_ledger_id)?;
        self.budget_schedule = BudgetSchedule::new(periods);
        Ok(())
    }

    /// The month the budget view is pointed at, which anchors every budget lookup.
    pub(crate) fn selected_budget_key(&self) -> Option<BudgetMonth> {
        match (self.selected_budget_year(), self.selected_budget_month) {
            (Some(year), Some(month)) => Some(BudgetMonth::new(year, month)),
            _ => None,
        }
    }

    /// Today's month, used by the catalog, which has no month to work from.
    pub(crate) fn current_budget_key() -> BudgetMonth {
        let now = chrono::Local::now();
        BudgetMonth::new(now.year(), now.month())
    }

    fn transaction_store_for_path(database_path: &Path, ledger_id: i64) -> SqliteTransactionStore {
        SqliteTransactionStore::new(SqliteDatabase::new(database_path), ledger_id)
    }

    pub(crate) fn transaction_store(&self) -> SqliteTransactionStore {
        Self::transaction_store_for_path(&self.database_path, self.active_ledger_id)
    }

    /// The schema version stored in a database file when it is newer than this build handles.
    fn newer_schema_version(database_path: &Path) -> Option<i64> {
        let conn = SqliteDatabase::new(database_path)
            .open_connection("version check")
            .ok()?;
        let current: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .ok()?;
        (current > SCHEMA_VERSION).then_some(current)
    }

    fn ledger_store_for_path(database_path: &Path) -> SqliteLedgerStore {
        SqliteLedgerStore::new(SqliteDatabase::new(database_path))
    }

    pub(crate) fn ledger_store(&self) -> SqliteLedgerStore {
        Self::ledger_store_for_path(&self.database_path)
    }

    pub(crate) fn active_ledger_name(&self) -> &str {
        self.ledgers
            .iter()
            .find(|ledger| ledger.id == self.active_ledger_id)
            .map(|ledger| ledger.name.as_str())
            .unwrap_or("Main")
    }

    /// Reload the working transaction set from the database and re-derive the in-memory
    /// generated recurring occurrences. Call after any mutation that touched the store.
    pub(crate) fn reload_transactions_from_db(&mut self) -> Result<(), Error> {
        self.transactions = self.transaction_store().list()?;
        // Re-derives generated occurrences and recomputes sort/filter/summaries.
        self.generate_recurring_transactions();
        Ok(())
    }

    pub(crate) fn reset_table_selection(&mut self) {
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    /// Reload everything that is derived from the database after the active ledger or the
    /// database path changes. Leaves `mode` alone so callers control navigation.
    pub(crate) fn reload_working_set(&mut self) -> Result<(), Error> {
        self.reload_categories_from_store()?;
        self.reload_transactions_from_db()?;
        self.reload_portfolio()?;
        self.refresh_budget_years();
        self.reset_table_selection();
        Ok(())
    }

    /// Point the app at another ledger in the same database and reload from it. Filters are
    /// cleared because they were written against a different set of transactions.
    pub(crate) fn switch_ledger(&mut self, ledger_id: i64) -> Result<(), Error> {
        if ledger_id != self.active_ledger_id {
            self.ledger_store().set_active_id(ledger_id)?;
            self.active_ledger_id = ledger_id;
        }
        self.clear_all_filter_fields();
        self.reload_working_set()
    }

    /// Re-resolve the ledgers of the current database, repairing the active selection when it
    /// no longer exists (a different database file, or the active ledger was deleted).
    pub(crate) fn refresh_ledgers(&mut self) -> Result<(), Error> {
        let selection = self.ledger_store().initialize()?;
        self.ledgers = selection.ledgers;
        self.active_ledger_id = selection.active_id;
        Ok(())
    }

    /// One-time, non-destructive migration of the legacy transactions CSV into the database.
    /// Gated by a metadata flag so it runs at most once. Returns an optional status message.
    fn run_one_time_csv_migration(
        database_path: &Path,
        data_file_path: &Path,
        ledger_id: i64,
    ) -> Result<Option<String>, Error> {
        let database = SqliteDatabase::new(database_path);
        let mut conn = database.open_connection("transaction migration")?;
        database.run_migrations(&mut conn)?;

        if database
            .metadata_value(&conn, "transactions_migrated")?
            .is_some()
        {
            return Ok(None);
        }

        let mut status = None;
        if data_file_path.exists() {
            // Only real rows are imported; generated occurrences are re-derived from sources.
            let real_rows: Vec<Transaction> = load_transactions(data_file_path)?
                .into_iter()
                .filter(|tx| !tx.is_generated_from_recurring)
                .collect();

            if !real_rows.is_empty() {
                let store = SqliteTransactionStore::new(database.clone(), ledger_id);
                let summary = store.import_merge(&real_rows)?;

                // Preserve the original file (never delete) by renaming it aside.
                let backup = {
                    let mut name = data_file_path.to_path_buf().into_os_string();
                    name.push(".migrated-backup");
                    PathBuf::from(name)
                };
                let backup_note = match std::fs::rename(data_file_path, &backup) {
                    Ok(_) => format!(" Original CSV saved as {}.", backup.display()),
                    Err(_) => String::new(),
                };
                status = Some(format!(
                    "Migrated {} transactions from CSV to the database.{}",
                    summary.added, backup_note
                ));
            }
        }

        database.set_metadata_value(&conn, "transactions_migrated", "1")?;
        Ok(status)
    }

    pub(crate) fn initialize_category_database(
        database_path: &Path,
        seed_categories: &[CategoryInfo],
    ) -> Result<(), Error> {
        let store = Self::category_store_for_path(database_path);
        store.initialize(seed_categories)?;
        Ok(())
    }

    pub(crate) fn prepare_category_database_for_path_change(
        current_database_path: &Path,
        new_database_path: &Path,
        seed_categories: &[CategoryInfo],
    ) -> Result<(), Error> {
        if current_database_path != new_database_path
            && !new_database_path.exists()
            && current_database_path.exists()
        {
            let destination_database = SqliteDatabase::new(new_database_path);
            destination_database.ensure_parent_dir()?;
            copy(current_database_path, new_database_path).map_err(|err| {
                Error::other(format!(
                    "Failed to copy database from '{}' to '{}': {}",
                    current_database_path.display(),
                    new_database_path.display(),
                    err
                ))
            })?;
        }

        Self::initialize_category_database(new_database_path, seed_categories)
    }

    fn load_category_records(
        database_path: &Path,
        seed_categories: &[CategoryInfo],
    ) -> Result<Vec<CategoryRecord>, Error> {
        let store = Self::category_store_for_path(database_path);
        store.initialize(seed_categories)?;
        store.list()
    }

    pub(crate) fn refresh_category_state(&mut self, records: Vec<CategoryRecord>) {
        self.category_records = records;
        self.categories = Self::project_categories(&self.category_records);
        self.apply_category_filter();
    }

    pub(crate) fn refresh_categories_from_database(&mut self) -> Result<(), Error> {
        let store = self.category_store();
        let records = store.list()?;
        // Budgets are keyed on category ids and SQLite hands deleted ids straight back, so
        // a stale schedule would show a dead category's budget on a newly created one.
        // Reloading both together is what stops them drifting apart.
        self.reload_budget_schedule()?;
        self.refresh_category_state(records);
        Ok(())
    }

    pub(crate) fn reload_categories_from_store(&mut self) -> Result<(), Error> {
        self.refresh_categories_from_database()
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
    pub fn next_item(&mut self) {
        let list_len = match self.mode {
            AppMode::Normal | AppMode::Filtering => self.filtered_indices.len(),
            AppMode::Summary => 12,
            AppMode::CategorySummary => self.cached_visible_category_items.len(),
            AppMode::Budget => 12,
            _ => 0,
        };
        if list_len == 0 {
            return;
        }
        let current_selection = self.table_state.selected().unwrap_or(0);
        let next_selection = if current_selection >= list_len - 1 {
            0
        } else {
            current_selection + 1
        };
        self.table_state.select(Some(next_selection));
    }
    pub fn previous_item(&mut self) {
        let list_len = match self.mode {
            AppMode::Normal | AppMode::Filtering => self.filtered_indices.len(),
            AppMode::Summary => 12,
            AppMode::CategorySummary => self.cached_visible_category_items.len(),
            AppMode::Budget => 12,
            _ => 0,
        };
        if list_len == 0 {
            return;
        }
        let current_selection = self.table_state.selected().unwrap_or(0);
        let prev_selection = if current_selection == 0 {
            list_len - 1
        } else {
            current_selection - 1
        };
        self.table_state.select(Some(prev_selection));
    }
    pub fn decrement_date(&mut self) {
        self.adjust_date(-1, DateUnit::Day);
    }
    pub fn increment_date(&mut self) {
        self.adjust_date(1, DateUnit::Day);
    }
    pub fn decrement_month(&mut self) {
        self.adjust_date(-1, DateUnit::Month);
    }
    pub fn increment_month(&mut self) {
        self.adjust_date(1, DateUnit::Month);
    }

    /// Jump to the very first item in the transaction list
    /// Only works in Normal and Filtering modes
    pub fn jump_to_first(&mut self) {
        match self.mode {
            AppMode::Normal | AppMode::Filtering => {
                let list_len = self.filtered_indices.len();
                if list_len > 0 {
                    self.table_state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    /// Jump to the very last item in the transaction list
    /// Only works in Normal and Filtering modes
    pub fn jump_to_last(&mut self) {
        match self.mode {
            AppMode::Normal | AppMode::Filtering => {
                let list_len = self.filtered_indices.len();
                if list_len > 0 {
                    let last_index = list_len - 1;
                    self.table_state.select(Some(last_index));
                }
            }
            _ => {}
        }
    }

    /// Page size for transaction navigation (PageUp/PageDown)
    const TRANSACTION_PAGE_SIZE: usize = 20;

    /// Jump up by approximately one page worth of transactions
    /// Only works in Normal and Filtering modes
    pub fn page_up(&mut self) {
        match self.mode {
            AppMode::Normal | AppMode::Filtering => {
                let list_len = self.filtered_indices.len();
                if list_len == 0 {
                    return;
                }

                let page_size = Self::TRANSACTION_PAGE_SIZE;
                let current_selection = self.table_state.selected().unwrap_or(0);
                let new_selection = current_selection.saturating_sub(page_size);

                self.table_state.select(Some(new_selection));
            }
            _ => {}
        }
    }

    /// Jump down by approximately one page worth of transactions
    /// Only works in Normal and Filtering modes
    pub fn page_down(&mut self) {
        match self.mode {
            AppMode::Normal | AppMode::Filtering => {
                let list_len = self.filtered_indices.len();
                if list_len == 0 {
                    return;
                }

                let page_size = Self::TRANSACTION_PAGE_SIZE;
                let current_selection = self.table_state.selected().unwrap_or(0);
                let new_selection = (current_selection + page_size).min(list_len - 1);

                self.table_state.select(Some(new_selection));
            }
            _ => {}
        }
    }
    pub(crate) fn get_original_index(&self, filtered_view_index: usize) -> Option<usize> {
        self.filtered_indices.get(filtered_view_index).copied()
    }
    pub(crate) fn sort_transactions(&mut self) {
        crate::app::util::sort_transactions_impl(
            &mut self.transactions,
            self.sort_by,
            self.sort_order,
        );
    }
    pub(crate) fn calculate_monthly_summaries(&mut self) {
        self.monthly_summaries.clear();
        let mut years = Vec::new();
        for &idx in &self.filtered_indices {
            if let Some(tx) = self.transactions.get(idx) {
                let year = tx.date.year();
                let month = tx.date.month();
                let summary = self.monthly_summaries.entry((year, month)).or_default();
                match tx.transaction_type {
                    TransactionType::Income => summary.income += tx.amount,
                    TransactionType::Expense => summary.expense += tx.amount,
                }
                if !years.contains(&year) {
                    years.push(year);
                }
            }
        }
        years.sort_unstable();
        self.summary_years = years;
        if !self.summary_years.is_empty() {
            self.selected_summary_year_index = self
                .selected_summary_year_index
                .min(self.summary_years.len() - 1);
        } else {
            self.selected_summary_year_index = 0;
        }
        self.refresh_budget_years();
    }
    pub(crate) fn calculate_category_summaries(&mut self) {
        self.category_summaries.clear();
        let mut years = HashSet::new();
        for tx in self
            .filtered_indices
            .iter()
            .map(|&idx| &self.transactions[idx])
        {
            let year = tx.date.year();
            let month = tx.date.month();
            years.insert(year);
            let category_key = tx.category.trim();
            let subcategory_key = tx.subcategory.trim();
            let final_category = if category_key.is_empty() {
                "Uncategorized"
            } else {
                category_key
            };
            let month_map = self.category_summaries.entry((year, month)).or_default();
            let summary = month_map
                .entry((final_category.to_string(), subcategory_key.to_string()))
                .or_default();
            match tx.transaction_type {
                TransactionType::Income => summary.income += tx.amount,
                TransactionType::Expense => summary.expense += tx.amount,
            }
        }
        self.category_summary_years = years.into_iter().collect();
        self.category_summary_years.sort_unstable();
        if !self.category_summary_years.is_empty() {
            self.category_summary_year_index = self
                .category_summary_year_index
                .min(self.category_summary_years.len() - 1);
        } else {
            self.category_summary_year_index = 0;
        }
        let list_len = self.cached_visible_category_items.len();
        if list_len == 0 {
            self.category_summary_table_state.select(None);
        } else {
            let current_selection = self.category_summary_table_state.selected().unwrap_or(0);
            let new_selection = current_selection.min(list_len - 1);
            self.category_summary_table_state
                .select(Some(new_selection));
        }
    }
    /// Step whichever date field currently has focus, in whatever form is open.
    pub(crate) fn adjust_date(&mut self, amount: i64, unit: DateUnit) {
        let Some((content, _)) = self.active_date_input() else {
            return;
        };
        let current = content.clone();

        let Ok(current_date) = NaiveDate::parse_from_str(&current, crate::model::DATE_FORMAT)
        else {
            self.set_status_message(
                format!(
                    "Error: Could not parse date '{}'. Use YYYY-MM-DD format.",
                    current
                ),
                None,
            );
            return;
        };

        let new_date = match unit {
            DateUnit::Day => {
                if amount > 0 {
                    current_date + Duration::days(amount)
                } else {
                    current_date - Duration::days(-amount)
                }
            }
            DateUnit::Month => crate::validation::add_months(current_date, amount as i32),
        };

        if let Some((content, cursor)) = self.active_date_input() {
            *content = new_date.format(crate::model::DATE_FORMAT).to_string();
            *cursor = content.len();
        }
        self.clear_status_message();
    }
    pub fn get_default_data_file_path() -> Result<PathBuf, Error> {
        const DATA_FILE_NAME: &str = "transactions.csv";
        const APP_DATA_SUBDIR: &str = "BudgetTracker";
        match dirs::data_dir() {
            Some(mut path) => {
                path.push(APP_DATA_SUBDIR);
                create_dir_all(&path)?;
                path.push(DATA_FILE_NAME);
                Ok(path)
            }
            None => Err(Error::new(
                ErrorKind::NotFound,
                "Could not find user data directory",
            )),
        }
    }

    pub fn default_database_path_for_data_path(data_file_path: &Path) -> PathBuf {
        const DATABASE_FILE_NAME: &str = "budget.db";

        match data_file_path.parent() {
            Some(parent) => parent.join(DATABASE_FILE_NAME),
            None => PathBuf::from(DATABASE_FILE_NAME),
        }
    }

    pub fn get_default_database_file_path() -> Result<PathBuf, Error> {
        Self::get_default_data_file_path()
            .map(|data_path| Self::default_database_path_for_data_path(&data_path))
    }
    // --- Sorting Logic ---
    pub(crate) fn set_sort_column(&mut self, column: SortColumn) {
        if self.sort_by == column {
            self.sort_order = match self.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.sort_by = column;
            self.sort_order = SortOrder::Ascending;
        }

        // Preserve the current filter type when sorting
        // Check if any advanced filter fields are active
        let has_advanced_filters = !self.advanced_filter_fields.all_empty();

        if has_advanced_filters {
            self.apply_advanced_filter();
        } else {
            self.apply_filter();
        }
    }

    pub fn set_status_message<S: Into<String>>(&mut self, message: S, duration: Option<Duration>) {
        self.status_message = Some(message.into());
        self.status_expiry = duration.map(|d| {
            std::time::Instant::now() + std::time::Duration::from_secs(d.num_seconds() as u64)
        });
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
        self.status_expiry = None;
    }
}
