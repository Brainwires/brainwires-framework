//! Approval system for tool execution
//!
//! This module provides the infrastructure for prompting users before
//! executing potentially dangerous tools (file writes, deletes, bash commands, etc.)
//!
//! ## Architecture
//!
//! ```text
//! Tool Executor                    TUI Event Loop
//!      |                                |
//!      | ApprovalRequest                |
//!      |------------------------------->|
//!      |                                | (show modal dialog)
//!      |                                | (user presses y/n/a/d)
//!      |         ApprovalResponse       |
//!      |<-------------------------------|
//!      |                                |
//!      | (continue or deny)             |
//! ```

pub mod manager;
pub mod types;

pub use manager::ApprovalManager;
pub use types::{ApprovalAction, ApprovalDetails, ApprovalRequest, ApprovalResponse};
