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
    ApprovalMode,
    ConversationViewStyle,
    FocusedPanel,
    PromptMode,
    ShellExecution,
    SubAgentPanelFocus,
    SubAgentSummary,
    SubAgentViewerState,
    ToolExecution,
    ToolExecutionEntry,
    ToolPickerState,
    ToolStatus,
    TuiMessage,
};
pub use journal_tree::{JournalTreeState, JournalNodeId};
pub use file_explorer::{FileExplorerState, FileEntry, EntryType, FileExplorerMode};
pub use nano_editor::{NanoEditorState, CursorDirection};
pub use git_scm::{GitScmState, GitFileStatus, GitFileEntry, ScmPanel, GitOperationMode, GitAction};
pub use find_replace::{FindReplaceState, FindReplaceMode, FindReplaceContext, DialogFocus};
pub use help_dialog::{HelpDialogState, HelpFocus};
pub use suspend_dialog::{SuspendDialogState, SuspendFocus, SuspendAction};
pub use exit_dialog::{ExitDialogState, ExitFocus, ExitAction};

// Re-export hotkey dialog types from ratatui_interact
pub use ratatui_interact::components::hotkey_dialog::{
    HotkeyDialogState, HotkeyFocus, HotkeyDialogAction, HotkeyDialogStyle,
    handle_hotkey_dialog_key, handle_hotkey_dialog_mouse,
};

