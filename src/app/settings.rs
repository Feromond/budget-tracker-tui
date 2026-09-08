use super::state::{App, AppMode};
use crate::app::settings_types::{SettingKey, SettingType, SettingsState};
use crate::config::{AppSettings, save_settings};
use crate::csv_io::load_seed_categories;
use chrono::Duration;
use std::path::PathBuf;

pub(crate) fn toggle_value(enabled: bool) -> String {
    if enabled { "◀ Yes " } else { " No ▶" }.to_string()
}

impl App {
    // --- Settings Mode Logic ---
    // Handles entering/exiting settings mode, saving settings, and resetting the data file path.
    pub(crate) fn enter_settings_mode(&mut self) {
        self.mode = crate::app::state::AppMode::Settings;

        // Initialize Settings State
        self.settings_state = SettingsState::new();

        // Load Config
        let loaded_settings = crate::config::load_settings().unwrap_or_default();

        // --- Data Management Section ---
        self.settings_state.add_header("Data Management");

        let database_path_str = self.database_path.to_string_lossy().to_string();
        let database_path_val = crate::validation::strip_path_quotes(&database_path_str);
        self.settings_state.add_setting(
            SettingKey::DatabasePath,
            "Database Path",
            database_path_val,
            SettingType::Path,
            "Absolute path to your SQLite database (transactions and categories).",
        );
        self.settings_state.add_setting(
            SettingKey::ManageLedgers,
            "Ledger",
            self.active_ledger_name().to_string(),
            SettingType::Action,
            "Switch between transaction ledgers, or add, rename, and delete them.",
        );
        self.settings_state.add_setting(
            SettingKey::ManageCategories,
            "Manage Categories",
            "Open Category Catalog".to_string(),
            SettingType::Action,
            "Open the category catalog to add, edit, or delete categories.",
        );
        self.settings_state.add_setting(
            SettingKey::ImportTransactions,
            "Import Transactions (CSV)",
            "Choose a file to import".to_string(),
            SettingType::Action,
            "Press Enter to choose a CSV file to import (new rows are added, duplicates skipped).",
        );
        self.settings_state.add_setting(
            SettingKey::ExportTransactions,
            "Export Transactions (CSV)",
            "Choose a destination to export".to_string(),
            SettingType::Action,
            "Press Enter to choose a destination and export all transactions to CSV.",
        );

        // --- Backups Section ---
        self.settings_state.add_header("Backups");

        let backups_enabled = loaded_settings.backups_enabled.unwrap_or(true);
        self.settings_state.add_setting(
            SettingKey::AutomaticBackups,
            "Automatic Backups",
            toggle_value(backups_enabled),
            SettingType::Toggle,
            "Take daily and pre-upgrade snapshots at startup. Turning this off disables both.",
        );

        if backups_enabled {
            self.settings_state.add_setting(
                SettingKey::BackupsToKeep,
                "Automatic Backups to Keep",
                self.backup_keep.to_string(),
                SettingType::Integer,
                &format!(
                    "How many automatic snapshots to keep for this install's ID (1-{}). Other kinds stay until you delete them.",
                    crate::db::backup::MAX_KEEP
                ),
            );
        }

        self.settings_state.add_setting(
            SettingKey::ManageBackups,
            "Manage Backups",
            "Browse and restore".to_string(),
            SettingType::Action,
            "Press Enter to create, restore, or delete backups.",
        );

        // --- Transaction View Section ---
        self.settings_state.add_header("Transaction View");

        let hourly_rate_val = loaded_settings
            .hourly_rate
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.settings_state.add_setting(
            SettingKey::HourlyRate,
            "Hourly Rate ($)",
            hourly_rate_val.clone(),
            SettingType::Number,
            "Optional. Enter your hourly earning rate to see costs in hours.",
        );

        // Show Hours is only relevant once an hourly rate is set.
        if !hourly_rate_val.is_empty() {
            self.settings_state.add_setting(
                SettingKey::ShowHours,
                "Show Costs in Hours",
                toggle_value(loaded_settings.show_hours.unwrap_or(false)),
                SettingType::Toggle,
                "Toggle to display transaction amounts as hours worked.",
            );
        }

        // --- Recurring Transactions Section ---
        self.settings_state.add_header("Recurring Transactions");

        let forecast_val = self.recurring_forecast_months.to_string();
        self.settings_state.add_setting(
            SettingKey::RecurringForecastMonths,
            "Forecast Months Ahead",
            forecast_val,
            SettingType::Integer,
            &format!(
                "Project recurring occurrences this many months past today (0-{}). 0 stops at today.",
                crate::recurring::MAX_FORECAST_MONTHS
            ),
        );

        // --- Input Preferences Section ---
        self.settings_state.add_header("Input Preferences");

        self.settings_state.add_setting(
            SettingKey::FuzzySearch,
            "Fuzzy Search Categories",
            toggle_value(loaded_settings.fuzzy_search_mode.unwrap_or(false)),
            SettingType::Toggle,
            "Toggle to enable fuzzy searching for categories/subcategories.",
        );

        // --- General Preferences Section ---
        self.settings_state.add_header("General Preferences");

        self.settings_state.add_setting(
            SettingKey::HideHelpBar,
            "Hide Help Bar (NOT RECOMMENDED)",
            toggle_value(loaded_settings.hide_help_bar.unwrap_or(false)),
            SettingType::Toggle,
            "Toggle to hide the bottom help bar (Ctrl+H will still work).",
        );

        // Select first valid item (skip headers)
        self.settings_state.selected_index = 0;
        while self.settings_state.selected_index < self.settings_state.items.len() {
            if self.settings_state.items[self.settings_state.selected_index].setting_type
                != SettingType::SectionHeader
            {
                break;
            }
            self.settings_state.selected_index += 1;
        }

        if let Some(item) = self
            .settings_state
            .items
            .get(self.settings_state.selected_index)
        {
            self.settings_state.edit_cursor = item.value.len();
        }

        self.clear_status_message();
    }

    pub(crate) fn select_settings_row(&mut self, key: SettingKey) {
        if let Some(index) = self
            .settings_state
            .items
            .iter()
            .position(|item| item.key == key)
        {
            self.settings_state.selected_index = index;
            self.settings_state.edit_cursor = self.settings_state.items[index].value.len();
        }
    }

    pub(crate) fn exit_settings_mode(&mut self) {
        self.mode = crate::app::state::AppMode::Normal;
        self.settings_state = SettingsState::default();
        self.io_path_input.clear();
        self.io_path_cursor = 0;
        self.clear_status_message();
    }

    pub(crate) fn save_settings(&mut self) {
        // Retrieve values from state
        let mut new_database_path_str = String::new();
        let mut hourly_rate_str = String::new();
        let mut forecast_months_str = String::new();
        let mut backup_keep_str = String::new();
        let mut backups_enabled_val = None;
        let mut show_hours_val = None;
        let mut fuzzy_search_val = None;
        let mut hide_help_bar_val = None;

        if let Some(val) = self.settings_state.get_value(SettingKey::DatabasePath) {
            new_database_path_str = crate::validation::strip_path_quotes(val);
        }
        if let Some(val) = self.settings_state.get_value(SettingKey::HourlyRate) {
            hourly_rate_str = val.trim().to_string();
        }
        if let Some(val) = self
            .settings_state
            .get_value(SettingKey::RecurringForecastMonths)
        {
            forecast_months_str = val.trim().to_string();
        }
        if let Some(val) = self.settings_state.get_value(SettingKey::BackupsToKeep) {
            backup_keep_str = val.trim().to_string();
        }
        if let Some(val) = self.settings_state.get_value(SettingKey::AutomaticBackups) {
            backups_enabled_val = Some(val.to_lowercase().contains("yes"));
        }
        if let Some(val) = self.settings_state.get_value(SettingKey::ShowHours) {
            show_hours_val = Some(val.to_lowercase().contains("yes"));
        }
        if let Some(val) = self.settings_state.get_value(SettingKey::FuzzySearch) {
            fuzzy_search_val = Some(val.to_lowercase().contains("yes"));
        }
        if let Some(val) = self.settings_state.get_value(SettingKey::HideHelpBar) {
            hide_help_bar_val = Some(val.to_lowercase().contains("yes"));
        }

        // Validate Hourly Rate
        let hourly_rate = if hourly_rate_str.is_empty() {
            None
        } else {
            match crate::validation::validate_amount_string(&hourly_rate_str) {
                Ok(val) => Some(val),
                Err(msg) => {
                    self.set_status_message(format!("Error: Hourly rate - {}", msg), None);
                    return;
                }
            }
        };

        // Validate Recurring Forecast Horizon
        let forecast_months = if forecast_months_str.is_empty() {
            0
        } else {
            match forecast_months_str.parse::<u32>() {
                Ok(val) if val <= crate::recurring::MAX_FORECAST_MONTHS => val,
                _ => {
                    self.set_status_message(
                        format!(
                            "Error: Forecast months must be a whole number between 0 and {}.",
                            crate::recurring::MAX_FORECAST_MONTHS
                        ),
                        None,
                    );
                    return;
                }
            }
        };

        let backup_keep = if backup_keep_str.is_empty() {
            self.backup_keep
        } else {
            match backup_keep_str.parse::<u32>() {
                Ok(val) if (1..=crate::db::backup::MAX_KEEP).contains(&val) => val,
                _ => {
                    self.set_status_message(
                        format!(
                            "Error: Backups to keep must be a whole number between 1 and {}.",
                            crate::db::backup::MAX_KEEP
                        ),
                        None,
                    );
                    return;
                }
            }
        };

        // Validate Path
        if new_database_path_str.is_empty() {
            self.set_status_message("Error: Database path cannot be empty.", None);
            return;
        }
        let new_database_path = PathBuf::from(&new_database_path_str);

        let seed_categories = if self.categories.is_empty() {
            load_seed_categories().unwrap_or_default()
        } else {
            self.categories.clone()
        };
        if let Err(e) = Self::prepare_category_database_for_path_change(
            &self.database_path,
            &new_database_path,
            &seed_categories,
        ) {
            self.set_status_message(
                format!(
                    "Error preparing database '{}': {}. Check path and permissions.",
                    new_database_path.display(),
                    e
                ),
                None,
            );
            return;
        }

        // Save to Config. The legacy data file path is retained only so a one-time CSV
        // migration can still locate it and as the default for import/export.
        let settings = AppSettings {
            data_file_path: Some(self.data_file_path.to_string_lossy().to_string()),
            database_path: Some(new_database_path_str.clone()),
            target_budget: None,
            hourly_rate,
            show_hours: show_hours_val,
            fuzzy_search_mode: fuzzy_search_val,
            hide_help_bar: hide_help_bar_val,
            recurring_forecast_months: Some(forecast_months),
            backups_enabled: backups_enabled_val,
            backup_keep: Some(backup_keep),
            instance_id: Some(self.backup_instance_id.clone()),
        };
        if let Err(e) = save_settings(&settings) {
            self.set_status_message(format!("Error saving config file: {}", e), None);
            return;
        }

        // Set before reloading so occurrences are re-derived against the new horizon.
        self.recurring_forecast_months = forecast_months;

        // Point at the new database and reload everything from it. The ledgers are
        // re-resolved first because the previous active id belongs to the old file.
        self.database_path = new_database_path.clone();
        self.clear_all_filter_fields();
        if let Err(e) = self
            .refresh_ledgers()
            .and_then(|_| self.reload_working_set())
        {
            self.set_status_message(
                format!(
                    "Error loading '{}': {}. Check database path and permissions.",
                    self.database_path.display(),
                    e
                ),
                None,
            );
            return;
        }

        self.exit_settings_mode();

        self.set_status_message(
            format!("Settings saved. Database: {}", self.database_path.display()),
            Some(Duration::seconds(3)),
        );
        self.hourly_rate = hourly_rate;
        self.backups_enabled = backups_enabled_val.unwrap_or(true);
        self.backup_keep = backup_keep;
        self.show_hours = show_hours_val.unwrap_or(false);
        self.fuzzy_search_mode = fuzzy_search_val.unwrap_or(false);
        self.hide_help_bar = hide_help_bar_val.unwrap_or(false);
    }

    pub(crate) fn reset_settings_database_path_to_default(&mut self) {
        let data_path_value =
            crate::validation::strip_path_quotes(&self.data_file_path.to_string_lossy());

        let default_path = if data_path_value.trim().is_empty() {
            Self::get_default_database_file_path().unwrap_or_else(|_| PathBuf::from("budget.db"))
        } else {
            Self::default_database_path_for_data_path(PathBuf::from(data_path_value).as_path())
        };

        let clean_path =
            crate::validation::strip_path_quotes(default_path.to_string_lossy().as_ref());

        if let Some(idx) = self
            .settings_state
            .items
            .iter()
            .position(|i| i.key == SettingKey::DatabasePath)
        {
            self.settings_state.items[idx].value = clean_path;
            if self.settings_state.selected_index == idx {
                self.settings_state.edit_cursor = self.settings_state.items[idx].value.len();
            }
        }

        self.set_status_message(
            "Database path reset to default for the current data file. Press Enter to save.",
            None,
        );
    }

    pub(crate) fn activate_selected_setting(&mut self) {
        let selected_key = self
            .settings_state
            .items
            .get(self.settings_state.selected_index)
            .map(|item| item.key);

        match selected_key {
            Some(SettingKey::ManageLedgers) => self.open_ledger_manager(),
            Some(SettingKey::ManageCategories) => self.open_category_catalog(AppMode::Settings),
            Some(SettingKey::ImportTransactions) => {
                self.open_transaction_io(AppMode::ImportTransactions)
            }
            Some(SettingKey::ExportTransactions) => {
                self.open_transaction_io(AppMode::ExportTransactions)
            }
            Some(SettingKey::ManageBackups) => self.open_backup_manager(),
            _ => self.save_settings(),
        }
    }

    /// Generalized method to ensure a setting is visible or hidden based on a condition.
    /// This handles finding the correct position and maintaining list integrity.
    fn ensure_setting_visibility<F>(
        &mut self,
        target_key: SettingKey,
        should_be_visible: bool,
        insert_after_key: SettingKey,
        item_creator: F,
    ) where
        F: FnOnce() -> crate::app::settings_types::SettingItem,
    {
        let target_idx = self
            .settings_state
            .items
            .iter()
            .position(|i| i.key == target_key);

        if should_be_visible {
            if target_idx.is_none() {
                // Find where to insert
                if let Some(after_idx) = self
                    .settings_state
                    .items
                    .iter()
                    .position(|i| i.key == insert_after_key)
                {
                    let item = item_creator();
                    self.settings_state.items.insert(after_idx + 1, item);

                    // If we inserted before the selection, shift selection down
                    if self.settings_state.selected_index > after_idx {
                        self.settings_state.selected_index += 1;
                    }
                }
            }
        } else if let Some(idx) = target_idx {
            self.settings_state.items.remove(idx);

            // If we removed the item before or at the selection, shift selection up
            if self.settings_state.selected_index >= idx {
                self.settings_state.selected_index =
                    self.settings_state.selected_index.saturating_sub(1);
            }
        }
    }

    pub(crate) fn update_settings_visibility(&mut self) {
        // "Show Costs in Hours" is only shown once an hourly rate has a value.
        let hourly_rate_has_value = self
            .settings_state
            .get_value(SettingKey::HourlyRate)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);

        self.ensure_setting_visibility(
            SettingKey::ShowHours,
            hourly_rate_has_value,
            SettingKey::HourlyRate,
            || crate::app::settings_types::SettingItem {
                key: SettingKey::ShowHours,
                label: "Show Costs in Hours".to_string(),
                value: toggle_value(false),
                setting_type: crate::app::settings_types::SettingType::Toggle,
                help: "Toggle to display transaction amounts as hours worked.".to_string(),
            },
        );

        let backups_on = self
            .settings_state
            .get_value(SettingKey::AutomaticBackups)
            .map(|value| value.to_lowercase().contains("yes"))
            .unwrap_or(false);
        let keep = self.backup_keep;

        self.ensure_setting_visibility(
            SettingKey::BackupsToKeep,
            backups_on,
            SettingKey::AutomaticBackups,
            || crate::app::settings_types::SettingItem {
                key: SettingKey::BackupsToKeep,
                label: "Automatic Backups to Keep".to_string(),
                value: keep.to_string(),
                setting_type: crate::app::settings_types::SettingType::Integer,
                help: format!(
                    "How many automatic snapshots to keep for this install's ID (1-{}). Other kinds stay until you delete them.",
                    crate::db::backup::MAX_KEEP
                ),
            },
        );
    }
}
