use crate::db::database::{SCHEMA_VERSION, SqliteDatabase};
use chrono::{Duration, Local, NaiveDateTime};
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

pub const DEFAULT_KEEP: u32 = 5;
pub const MAX_KEEP: u32 = 50;

const BACKUPS_DIR: &str = "backups";
const EXTENSION: &str = "db";
const PARTIAL_SUFFIX: &str = ".partial";
const TIMESTAMP_FORMAT: &str = "%Y%m%d-%H%M%S";
const SEPARATOR: &str = "__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    Auto,
    Manual,
    /// The schema version before the upgrade.
    PreMigrate(i64),
    PreRestore,
}

impl BackupKind {
    fn tag(&self) -> String {
        match self {
            BackupKind::Auto => "auto".to_string(),
            BackupKind::Manual => "manual".to_string(),
            BackupKind::PreMigrate(version) => format!("premigrate-v{}", version),
            BackupKind::PreRestore => "prerestore".to_string(),
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "auto" => Some(BackupKind::Auto),
            "manual" => Some(BackupKind::Manual),
            "prerestore" => Some(BackupKind::PreRestore),
            other => other
                .strip_prefix("premigrate-v")
                .and_then(|version| version.parse().ok())
                .map(BackupKind::PreMigrate),
        }
    }

    pub fn label(&self) -> String {
        match self {
            BackupKind::Auto => "Automatic".to_string(),
            BackupKind::Manual => "Manual".to_string(),
            BackupKind::PreMigrate(version) => format!("Before upgrade (v{})", version),
            BackupKind::PreRestore => "Before restore".to_string(),
        }
    }

    fn is_prunable(&self) -> bool {
        matches!(self, BackupKind::Auto)
    }
}

#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub kind: BackupKind,
    pub taken_at: NaiveDateTime,
    pub instance: String,
    /// Matches the config ID, not the hardware.
    pub is_this_device: bool,
    pub size_bytes: u64,
    pub schema_version: Option<i64>,
}

/// Reduces filename collisions between installs; not guaranteed unique.
pub fn generate_instance_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = RandomState::new().build_hasher().finish();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos() as u64)
        .unwrap_or(0);
    let mixed = seed ^ nanos ^ ((std::process::id() as u64) << 32);
    format!("{:08x}", (mixed ^ (mixed >> 32)) as u32)
}

pub fn backups_dir(database_path: &Path) -> PathBuf {
    let parent = database_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = database_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "database".to_string());
    parent.join(BACKUPS_DIR).join(stem)
}

fn file_name(taken_at: NaiveDateTime, kind: BackupKind, instance: &str) -> String {
    format!(
        "{}{}{}{}{}.{}",
        taken_at.format(TIMESTAMP_FORMAT),
        SEPARATOR,
        kind.tag(),
        SEPARATOR,
        instance,
        EXTENSION
    )
}

fn parse_file_name(stem: &str) -> Option<(NaiveDateTime, BackupKind, String)> {
    let mut fields = stem.split(SEPARATOR);
    let taken_at = NaiveDateTime::parse_from_str(fields.next()?, TIMESTAMP_FORMAT).ok()?;
    let kind = BackupKind::from_tag(fields.next()?)?;
    let instance = fields.next()?.to_string();
    if fields.next().is_some() || instance.is_empty() {
        return None;
    }
    Some((taken_at, kind, instance))
}

pub fn database_has_content(database_path: &Path) -> bool {
    fs::metadata(database_path)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
}

fn schema_version_of(path: &Path) -> Option<i64> {
    SqliteDatabase::new(path)
        .open_connection("backup inspection")
        .ok()?
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .ok()
}

pub fn pending_migration_version(database_path: &Path) -> Option<i64> {
    schema_version_of(database_path).filter(|version| *version < SCHEMA_VERSION)
}

pub fn list(database_path: &Path, instance: &str) -> Result<Vec<BackupEntry>> {
    let dir = backups_dir(database_path);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for candidate in fs::read_dir(&dir)? {
        let candidate = candidate?;
        let path = candidate.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(EXTENSION) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Some((taken_at, kind, owner)) = parse_file_name(stem) else {
            continue;
        };

        entries.push(BackupEntry {
            schema_version: schema_version_of(&path),
            size_bytes: candidate.metadata().map(|meta| meta.len()).unwrap_or(0),
            is_this_device: owner == instance,
            instance: owner,
            taken_at,
            kind,
            path,
        });
    }

    entries.sort_by(|a, b| {
        b.taken_at
            .cmp(&a.taken_at)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(entries)
}

pub fn has_auto_backup_today(database_path: &Path, instance: &str) -> Result<bool> {
    let today = Local::now().date_naive();
    Ok(list(database_path, instance)?.iter().any(|entry| {
        entry.is_this_device && entry.kind == BackupKind::Auto && entry.taken_at.date() == today
    }))
}

pub fn create(database_path: &Path, instance: &str, kind: BackupKind) -> Result<BackupEntry> {
    if !database_has_content(database_path) {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("No database to back up at '{}'.", database_path.display()),
        ));
    }

    let dir = backups_dir(database_path);
    fs::create_dir_all(&dir)?;

    let mut taken_at = Local::now().naive_local();
    let mut path = dir.join(file_name(taken_at, kind, instance));
    while path.exists() {
        taken_at += Duration::seconds(1);
        path = dir.join(file_name(taken_at, kind, instance));
    }

    // VACUUM INTO can leave partial output; keep it out of the backup list.
    let partial = with_partial_suffix(&path);
    let conn = SqliteDatabase::new(database_path).open_connection("backup")?;
    let written = conn.execute_batch(&format!("VACUUM INTO '{}';", sql_literal(&partial)));
    if let Err(err) = written {
        let _ = fs::remove_file(&partial);
        return Err(Error::other(format!(
            "Failed to write backup '{}': {}",
            path.display(),
            err
        )));
    }
    fs::rename(&partial, &path).map_err(|err| {
        let _ = fs::remove_file(&partial);
        Error::other(format!(
            "Failed to finish backup '{}': {}",
            path.display(),
            err
        ))
    })?;

    Ok(BackupEntry {
        schema_version: schema_version_of(&path),
        size_bytes: fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0),
        is_this_device: true,
        instance: instance.to_string(),
        taken_at,
        kind,
        path,
    })
}

fn with_partial_suffix(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(PARTIAL_SUFFIX);
    PathBuf::from(name)
}

fn sql_literal(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

pub fn prune(database_path: &Path, instance: &str, keep: u32) -> Result<usize> {
    remove_partials(&backups_dir(database_path));

    let mut removed = 0;
    for entry in list(database_path, instance)?
        .iter()
        .filter(|entry| entry.is_this_device && entry.kind.is_prunable())
        .skip(keep as usize)
    {
        if fs::remove_file(&entry.path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

fn remove_partials(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some(&PARTIAL_SUFFIX[1..]) {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn delete(backup_path: &Path) -> Result<()> {
    fs::remove_file(backup_path).map_err(|err| {
        Error::other(format!(
            "Failed to delete backup '{}': {}",
            backup_path.display(),
            err
        ))
    })
}

fn verify(backup_path: &Path) -> Result<()> {
    if !database_has_content(backup_path) {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("Backup '{}' is missing or empty.", backup_path.display()),
        ));
    }

    let conn = SqliteDatabase::new(backup_path).open_connection("backup verification")?;
    let check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|err| {
            Error::other(format!(
                "Failed to check backup '{}': {}",
                backup_path.display(),
                err
            ))
        })?;
    if check != "ok" {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Backup '{}' failed its integrity check: {}",
                backup_path.display(),
                check
            ),
        ));
    }

    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|err| {
            Error::other(format!(
                "Failed to read the schema version of '{}': {}",
                backup_path.display(),
                err
            ))
        })?;
    if version > SCHEMA_VERSION {
        return Err(Error::new(
            ErrorKind::Unsupported,
            format!(
                "Backup '{}' was written by a newer version of Budget Tracker \
                 (data version {}; this build supports up to {}).",
                backup_path.display(),
                version,
                SCHEMA_VERSION
            ),
        ));
    }
    Ok(())
}

pub fn restore(
    database_path: &Path,
    backup_path: &Path,
    instance: &str,
) -> Result<Option<PathBuf>> {
    verify(backup_path)?;

    let safety = if database_has_content(database_path) {
        Some(create(database_path, instance, BackupKind::PreRestore)?.path)
    } else {
        None
    };

    let database = SqliteDatabase::new(database_path);
    database.ensure_parent_dir()?;
    fs::copy(backup_path, database_path).map_err(|err| {
        Error::other(format!(
            "Failed to restore '{}' over '{}': {}",
            backup_path.display(),
            database_path.display(),
            err
        ))
    })?;

    // Don't let old journals affect the restored database.
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = database_path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let _ = fs::remove_file(PathBuf::from(sidecar));
    }

    Ok(safety)
}
