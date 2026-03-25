#![deny(missing_docs)]
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
//! - **email** (`email` feature) - Email send/search/read/list via IMAP/SMTP
//! - **calendar** (`calendar` feature) - Calendar CRUD and free/busy via Google Calendar/CalDAV
//!
//! ## Registry
//! The `ToolRegistry` is a composable container. Create one and register
//! whichever tools you need, or use `ToolRegistry::with_builtins()` for all.
//!
//! ```ignore
//! use brainwires_tool_system::{ToolRegistry, BashTool, FileOpsTool};
//!
//! let mut registry = ToolRegistry::new();
//! registry.register_tools(BashTool::get_tools());
//! registry.register_tools(FileOpsTool::get_tools());
//! ```

// Re-export core types for convenience
pub use brainwires_core::{
    CommitResult, IdempotencyRecord, IdempotencyRegistry, StagedWrite, StagingBackend, Tool,
    ToolContext, ToolInputSchema, ToolResult,
};

// ── Always-available modules (pure logic, WASM-safe) ────────────────────────

mod error;
pub mod executor;
mod registry;
pub mod sanitization;
mod tool_search;

// ── Native-only modules (require filesystem, process, network) ──────────────

#[cfg(feature = "native")]
mod bash;
#[cfg(feature = "native")]
mod file_ops;
#[cfg(feature = "native")]
mod git;
#[cfg(feature = "native")]
mod search;
#[cfg(feature = "native")]
pub mod transaction;
#[cfg(feature = "native")]
pub mod validation;
#[cfg(feature = "native")]
mod web;

// ── Feature-gated modules ────────────────────────────────────────────────────

#[cfg(any(feature = "orchestrator", feature = "orchestrator-wasm"))]
pub mod orchestrator;

#[cfg(feature = "interpreters")]
mod code_exec;

#[cfg(feature = "rag")]
mod semantic_search;

#[cfg(feature = "rag")]
mod tool_embedding;

pub mod smart_router;

#[cfg(feature = "openapi")]
pub mod openapi;

#[cfg(feature = "email")]
mod email;

#[cfg(feature = "calendar")]
mod calendar;

// ── Public re-exports ────────────────────────────────────────────────────────

// Always-available tools
pub use error::{ResourceType, RetryStrategy, ToolErrorCategory, ToolOutcome, classify_error};
pub use executor::{PreHookDecision, ToolExecutor, ToolPreHook};
pub use registry::{ToolCategory, ToolRegistry};
pub use sanitization::{
    contains_sensitive_data, filter_tool_output, is_injection_attempt, redact_sensitive_data,
    sanitize_external_content, wrap_with_content_source,
};
pub use tool_search::ToolSearchTool;

// Native-only tools
#[cfg(feature = "native")]
pub use bash::BashTool;
#[cfg(feature = "native")]
pub use file_ops::FileOpsTool;
#[cfg(feature = "native")]
pub use git::GitTool;
#[cfg(feature = "native")]
pub use search::SearchTool;
#[cfg(feature = "native")]
pub use transaction::TransactionManager;
#[cfg(feature = "native")]
pub use validation::{ValidationTool, get_validation_tools};
#[cfg(feature = "native")]
pub use web::WebTool;

// Feature-gated tools
#[cfg(any(feature = "orchestrator", feature = "orchestrator-wasm"))]
pub use orchestrator::OrchestratorTool;

#[cfg(feature = "interpreters")]
pub use code_exec::CodeExecTool;

#[cfg(feature = "rag")]
pub use semantic_search::SemanticSearchTool;

#[cfg(feature = "rag")]
pub use tool_embedding::ToolEmbeddingIndex;

pub use smart_router::{
    analyze_messages, analyze_query, get_context_for_analysis, get_smart_tools,
    get_smart_tools_with_mcp, get_tools_for_categories,
};

#[cfg(feature = "openapi")]
pub use openapi::{
    HttpMethod, OpenApiAuth, OpenApiEndpoint, OpenApiParam, OpenApiTool, execute_openapi_tool,
    openapi_to_tools,
};

#[cfg(feature = "email")]
pub use email::EmailTool;

#[cfg(feature = "calendar")]
pub use calendar::CalendarTool;
