use super::settings_types::SettingKey;
use super::state::App;
use crate::app::places::Place;
use crate::app::util::TypeToSelect;
use chrono::{DateTime, Local, NaiveDate};
use ratatui::widgets::{ListState, TableState};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowseTarget {
    DatabasePath,
    Import,
    Export,
}

impl BrowseTarget {
    fn extensions(self) -> &'static [&'static str] {
        match self {
            BrowseTarget::DatabasePath => &["db", "sqlite", "sqlite3"],
            BrowseTarget::Import | BrowseTarget::Export => &["csv"],
        }
    }

    fn saves(self) -> bool {
        matches!(self, BrowseTarget::DatabasePath | BrowseTarget::Export)
    }

    fn default_file_name(self) -> &'static str {
        match self {
            BrowseTarget::DatabasePath => "budget.db",
            BrowseTarget::Import | BrowseTarget::Export => "transactions.csv",
        }
    }

    pub(crate) fn extension_list(self) -> String {
        self.extensions()
            .iter()
            .map(|ext| format!(".{ext}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(crate) fn use_folder_label(self) -> &'static str {
        match self {
            BrowseTarget::Export => "Save here as ",
            BrowseTarget::DatabasePath | BrowseTarget::Import => "Use this folder for ",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            BrowseTarget::DatabasePath => " Choose Database File ",
            BrowseTarget::Import => " Choose CSV to Import ",
            BrowseTarget::Export => " Choose Export Location ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Parent,
    UseFolder,
    Folder,
    File,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserEntry {
    pub(crate) name: String,
    pub(crate) kind: EntryKind,
    pub(crate) size: Option<u64>,
    pub(crate) modified: Option<NaiveDate>,
}

impl BrowserEntry {
    fn new(name: String, kind: EntryKind) -> Self {
        Self {
            name,
            kind,
            size: None,
            modified: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pane {
    Places,
    Files,
}

pub(crate) struct FileBrowser {
    pub(crate) target: BrowseTarget,
    pub(crate) dir: PathBuf,
    pub(crate) entries: Vec<BrowserEntry>,
    pub(crate) table_state: TableState,
    pub(crate) places: Vec<Place>,
    pub(crate) places_state: ListState,
    pub(crate) focus: Pane,
    pub(crate) show_hidden: bool,
    pub(crate) error: Option<String>,
    file_name: String,
    /// The original file, even after browsing elsewhere.
    pub(crate) current: Option<PathBuf>,
    type_to_select: TypeToSelect,
}

impl FileBrowser {
    pub(crate) fn open(target: BrowseTarget, current: &str) -> Self {
        let current = crate::validation::resolve_user_path(current).map(|path| normalize(&path));
        let file_name = current
            .as_deref()
            .filter(|path| !path.is_dir())
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| target.default_file_name().to_string());
        let candidates: Vec<PathBuf> = current
            .as_deref()
            .map(|path| path.ancestors().map(Path::to_path_buf).collect())
            .unwrap_or_default();
        let mut browser = Self {
            target,
            dir: PathBuf::new(),
            entries: Vec::new(),
            table_state: TableState::default(),
            places: crate::app::places::discover(),
            places_state: ListState::default(),
            focus: Pane::Files,
            show_hidden: false,
            error: None,
            file_name,
            current: current.filter(|path| path.is_file()),
            type_to_select: TypeToSelect::new(),
        };
        let mut first_error = None;
        for dir in candidates
            .into_iter()
            .filter(|dir| dir.is_dir())
            .chain(dirs::home_dir())
        {
            match browser.enter(dir) {
                Ok(()) => break,
                Err(err) => {
                    first_error.get_or_insert(err);
                }
            }
        }
        browser.error = first_error;
        browser.places_state.select(browser.current_place());
        browser
    }

    pub(crate) fn is_typing(&self) -> bool {
        self.focus == Pane::Files && self.type_to_select.is_typing()
    }

    fn enter(&mut self, dir: PathBuf) -> Result<(), String> {
        let read =
            fs::read_dir(&dir).map_err(|err| format!("Can't open '{}': {}", dir.display(), err))?;
        let extensions = self.target.extensions();
        let mut folders = Vec::new();
        let mut files = Vec::new();
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            // Follow symlinks to check their targets.
            let is_link = entry.file_type().is_ok_and(|kind| kind.is_symlink());
            let is_dir = if is_link {
                path.is_dir()
            } else {
                entry.file_type().is_ok_and(|kind| kind.is_dir())
            };
            if is_dir {
                folders.push(BrowserEntry::new(name, EntryKind::Folder));
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.iter().any(|want| ext.eq_ignore_ascii_case(want)))
            {
                let meta = if is_link {
                    fs::metadata(&path).ok()
                } else {
                    entry.metadata().ok()
                };
                files.push(BrowserEntry {
                    size: meta.as_ref().map(fs::Metadata::len),
                    modified: meta
                        .and_then(|meta| meta.modified().ok())
                        .map(|time| DateTime::<Local>::from(time).date_naive()),
                    ..BrowserEntry::new(name, EntryKind::File)
                });
            }
        }
        folders.sort_by_cached_key(|entry| entry.name.to_lowercase());
        files.sort_by_cached_key(|entry| entry.name.to_lowercase());

        let mut entries = Vec::new();
        if dir.parent().is_some() {
            entries.push(BrowserEntry::new("..".to_string(), EntryKind::Parent));
        }
        if self.target.saves() {
            entries.push(BrowserEntry::new(
                self.file_name.clone(),
                EntryKind::UseFolder,
            ));
        }
        entries.extend(folders);
        entries.extend(files);

        self.dir = dir;
        self.entries = entries;
        self.error = None;
        self.type_to_select.clear();
        let first = self
            .entries
            .iter()
            .position(|entry| entry.kind != EntryKind::Parent)
            .unwrap_or(0);
        self.table_state
            .select((!self.entries.is_empty()).then_some(first));
        Ok(())
    }

    fn open_dir(&mut self, dir: PathBuf) {
        if let Err(err) = self.enter(dir) {
            self.error = Some(err);
        }
    }

    pub(crate) fn is_current(&self, entry: &BrowserEntry) -> bool {
        entry.kind == EntryKind::File
            && self
                .current
                .as_deref()
                .is_some_and(|current| current == self.dir.join(&entry.name))
    }

    pub(crate) fn selected(&self) -> Option<&BrowserEntry> {
        self.table_state
            .selected()
            .and_then(|index| self.entries.get(index))
    }

    pub(crate) fn current_place(&self) -> Option<usize> {
        self.places
            .iter()
            .enumerate()
            .filter_map(|(index, place)| Some((index, place.path.as_ref()?)))
            .filter(|(_, path)| self.dir.starts_with(path))
            .max_by_key(|(_, path)| path.components().count())
            .map(|(index, _)| index)
    }

    pub(crate) fn activate(&mut self) -> Option<PathBuf> {
        if self.focus == Pane::Places {
            self.open_selected_place();
            return None;
        }
        let entry = self.selected()?.clone();
        match entry.kind {
            EntryKind::Parent => {
                self.go_up();
                None
            }
            EntryKind::Folder => {
                self.open_dir(self.dir.join(&entry.name));
                None
            }
            EntryKind::UseFolder | EntryKind::File => Some(self.dir.join(&entry.name)),
        }
    }

    fn open_selected_place(&mut self) {
        let Some(path) = self
            .places_state
            .selected()
            .and_then(|index| self.places.get(index))
            .and_then(|place| place.path.clone())
        else {
            return;
        };
        self.open_dir(path);
        if self.error.is_none() {
            self.focus = Pane::Files;
        }
    }

    pub(crate) fn enter_selected(&mut self) {
        if self.focus == Pane::Places {
            self.open_selected_place();
        } else if let Some(entry) = self.selected()
            && entry.kind == EntryKind::Folder
        {
            let dir = self.dir.join(&entry.name);
            self.open_dir(dir);
        }
    }

    pub(crate) fn go_up(&mut self) {
        if self.focus == Pane::Places {
            self.focus = Pane::Files;
            return;
        }
        let Some(parent) = self.dir.parent().map(Path::to_path_buf) else {
            return;
        };
        let left = self
            .dir
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
        self.open_dir(parent);
        if let Some(index) = left.and_then(|left| {
            self.entries
                .iter()
                .position(|entry| entry.kind == EntryKind::Folder && entry.name == left)
        }) {
            self.table_state.select(Some(index));
        }
    }

    pub(crate) fn go_home(&mut self) {
        if let Some(home) = dirs::home_dir() {
            self.open_dir(home);
            self.focus = Pane::Files;
        }
    }

    pub(crate) fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Pane::Places => Pane::Files,
            Pane::Files => {
                if self.places_state.selected().is_none() {
                    self.places_state
                        .select(self.current_place().or_else(|| self.next_place(0, 1)));
                }
                Pane::Places
            }
        };
    }

    pub(crate) fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        let selected = self.selected().map(|entry| entry.name.clone());
        self.open_dir(self.dir.clone());
        if let Some(index) =
            selected.and_then(|name| self.entries.iter().position(|entry| entry.name == name))
        {
            self.table_state.select(Some(index));
        }
    }

    fn next_place(&self, from: usize, step: isize) -> Option<usize> {
        let mut index = from as isize;
        while (0..self.places.len() as isize).contains(&index) {
            if self.places[index as usize].path.is_some() {
                return Some(index as usize);
            }
            index += step;
        }
        None
    }

    pub(crate) fn move_by(&mut self, delta: isize) {
        match self.focus {
            Pane::Places => {
                let current = self.places_state.selected().unwrap_or(0);
                let step = delta.signum();
                let target = current.saturating_add_signed(delta);
                if let Some(index) = self
                    .next_place(target.min(self.places.len().saturating_sub(1)), step)
                    .or_else(|| self.next_place(current, step))
                {
                    self.places_state.select(Some(index));
                }
            }
            Pane::Files => {
                if self.entries.is_empty() {
                    return;
                }
                let last = self.entries.len() - 1;
                let current = self.table_state.selected().unwrap_or(0);
                let next = current.saturating_add_signed(delta).min(last);
                self.table_state.select(Some(next));
            }
        }
    }

    pub(crate) fn select_first(&mut self) {
        match self.focus {
            Pane::Places => self.places_state.select(self.next_place(0, 1)),
            Pane::Files if !self.entries.is_empty() => self.table_state.select(Some(0)),
            Pane::Files => {}
        }
    }

    pub(crate) fn select_last(&mut self) {
        match self.focus {
            Pane::Places => self
                .places_state
                .select(self.next_place(self.places.len().saturating_sub(1), -1)),
            Pane::Files if !self.entries.is_empty() => {
                self.table_state.select(Some(self.entries.len() - 1))
            }
            Pane::Files => {}
        }
    }

    pub(crate) fn type_char(&mut self, c: char) {
        if self.focus == Pane::Places {
            return;
        }
        let names: Vec<&str> = self
            .entries
            .iter()
            .map(|entry| match entry.kind {
                EntryKind::Folder | EntryKind::File => entry.name.as_str(),
                EntryKind::Parent | EntryKind::UseFolder => "",
            })
            .collect();
        if let Some(index) = self.type_to_select.handle_char(c, &names, |name| name) {
            self.table_state.select(Some(index));
        }
    }
}

// Keep symlink paths for Places matching; only resolve them before `..`.
fn normalize(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let is_link = std::fs::symlink_metadata(&normal)
                    .is_ok_and(|meta| meta.file_type().is_symlink());
                if let Some(target) = is_link
                    .then(|| std::fs::canonicalize(&normal).ok())
                    .flatten()
                {
                    normal = target;
                }
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}

impl App {
    pub(crate) fn open_file_browser(&mut self, target: BrowseTarget) {
        let current = match target {
            BrowseTarget::DatabasePath => self
                .settings_state
                .get_value(SettingKey::DatabasePath)
                .cloned()
                .unwrap_or_default(),
            BrowseTarget::Import | BrowseTarget::Export => self.io_path_input.clone(),
        };
        self.clear_status_message();
        self.file_browser = Some(FileBrowser::open(target, &current));
    }

    pub(crate) fn close_file_browser(&mut self) {
        self.file_browser = None;
    }

    pub(crate) fn choose_file_browser_entry(&mut self) {
        let Some(browser) = self.file_browser.as_mut() else {
            return;
        };
        let Some(path) = browser.activate() else {
            return;
        };
        let target = browser.target;
        let path = path.to_string_lossy().to_string();
        match target {
            BrowseTarget::DatabasePath => {
                if let Some(item) = self
                    .settings_state
                    .items
                    .iter_mut()
                    .find(|item| item.key == SettingKey::DatabasePath)
                {
                    item.value = path;
                    self.settings_state.edit_cursor = item.value.len();
                }
            }
            BrowseTarget::Import | BrowseTarget::Export => {
                self.io_path_cursor = path.len();
                self.io_path_input = path;
            }
        }
        self.file_browser = None;
    }
}
