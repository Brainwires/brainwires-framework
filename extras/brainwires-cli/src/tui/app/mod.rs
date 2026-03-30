//! TUI Application State
//!
//! Manages the state of the TUI application including conversation history,
//! tool execution status, and input handling.

// Module declarations
mod state;
mod events;
mod message_processing;
pub mod session_management;
mod autocomplete;
mod history;
pub mod file_explorer;
pub mod nano_editor;
pub mod git_scm;
pub mod find_replace;
pub mod help_dialog;
pub mod suspend_dialog;
pub mod approval_dialog;
pub mod sudo_dialog;
pub mod exit_dialog;
mod plan_mode;
mod prompt_mode;
pub mod journal_tree;

// Re-export public types and the App struct
pub use state::{
    App,
    AppMode,
    ConversationViewStyle,
    FocusedPanel,
    PromptMode,
    ShellExecution,
    SubAgentPanelFocus,
    ToolExecutionEntry,
    ToolPickerState,
    TuiMessage,
};
pub use file_explorer::{FileEntry, EntryType, FileExplorerMode};
pub use git_scm::{GitFileStatus, GitFileEntry, ScmPanel, GitOperationMode};
pub use find_replace::{FindReplaceMode, FindReplaceContext, DialogFocus};
pub use suspend_dialog::SuspendFocus;
pub use exit_dialog::ExitFocus;

// Re-export hotkey dialog types from ratatui_interact

