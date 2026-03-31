//! File Explorer State Management
//!
//! Manages the state for the TUI file explorer popup.
//!
//! Security: The file explorer is jailed to the user's home directory
//! to prevent directory traversal attacks.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Mode within the file explorer
#[derive(Debug, Clone, PartialEq)]
pub enum FileExplorerMode {
    /// Browsing directories and files
    Browser,
    /// Filtering files with search query
    Search,
}

/// Entry type for display
#[derive(Debug, Clone)]
pub enum EntryType {
    /// Directory entry
    Directory,
    /// Regular file with size and extension
    File {
        size: u64,
        extension: Option<String>,
    },
    /// Symbolic link with target path
    Symlink { target: PathBuf },
    /// Parent directory entry (..)
    ParentDir,
}

/// A file/directory entry in the explorer
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Display name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Type of entry
    pub entry_type: EntryType,
}

/// State for the file explorer
#[derive(Debug, Clone)]
pub struct FileExplorerState {
    /// Current directory being browsed
    pub current_dir: PathBuf,
    /// Root directory that bounds navigation (security jail)
    root_jail: PathBuf,
    /// List of entries in current directory
    pub entries: Vec<FileEntry>,
    /// Currently highlighted entry index
    pub cursor_index: usize,
    /// Scroll offset for long lists
    pub scroll: u16,
    /// Set of selected file paths (for multi-select)
    pub selected_files: HashSet<PathBuf>,
    /// Current mode (browser or search)
    pub mode: FileExplorerMode,
    /// Search/filter query when in search mode
    pub search_query: String,
    /// Filtered entries based on search (indices into entries)
    pub filtered_indices: Option<Vec<usize>>,
    /// Error message to display
    pub error: Option<String>,
    /// Show hidden files (dotfiles)
    pub show_hidden: bool,
    /// Navigation history for back navigation
    pub history: Vec<PathBuf>,
}

impl FileExplorerState {
    /// Allowed read-only paths outside home directory
    /// These are system directories that are safe to browse but should be read-only
    const ALLOWED_READONLY_PATHS: [&str; 4] = [
        "/etc",       // System configuration
        "/usr/share", // Shared data
        "/opt",       // Optional software
        "/tmp",       // Temporary files (for testing)
    ];

    /// Create new state starting at the given directory
    ///
    /// The file explorer allows browsing:
    /// - User's home directory (full access)
    /// - /etc, /usr/share, /opt, /tmp (read-only system directories)
    ///
    /// This prevents access to sensitive system directories like /root, /var, /proc
    pub fn new(start_dir: PathBuf) -> Self {
        // Determine the primary root jail (user's home directory)
        let root_jail = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        // Canonicalize the start directory, falling back to home if invalid
        let canonical_start = start_dir
            .canonicalize()
            .unwrap_or_else(|_| root_jail.clone());

        // Ensure start_dir is within an allowed path
        let validated_start = if Self::is_allowed_path_static(&canonical_start, &root_jail) {
            canonical_start
        } else {
            tracing::warn!(
                "Start directory {} is not in an allowed location, using home instead",
                start_dir.display()
            );
            root_jail.clone()
        };

        let mut state = Self {
            current_dir: validated_start,
            root_jail,
            entries: Vec::new(),
            cursor_index: 0,
            scroll: 0,
            selected_files: HashSet::new(),
            mode: FileExplorerMode::Browser,
            search_query: String::new(),
            filtered_indices: None,
            error: None,
            show_hidden: false,
            history: Vec::new(),
        };
        // Refresh to populate entries
        let _ = state.refresh();
        state
    }

    /// Check if a path is within allowed locations (static version for use in new())
    fn is_allowed_path_static(path: &Path, home_dir: &Path) -> bool {
        // Home directory is always allowed
        if path.starts_with(home_dir) {
            return true;
        }

        // Check against allowed read-only paths
        for allowed in Self::ALLOWED_READONLY_PATHS {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// Check if a path is within the allowed locations
    fn is_within_jail(&self, path: &Path) -> bool {
        match path.canonicalize() {
            Ok(canonical) => Self::is_allowed_path_static(&canonical, &self.root_jail),
            Err(_) => false,
        }
    }

    /// Refresh the entries from filesystem
    pub fn refresh(&mut self) -> Result<()> {
        self.entries.clear();
        self.error = None;

        // Add parent directory entry only if we're not at the jail root
        if let Some(parent) = self.current_dir.parent() {
            // Only show parent if it's still within the jail
            if self.is_within_jail(parent) {
                self.entries.push(FileEntry {
                    name: "..".to_string(),
                    path: parent.to_path_buf(),
                    entry_type: EntryType::ParentDir,
                });
            }
        }

        // Read directory entries
        let read_dir = std::fs::read_dir(&self.current_dir)
            .with_context(|| format!("Failed to read directory: {}", self.current_dir.display()))?;

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files if not showing them
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }

            let path = entry.path();
            let metadata = entry.metadata();

            let entry_type = if let Ok(meta) = &metadata {
                if meta.is_dir() {
                    EntryType::Directory
                } else if meta.file_type().is_symlink() {
                    let target = std::fs::read_link(&path).unwrap_or_default();
                    EntryType::Symlink { target }
                } else {
                    let extension = path.extension().map(|e| e.to_string_lossy().to_string());
                    EntryType::File {
                        size: meta.len(),
                        extension,
                    }
                }
            } else {
                EntryType::File {
                    size: 0,
                    extension: None,
                }
            };

            let file_entry = FileEntry {
                name,
                path,
                entry_type: entry_type.clone(),
            };

            match entry_type {
                EntryType::Directory => dirs.push(file_entry),
                _ => files.push(file_entry),
            }
        }

        // Sort directories and files alphabetically (case-insensitive)
        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Add directories first, then files
        self.entries.extend(dirs);
        self.entries.extend(files);

        // Reset cursor if out of bounds
        if self.cursor_index >= self.entries.len() {
            self.cursor_index = self.entries.len().saturating_sub(1);
        }

        // Clear search filter
        self.filtered_indices = None;
        self.search_query.clear();

        Ok(())
    }

    /// Navigate into a directory
    ///
    /// Security: Validates the path is within allowed locations before navigating.
    pub fn enter_directory(&mut self, path: &Path) -> Result<()> {
        // Resolve symlinks and canonicalize
        let canonical = path
            .canonicalize()
            .with_context(|| format!("Cannot access directory: {}", path.display()))?;

        // Security: Validate path is within allowed locations
        if !Self::is_allowed_path_static(&canonical, &self.root_jail) {
            self.error = Some("Cannot navigate to this location".to_string());
            return Ok(());
        }

        // Verify it's a directory
        if !canonical.is_dir() {
            self.error = Some("Path is not a directory".to_string());
            return Ok(());
        }

        // Save current directory to history
        self.history.push(self.current_dir.clone());

        // Change to new directory
        self.current_dir = canonical;
        self.cursor_index = 0;
        self.scroll = 0;

        self.refresh()
    }

    /// Go to parent directory
    ///
    /// Security: Will not navigate to disallowed locations.
    pub fn go_up(&mut self) -> Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            let parent_path = parent.to_path_buf();

            // Security: Don't navigate to disallowed locations
            if !self.is_within_jail(&parent_path) {
                self.error = Some("Cannot navigate to parent directory".to_string());
                return Ok(());
            }

            self.history.push(self.current_dir.clone());
            self.current_dir = parent_path;
            self.cursor_index = 0;
            self.scroll = 0;
            self.refresh()?;
        }
        Ok(())
    }

    /// Go back in history
    ///
    /// Security: Validates the history entry is still within the jail.
    pub fn go_back(&mut self) {
        if let Some(prev_dir) = self.history.pop() {
            // Security: Validate the history entry is still within the jail
            if self.is_within_jail(&prev_dir) {
                self.current_dir = prev_dir;
                self.cursor_index = 0;
                self.scroll = 0;
                let _ = self.refresh();
            } else {
                self.error = Some("Cannot navigate to previous directory".to_string());
            }
        }
    }

    /// Toggle selection on current entry
    pub fn toggle_selection(&mut self) {
        if let Some(entry) = self.current_entry() {
            // Only allow selecting files, not directories
            if matches!(entry.entry_type, EntryType::File { .. }) {
                let path = entry.path.clone();
                if self.selected_files.contains(&path) {
                    self.selected_files.remove(&path);
                } else {
                    self.selected_files.insert(path);
                }
            }
        }
    }

    /// Get the currently highlighted entry
    pub fn current_entry(&self) -> Option<&FileEntry> {
        let idx = self.effective_cursor_index();
        self.entries.get(idx)
    }

    /// Get effective cursor index (accounting for filtering)
    fn effective_cursor_index(&self) -> usize {
        if let Some(ref indices) = self.filtered_indices {
            indices.get(self.cursor_index).copied().unwrap_or(0)
        } else {
            self.cursor_index
        }
    }

    /// Get the number of visible entries
    pub fn visible_count(&self) -> usize {
        if let Some(ref indices) = self.filtered_indices {
            indices.len()
        } else {
            self.entries.len()
        }
    }

    /// Start search/filter mode
    pub fn start_search(&mut self) {
        self.mode = FileExplorerMode::Search;
        self.search_query.clear();
        self.filtered_indices = None;
    }

    /// Exit search mode
    pub fn exit_search(&mut self) {
        self.mode = FileExplorerMode::Browser;
        self.search_query.clear();
        self.filtered_indices = None;
    }

    /// Update search filter
    pub fn update_search(&mut self, query: &str) {
        self.search_query = query.to_string();

        if query.is_empty() {
            self.filtered_indices = None;
            self.cursor_index = 0;
            return;
        }

        let query_lower = query.to_lowercase();
        let indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.name.to_lowercase().contains(&query_lower))
            .map(|(i, _)| i)
            .collect();

        self.filtered_indices = Some(indices);
        self.cursor_index = 0;
    }

    /// Get selected file paths
    pub fn get_selected_paths(&self) -> Vec<PathBuf> {
        self.selected_files.iter().cloned().collect()
    }

    /// Select all visible files
    pub fn select_all_files(&mut self) {
        for entry in &self.entries {
            if matches!(entry.entry_type, EntryType::File { .. }) {
                self.selected_files.insert(entry.path.clone());
            }
        }
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_files.clear();
    }

    /// Toggle hidden files visibility
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        let _ = self.refresh();
    }

    /// Move cursor up
    pub fn cursor_up(&mut self) {
        if self.cursor_index > 0 {
            self.cursor_index -= 1;
        }
        self.adjust_scroll();
    }

    /// Move cursor down
    pub fn cursor_down(&mut self) {
        let max = self.visible_count().saturating_sub(1);
        if self.cursor_index < max {
            self.cursor_index += 1;
        }
        self.adjust_scroll();
    }

    /// Page up
    pub fn page_up(&mut self, page_size: usize) {
        self.cursor_index = self.cursor_index.saturating_sub(page_size);
        self.adjust_scroll();
    }

    /// Page down
    pub fn page_down(&mut self, page_size: usize) {
        let max = self.visible_count().saturating_sub(1);
        self.cursor_index = (self.cursor_index + page_size).min(max);
        self.adjust_scroll();
    }

    /// Adjust scroll to keep cursor visible
    fn adjust_scroll(&mut self) {
        // Keep cursor in visible area (assuming ~20 visible lines)
        let visible_height = 20u16;
        let cursor = self.cursor_index as u16;

        if cursor < self.scroll {
            self.scroll = cursor;
        } else if cursor >= self.scroll + visible_height {
            self.scroll = cursor.saturating_sub(visible_height - 1);
        }
    }

    /// Check if a file is selected
    pub fn is_selected(&self, path: &PathBuf) -> bool {
        self.selected_files.contains(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_new_file_explorer() {
        let current_dir = env::current_dir().unwrap();
        let state = FileExplorerState::new(current_dir.clone());

        assert_eq!(state.current_dir, current_dir);
        assert_eq!(state.cursor_index, 0);
        assert!(state.selected_files.is_empty());
        assert_eq!(state.mode, FileExplorerMode::Browser);
    }

    #[test]
    fn test_cursor_navigation() {
        let current_dir = env::current_dir().unwrap();
        let mut state = FileExplorerState::new(current_dir);

        // Should have at least the parent directory entry
        assert!(!state.entries.is_empty());

        state.cursor_down();
        // Cursor should move down if there are more entries
        if state.entries.len() > 1 {
            assert_eq!(state.cursor_index, 1);
        }

        state.cursor_up();
        assert_eq!(state.cursor_index, 0);
    }

    #[test]
    fn test_search_filter() {
        let current_dir = env::current_dir().unwrap();
        let mut state = FileExplorerState::new(current_dir);

        state.start_search();
        assert_eq!(state.mode, FileExplorerMode::Search);

        state.update_search("Cargo");
        // Should filter entries containing "Cargo"
        if let Some(ref indices) = state.filtered_indices {
            for &idx in indices {
                assert!(state.entries[idx].name.to_lowercase().contains("cargo"));
            }
        }

        state.exit_search();
        assert_eq!(state.mode, FileExplorerMode::Browser);
        assert!(state.filtered_indices.is_none());
    }

    #[test]
    fn test_toggle_hidden() {
        let current_dir = env::current_dir().unwrap();
        let mut state = FileExplorerState::new(current_dir);

        let initial_show = state.show_hidden;
        state.toggle_hidden();
        assert_ne!(state.show_hidden, initial_show);
    }
}
