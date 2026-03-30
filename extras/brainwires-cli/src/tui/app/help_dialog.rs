//! Help dialog state management.
//!
//! This module contains the state and logic for the interactive help dialog.

use ratatui::layout::Rect;

use crate::tui::help_content::{search_help, HelpCategory, HelpEntry};

/// Focus states within the help dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpFocus {
    /// Search input field
    SearchInput,
    /// Category list on the left
    #[default]
    CategoryList,
    /// Content area on the right
    ContentArea,
}

impl HelpFocus {
    /// Move to next focus area.
    pub fn next(&self) -> Self {
        match self {
            HelpFocus::SearchInput => HelpFocus::CategoryList,
            HelpFocus::CategoryList => HelpFocus::ContentArea,
            HelpFocus::ContentArea => HelpFocus::SearchInput,
        }
    }

    /// Move to previous focus area.
    pub fn prev(&self) -> Self {
        match self {
            HelpFocus::SearchInput => HelpFocus::ContentArea,
            HelpFocus::CategoryList => HelpFocus::SearchInput,
            HelpFocus::ContentArea => HelpFocus::CategoryList,
        }
    }
}

/// Click region for category items.
#[derive(Debug, Clone)]
pub struct CategoryClickRegion {
    pub area: Rect,
    pub category: HelpCategory,
}

/// State for the help dialog.
#[derive(Debug, Clone)]
pub struct HelpDialogState {
    /// Current search query
    pub search_query: String,
    /// Cursor position in search field
    pub search_cursor_pos: usize,
    /// Currently selected category
    pub selected_category: HelpCategory,
    /// Scroll offset for category list (if needed)
    pub category_scroll: usize,
    /// Scroll offset for content area
    pub content_scroll: usize,
    /// Currently focused area
    pub focus: HelpFocus,
    /// Click regions for categories (populated during render)
    pub click_regions: Vec<CategoryClickRegion>,
    /// Cached search results
    cached_search_results: Option<Vec<(HelpCategory, HelpEntry)>>,
}

impl Default for HelpDialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpDialogState {
    /// Create a new help dialog state.
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            search_cursor_pos: 0,
            selected_category: HelpCategory::default(),
            category_scroll: 0,
            content_scroll: 0,
            focus: HelpFocus::CategoryList,
            click_regions: Vec::new(),
            cached_search_results: None,
        }
    }

    /// Move to next category.
    pub fn next_category(&mut self) {
        self.selected_category = self.selected_category.next();
        self.content_scroll = 0;
    }

    /// Move to previous category.
    pub fn prev_category(&mut self) {
        self.selected_category = self.selected_category.prev();
        self.content_scroll = 0;
    }

    /// Scroll content down.
    pub fn scroll_content_down(&mut self, amount: usize) {
        let max_scroll = self.get_max_content_scroll();
        self.content_scroll = (self.content_scroll + amount).min(max_scroll);
    }

    /// Scroll content up.
    pub fn scroll_content_up(&mut self, amount: usize) {
        self.content_scroll = self.content_scroll.saturating_sub(amount);
    }

    /// Get maximum scroll value for content.
    fn get_max_content_scroll(&self) -> usize {
        let entries = if self.is_searching() {
            self.get_search_results().len()
        } else {
            self.selected_category.entries().len()
        };
        entries.saturating_sub(1)
    }

    /// Move to next focus area.
    pub fn focus_next(&mut self) {
        self.focus = self.focus.next();
    }

    /// Move to previous focus area.
    pub fn focus_prev(&mut self) {
        self.focus = self.focus.prev();
    }

    /// Check if we're in search mode.
    pub fn is_searching(&self) -> bool {
        !self.search_query.is_empty()
    }

    /// Get filtered entries based on search query.
    pub fn get_search_results(&self) -> Vec<(HelpCategory, HelpEntry)> {
        if self.search_query.is_empty() {
            return vec![];
        }
        search_help(&self.search_query)
    }

    /// Get current content entries to display.
    pub fn get_current_entries(&self) -> Vec<HelpEntry> {
        if self.is_searching() {
            self.get_search_results()
                .into_iter()
                .map(|(_, entry)| entry)
                .collect()
        } else {
            self.selected_category.entries()
        }
    }

    /// Insert a character into the search query.
    pub fn insert_char(&mut self, c: char) {
        let byte_pos = self.char_to_byte_index(self.search_cursor_pos);
        self.search_query.insert(byte_pos, c);
        self.search_cursor_pos += 1;
        self.content_scroll = 0;
        self.cached_search_results = None;
    }

    /// Delete character before cursor.
    pub fn delete_char_backward(&mut self) -> bool {
        if self.search_cursor_pos == 0 {
            return false;
        }

        self.search_cursor_pos -= 1;
        let byte_pos = self.char_to_byte_index(self.search_cursor_pos);
        if let Some(c) = self.search_query[byte_pos..].chars().next() {
            self.search_query
                .replace_range(byte_pos..byte_pos + c.len_utf8(), "");
            self.cached_search_results = None;
            return true;
        }
        false
    }

    /// Delete character at cursor.
    pub fn delete_char_forward(&mut self) -> bool {
        let byte_pos = self.char_to_byte_index(self.search_cursor_pos);
        if byte_pos < self.search_query.len() {
            if let Some(c) = self.search_query[byte_pos..].chars().next() {
                self.search_query
                    .replace_range(byte_pos..byte_pos + c.len_utf8(), "");
                self.cached_search_results = None;
                return true;
            }
        }
        false
    }

    /// Move cursor left.
    pub fn move_cursor_left(&mut self) {
        if self.search_cursor_pos > 0 {
            self.search_cursor_pos -= 1;
        }
    }

    /// Move cursor right.
    pub fn move_cursor_right(&mut self) {
        let max = self.search_query.chars().count();
        if self.search_cursor_pos < max {
            self.search_cursor_pos += 1;
        }
    }

    /// Move cursor to start.
    pub fn move_cursor_home(&mut self) {
        self.search_cursor_pos = 0;
    }

    /// Move cursor to end.
    pub fn move_cursor_end(&mut self) {
        self.search_cursor_pos = self.search_query.chars().count();
    }

    /// Clear search query.
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_cursor_pos = 0;
        self.content_scroll = 0;
        self.cached_search_results = None;
    }

    /// Convert character index to byte index.
    fn char_to_byte_index(&self, char_idx: usize) -> usize {
        self.search_query
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.search_query.len())
    }

    /// Get text before cursor for rendering.
    pub fn text_before_cursor(&self) -> &str {
        let byte_pos = self.char_to_byte_index(self.search_cursor_pos);
        &self.search_query[..byte_pos]
    }

    /// Get text after cursor for rendering.
    pub fn text_after_cursor(&self) -> &str {
        let byte_pos = self.char_to_byte_index(self.search_cursor_pos);
        &self.search_query[byte_pos..]
    }

    /// Clear click regions (call before render).
    pub fn clear_click_regions(&mut self) {
        self.click_regions.clear();
    }

    /// Add a click region for a category.
    pub fn add_click_region(&mut self, area: Rect, category: HelpCategory) {
        self.click_regions.push(CategoryClickRegion { area, category });
    }

    /// Handle a click at the given position.
    /// Returns true if a category was clicked.
    pub fn handle_click(&mut self, col: u16, row: u16) -> bool {
        for region in &self.click_regions {
            if col >= region.area.x
                && col < region.area.x + region.area.width
                && row >= region.area.y
                && row < region.area.y + region.area.height
            {
                self.selected_category = region.category;
                self.content_scroll = 0;
                self.focus = HelpFocus::CategoryList;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = HelpDialogState::new();
        assert!(state.search_query.is_empty());
        assert_eq!(state.focus, HelpFocus::CategoryList);
        assert_eq!(state.selected_category, HelpCategory::Navigation);
    }

    #[test]
    fn test_category_navigation() {
        let mut state = HelpDialogState::new();
        assert_eq!(state.selected_category, HelpCategory::Navigation);

        state.next_category();
        assert_eq!(state.selected_category, HelpCategory::TextEditing);

        state.prev_category();
        assert_eq!(state.selected_category, HelpCategory::Navigation);
    }

    #[test]
    fn test_focus_cycling() {
        let mut state = HelpDialogState::new();
        state.focus = HelpFocus::CategoryList;

        state.focus_next();
        assert_eq!(state.focus, HelpFocus::ContentArea);

        state.focus_next();
        assert_eq!(state.focus, HelpFocus::SearchInput);

        state.focus_next();
        assert_eq!(state.focus, HelpFocus::CategoryList);
    }

    #[test]
    fn test_search_input() {
        let mut state = HelpDialogState::new();

        state.insert_char('c');
        state.insert_char('t');
        state.insert_char('r');
        state.insert_char('l');

        assert_eq!(state.search_query, "ctrl");
        assert_eq!(state.search_cursor_pos, 4);

        state.delete_char_backward();
        assert_eq!(state.search_query, "ctr");
        assert_eq!(state.search_cursor_pos, 3);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = HelpDialogState::new();
        state.search_query = "test".to_string();
        state.search_cursor_pos = 4;

        state.move_cursor_left();
        assert_eq!(state.search_cursor_pos, 3);

        state.move_cursor_home();
        assert_eq!(state.search_cursor_pos, 0);

        state.move_cursor_end();
        assert_eq!(state.search_cursor_pos, 4);
    }

    #[test]
    fn test_is_searching() {
        let mut state = HelpDialogState::new();
        assert!(!state.is_searching());

        state.insert_char('a');
        assert!(state.is_searching());

        state.clear_search();
        assert!(!state.is_searching());
    }
}
