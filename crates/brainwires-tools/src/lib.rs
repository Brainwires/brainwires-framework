#![deny(missing_docs)]
//! `brainwires-tools` — Built-in tool implementations for the Brainwires Agent Framework.
//!
//! This crate is now a **façade**:
//!
//! - The execution runtime (`ToolExecutor` trait, `ToolRegistry`, error
//!   taxonomy, sanitization, validation, transactions, smart router, plus
//!   optional orchestrator / OAuth / OpenAPI / sandbox / sessions / RAG-tool
//!   modules) lives in [`brainwires-tool-runtime`](https://docs.rs/brainwires-tool-runtime).
//! - The concrete builtin tools — `bash`, `file_ops`, `git`, `web`, `search`,
//!   `code_exec` (+ `interpreters/`), `semantic_search`, `browser`, `email`,
//!   `calendar`, `system` — and the `BuiltinToolExecutor` that hardcodes
//!   dispatch to them, live in this crate.
//!
//! Both layers are surfaced here at `brainwires_tools::*` so existing
//! consumers do not need to update imports.
//!
//! ## Always Available (concrete tools, native feature)
//! - **bash** — Shell command execution with proactive output management
//! - **file_ops** — File read/write/edit/patch/list/search/delete/create_directory
//! - **git** — Git operations (status, diff, log, stage, commit, push, pull, etc.)
//! - **web** — URL fetching
//! - **search** — Regex-based code search (respects .gitignore)
//!
//! ## Always Available (runtime, surfaced via re-export)
//! - **executor / registry / error / sanitization / smart_router / tool_search**
//! - **validation / transaction** (native feature)
//!
//! ## Feature-Gated builtins
//! - **code_exec / interpreters** (`interpreters` feature)
//! - **semantic_search** (`rag` feature)
//! - **email** (`email` feature)
//! - **calendar** (`calendar` feature)
//! - **browser** (`browser` feature)
//! - **system** (`system` feature)
//!
//! ## Feature-Gated runtime (passthrough to brainwires-tool-runtime)
//! - **orchestrator** (`orchestrator` feature)
//! - **oauth** / **openapi** / **sandbox** / **sessions**

// ── Re-export the runtime crate's public surface ────────────────────────────

// Module re-exports — preserve `brainwires_tools::<module>::*` import paths.
pub use brainwires_tool_runtime::{error, executor, registry, sanitization, smart_router, tool_search};

#[cfg(feature = "native")]
pub use brainwires_tool_runtime::{transaction, validation};

#[cfg(any(feature = "orchestrator", feature = "orchestrator-wasm"))]
pub use brainwires_tool_runtime::orchestrator;

#[cfg(feature = "oauth")]
pub use brainwires_tool_runtime::oauth;

#[cfg(feature = "openapi")]
pub use brainwires_tool_runtime::openapi;

#[cfg(feature = "sandbox")]
pub use brainwires_tool_runtime::sandbox_executor;

#[cfg(feature = "sessions")]
pub use brainwires_tool_runtime::sessions;

#[cfg(feature = "rag")]
pub use brainwires_tool_runtime::tool_embedding;

// Type re-exports — preserve `brainwires_tools::<Type>` paths.
pub use brainwires_tool_runtime::{
    CommitResult, IdempotencyRecord, IdempotencyRegistry, PreHookDecision, ResourceType,
    RetryStrategy, StagedWrite, StagingBackend, Tool, ToolCategory, ToolContext, ToolErrorCategory,
    ToolExecutor, ToolInputSchema, ToolOutcome, ToolPreHook, ToolRegistry, ToolResult,
    ToolSearchTool, analyze_messages, analyze_query, classify_error, contains_sensitive_data,
    filter_tool_output, get_context_for_analysis, get_smart_tools, get_smart_tools_with_mcp,
    get_tools_for_categories, is_injection_attempt, redact_sensitive_data, sanitize_external_content,
    wrap_with_content_source,
};

#[cfg(feature = "native")]
pub use brainwires_tool_runtime::{TransactionManager, ValidationTool, get_validation_tools};

#[cfg(any(feature = "orchestrator", feature = "orchestrator-wasm"))]
pub use brainwires_tool_runtime::OrchestratorTool;

#[cfg(feature = "openapi")]
pub use brainwires_tool_runtime::{
    HttpMethod, OpenApiAuth, OpenApiEndpoint, OpenApiParam, OpenApiTool, execute_openapi_tool,
    openapi_to_tools,
};

#[cfg(feature = "sandbox")]
pub use brainwires_tool_runtime::SandboxedToolExecutor;

#[cfg(feature = "sessions")]
pub use brainwires_tool_runtime::{
    SessionBroker, SessionId, SessionMessage, SessionSummary, SessionsTool, SpawnRequest,
    SpawnedSession,
};

#[cfg(feature = "rag")]
pub use brainwires_tool_runtime::ToolEmbeddingIndex;

// ── Builtin modules that still live in this crate ───────────────────────────

mod default_executor;

#[cfg(feature = "native")]
mod bash;
#[cfg(feature = "native")]
mod file_ops;
#[cfg(feature = "native")]
mod git;
#[cfg(feature = "native")]
mod search;
#[cfg(feature = "native")]
mod web;

#[cfg(feature = "interpreters")]
mod code_exec;

#[cfg(feature = "rag")]
mod semantic_search;

#[cfg(feature = "email")]
mod email;

#[cfg(feature = "calendar")]
pub mod calendar;

#[cfg(feature = "browser")]
mod browser;

/// OS-level primitives — filesystem event watching and service management
/// (absorbed from brainwires-system).
#[cfg(feature = "system")]
pub mod system;

/// Sandboxed multi-language code interpreters (absorbed from brainwires-code-interpreters).
#[cfg(feature = "interpreters")]
pub mod interpreters;

// ── Builtin re-exports ──────────────────────────────────────────────────────

pub use default_executor::BuiltinToolExecutor;

/// Build a [`ToolRegistry`] pre-populated with every concrete builtin tool
/// gated on by the active feature set.
///
/// Replaces the old `ToolRegistry::with_builtins()` constructor which lived
/// in `brainwires-tool-runtime` but couldn't actually reference the builtins
/// after the runtime/builtins split.
pub fn registry_with_builtins() -> ToolRegistry {
    let mut registry = ToolRegistry::with_runtime_meta_tools();

    #[cfg(feature = "native")]
    {
        registry.register_tools(BashTool::get_tools());
        registry.register_tools(FileOpsTool::get_tools());
        registry.register_tools(GitTool::get_tools());
        registry.register_tools(WebTool::get_tools());
        registry.register_tools(SearchTool::get_tools());
        registry.register_tools(get_validation_tools());
    }

    #[cfg(any(feature = "orchestrator", feature = "orchestrator-wasm"))]
    registry.register_tools(OrchestratorTool::get_tools());

    #[cfg(feature = "interpreters")]
    registry.register_tools(CodeExecTool::get_tools());

    #[cfg(feature = "rag")]
    registry.register_tools(SemanticSearchTool::get_tools());

    registry
}

#[cfg(feature = "native")]
pub use bash::BashTool;
#[cfg(feature = "native")]
pub use file_ops::FileOpsTool;
#[cfg(feature = "native")]
pub use git::GitTool;
#[cfg(feature = "native")]
pub use search::SearchTool;
#[cfg(feature = "native")]
pub use web::WebTool;

#[cfg(feature = "interpreters")]
pub use code_exec::CodeExecTool;

#[cfg(feature = "rag")]
pub use semantic_search::SemanticSearchTool;

#[cfg(feature = "email")]
pub use email::{EmailConfig, EmailProvider, EmailSource, EmailTool, gmail_push};

#[cfg(feature = "calendar")]
pub use calendar::CalendarTool;

#[cfg(feature = "browser")]
pub use browser::BrowserTool;
