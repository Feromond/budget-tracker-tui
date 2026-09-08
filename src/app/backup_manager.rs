use super::state::{App, AppMode};
use crate::app::settings_types::SettingKey;
use crate::db::backup::{self, BackupEntry, BackupKind};
use chrono::Duration;
use std::io::Error;

impl App {
    pub(crate) fn open_backup_manager(&mut self) {
        if let Err(err) = self.refresh_backups() {
            self.set_status_message(format!("Error listing backups: {}", err), None);
            return;
        }

        self.mode = AppMode::BackupManager;
        self.select_first_backup();
        self.clear_status_message();
    }

    pub(crate) fn exit_backup_manager(&mut self) {
        self.backup_confirm_prompt.clear();
        self.enter_settings_mode();
        self.select_settings_row(SettingKey::ManageBackups);
    }

    pub(crate) fn refresh_backups(&mut self) -> Result<(), Error> {
        self.backup_entries = backup::list(&self.database_path, &self.backup_instance_id)?;
        let len = self.backup_entries.len();
        match self.backup_table_state.selected() {
            _ if len == 0 => self.backup_table_state.select(None),
            Some(index) if index >= len => self.backup_table_state.select(Some(len - 1)),
            Some(_) => {}
            None => self.backup_table_state.select(Some(0)),
        }
        Ok(())
    }

    fn select_first_backup(&mut self) {
        self.backup_table_state
            .select((!self.backup_entries.is_empty()).then_some(0));
    }

    fn selected_backup(&self) -> Option<&BackupEntry> {
        self.backup_table_state
            .selected()
            .and_then(|index| self.backup_entries.get(index))
    }

    pub(crate) fn next_backup(&mut self) {
        let len = self.backup_entries.len();
        if len == 0 {
            return;
        }
        let index = match self.backup_table_state.selected() {
            Some(current) if current + 1 < len => current + 1,
            _ => 0,
        };
        self.backup_table_state.select(Some(index));
    }

    pub(crate) fn previous_backup(&mut self) {
        let len = self.backup_entries.len();
        if len == 0 {
            return;
        }
        let index = match self.backup_table_state.selected() {
            Some(0) | None => len - 1,
            Some(current) => current - 1,
        };
        self.backup_table_state.select(Some(index));
    }

    pub(crate) fn create_backup_now(&mut self) {
        let entry = match backup::create(
            &self.database_path,
            &self.backup_instance_id,
            BackupKind::Manual,
        ) {
            Ok(entry) => entry,
            Err(err) => {
                self.set_status_message(format!("Backup failed: {}", err), None);
                return;
            }
        };

        if let Err(err) = self.refresh_backups() {
            self.set_status_message(format!("Backup taken, but listing failed: {}", err), None);
            return;
        }

        if let Some(index) = self
            .backup_entries
            .iter()
            .position(|candidate| candidate.path == entry.path)
        {
            self.backup_table_state.select(Some(index));
        }
        self.set_status_message(
            format!("Backed up to {}.", entry.path.display()),
            Some(Duration::seconds(4)),
        );
    }

    pub(crate) fn prepare_restore_backup(&mut self) {
        let Some(entry) = self.selected_backup() else {
            return;
        };

        self.backup_confirm_prompt = format!(
            "Restore the {} backup from {}? The current database is backed up first. (y/n)",
            entry.kind.label().to_lowercase(),
            entry.taken_at.format("%Y-%m-%d %H:%M")
        );
        self.mode = AppMode::ConfirmBackupRestore;
    }

    pub(crate) fn prepare_delete_backup(&mut self) {
        let Some(entry) = self.selected_backup() else {
            return;
        };

        let origin = if entry.is_this_device {
            String::new()
        } else {
            format!(" from another device ({})", entry.instance)
        };
        self.backup_confirm_prompt = format!(
            "Delete the backup from {}{}? (y/n)",
            entry.taken_at.format("%Y-%m-%d %H:%M"),
            origin
        );
        self.mode = AppMode::ConfirmBackupDelete;
    }

    pub(crate) fn cancel_backup_confirm(&mut self) {
        self.backup_confirm_prompt.clear();
        self.mode = AppMode::BackupManager;
        self.clear_status_message();
    }

    pub(crate) fn confirm_restore_backup(&mut self) {
        let Some(entry) = self.selected_backup() else {
            self.cancel_backup_confirm();
            return;
        };
        let backup_path = entry.path.clone();
        let taken_at = entry.taken_at;

        let result = backup::restore(&self.database_path, &backup_path, &self.backup_instance_id);

        self.backup_confirm_prompt.clear();
        self.mode = AppMode::BackupManager;

        let safety = match result {
            Ok(safety) => safety,
            Err(err) => {
                self.set_status_message(format!("Restore failed: {}", err), None);
                return;
            }
        };

        // Refreshing ledgers also runs any migrations the restored database needs.
        self.clear_all_filter_fields();
        if let Err(err) = self
            .refresh_ledgers()
            .and_then(|_| self.reload_working_set())
        {
            self.set_status_message(format!("Restored, but reloading failed: {}", err), None);
            return;
        }

        let _ = self.refresh_backups();
        let undo = match safety {
            Some(path) => format!(" Previous data saved as {}.", path.display()),
            None => String::new(),
        };
        self.set_status_message(
            format!(
                "Restored the backup from {}.{}",
                taken_at.format("%Y-%m-%d %H:%M"),
                undo
            ),
            Some(Duration::seconds(6)),
        );
    }

    pub(crate) fn confirm_delete_backup(&mut self) {
        let Some(entry) = self.selected_backup() else {
            self.cancel_backup_confirm();
            return;
        };
        let path = entry.path.clone();
        let taken_at = entry.taken_at;

        let result = backup::delete(&path);

        self.backup_confirm_prompt.clear();
        self.mode = AppMode::BackupManager;

        if let Err(err) = result {
            self.set_status_message(format!("{}", err), None);
            return;
        }

        if let Err(err) = self.refresh_backups() {
            self.set_status_message(format!("Backup deleted, but listing failed: {}", err), None);
            return;
        }
        self.set_status_message(
            format!(
                "Deleted the backup from {}.",
                taken_at.format("%Y-%m-%d %H:%M")
            ),
            Some(Duration::seconds(3)),
        );
    }
}
