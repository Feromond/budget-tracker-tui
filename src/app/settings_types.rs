#[derive(Debug, Clone, PartialEq)]
pub enum SettingType {
    SectionHeader,
    Path,
    Number,
    Integer,
    Toggle,
    Action,
}

/// Stable identity for each settings row, so lookups and dispatch are type-checked rather than
/// relying on string keys. Section headers share `Section` since they are never looked up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKey {
    Section,
    DatabasePath,
    ManageLedgers,
    ManageCategories,
    ImportTransactions,
    ExportTransactions,
    AutomaticBackups,
    BackupsToKeep,
    ManageBackups,
    HourlyRate,
    ShowHours,
    FuzzySearch,
    HideHelpBar,
    RecurringForecastMonths,
}

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub key: SettingKey,
    pub label: String,
    pub value: String,
    pub setting_type: SettingType,
    pub help: String,
}

#[derive(Debug, Default)]
pub struct SettingsState {
    pub items: Vec<SettingItem>,
    pub selected_index: usize,
    // For text editing:
    pub edit_cursor: usize,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected_index: 0,
            edit_cursor: 0,
        }
    }

    pub fn add_setting(
        &mut self,
        key: SettingKey,
        label: &str,
        value: String,
        setting_type: SettingType,
        help: &str,
    ) {
        self.items.push(SettingItem {
            key,
            label: label.to_string(),
            value,
            setting_type,
            help: help.to_string(),
        });
    }

    pub fn add_header(&mut self, label: &str) {
        self.add_setting(
            SettingKey::Section,
            label,
            String::new(),
            SettingType::SectionHeader,
            "",
        );
    }

    pub fn get_value(&self, key: SettingKey) -> Option<&String> {
        self.items
            .iter()
            .find(|item| item.key == key)
            .map(|item| &item.value)
    }
}
