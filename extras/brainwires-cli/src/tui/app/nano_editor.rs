//! Nano-style Editor State Management
//!
//! Manages the state for the TUI nano-style text editor.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Maximum file size for editing (1MB)
const MAX_EDIT_SIZE: u64 = 1024 * 1024;

/// Cursor movement direction
#[derive(Debug, Clone, Copy)]
pub enum CursorDirection {
    Up,
    Down,
    Left,
    Right,
}

/// State for the nano-style editor
#[derive(Debug, Clone)]
pub struct NanoEditorState {
    /// Path to the file being edited
    pub file_path: PathBuf,
    /// File content as lines
    pub lines: Vec<String>,
    /// Original content (for detecting changes)
    original_lines: Vec<String>,
    /// Cursor row (0-indexed line number)
    pub cursor_row: usize,
    /// Cursor column (0-indexed character position)
    pub cursor_col: usize,
    /// Scroll offset (first visible line)
    pub scroll_row: u16,
    /// Horizontal scroll offset
    pub scroll_col: u16,
    /// Whether the file has been modified
    pub modified: bool,
    /// Search query (for Ctrl+W where-is search)
    pub search_query: Option<String>,
    /// Current search matches (line, col) positions
    pub search_matches: Vec<(usize, usize)>,
    /// Current match index being highlighted
    pub current_match: usize,
    /// Status message (shown in footer)
    pub status_message: Option<String>,
    /// Read-only mode (for viewing binary/large files)
    pub read_only: bool,
    /// Clipboard for cut/copy operations
    pub clipboard: Vec<String>,
    /// File encoding (for display)
    pub encoding: String,
}

impl NanoEditorState {
    /// Open a file for editing
    pub fn open(path: PathBuf) -> Result<Self> {
        // Check file size
        let metadata = std::fs::metadata(&path)
            .with_context(|| format!("Failed to read file metadata: {}", path.display()))?;

        let read_only = metadata.len() > MAX_EDIT_SIZE;

        // Read file content
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        // Check for binary content (null bytes)
        let is_binary = content.bytes().any(|b| b == 0);
        let read_only = read_only || is_binary;

        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };

        let status_message = if is_binary {
            Some("Binary file - Read only".to_string())
        } else if metadata.len() > MAX_EDIT_SIZE {
            Some(format!(
                "Large file ({:.1}MB) - Read only",
                metadata.len() as f64 / 1024.0 / 1024.0
            ))
        } else {
            None
        };

        Ok(Self {
            file_path: path,
            original_lines: lines.clone(),
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            search_query: None,
            search_matches: Vec::new(),
            current_match: 0,
            status_message,
            read_only,
            clipboard: Vec::new(),
            encoding: "UTF-8".to_string(),
        })
    }

    /// Save the file
    pub fn save(&mut self) -> Result<()> {
        if self.read_only {
            return Err(anyhow::anyhow!("File is read-only"));
        }

        let content = self.lines.join("\n");
        std::fs::write(&self.file_path, &content)
            .with_context(|| format!("Failed to write file: {}", self.file_path.display()))?;

        self.original_lines = self.lines.clone();
        self.modified = false;
        self.status_message = Some("File saved".to_string());

        Ok(())
    }

    /// Insert a character at cursor
    pub fn insert_char(&mut self, c: char) {
        if self.read_only {
            return;
        }

        // Ensure we have a line at cursor_row
        while self.lines.len() <= self.cursor_row {
            self.lines.push(String::new());
        }

        let line = &mut self.lines[self.cursor_row];

        // Ensure cursor_col is valid
        let col = self.cursor_col.min(line.len());

        line.insert(col, c);
        self.cursor_col = col + 1;
        self.modified = true;
    }

    /// Delete character before cursor (backspace)
    pub fn delete_backward(&mut self) {
        if self.read_only {
            return;
        }

        if self.cursor_col > 0 {
            // Delete character before cursor
            let line = &mut self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            if col > 0 {
                line.remove(col - 1);
                self.cursor_col = col - 1;
                self.modified = true;
            }
        } else if self.cursor_row > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.modified = true;
        }
    }

    /// Delete character at cursor (delete)
    pub fn delete_forward(&mut self) {
        if self.read_only {
            return;
        }

        let line = &self.lines[self.cursor_row];
        if self.cursor_col < line.len() {
            // Delete character at cursor
            let line = &mut self.lines[self.cursor_row];
            line.remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Join with next line
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
            self.modified = true;
        }
    }

    /// Insert a new line at cursor
    pub fn insert_newline(&mut self) {
        if self.read_only {
            return;
        }

        let line = &self.lines[self.cursor_row];
        let col = self.cursor_col.min(line.len());

        // Split the current line
        let remainder = self.lines[self.cursor_row][col..].to_string();
        self.lines[self.cursor_row].truncate(col);

        // Insert new line
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, remainder);
        self.cursor_col = 0;
        self.modified = true;
    }

    /// Move cursor in direction
    pub fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    // Clamp column to line length
                    let line_len = self.lines[self.cursor_row].len();
                    self.cursor_col = self.cursor_col.min(line_len);
                }
            }
            CursorDirection::Down => {
                if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    // Clamp column to line length
                    let line_len = self.lines[self.cursor_row].len();
                    self.cursor_col = self.cursor_col.min(line_len);
                }
            }
            CursorDirection::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    // Move to end of previous line
                    self.cursor_row -= 1;
                    self.cursor_col = self.lines[self.cursor_row].len();
                }
            }
            CursorDirection::Right => {
                let line_len = self.lines[self.cursor_row].len();
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.lines.len() {
                    // Move to start of next line
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
        }
    }

    /// Move to start/end of line
    pub fn move_to_line_boundary(&mut self, start: bool) {
        if start {
            self.cursor_col = 0;
        } else {
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    /// Page up/down
    pub fn page_move(&mut self, up: bool, page_size: usize) {
        if up {
            self.cursor_row = self.cursor_row.saturating_sub(page_size);
        } else {
            self.cursor_row = (self.cursor_row + page_size).min(self.lines.len().saturating_sub(1));
        }
        // Clamp column to line length
        let line_len = self.lines[self.cursor_row].len();
        self.cursor_col = self.cursor_col.min(line_len);
    }

    /// Cut current line
    pub fn cut_line(&mut self) {
        if self.read_only {
            return;
        }

        if !self.lines.is_empty() {
            let line = self.lines.remove(self.cursor_row);
            self.clipboard.push(line);

            // Ensure we have at least one line
            if self.lines.is_empty() {
                self.lines.push(String::new());
            }

            // Adjust cursor
            if self.cursor_row >= self.lines.len() {
                self.cursor_row = self.lines.len().saturating_sub(1);
            }
            self.cursor_col = self.cursor_col.min(self.lines[self.cursor_row].len());
            self.modified = true;
        }
    }

    /// Paste clipboard content
    pub fn paste(&mut self) {
        if self.read_only {
            return;
        }

        if !self.clipboard.is_empty() {
            for (i, line) in self.clipboard.clone().iter().enumerate() {
                self.lines.insert(self.cursor_row + i, line.clone());
            }
            self.cursor_row += self.clipboard.len();
            self.modified = true;
        }
    }

    /// Search for text
    pub fn search(&mut self, query: &str) {
        self.search_query = Some(query.to_string());
        self.search_matches.clear();
        self.current_match = 0;

        let query_lower = query.to_lowercase();

        for (row, line) in self.lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let mut start = 0;
            while let Some(col) = line_lower[start..].find(&query_lower) {
                self.search_matches.push((row, start + col));
                start += col + 1;
            }
        }

        // Move to first match
        if let Some(&(row, col)) = self.search_matches.first() {
            self.cursor_row = row;
            self.cursor_col = col;
        }
    }

    /// Go to next/previous match
    pub fn next_match(&mut self, forward: bool) {
        if self.search_matches.is_empty() {
            return;
        }

        if forward {
            self.current_match = (self.current_match + 1) % self.search_matches.len();
        } else {
            self.current_match = if self.current_match == 0 {
                self.search_matches.len() - 1
            } else {
                self.current_match - 1
            };
        }

        let (row, col) = self.search_matches[self.current_match];
        self.cursor_row = row;
        self.cursor_col = col;
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_matches.clear();
        self.current_match = 0;
    }

    /// Has unsaved changes
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Ensure cursor is visible in viewport
    pub fn ensure_cursor_visible(&mut self, visible_rows: u16, visible_cols: u16) {
        // Vertical scrolling
        let cursor_row = self.cursor_row as u16;
        if cursor_row < self.scroll_row {
            self.scroll_row = cursor_row;
        } else if cursor_row >= self.scroll_row + visible_rows {
            self.scroll_row = cursor_row.saturating_sub(visible_rows - 1);
        }

        // Horizontal scrolling
        let cursor_col = self.cursor_col as u16;
        if cursor_col < self.scroll_col {
            self.scroll_col = cursor_col;
        } else if cursor_col >= self.scroll_col + visible_cols {
            self.scroll_col = cursor_col.saturating_sub(visible_cols - 1);
        }
    }

    /// Get the current line content
    pub fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get the file name for display
    pub fn file_name(&self) -> String {
        self.file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.file_path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_open_file() {
        let file = create_temp_file("line 1\nline 2\nline 3");
        let state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        assert_eq!(state.lines.len(), 3);
        assert_eq!(state.lines[0], "line 1");
        assert_eq!(state.lines[1], "line 2");
        assert_eq!(state.lines[2], "line 3");
        assert!(!state.modified);
        assert!(!state.read_only);
    }

    #[test]
    fn test_insert_char() {
        let file = create_temp_file("hello");
        let mut state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        state.cursor_col = 5;
        state.insert_char('!');

        assert_eq!(state.lines[0], "hello!");
        assert!(state.modified);
    }

    #[test]
    fn test_delete_backward() {
        let file = create_temp_file("hello");
        let mut state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        state.cursor_col = 5;
        state.delete_backward();

        assert_eq!(state.lines[0], "hell");
        assert_eq!(state.cursor_col, 4);
    }

    #[test]
    fn test_insert_newline() {
        let file = create_temp_file("hello world");
        let mut state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        state.cursor_col = 5;
        state.insert_newline();

        assert_eq!(state.lines.len(), 2);
        assert_eq!(state.lines[0], "hello");
        assert_eq!(state.lines[1], " world");
        assert_eq!(state.cursor_row, 1);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let file = create_temp_file("line 1\nline 2");
        let mut state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        state.move_cursor(CursorDirection::Down);
        assert_eq!(state.cursor_row, 1);

        state.move_cursor(CursorDirection::Up);
        assert_eq!(state.cursor_row, 0);

        state.move_cursor(CursorDirection::Right);
        assert_eq!(state.cursor_col, 1);

        state.move_cursor(CursorDirection::Left);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_cut_paste() {
        let file = create_temp_file("line 1\nline 2\nline 3");
        let mut state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        state.cursor_row = 1;
        state.cut_line();

        assert_eq!(state.lines.len(), 2);
        assert_eq!(state.lines[0], "line 1");
        assert_eq!(state.lines[1], "line 3");
        assert_eq!(state.clipboard.len(), 1);
        assert_eq!(state.clipboard[0], "line 2");

        state.cursor_row = 0;
        state.paste();

        assert_eq!(state.lines.len(), 3);
        assert_eq!(state.lines[0], "line 2");
        assert_eq!(state.lines[1], "line 1");
    }

    #[test]
    fn test_search() {
        let file = create_temp_file("foo bar\nbaz foo\nfoo");
        let mut state = NanoEditorState::open(file.path().to_path_buf()).unwrap();

        state.search("foo");

        assert_eq!(state.search_matches.len(), 3);
        assert_eq!(state.search_matches[0], (0, 0));
        assert_eq!(state.search_matches[1], (1, 4));
        assert_eq!(state.search_matches[2], (2, 0));
    }
}
