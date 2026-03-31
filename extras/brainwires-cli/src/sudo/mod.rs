//! Sudo password system for interactive sudo command execution
//!
//! This module provides the infrastructure for prompting users for their
//! sudo password when the AI agent runs a `sudo` command via the bash tool.
//!
//! ## Security
//!
//! - Password is wrapped in `Zeroizing<String>` — zeros on drop
//! - Dropped immediately after writing to stdin
//! - Never included in `ToolResult` content
//! - Stderr filtered to remove `[sudo] password for` prompt
//! - No caching — fresh prompt every time
//! - No logging of password value

pub mod types;

pub use types::{SudoPasswordRequest, SudoPasswordResponse};
