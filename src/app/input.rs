use super::state::App;
use crate::app::fields::{FieldKey, FieldKind};
use crate::app::state::AppMode;
use crate::model::DATE_FORMAT;
use chrono::NaiveDate;

impl App {
    // Returns (content, cursor, kind) for the field being typed into, or None if the current
    // field is not one (a toggle or a picker).
    fn get_active_input_mut(&mut self) -> Option<(&mut String, &mut usize, FieldKind)> {
        match self.mode {
            AppMode::Filtering => Some((
                &mut self.simple_filter_content,
                &mut self.simple_filter_cursor,
                FieldKind::Text,
            )),
            AppMode::CategoryCatalogFilter => Some((
                &mut self.category_filter_query,
                &mut self.category_filter_cursor,
                FieldKind::Text,
            )),
            AppMode::Adding | AppMode::Editing => {
                let field = self.add_edit_fields.focused();
                if !field.kind().is_editable() {
                    return None;
                }
                Some((
                    &mut self.add_edit_fields[field],
                    &mut self.add_edit_cursor,
                    field.kind(),
                ))
            }
            AppMode::CategoryEditor => {
                let field = self.category_edit_fields.focused();
                if !field.kind().is_editable() {
                    return None;
                }
                Some((
                    &mut self.category_edit_fields[field],
                    &mut self.category_edit_cursor,
                    field.kind(),
                ))
            }
            AppMode::InvestmentAccountEditor => {
                let field = self.investment_account_fields.focused();
                if !field.kind().is_editable() {
                    return None;
                }
                Some((
                    &mut self.investment_account_fields[field],
                    &mut self.investment_account_cursor,
                    field.kind(),
                ))
            }
            AppMode::InvestmentEntryEditor => {
                let field = self.investment_entry_fields.focused();
                if !field.kind().is_editable() {
                    return None;
                }
                Some((
                    &mut self.investment_entry_fields[field],
                    &mut self.investment_entry_cursor,
                    field.kind(),
                ))
            }
            AppMode::BudgetCategoryEditor => Some((
                &mut self.budget_edit_input,
                &mut self.budget_edit_cursor,
                FieldKind::Amount,
            )),
            AppMode::LedgerEditor => Some((
                &mut self.ledger_name_input,
                &mut self.ledger_name_cursor,
                FieldKind::Text,
            )),
            AppMode::ImportTransactions | AppMode::ExportTransactions => Some((
                &mut self.io_path_input,
                &mut self.io_path_cursor,
                FieldKind::Text,
            )),
            AppMode::AdvancedFiltering => {
                let field = self.advanced_filter_fields.focused();
                if !field.kind().is_editable() {
                    return None;
                }
                Some((
                    &mut self.advanced_filter_fields[field],
                    &mut self.advanced_filter_cursor,
                    field.kind(),
                ))
            }
            _ => None,
        }
    }

    /// The focused input when it is a date field, so date stepping works in every form
    /// without each one naming its own field.
    pub(crate) fn active_date_input(&mut self) -> Option<(&mut String, &mut usize)> {
        match self.get_active_input_mut() {
            Some((content, cursor, FieldKind::Date)) => Some((content, cursor)),
            _ => None,
        }
    }

    // --- Input Handling ---

    pub(crate) fn move_cursor_left(&mut self) {
        if let Some((content, cursor, _)) = self.get_active_input_mut()
            && *cursor > 0
        {
            let mut prev = *cursor - 1;
            while !content.is_char_boundary(prev) {
                prev -= 1;
            }
            *cursor = prev;
        }
    }

    pub(crate) fn move_cursor_right(&mut self) {
        if let Some((content, cursor, _)) = self.get_active_input_mut()
            && *cursor < content.len()
            && let Some(c) = content[*cursor..].chars().next()
        {
            *cursor += c.len_utf8();
        }
    }

    pub(crate) fn insert_char_at_cursor(&mut self, c: char) {
        if let Some((content, cursor, kind)) = self.get_active_input_mut() {
            match kind {
                FieldKind::Date => {
                    if let Some(new_content) =
                        crate::validation::validate_and_insert_date_char(content, c)
                    {
                        *content = new_content;
                        *cursor = content.len();
                    }
                }
                FieldKind::Amount => {
                    if crate::validation::validate_amount_char(content, c) {
                        if *cursor >= content.len() {
                            content.push(c);
                        } else {
                            content.insert(*cursor, c);
                        }
                        *cursor += c.len_utf8();
                    }
                }
                FieldKind::Text => {
                    if *cursor >= content.len() {
                        content.push(c);
                    } else {
                        content.insert(*cursor, c);
                    }
                    *cursor += c.len_utf8();
                }
                // get_active_input_mut never yields these.
                FieldKind::Toggle | FieldKind::Selection => {}
            }
        }
    }

    pub(crate) fn delete_char_before_cursor(&mut self) {
        if let Some((content, cursor, kind)) = self.get_active_input_mut() {
            match kind {
                FieldKind::Date => {
                    // Date backspace logic is specific
                    crate::validation::handle_date_backspace(content);
                    *cursor = content.len();
                }
                _ => {
                    if *cursor > 0 {
                        let mut prev = *cursor - 1;
                        while !content.is_char_boundary(prev) {
                            prev -= 1;
                        }
                        if prev < content.len() {
                            content.remove(prev);
                            *cursor = prev;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn delete_char_after_cursor(&mut self) {
        if let Some((content, cursor, _)) = self.get_active_input_mut()
            && *cursor < content.len()
        {
            content.remove(*cursor);
        }
    }

    // --- Settings Input ---

    fn strip_quotes_from_current_setting(&mut self) {
        let idx = self.settings_state.selected_index;
        if let Some(item) = self.settings_state.items.get_mut(idx)
            && item.setting_type == crate::app::settings_types::SettingType::Path
        {
            let stripped = crate::validation::strip_path_quotes(&item.value);
            item.value = stripped;
            self.settings_state.edit_cursor = item.value.len();
        }
    }

    pub(crate) fn next_settings_field(&mut self) {
        self.strip_quotes_from_current_setting();

        let len = self.settings_state.items.len();
        if len == 0 {
            return;
        }

        loop {
            self.settings_state.selected_index = (self.settings_state.selected_index + 1) % len;
            let idx = self.settings_state.selected_index;

            // Skip headers
            if self.settings_state.items[idx].setting_type
                != crate::app::settings_types::SettingType::SectionHeader
            {
                self.settings_state.edit_cursor = self.settings_state.items[idx].value.len();
                break;
            }
        }
    }

    pub(crate) fn previous_settings_field(&mut self) {
        self.strip_quotes_from_current_setting();

        let len = self.settings_state.items.len();
        if len == 0 {
            return;
        }

        loop {
            if self.settings_state.selected_index == 0 {
                self.settings_state.selected_index = len - 1;
            } else {
                self.settings_state.selected_index -= 1;
            }
            let idx = self.settings_state.selected_index;

            // Skip headers
            if self.settings_state.items[idx].setting_type
                != crate::app::settings_types::SettingType::SectionHeader
            {
                self.settings_state.edit_cursor = self.settings_state.items[idx].value.len();
                break;
            }
        }
    }

    pub(crate) fn move_cursor_left_settings(&mut self) {
        if self.set_settings_toggle(false) {
            return;
        }
        if self.settings_state.edit_cursor > 0 {
            self.settings_state.edit_cursor -= 1;
        }
    }

    /// Returns true when the row should skip text-cursor movement.
    fn set_settings_toggle(&mut self, enabled: bool) -> bool {
        let idx = self.settings_state.selected_index;
        let Some(item) = self.settings_state.items.get_mut(idx) else {
            return false;
        };
        match item.setting_type {
            crate::app::settings_types::SettingType::Action => return true,
            crate::app::settings_types::SettingType::Toggle => {}
            _ => return false,
        }

        item.value = crate::app::settings::toggle_value(enabled);
        self.settings_state.edit_cursor = self.settings_state.items[idx].value.len();
        self.update_settings_visibility();
        true
    }

    pub(crate) fn move_cursor_right_settings(&mut self) {
        if self.set_settings_toggle(true) {
            return;
        }
        if let Some(item) = self
            .settings_state
            .items
            .get(self.settings_state.selected_index)
            && self.settings_state.edit_cursor < item.value.len()
        {
            self.settings_state.edit_cursor += 1;
        }
    }

    fn insert_char_at_settings_cursor(&mut self, idx: usize, c: char) {
        let item = &mut self.settings_state.items[idx];
        if self.settings_state.edit_cursor >= item.value.len() {
            item.value.push(c);
        } else {
            item.value.insert(self.settings_state.edit_cursor, c);
        }
        self.settings_state.edit_cursor += c.len_utf8();
    }

    pub(crate) fn insert_char_settings(&mut self, c: char) {
        let idx = self.settings_state.selected_index;
        if idx >= self.settings_state.items.len() {
            return;
        }

        let setting_type = self.settings_state.items[idx].setting_type.clone();

        match setting_type {
            crate::app::settings_types::SettingType::SectionHeader => {
                // Do nothing
            }
            crate::app::settings_types::SettingType::Number => {
                if crate::validation::validate_amount_char(&self.settings_state.items[idx].value, c)
                {
                    self.insert_char_at_settings_cursor(idx, c);
                }
            }
            crate::app::settings_types::SettingType::Integer => {
                if c.is_ascii_digit() {
                    self.insert_char_at_settings_cursor(idx, c);
                }
            }
            crate::app::settings_types::SettingType::Path => {
                let item = &mut self.settings_state.items[idx];
                item.value.insert(self.settings_state.edit_cursor, c);

                let original_len = item.value.len();
                let stripped = crate::validation::strip_path_quotes(&item.value);
                let new_len = stripped.len();
                item.value = stripped;

                let chars_removed = original_len - new_len;
                if chars_removed > 0 {
                    if self.settings_state.edit_cursor > chars_removed {
                        self.settings_state.edit_cursor -= chars_removed;
                    } else {
                        self.settings_state.edit_cursor = 0;
                    }
                    self.settings_state.edit_cursor = item.value.len();
                } else {
                    self.settings_state.edit_cursor += c.len_utf8();
                }
            }
            crate::app::settings_types::SettingType::Toggle => {}
            crate::app::settings_types::SettingType::Action => {}
        }
        self.update_settings_visibility();
    }

    pub(crate) fn delete_char_settings(&mut self) {
        let idx = self.settings_state.selected_index;
        if idx >= self.settings_state.items.len() {
            return;
        }

        let setting_type = self.settings_state.items[idx].setting_type.clone();

        match setting_type {
            crate::app::settings_types::SettingType::SectionHeader => {}
            crate::app::settings_types::SettingType::Number
            | crate::app::settings_types::SettingType::Integer => {
                let cursor = self.settings_state.edit_cursor;
                let item = &mut self.settings_state.items[idx];
                if cursor > 0 && !item.value.is_empty() {
                    let mut prev = cursor - 1;
                    while !item.value.is_char_boundary(prev) {
                        prev -= 1;
                    }
                    if prev < item.value.len() {
                        item.value.remove(prev);
                        self.settings_state.edit_cursor = prev;
                    }
                }
            }
            crate::app::settings_types::SettingType::Toggle => {
                // No-op for delete on toggle
            }
            crate::app::settings_types::SettingType::Action => {}
            _ => {
                let cursor = self.settings_state.edit_cursor;
                let item = &mut self.settings_state.items[idx];
                if cursor > 0 && !item.value.is_empty() {
                    let mut prev = cursor - 1;
                    while !item.value.is_char_boundary(prev) {
                        prev -= 1;
                    }
                    if prev < item.value.len() {
                        item.value.remove(prev);
                        self.settings_state.edit_cursor = prev;
                    }

                    if setting_type == crate::app::settings_types::SettingType::Path {
                        let stripped = crate::validation::strip_path_quotes(&item.value);
                        item.value = stripped;
                        if self.settings_state.edit_cursor > item.value.len() {
                            self.settings_state.edit_cursor = item.value.len();
                        }
                    }
                }
            }
        }
        self.update_settings_visibility();
    }

    pub(crate) fn clear_settings_field(&mut self) {
        let idx = self.settings_state.selected_index;
        if let Some(item) = self.settings_state.items.get_mut(idx)
            && item.setting_type != crate::app::settings_types::SettingType::SectionHeader
            && item.setting_type != crate::app::settings_types::SettingType::Action
            && item.setting_type != crate::app::settings_types::SettingType::Toggle
        {
            item.value.clear();
            self.settings_state.edit_cursor = 0;
        }
        self.update_settings_visibility();
    }

    /// Handle bracketed-paste into the active path field. Only the settings path fields and
    /// the import/export prompt accept pasted text; other modes ignore it.
    pub(crate) fn handle_paste(&mut self, text: &str) {
        match self.mode {
            AppMode::Settings => {
                let idx = self.settings_state.selected_index;
                if let Some(item) = self.settings_state.items.get_mut(idx)
                    && item.setting_type == crate::app::settings_types::SettingType::Path
                {
                    let at = self.settings_state.edit_cursor.min(item.value.len());
                    item.value.insert_str(at, text);
                    item.value = crate::validation::strip_path_quotes(&item.value);
                    self.settings_state.edit_cursor = item.value.len();
                }
            }
            AppMode::ImportTransactions | AppMode::ExportTransactions => {
                let at = self.io_path_cursor.min(self.io_path_input.len());
                self.io_path_input.insert_str(at, text);
                self.io_path_input = crate::validation::strip_path_quotes(&self.io_path_input);
                self.io_path_cursor = self.io_path_input.len();
            }
            _ => {}
        }
    }

    // --- Date Navigation ---
    // Date field navigation utilities for input fields - public for use by other modules
    pub fn increment_date_field(&self, field: &str) -> Option<String> {
        if field.is_empty() {
            let today = chrono::Local::now().date_naive();
            return Some(today.format(DATE_FORMAT).to_string());
        }

        if let Ok(date) = NaiveDate::parse_from_str(field, DATE_FORMAT) {
            let new_date = date + chrono::Duration::days(1);
            Some(new_date.format(DATE_FORMAT).to_string())
        } else {
            None
        }
    }

    pub fn decrement_date_field(&self, field: &str) -> Option<String> {
        if field.is_empty() {
            let today = chrono::Local::now().date_naive();
            return Some(today.format(DATE_FORMAT).to_string());
        }

        if let Ok(date) = NaiveDate::parse_from_str(field, DATE_FORMAT) {
            let new_date = date - chrono::Duration::days(1);
            Some(new_date.format(DATE_FORMAT).to_string())
        } else {
            None
        }
    }

    pub fn increment_month_field(&self, field: &str) -> Option<String> {
        if field.is_empty() {
            let today = chrono::Local::now().date_naive();
            return Some(today.format(DATE_FORMAT).to_string());
        }

        if let Ok(date) = NaiveDate::parse_from_str(field, DATE_FORMAT) {
            let new_date = crate::validation::add_months(date, 1);
            Some(new_date.format(DATE_FORMAT).to_string())
        } else {
            None
        }
    }

    pub fn decrement_month_field(&self, field: &str) -> Option<String> {
        if field.is_empty() {
            let today = chrono::Local::now().date_naive();
            return Some(today.format(DATE_FORMAT).to_string());
        }

        if let Ok(date) = NaiveDate::parse_from_str(field, DATE_FORMAT) {
            let new_date = crate::validation::add_months(date, -1);
            Some(new_date.format(DATE_FORMAT).to_string())
        } else {
            None
        }
    }
}
