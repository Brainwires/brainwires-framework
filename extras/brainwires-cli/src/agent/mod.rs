//! Agent Process Module
//!
//! Contains the Agent process that holds all session state and communicates
//! with TUI viewers via IPC.
//!
//! The Agent runs as a background process that maintains:
//! - Conversation history
//! - MCP server connections
//! - Tool executor
//! - Task manager
//! - Provider connection
//!
//! TUI viewers can attach/detach without losing state.

pub mod hibernate;
pub mod message_queue;
pub mod plan_mode;
pub mod process;
pub mod spawn;
pub mod state;
pub mod worktree;

pub use hibernate::*;
pub use message_queue::*;
pub use process::*;
pub use spawn::*;
pub use state::*;
pub use worktree::{WorktreeGuard, prune_orphans as prune_worktree_orphans};
