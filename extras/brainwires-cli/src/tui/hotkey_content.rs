//! Hotkey content data for the hotkey configuration dialog.
//!
//! This module contains all hotkey definitions organized by category and context.
//!
//! This module implements the `HotkeyCategory` and `HotkeyProvider` traits from
//! `ratatui_interact` to enable the generic hotkey dialog component.

use std::fmt;

use ratatui_interact::components::hotkey_dialog::{
    HotkeyCategory as HotkeyCategoryTrait, HotkeyEntryData, HotkeyProvider,
};

/// Context in which a hotkey is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyContext {
    /// Active in all modes
    Global,
    /// Normal input mode
    Normal,
    /// Waiting for AI response
    Waiting,
    /// Session picker dialog
    SessionPicker,
    /// Reverse search mode
    ReverseSearch,
    /// Console view mode
    ConsoleView,
    /// Task viewer mode
    TaskViewer,
    /// File explorer mode
    FileExplorer,
    /// Nano editor mode
    NanoEditor,
    /// Git SCM mode
    GitScm,
    /// Find/Replace dialog
    FindDialog,
    /// Help dialog
    HelpDialog,
    /// Tool picker mode
    ToolPicker,
    /// Plan mode - isolated planning context
    PlanMode,
}

impl fmt::Display for HotkeyContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HotkeyContext::Global => write!(f, "Global"),
            HotkeyContext::Normal => write!(f, "Normal"),
            HotkeyContext::Waiting => write!(f, "Waiting"),
            HotkeyContext::SessionPicker => write!(f, "Session Picker"),
            HotkeyContext::ReverseSearch => write!(f, "Reverse Search"),
            HotkeyContext::ConsoleView => write!(f, "Console View"),
            HotkeyContext::TaskViewer => write!(f, "Task Viewer"),
            HotkeyContext::FileExplorer => write!(f, "File Explorer"),
            HotkeyContext::NanoEditor => write!(f, "Nano Editor"),
            HotkeyContext::GitScm => write!(f, "Git SCM"),
            HotkeyContext::FindDialog => write!(f, "Find Dialog"),
            HotkeyContext::HelpDialog => write!(f, "Help Dialog"),
            HotkeyContext::ToolPicker => write!(f, "Tool Picker"),
            HotkeyContext::PlanMode => write!(f, "Plan Mode"),
        }
    }
}

/// A single hotkey entry with key combination, action, and context.
#[derive(Debug, Clone)]
pub struct HotkeyEntry {
    /// The keyboard shortcut (e.g., "Ctrl+C", "F1")
    pub key_combination: String,
    /// Description of what the hotkey does
    pub action: String,
    /// Context(s) where this hotkey is active
    pub contexts: Vec<HotkeyContext>,
    /// Whether this hotkey can be customized (future feature)
    pub is_customizable: bool,
}

impl HotkeyEntry {
    /// Create a new hotkey entry.
    pub fn new(
        key_combination: impl Into<String>,
        action: impl Into<String>,
        contexts: Vec<HotkeyContext>,
    ) -> Self {
        Self {
            key_combination: key_combination.into(),
            action: action.into(),
            contexts,
            is_customizable: true,
        }
    }

    /// Create a hotkey entry that cannot be customized.
    pub fn fixed(
        key_combination: impl Into<String>,
        action: impl Into<String>,
        contexts: Vec<HotkeyContext>,
    ) -> Self {
        Self {
            key_combination: key_combination.into(),
            action: action.into(),
            contexts,
            is_customizable: false,
        }
    }

    /// Get a formatted context string.
    pub fn context_string(&self) -> String {
        if self.contexts.len() == 1 && self.contexts[0] == HotkeyContext::Global {
            "Global".to_string()
        } else if self.contexts.is_empty() {
            "None".to_string()
        } else {
            self.contexts
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// Hotkey category enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HotkeyCategory {
    #[default]
    GlobalActions,
    Navigation,
    TextEditing,
    ViewsModes,
    Dialogs,
    FileExplorer,
    GitScm,
    NanoEditor,
    TaskViewer,
    ToolPicker,
    PlanMode,
}

impl HotkeyCategory {
    /// Get all categories in order.
    pub fn all() -> &'static [HotkeyCategory] {
        &[
            HotkeyCategory::GlobalActions,
            HotkeyCategory::Navigation,
            HotkeyCategory::TextEditing,
            HotkeyCategory::ViewsModes,
            HotkeyCategory::Dialogs,
            HotkeyCategory::FileExplorer,
            HotkeyCategory::GitScm,
            HotkeyCategory::NanoEditor,
            HotkeyCategory::TaskViewer,
            HotkeyCategory::ToolPicker,
            HotkeyCategory::PlanMode,
        ]
    }

    /// Get the display name for this category.
    pub fn display_name(&self) -> &'static str {
        match self {
            HotkeyCategory::GlobalActions => "Global Actions",
            HotkeyCategory::Navigation => "Navigation",
            HotkeyCategory::TextEditing => "Text Editing",
            HotkeyCategory::ViewsModes => "Views & Modes",
            HotkeyCategory::Dialogs => "Dialogs",
            HotkeyCategory::FileExplorer => "File Explorer",
            HotkeyCategory::GitScm => "Git SCM",
            HotkeyCategory::NanoEditor => "Nano Editor",
            HotkeyCategory::TaskViewer => "Task Viewer",
            HotkeyCategory::ToolPicker => "Tool Picker",
            HotkeyCategory::PlanMode => "Plan Mode",
        }
    }

    /// Get the icon for this category.
    pub fn icon(&self) -> &'static str {
        match self {
            HotkeyCategory::GlobalActions => "⚡",
            HotkeyCategory::Navigation => "🧭",
            HotkeyCategory::TextEditing => "✏️",
            HotkeyCategory::ViewsModes => "🖥️",
            HotkeyCategory::Dialogs => "📋",
            HotkeyCategory::FileExplorer => "📁",
            HotkeyCategory::GitScm => "🔀",
            HotkeyCategory::NanoEditor => "📝",
            HotkeyCategory::TaskViewer => "📊",
            HotkeyCategory::ToolPicker => "🔧",
            HotkeyCategory::PlanMode => "📐",
        }
    }

    /// Get hotkey entries for this category.
    pub fn entries(&self) -> Vec<HotkeyEntry> {
        match self {
            HotkeyCategory::GlobalActions => get_global_actions(),
            HotkeyCategory::Navigation => get_navigation_hotkeys(),
            HotkeyCategory::TextEditing => get_text_editing_hotkeys(),
            HotkeyCategory::ViewsModes => get_views_modes_hotkeys(),
            HotkeyCategory::Dialogs => get_dialogs_hotkeys(),
            HotkeyCategory::FileExplorer => get_file_explorer_hotkeys(),
            HotkeyCategory::GitScm => get_git_scm_hotkeys(),
            HotkeyCategory::NanoEditor => get_nano_editor_hotkeys(),
            HotkeyCategory::TaskViewer => get_task_viewer_hotkeys(),
            HotkeyCategory::ToolPicker => get_tool_picker_hotkeys(),
            HotkeyCategory::PlanMode => get_plan_mode_hotkeys(),
        }
    }

    /// Get the next category (wraps around).
    pub fn next(&self) -> HotkeyCategory {
        let all = Self::all();
        let idx = all.iter().position(|c| c == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    /// Get the previous category (wraps around).
    pub fn prev(&self) -> HotkeyCategory {
        let all = Self::all();
        let idx = all.iter().position(|c| c == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

// Implement the trait from ratatui_interact
impl HotkeyCategoryTrait for HotkeyCategory {
    fn all() -> &'static [Self] {
        HotkeyCategory::all()
    }

    fn display_name(&self) -> &str {
        HotkeyCategory::display_name(self)
    }

    fn icon(&self) -> &str {
        HotkeyCategory::icon(self)
    }

    fn next(&self) -> Self {
        HotkeyCategory::next(self)
    }

    fn prev(&self) -> Self {
        HotkeyCategory::prev(self)
    }
}

/// Provider for brainwires-cli hotkeys.
///
/// Implements `HotkeyProvider` from ratatui_interact to supply hotkey data
/// to the generic hotkey dialog component.
pub struct BrainwiresHotkeyProvider;

impl HotkeyProvider for BrainwiresHotkeyProvider {
    type Category = HotkeyCategory;

    fn entries_for_category(&self, category: Self::Category) -> Vec<HotkeyEntryData> {
        category.entries().into_iter().map(|e| e.into()).collect()
    }

    fn search(&self, query: &str) -> Vec<(Self::Category, HotkeyEntryData)> {
        search_hotkeys(query)
            .into_iter()
            .map(|(cat, entry)| (cat, entry.into()))
            .collect()
    }
}

impl From<HotkeyEntry> for HotkeyEntryData {
    fn from(entry: HotkeyEntry) -> Self {
        let is_global = entry.contexts.len() == 1 && entry.contexts[0] == HotkeyContext::Global;
        let context = entry.context_string();
        HotkeyEntryData {
            key_combination: entry.key_combination,
            action: entry.action,
            context,
            is_global,
            is_customizable: entry.is_customizable,
        }
    }
}

/// Get global action hotkeys.
fn get_global_actions() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::fixed("Ctrl+C", "Quit application", vec![HotkeyContext::Global]),
        HotkeyEntry::new(
            "Ctrl+Z",
            "Open suspend/background dialog",
            vec![HotkeyContext::Global],
        ),
        HotkeyEntry::new(
            "Escape",
            "Exit current mode/dialog",
            vec![HotkeyContext::Global],
        ),
        HotkeyEntry::new("F1", "Open help dialog", vec![HotkeyContext::Global]),
        HotkeyEntry::new("Enter", "Submit message", vec![HotkeyContext::Normal]),
        HotkeyEntry::new(
            "Tab",
            "Toggle focus between panels",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Shift+Tab",
            "Reverse focus navigation",
            vec![HotkeyContext::Normal],
        ),
    ]
}

/// Get navigation hotkeys.
fn get_navigation_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new("Ctrl+L", "Open session picker", vec![HotkeyContext::Normal]),
        HotkeyEntry::new("Ctrl+D", "Toggle console view", vec![HotkeyContext::Normal]),
        HotkeyEntry::new("Ctrl+T", "Open task viewer", vec![HotkeyContext::Normal]),
        HotkeyEntry::new(
            "Ctrl+R",
            "Reverse search history",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Up / Down",
            "Scroll conversation / Navigate lists",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "PageUp / PageDown",
            "Page scroll",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+Home",
            "Scroll to document start",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+End",
            "Scroll to document end",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+Up",
            "Scroll to document start (alt)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+Down",
            "Scroll to document end (alt)",
            vec![HotkeyContext::Normal],
        ),
    ]
}

/// Get text editing hotkeys.
fn get_text_editing_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new(
            "Shift+Enter",
            "Insert newline (multiline)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Alt+Enter",
            "Insert newline (multiline)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+J",
            "Insert newline (Unix standard)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+A",
            "Move cursor to line start",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+E",
            "Move cursor to line end",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Home",
            "Move cursor to line start",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "End",
            "Move cursor to line end",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+U",
            "Delete from cursor to line start",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+K",
            "Delete from cursor to line end",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+W",
            "Delete word backward",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Alt+Backspace",
            "Delete word backward",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Alt+Delete",
            "Delete word forward",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+Left",
            "Move word backward",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+Right",
            "Move word forward",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Alt+Left",
            "Move word backward (alt)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Alt+Right",
            "Move word forward (alt)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Left / Right",
            "Move cursor by character",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Backspace",
            "Delete character before cursor",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Delete",
            "Delete character at cursor",
            vec![HotkeyContext::Normal],
        ),
    ]
}

/// Get views and modes hotkeys.
fn get_views_modes_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new("F10", "Toggle fullscreen mode", vec![HotkeyContext::Normal]),
        HotkeyEntry::new(
            "Ctrl+Alt+F",
            "Toggle fullscreen mode (alt)",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "F9",
            "Toggle conversation view style",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "m",
            "Toggle mouse capture (for text selection)",
            vec![HotkeyContext::ConsoleView],
        ),
        HotkeyEntry::new(
            "c",
            "Copy console content to clipboard",
            vec![HotkeyContext::ConsoleView],
        ),
    ]
}

/// Get dialog hotkeys.
fn get_dialogs_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new("Ctrl+F", "Open find dialog", vec![HotkeyContext::Normal]),
        HotkeyEntry::new(
            "Ctrl+H",
            "Open find & replace dialog",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new(
            "Ctrl+Alt+F",
            "Open file explorer",
            vec![HotkeyContext::Normal],
        ),
        HotkeyEntry::new("Ctrl+G", "Open Git SCM panel", vec![HotkeyContext::Normal]),
        HotkeyEntry::new("F3", "Find next match", vec![HotkeyContext::FindDialog]),
        HotkeyEntry::new(
            "Shift+F3",
            "Find previous match",
            vec![HotkeyContext::FindDialog],
        ),
        HotkeyEntry::new(
            "Tab",
            "Cycle through dialog elements",
            vec![HotkeyContext::FindDialog, HotkeyContext::HelpDialog],
        ),
        HotkeyEntry::new(
            "Shift+Tab",
            "Cycle backward through elements",
            vec![HotkeyContext::FindDialog, HotkeyContext::HelpDialog],
        ),
        HotkeyEntry::new("Space", "Toggle checkbox", vec![HotkeyContext::FindDialog]),
    ]
}

/// Get file explorer hotkeys.
fn get_file_explorer_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new(
            "Up / Down",
            "Navigate file list",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "Enter",
            "Open directory or file",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "Space",
            "Toggle file selection",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "Left / Backspace",
            "Go to parent directory",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "Right",
            "Enter directory",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "/",
            "Start search/filter",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new("e", "Edit current file", vec![HotkeyContext::FileExplorer]),
        HotkeyEntry::new("a", "Select all files", vec![HotkeyContext::FileExplorer]),
        HotkeyEntry::new("n", "Clear selection", vec![HotkeyContext::FileExplorer]),
        HotkeyEntry::new(
            ".",
            "Toggle hidden files",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new("r", "Refresh directory", vec![HotkeyContext::FileExplorer]),
        HotkeyEntry::new(
            "i",
            "Insert selected to working set",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "PageUp / PageDown",
            "Page navigation",
            vec![HotkeyContext::FileExplorer],
        ),
        HotkeyEntry::new(
            "Escape",
            "Exit file explorer",
            vec![HotkeyContext::FileExplorer],
        ),
    ]
}

/// Get Git SCM hotkeys.
fn get_git_scm_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new("Tab", "Switch between panels", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new(
            "Up / Down",
            "Navigate file list",
            vec![HotkeyContext::GitScm],
        ),
        HotkeyEntry::new(
            "Space",
            "Toggle file selection",
            vec![HotkeyContext::GitScm],
        ),
        HotkeyEntry::new(
            "s / Enter",
            "Stage selected file(s)",
            vec![HotkeyContext::GitScm],
        ),
        HotkeyEntry::new("u", "Unstage selected file(s)", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new(
            "d",
            "Discard changes (with confirm)",
            vec![HotkeyContext::GitScm],
        ),
        HotkeyEntry::new(
            "c",
            "Start commit (enter message)",
            vec![HotkeyContext::GitScm],
        ),
        HotkeyEntry::new("P", "Push to remote", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new("p", "Pull from remote", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new("f", "Fetch from remote", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new("r", "Refresh status", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new("PageUp / PageDown", "Scroll", vec![HotkeyContext::GitScm]),
        HotkeyEntry::new("Escape", "Close Git SCM", vec![HotkeyContext::GitScm]),
    ]
}

/// Get Nano editor hotkeys.
fn get_nano_editor_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new("Arrow Keys", "Move cursor", vec![HotkeyContext::NanoEditor]),
        HotkeyEntry::new(
            "Ctrl+S / Ctrl+O",
            "Save file",
            vec![HotkeyContext::NanoEditor],
        ),
        HotkeyEntry::new("Ctrl+X", "Exit editor", vec![HotkeyContext::NanoEditor]),
        HotkeyEntry::new("Ctrl+K", "Cut line", vec![HotkeyContext::NanoEditor]),
        HotkeyEntry::new("Ctrl+U", "Paste", vec![HotkeyContext::NanoEditor]),
        HotkeyEntry::new(
            "Home / End",
            "Line start/end",
            vec![HotkeyContext::NanoEditor],
        ),
        HotkeyEntry::new(
            "PageUp / PageDown",
            "Page navigation",
            vec![HotkeyContext::NanoEditor],
        ),
        HotkeyEntry::new(
            "Backspace",
            "Delete backward",
            vec![HotkeyContext::NanoEditor],
        ),
        HotkeyEntry::new("Delete", "Delete forward", vec![HotkeyContext::NanoEditor]),
        HotkeyEntry::new("Enter", "Insert newline", vec![HotkeyContext::NanoEditor]),
        HotkeyEntry::new(
            "Tab",
            "Insert tab character",
            vec![HotkeyContext::NanoEditor],
        ),
        HotkeyEntry::new(
            "Escape",
            "Exit (warns if unsaved)",
            vec![HotkeyContext::NanoEditor],
        ),
    ]
}

/// Get task viewer hotkeys.
fn get_task_viewer_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new(
            "Up / Down",
            "Navigate tasks",
            vec![HotkeyContext::TaskViewer],
        ),
        HotkeyEntry::new(
            "Enter / Left / Right",
            "Toggle expand/collapse",
            vec![HotkeyContext::TaskViewer],
        ),
        HotkeyEntry::new(
            "Space",
            "Toggle task status",
            vec![HotkeyContext::TaskViewer],
        ),
        HotkeyEntry::new(
            "PageUp / PageDown",
            "Scroll",
            vec![HotkeyContext::TaskViewer],
        ),
        HotkeyEntry::new(
            "Escape",
            "Close task viewer",
            vec![HotkeyContext::TaskViewer],
        ),
    ]
}

/// Get tool picker hotkeys.
fn get_tool_picker_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new(
            "Up / Down",
            "Navigate tools",
            vec![HotkeyContext::ToolPicker],
        ),
        HotkeyEntry::new(
            "Space",
            "Toggle tool selection",
            vec![HotkeyContext::ToolPicker],
        ),
        HotkeyEntry::new("a / A", "Select all tools", vec![HotkeyContext::ToolPicker]),
        HotkeyEntry::new("n / N", "Select no tools", vec![HotkeyContext::ToolPicker]),
        HotkeyEntry::new("Left", "Collapse category", vec![HotkeyContext::ToolPicker]),
        HotkeyEntry::new("Right", "Expand category", vec![HotkeyContext::ToolPicker]),
        HotkeyEntry::new(
            "Enter",
            "Confirm selection",
            vec![HotkeyContext::ToolPicker],
        ),
        HotkeyEntry::new("Escape", "Cancel picker", vec![HotkeyContext::ToolPicker]),
    ]
}

/// Get plan mode hotkeys.
fn get_plan_mode_hotkeys() -> Vec<HotkeyEntry> {
    vec![
        HotkeyEntry::new("Ctrl+P", "Toggle plan mode", vec![HotkeyContext::Global]),
        HotkeyEntry::new(
            "Enter",
            "Submit message in plan mode",
            vec![HotkeyContext::PlanMode],
        ),
        HotkeyEntry::new("Ctrl+P", "Exit plan mode", vec![HotkeyContext::PlanMode]),
        HotkeyEntry::new("Escape", "Exit plan mode", vec![HotkeyContext::PlanMode]),
        HotkeyEntry::new(
            "Ctrl+E",
            "Export plan mode session",
            vec![HotkeyContext::PlanMode],
        ),
        HotkeyEntry::new(
            "Up / Down",
            "Scroll / Navigate history",
            vec![HotkeyContext::PlanMode],
        ),
    ]
}

/// Search across all categories for matching entries.
pub fn search_hotkeys(query: &str) -> Vec<(HotkeyCategory, HotkeyEntry)> {
    if query.is_empty() {
        return vec![];
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for category in HotkeyCategory::all() {
        for entry in category.entries() {
            if entry.key_combination.to_lowercase().contains(&query_lower)
                || entry.action.to_lowercase().contains(&query_lower)
                || entry
                    .contexts
                    .iter()
                    .any(|c| c.to_string().to_lowercase().contains(&query_lower))
            {
                results.push((*category, entry));
            }
        }
    }

    results
}

/// Get total count of hotkeys.
#[cfg(test)]
pub fn hotkey_count() -> usize {
    HotkeyCategory::all()
        .iter()
        .map(|c| c.entries().len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_categories() {
        let all = HotkeyCategory::all();
        assert_eq!(all.len(), 11);
        assert_eq!(all[0], HotkeyCategory::GlobalActions);
    }

    #[test]
    fn test_category_navigation() {
        let cat = HotkeyCategory::GlobalActions;
        assert_eq!(cat.next(), HotkeyCategory::Navigation);
        assert_eq!(cat.prev(), HotkeyCategory::PlanMode);
    }

    #[test]
    fn test_category_entries() {
        let entries = HotkeyCategory::GlobalActions.entries();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.key_combination.contains("Ctrl+C")));
    }

    #[test]
    fn test_search_hotkeys() {
        let results = search_hotkeys("ctrl");
        assert!(!results.is_empty());

        let results = search_hotkeys("quit");
        assert!(!results.is_empty());

        let results = search_hotkeys("xyznonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_display_names() {
        assert_eq!(
            HotkeyCategory::GlobalActions.display_name(),
            "Global Actions"
        );
        assert_eq!(HotkeyCategory::GitScm.display_name(), "Git SCM");
    }

    #[test]
    fn test_hotkey_count() {
        let count = hotkey_count();
        assert!(count > 50, "Expected more than 50 hotkeys, got {}", count);
    }

    #[test]
    fn test_context_string() {
        let entry = HotkeyEntry::new("Ctrl+C", "Quit", vec![HotkeyContext::Global]);
        assert_eq!(entry.context_string(), "Global");

        let entry = HotkeyEntry::new(
            "Tab",
            "Navigate",
            vec![HotkeyContext::FindDialog, HotkeyContext::HelpDialog],
        );
        assert!(entry.context_string().contains("Find Dialog"));
        assert!(entry.context_string().contains("Help Dialog"));
    }

    #[test]
    fn test_fixed_hotkey() {
        let entry = HotkeyEntry::fixed("Ctrl+C", "Quit", vec![HotkeyContext::Global]);
        assert!(!entry.is_customizable);

        let entry = HotkeyEntry::new("F1", "Help", vec![HotkeyContext::Global]);
        assert!(entry.is_customizable);
    }
}
