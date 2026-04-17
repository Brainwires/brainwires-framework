//! TUI Application State
//!
//! Manages the state of the TUI application including conversation history,
//! tool execution status, and input handling.

// Module declarations
pub mod approval_dialog;
mod autocomplete;
mod events;
pub mod exit_dialog;
pub mod file_explorer;
pub mod find_replace;
pub mod git_scm;
pub mod help_dialog;
mod history;
pub mod journal_tree;
mod message_processing;
pub mod nano_editor;
mod plan_mode;
mod prompt_mode;
pub mod session_management;
mod state;
pub mod sudo_dialog;
pub mod suspend_dialog;
pub mod user_question;

// Re-export public types and the App struct
pub use exit_dialog::ExitFocus;
pub use file_explorer::{EntryType, FileEntry, FileExplorerMode};
pub use find_replace::{DialogFocus, FindReplaceContext, FindReplaceMode};
pub use git_scm::{GitFileEntry, GitFileStatus, GitOperationMode, ScmPanel};
pub use state::{
    App, AppMode, ConversationViewStyle, FocusedPanel, LogLevel, PromptMode, ShellExecution,
    SubAgentPanelFocus, ToolExecutionEntry, ToolPickerState, TuiMessage,
};
pub use suspend_dialog::SuspendFocus;

// Re-export hotkey dialog types from ratatui_interact
