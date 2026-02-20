//! Brainwires Tools - Built-in tool implementations for the Brainwires Agent Framework
//!
//! This crate provides a composable set of tools that agents can use:
//!
//! ## Always Available
//! - **bash** - Shell command execution with proactive output management
//! - **file_ops** - File read/write/edit/patch/list/search/delete/create_directory
//! - **git** - Git operations (status, diff, log, stage, commit, push, pull, etc.)
//! - **web** - URL fetching
//! - **search** - Regex-based code search (respects .gitignore)
//! - **validation** - Code quality checks (duplicates, build, syntax)
//! - **tool_search** - Meta-tool for dynamic tool discovery
//! - **error** - Error taxonomy and classification for retry strategies
//!
//! ## Feature-Gated
//! - **orchestrator** (`orchestrator` feature) - Rhai script orchestration
//! - **code_exec** (`interpreters` feature) - Sandboxed multi-language code execution
//! - **semantic_search** (`rag` feature) - RAG-powered semantic codebase search
//!
//! ## Registry
//! The `ToolRegistry` is a composable container. Create one and register
//! whichever tools you need, or use `ToolRegistry::with_builtins()` for all.
//!
//! ```ignore
//! use brainwires_tools::{ToolRegistry, BashTool, FileOpsTool};
//!
//! let mut registry = ToolRegistry::new();
//! registry.register_tools(BashTool::get_tools());
//! registry.register_tools(FileOpsTool::get_tools());
//! ```

// Re-export core types for convenience
pub use brainwires_core::{Tool, ToolContext, ToolInputSchema, ToolResult};

// ── Core modules (always available) ──────────────────────────────────────────

mod bash;
mod error;
mod file_ops;
mod git;
mod registry;
mod search;
mod tool_search;
mod validation;
mod web;

// ── Feature-gated modules ────────────────────────────────────────────────────

#[cfg(feature = "orchestrator")]
mod orchestrator;

#[cfg(feature = "interpreters")]
mod code_exec;

#[cfg(feature = "rag")]
mod semantic_search;

// ── Public re-exports ────────────────────────────────────────────────────────

// Core tools
pub use bash::BashTool;
pub use file_ops::FileOpsTool;
pub use git::GitTool;
pub use search::SearchTool;
pub use tool_search::ToolSearchTool;
pub use validation::{get_validation_tools, ValidationTool};
pub use web::WebTool;

// Error taxonomy
pub use error::{classify_error, ResourceType, RetryStrategy, ToolErrorCategory, ToolOutcome};

// Registry
pub use registry::{ToolCategory, ToolRegistry};

// Feature-gated tools
#[cfg(feature = "orchestrator")]
pub use orchestrator::OrchestratorTool;

#[cfg(feature = "interpreters")]
pub use code_exec::CodeExecTool;

#[cfg(feature = "rag")]
pub use semantic_search::SemanticSearchTool;
