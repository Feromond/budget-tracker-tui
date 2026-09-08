use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::{Error, ErrorKind, Read, Write};
use std::path::PathBuf;

const APP_CONFIG_SUBDIR: &str = "BudgetTracker";
const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct AppSettings {
    pub(crate) data_file_path: Option<String>,
    pub(crate) database_path: Option<String>,
    pub(crate) target_budget: Option<Decimal>,
    pub(crate) hourly_rate: Option<Decimal>,
    pub(crate) show_hours: Option<bool>,
    pub(crate) fuzzy_search_mode: Option<bool>,
    pub(crate) hide_help_bar: Option<bool>,
    pub(crate) recurring_forecast_months: Option<u32>,
    pub(crate) backups_enabled: Option<bool>,
    pub(crate) backup_keep: Option<u32>,
    /// Identifies this install in backup filenames.
    pub(crate) instance_id: Option<String>,
}

fn get_config_file_path() -> Result<PathBuf, Error> {
    match dirs::config_dir() {
        Some(mut path) => {
            path.push(APP_CONFIG_SUBDIR);
            create_dir_all(&path)?; // Ensure the directory exists
            path.push(CONFIG_FILE_NAME);
            Ok(path)
        }
        None => Err(Error::new(
            ErrorKind::NotFound,
            "Could not find user config directory",
        )),
    }
}

pub(crate) fn load_settings() -> Result<AppSettings, Error> {
    let config_path = get_config_file_path()?;

    if !config_path.exists() {
        return Ok(AppSettings::default());
    }

    let mut file = File::open(config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    serde_json::from_str(&contents).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("Failed to parse config file: {}", e),
        )
    })
}

pub(crate) fn save_settings(settings: &AppSettings) -> Result<(), Error> {
    let config_path = get_config_file_path()?;

    let contents = serde_json::to_string_pretty(settings)
        .map_err(|e| Error::other(format!("Failed to serialize settings: {}", e)))?;

    let mut file = File::create(config_path)?;
    file.write_all(contents.as_bytes())?;

    Ok(())
}
