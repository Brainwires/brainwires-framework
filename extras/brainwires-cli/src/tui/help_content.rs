//! Help content data for the in-app help system.
//!
//! This module contains all the static help content organized by category.

/// A single help entry with a shortcut and description.
#[derive(Debug, Clone)]
pub struct HelpEntry {
    /// The keyboard shortcut or command (e.g., "Ctrl+L", "/help")
    pub shortcut: String,
    /// Description of what the shortcut does
    pub description: String,
}

impl HelpEntry {
    /// Create a new help entry.
    pub fn new(shortcut: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            shortcut: shortcut.into(),
            description: description.into(),
        }
    }
}

/// Help category enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HelpCategory {
    #[default]
    Navigation,
    TextEditing,
    ViewsModes,
    Dialogs,
    Commands,
    FileExplorer,
    GitIntegration,
    TipsTricks,
}

impl HelpCategory {
    /// Get all categories in order.
    pub fn all() -> &'static [HelpCategory] {
        &[
            HelpCategory::Navigation,
            HelpCategory::TextEditing,
            HelpCategory::ViewsModes,
            HelpCategory::Dialogs,
            HelpCategory::Commands,
            HelpCategory::FileExplorer,
            HelpCategory::GitIntegration,
            HelpCategory::TipsTricks,
        ]
    }

    /// Get the display name for this category.
    pub fn display_name(&self) -> &'static str {
        match self {
            HelpCategory::Navigation => "Navigation",
            HelpCategory::TextEditing => "Text Editing",
            HelpCategory::ViewsModes => "Views & Modes",
            HelpCategory::Dialogs => "Dialogs",
            HelpCategory::Commands => "Commands",
            HelpCategory::FileExplorer => "File Explorer",
            HelpCategory::GitIntegration => "Git Integration",
            HelpCategory::TipsTricks => "Tips & Tricks",
        }
    }

    /// Get the icon for this category.
    pub fn icon(&self) -> &'static str {
        match self {
            HelpCategory::Navigation => "🧭",
            HelpCategory::TextEditing => "✏️",
            HelpCategory::ViewsModes => "🖥️",
            HelpCategory::Dialogs => "📋",
            HelpCategory::Commands => "⌨️",
            HelpCategory::FileExplorer => "📁",
            HelpCategory::GitIntegration => "🔀",
            HelpCategory::TipsTricks => "💡",
        }
    }

    /// Get help entries for this category.
    pub fn entries(&self) -> Vec<HelpEntry> {
        match self {
            HelpCategory::Navigation => get_navigation_help(),
            HelpCategory::TextEditing => get_text_editing_help(),
            HelpCategory::ViewsModes => get_views_modes_help(),
            HelpCategory::Dialogs => get_dialogs_help(),
            HelpCategory::Commands => get_commands_help(),
            HelpCategory::FileExplorer => get_file_explorer_help(),
            HelpCategory::GitIntegration => get_git_integration_help(),
            HelpCategory::TipsTricks => get_tips_tricks_help(),
        }
    }

    /// Get the next category (wraps around).
    pub fn next(&self) -> HelpCategory {
        let all = Self::all();
        let idx = all.iter().position(|c| c == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    /// Get the previous category (wraps around).
    pub fn prev(&self) -> HelpCategory {
        let all = Self::all();
        let idx = all.iter().position(|c| c == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

/// Get navigation help entries.
pub fn get_navigation_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("Ctrl+L", "Open session picker"),
        HelpEntry::new("Ctrl+D", "Toggle console view"),
        HelpEntry::new("Ctrl+T", "Open task viewer"),
        HelpEntry::new("Ctrl+R", "Reverse search history"),
        HelpEntry::new("Tab", "Toggle focus between panels"),
        HelpEntry::new("↑/↓", "Scroll conversation/navigate"),
        HelpEntry::new("PgUp/PgDn", "Page scroll"),
        HelpEntry::new("Ctrl+Home", "Scroll to top"),
        HelpEntry::new("Ctrl+End", "Scroll to bottom"),
        HelpEntry::new("Ctrl+C", "Quit application"),
    ]
}

/// Get text editing help entries.
pub fn get_text_editing_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("Ctrl+A / Home", "Move cursor to start of line"),
        HelpEntry::new("Ctrl+E / End", "Move cursor to end of line"),
        HelpEntry::new("Ctrl+U", "Delete from cursor to start of line"),
        HelpEntry::new("Ctrl+K", "Delete from cursor to end of line"),
        HelpEntry::new("Ctrl+W", "Delete word before cursor"),
        HelpEntry::new("Alt+Backspace", "Delete word before cursor"),
        HelpEntry::new("Shift+Enter", "Insert newline (multiline input)"),
        HelpEntry::new("Alt+Enter", "Insert newline (multiline input)"),
        HelpEntry::new("Ctrl+J", "Insert newline (multiline input)"),
        HelpEntry::new("←/→", "Move cursor left/right"),
        HelpEntry::new("Ctrl+←/→", "Move cursor by word"),
        HelpEntry::new("Enter", "Submit message"),
    ]
}

/// Get views and modes help entries.
pub fn get_views_modes_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("F10", "Toggle fullscreen mode"),
        HelpEntry::new("Ctrl+Alt+F", "Toggle fullscreen mode"),
        HelpEntry::new("F9", "Toggle conversation view style"),
        HelpEntry::new("Ctrl+Enter", "Fullscreen current panel"),
        HelpEntry::new("F1", "Open this help dialog"),
    ]
}

/// Get dialogs help entries.
pub fn get_dialogs_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("Ctrl+F", "Open find dialog"),
        HelpEntry::new("Ctrl+H", "Open find & replace dialog"),
        HelpEntry::new("Ctrl+Alt+F", "Open file explorer"),
        HelpEntry::new("Ctrl+G", "Open Git SCM panel"),
        HelpEntry::new("Escape", "Close current dialog"),
        HelpEntry::new("Tab", "Navigate between dialog elements"),
    ]
}

/// Get commands help entries.
pub fn get_commands_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("/help", "Show available commands"),
        HelpEntry::new("/clear", "Clear conversation history"),
        HelpEntry::new("/compact", "Compact context to save tokens"),
        HelpEntry::new("/save [name]", "Save current session"),
        HelpEntry::new("/load [name]", "Load a saved session"),
        HelpEntry::new("/sessions", "List available sessions"),
        HelpEntry::new("/tools", "Open tool picker"),
        HelpEntry::new("/model [name]", "Switch AI model"),
        HelpEntry::new("/provider [name]", "Switch AI provider"),
        HelpEntry::new("/cost", "Show token usage and costs"),
        HelpEntry::new("/context", "Show context information"),
        HelpEntry::new("/working-set", "Manage working set files"),
        HelpEntry::new("/permissions", "View/modify permissions"),
        HelpEntry::new("/mcp", "Manage MCP servers"),
    ]
}

/// Get file explorer help entries.
pub fn get_file_explorer_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("Enter", "Open file or enter directory"),
        HelpEntry::new("Space", "Toggle file selection"),
        HelpEntry::new("Backspace", "Go to parent directory"),
        HelpEntry::new("/", "Start search/filter"),
        HelpEntry::new(".", "Toggle hidden files"),
        HelpEntry::new("a", "Select all files"),
        HelpEntry::new("n", "Clear selection"),
        HelpEntry::new("r", "Refresh directory"),
        HelpEntry::new("i", "Insert selected to working set"),
        HelpEntry::new("Escape", "Exit file explorer"),
    ]
}

/// Get Git integration help entries.
pub fn get_git_integration_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("s", "Stage selected file"),
        HelpEntry::new("u", "Unstage selected file"),
        HelpEntry::new("c", "Start commit"),
        HelpEntry::new("Tab", "Switch between panels"),
        HelpEntry::new("↑/↓", "Navigate files"),
        HelpEntry::new("Enter", "View file diff"),
        HelpEntry::new("a", "Stage all changes"),
        HelpEntry::new("r", "Refresh status"),
        HelpEntry::new("Escape", "Exit Git panel"),
    ]
}

/// Get tips and tricks help entries.
pub fn get_tips_tricks_help() -> Vec<HelpEntry> {
    vec![
        HelpEntry::new("Context", "Use /working-set to add files for AI context"),
        HelpEntry::new("Multiline", "Use Shift+Enter for multi-line messages"),
        HelpEntry::new("Sessions", "Save important conversations with /save"),
        HelpEntry::new("Tools", "Use /tools to see and configure available tools"),
        HelpEntry::new("Compact", "Use /compact when context gets too large"),
        HelpEntry::new("History", "Press Ctrl+R to search command history"),
        HelpEntry::new("Cancel", "AI operations can be cancelled with Escape"),
        HelpEntry::new("Copy", "Select text in conversation to copy"),
    ]
}

/// Search across all categories for matching entries.
pub fn search_help(query: &str) -> Vec<(HelpCategory, HelpEntry)> {
    if query.is_empty() {
        return vec![];
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for category in HelpCategory::all() {
        for entry in category.entries() {
            if entry.shortcut.to_lowercase().contains(&query_lower)
                || entry.description.to_lowercase().contains(&query_lower)
            {
                results.push((*category, entry));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_categories() {
        let all = HelpCategory::all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0], HelpCategory::Navigation);
    }

    #[test]
    fn test_category_navigation() {
        let cat = HelpCategory::Navigation;
        assert_eq!(cat.next(), HelpCategory::TextEditing);
        assert_eq!(cat.prev(), HelpCategory::TipsTricks);
    }

    #[test]
    fn test_category_entries() {
        let entries = HelpCategory::Navigation.entries();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.shortcut.contains("Ctrl+L")));
    }

    #[test]
    fn test_search_help() {
        let results = search_help("ctrl");
        assert!(!results.is_empty());

        let results = search_help("session");
        assert!(!results.is_empty());

        let results = search_help("xyznonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_display_names() {
        assert_eq!(HelpCategory::Navigation.display_name(), "Navigation");
        assert_eq!(HelpCategory::GitIntegration.display_name(), "Git Integration");
    }
}
