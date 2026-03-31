//! Find/Replace Dialog State
//!
//! Manages the state for find and find/replace dialogs in fullscreen modes.

use ratatui::layout::Rect;
use regex::RegexBuilder;

/// Mode for find/replace dialog
#[derive(Debug, Clone, PartialEq)]
pub enum FindReplaceMode {
    /// Find only (Ctrl+F) - available in both fullscreen modes
    Find,
    /// Find and Replace (Ctrl+H) - only in InputFullscreen
    Replace,
}

/// Context where find/replace was opened
#[derive(Debug, Clone, PartialEq)]
pub enum FindReplaceContext {
    /// Opened from ConversationFullscreen (read-only search)
    ConversationView,
    /// Opened from InputFullscreen (editable search + replace)
    InputView,
}

/// Focusable elements in the dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogFocus {
    /// Find input field
    FindInput,
    /// Replace input field (only in Replace mode)
    ReplaceInput,
    /// Case sensitive checkbox
    CaseCheckbox,
    /// Regex checkbox
    RegexCheckbox,
    /// Next button
    NextButton,
    /// Replace button (only in Replace mode)
    ReplaceButton,
    /// Replace All button (only in Replace mode)
    ReplaceAllButton,
}

/// Clickable region for mouse interaction
#[derive(Debug, Clone)]
pub struct ClickRegion {
    pub area: Rect,
    pub element: DialogFocus,
}

/// State for find/replace dialog
#[derive(Debug, Clone)]
pub struct FindReplaceState {
    /// Current mode (Find or Replace)
    pub mode: FindReplaceMode,
    /// Context where dialog was opened from
    pub context: FindReplaceContext,
    /// Search query text
    pub find_query: String,
    /// Cursor position in find query (character index)
    pub find_cursor_pos: usize,
    /// Replace text (only for Replace mode)
    pub replace_text: String,
    /// Cursor position in replace text (character index)
    pub replace_cursor_pos: usize,
    /// Currently focused element
    pub focus: DialogFocus,
    /// Case-sensitive search toggle
    pub case_sensitive: bool,
    /// Regex search toggle
    pub use_regex: bool,
    /// Search matches: (byte_index_start, byte_index_end) in the text
    pub matches: Vec<(usize, usize)>,
    /// Current match index being highlighted
    pub current_match_index: usize,
    /// Status message (e.g., "3/10 matches", "Replaced 5 occurrences")
    pub status_message: Option<String>,
    /// Clickable regions (populated during render)
    pub click_regions: Vec<ClickRegion>,
}

impl FindReplaceState {
    /// Create new Find dialog state
    pub fn new_find(context: FindReplaceContext) -> Self {
        Self {
            mode: FindReplaceMode::Find,
            context,
            find_query: String::new(),
            find_cursor_pos: 0,
            replace_text: String::new(),
            replace_cursor_pos: 0,
            focus: DialogFocus::FindInput,
            case_sensitive: false,
            use_regex: false,
            matches: Vec::new(),
            current_match_index: 0,
            status_message: None,
            click_regions: Vec::new(),
        }
    }

    /// Create new Replace dialog state
    pub fn new_replace(context: FindReplaceContext) -> Self {
        let mut state = Self::new_find(context);
        state.mode = FindReplaceMode::Replace;
        state
    }

    /// Check if current focus is on an input field
    pub fn is_input_focused(&self) -> bool {
        matches!(
            self.focus,
            DialogFocus::FindInput | DialogFocus::ReplaceInput
        )
    }

    /// Get all focusable elements in order for tab navigation
    fn focusable_elements(&self) -> Vec<DialogFocus> {
        let mut elements = vec![DialogFocus::FindInput];
        if self.mode == FindReplaceMode::Replace {
            elements.push(DialogFocus::ReplaceInput);
        }
        elements.push(DialogFocus::CaseCheckbox);
        elements.push(DialogFocus::RegexCheckbox);
        elements.push(DialogFocus::NextButton);
        if self.mode == FindReplaceMode::Replace {
            elements.push(DialogFocus::ReplaceButton);
            elements.push(DialogFocus::ReplaceAllButton);
        }
        elements
    }

    /// Move focus to next element (Tab)
    pub fn focus_next(&mut self) {
        let elements = self.focusable_elements();
        if let Some(idx) = elements.iter().position(|&e| e == self.focus) {
            self.focus = elements[(idx + 1) % elements.len()];
        }
    }

    /// Move focus to previous element (Shift+Tab)
    pub fn focus_prev(&mut self) {
        let elements = self.focusable_elements();
        if let Some(idx) = elements.iter().position(|&e| e == self.focus) {
            self.focus = elements[(idx + elements.len() - 1) % elements.len()];
        }
    }

    /// Set focus to a specific element
    pub fn set_focus(&mut self, element: DialogFocus) {
        // Validate the element is available in current mode
        match element {
            DialogFocus::ReplaceInput
            | DialogFocus::ReplaceButton
            | DialogFocus::ReplaceAllButton => {
                if self.mode == FindReplaceMode::Replace {
                    self.focus = element;
                }
            }
            _ => {
                self.focus = element;
            }
        }
    }

    /// Handle click at position, returns true if a region was clicked
    pub fn handle_click(&mut self, col: u16, row: u16) -> Option<DialogFocus> {
        for region in &self.click_regions {
            if col >= region.area.x
                && col < region.area.x + region.area.width
                && row >= region.area.y
                && row < region.area.y + region.area.height
            {
                return Some(region.element);
            }
        }
        None
    }

    /// Clear click regions (called before render)
    pub fn clear_click_regions(&mut self) {
        self.click_regions.clear();
    }

    /// Add a click region
    pub fn add_click_region(&mut self, area: Rect, element: DialogFocus) {
        self.click_regions.push(ClickRegion { area, element });
    }

    /// Update search matches based on current query and target text
    pub fn update_matches(&mut self, text: &str) {
        self.matches.clear();
        self.current_match_index = 0;

        if self.find_query.is_empty() {
            self.status_message = None;
            return;
        }

        if self.use_regex {
            self.update_matches_regex(text);
        } else {
            self.update_matches_literal(text);
        }

        // Update status message
        if self.matches.is_empty() {
            self.status_message = Some("No matches found".to_string());
        } else {
            self.status_message = Some(format!(
                "{}/{} matches",
                self.current_match_index + 1,
                self.matches.len()
            ));
        }
    }

    fn update_matches_literal(&mut self, text: &str) {
        let (search_text, query) = if self.case_sensitive {
            (text.to_string(), self.find_query.clone())
        } else {
            (text.to_lowercase(), self.find_query.to_lowercase())
        };

        let query_len = query.len();
        let mut start = 0;
        while let Some(pos) = search_text[start..].find(&query) {
            let byte_start = start + pos;
            let byte_end = byte_start + query_len;
            self.matches.push((byte_start, byte_end));
            start = byte_end;
        }
    }

    fn update_matches_regex(&mut self, text: &str) {
        let regex = match RegexBuilder::new(&self.find_query)
            .case_insensitive(!self.case_sensitive)
            .build()
        {
            Ok(r) => r,
            Err(_) => {
                self.status_message = Some("Invalid regex".to_string());
                return;
            }
        };

        for m in regex.find_iter(text) {
            self.matches.push((m.start(), m.end()));
        }
    }

    /// Navigate to next match
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match_index = (self.current_match_index + 1) % self.matches.len();
            self.update_status_message();
        }
    }

    /// Navigate to previous match
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            if self.current_match_index == 0 {
                self.current_match_index = self.matches.len() - 1;
            } else {
                self.current_match_index -= 1;
            }
            self.update_status_message();
        }
    }

    fn update_status_message(&mut self) {
        if !self.matches.is_empty() {
            self.status_message = Some(format!(
                "{}/{} matches",
                self.current_match_index + 1,
                self.matches.len()
            ));
        }
    }

    /// Get current match position (for cursor positioning)
    pub fn current_match_position(&self) -> Option<(usize, usize)> {
        self.matches.get(self.current_match_index).copied()
    }

    /// Replace current match and return whether replacement was successful
    pub fn replace_current(&mut self, text: &mut String) -> Result<(), String> {
        if let Some((start, end)) = self.current_match_position() {
            if end <= text.len() {
                text.replace_range(start..end, &self.replace_text);
                // Update matches after replacement
                self.update_matches(text);
                self.status_message = Some("Replaced 1 occurrence".to_string());
                Ok(())
            } else {
                Err("Match position out of bounds".to_string())
            }
        } else {
            Err("No match to replace".to_string())
        }
    }

    /// Replace all matches and return the count
    pub fn replace_all(&mut self, text: &mut String) -> Result<usize, String> {
        let count = self.matches.len();
        if count == 0 {
            return Err("No matches to replace".to_string());
        }

        // Replace in reverse order to maintain byte indices
        for &(start, end) in self.matches.iter().rev() {
            if end <= text.len() {
                text.replace_range(start..end, &self.replace_text);
            }
        }

        self.status_message = Some(format!("Replaced {} occurrence(s)", count));

        // Update matches after replacement (should now be empty or show new matches)
        self.update_matches(text);
        Ok(count)
    }

    /// Insert a character at cursor position in the focused field
    pub fn insert_char(&mut self, c: char) {
        match self.focus {
            DialogFocus::FindInput => {
                let byte_pos = char_to_byte_index(&self.find_query, self.find_cursor_pos);
                self.find_query.insert(byte_pos, c);
                self.find_cursor_pos += 1;
            }
            DialogFocus::ReplaceInput if self.mode == FindReplaceMode::Replace => {
                let byte_pos = char_to_byte_index(&self.replace_text, self.replace_cursor_pos);
                self.replace_text.insert(byte_pos, c);
                self.replace_cursor_pos += 1;
            }
            _ => {}
        }
    }

    /// Delete character before cursor in the focused field
    pub fn delete_char_backward(&mut self) {
        match self.focus {
            DialogFocus::FindInput if self.find_cursor_pos > 0 => {
                self.find_cursor_pos -= 1;
                let byte_pos = char_to_byte_index(&self.find_query, self.find_cursor_pos);
                if byte_pos < self.find_query.len() {
                    self.find_query.remove(byte_pos);
                }
            }
            DialogFocus::ReplaceInput
                if self.mode == FindReplaceMode::Replace && self.replace_cursor_pos > 0 =>
            {
                self.replace_cursor_pos -= 1;
                let byte_pos = char_to_byte_index(&self.replace_text, self.replace_cursor_pos);
                if byte_pos < self.replace_text.len() {
                    self.replace_text.remove(byte_pos);
                }
            }
            _ => {}
        }
    }

    /// Move cursor left in focused field
    pub fn move_cursor_left(&mut self) {
        match self.focus {
            DialogFocus::FindInput => {
                self.find_cursor_pos = self.find_cursor_pos.saturating_sub(1);
            }
            DialogFocus::ReplaceInput => {
                self.replace_cursor_pos = self.replace_cursor_pos.saturating_sub(1);
            }
            _ => {}
        }
    }

    /// Move cursor right in focused field
    pub fn move_cursor_right(&mut self) {
        match self.focus {
            DialogFocus::FindInput => {
                let max = self.find_query.chars().count();
                if self.find_cursor_pos < max {
                    self.find_cursor_pos += 1;
                }
            }
            DialogFocus::ReplaceInput => {
                let max = self.replace_text.chars().count();
                if self.replace_cursor_pos < max {
                    self.replace_cursor_pos += 1;
                }
            }
            _ => {}
        }
    }

    /// Toggle case sensitivity
    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    /// Toggle regex mode
    pub fn toggle_regex(&mut self) {
        self.use_regex = !self.use_regex;
    }
}

/// Convert character index to byte index in a string
pub fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Convert byte index to character index in a string
pub fn byte_to_char_index(s: &str, byte_idx: usize) -> usize {
    s.char_indices()
        .position(|(i, _)| i == byte_idx)
        .unwrap_or(s.chars().count())
}
