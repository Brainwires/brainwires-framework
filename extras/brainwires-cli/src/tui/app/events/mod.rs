//! Event Handling Module
//!
//! This module organizes event handlers by category for the TUI application.
//!
//! ## Structure
//! - `core.rs` - Main event dispatch and core input handling
//! - `viewers.rs` - Console, shell, and fullscreen viewer handlers
//! - `pickers.rs` - Session, tool, and file picker handlers
//! - `dialogs.rs` - Help, suspend, exit, hotkey, and approval dialog handlers
//! - `modals.rs` - Task viewer, nano editor, git SCM, and question handlers

mod core;
mod viewers;
mod pickers;
mod dialogs;
mod modals;

// All methods are implemented directly on App via the module files above.
// No additional re-exports needed since they're all trait-free impl blocks.
