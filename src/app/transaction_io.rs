use super::state::{App, AppMode};
use crate::csv_io::{load_transactions, save_transactions};
use crate::db::transaction_store::TransactionStore;
use crate::model::Transaction;
use chrono::Duration;
use std::path::{Path, PathBuf};

impl App {
    pub(crate) fn open_transaction_io(&mut self, mode: AppMode) {
        self.mode = mode;
        self.io_path_input = self.default_io_path_value();
        self.io_path_cursor = self.io_path_input.len();
        self.clear_status_message();
    }

    pub(crate) fn cancel_transaction_io(&mut self) {
        self.mode = AppMode::Settings;
        self.io_path_input.clear();
        self.io_path_cursor = 0;
        self.clear_status_message();
    }

    pub(crate) fn reset_transaction_io_path(&mut self) {
        self.io_path_input = self.default_io_path_value();
        self.io_path_cursor = self.io_path_input.len();
    }

    pub(crate) fn clear_transaction_io_path(&mut self) {
        self.io_path_input.clear();
        self.io_path_cursor = 0;
    }

    /// Defaults the prompt to the legacy data directory, the likeliest place to keep a CSV.
    fn default_io_path_value(&self) -> String {
        crate::validation::strip_path_quotes(&self.data_file_path.to_string_lossy())
    }

    pub(crate) fn import_transactions(&mut self) {
        let path_str = crate::validation::strip_path_quotes(&self.io_path_input);
        if path_str.trim().is_empty() {
            self.set_status_message("Error: enter a CSV path to import.", None);
            return;
        }
        let path = PathBuf::from(&path_str);
        if !path.exists() {
            self.set_status_message(format!("Error: file '{}' not found.", path.display()), None);
            return;
        }

        let rows = match load_transactions(&path) {
            Ok(rows) => rows,
            Err(e) => {
                self.set_status_message(format!("Error reading '{}': {}", path.display(), e), None);
                return;
            }
        };
        // Generated occurrences are re-derived from their sources, so only real rows are merged.
        let real_rows: Vec<Transaction> = match rows
            .iter()
            .filter(|row| !row.is_generated_from_recurring)
            .map(|row| self.accounts.link_csv(row))
            .collect()
        {
            Ok(rows) => rows,
            Err(msg) => {
                self.set_status_message(format!("Error: {}", msg), None);
                return;
            }
        };

        let summary = match self.transaction_store().import_merge(&real_rows) {
            Ok(summary) => summary,
            Err(e) => {
                self.set_status_message(format!("Error importing transactions: {}", e), None);
                return;
            }
        };
        // Clear filters so the freshly imported rows are actually visible.
        self.clear_all_filter_fields();
        if let Err(e) = self.reload_transactions_from_db() {
            self.set_status_message(format!("Imported, but reloading failed: {}", e), None);
            return;
        }

        self.exit_settings_mode();
        self.reset_table_selection();
        self.set_status_message(
            format!(
                "Imported {} new, skipped {} duplicates.",
                summary.added, summary.skipped
            ),
            Some(Duration::seconds(4)),
        );
    }

    /// The destination usually does not exist yet, so fall back to canonicalizing its parent to
    /// keep relative paths and symlinked directories comparable.
    fn resolve_io_path(path: &Path) -> PathBuf {
        if let Ok(canonical) = path.canonicalize() {
            return canonical;
        }

        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(path),
                Err(_) => path.to_path_buf(),
            }
        };

        let resolved = match (absolute.parent(), absolute.file_name()) {
            (Some(parent), Some(name)) => parent.canonicalize().map(|p| p.join(name)).ok(),
            _ => None,
        };
        resolved.unwrap_or(absolute)
    }

    pub(crate) fn export_transactions(&mut self) {
        let path_str = crate::validation::strip_path_quotes(&self.io_path_input);
        if path_str.trim().is_empty() {
            self.set_status_message("Error: enter a destination path to export.", None);
            return;
        }
        let path = PathBuf::from(&path_str);

        // save_transactions truncates whatever it opens.
        if Self::resolve_io_path(&path) == Self::resolve_io_path(&self.database_path) {
            self.set_status_message(
                "Error: that is the active database file. Choose a different export destination.",
                None,
            );
            return;
        }

        // Export the materialized view (real rows plus generated occurrences) for a complete CSV.
        let rows: Vec<_> = self
            .transactions
            .iter()
            .map(|tx| self.accounts.csv_row(tx))
            .collect();
        match save_transactions(&rows, &path) {
            Ok(_) => {
                let count = self.transactions.len();
                self.exit_settings_mode();
                self.set_status_message(
                    format!("Exported {} transactions to {}.", count, path.display()),
                    Some(Duration::seconds(4)),
                );
            }
            Err(e) => {
                self.set_status_message(
                    format!("Error exporting to '{}': {}", path.display(), e),
                    None,
                );
            }
        }
    }
}
