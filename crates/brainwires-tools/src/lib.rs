#![deny(missing_docs)]
//! `brainwires-tools` — Built-in tool implementations for the Brainwires Agent Framework.
//!
//! This crate is a **façade** re-exporting two underlying crates:
//!
//! - [`brainwires-tool-runtime`](https://docs.rs/brainwires-tool-runtime) — the
//!   execution-runtime layer (`ToolExecutor` trait, `ToolRegistry`, error
//!   taxonomy, sanitization, validation, transactions, smart router, plus
//!   optional orchestrator / OAuth / OpenAPI / sandbox / sessions / RAG-tool
//!   modules).
//! - [`brainwires-tool-builtins`](https://docs.rs/brainwires-tool-builtins) —
//!   the concrete builtin tools (`bash`, `file_ops`, `git`, `web`, `search`,
//!   `code_exec` + `interpreters`, `semantic_search`, `browser`, `email`,
//!   `calendar`, `system`) and the `BuiltinToolExecutor` that hardcodes
//!   dispatch to them.
//!
//! Both layers are surfaced here so existing imports (`brainwires_tools::*`,
//! `brainwires_tools::executor::*`, `brainwires_tools::sessions::*`, …) keep
//! working unchanged. New code should generally depend on whichever
//! sub-crate it actually needs:
//!
//! - building a custom tool framework on top of the runtime → depend on
//!   `brainwires-tool-runtime`.
//! - shipping the standard builtin tools → depend on `brainwires-tool-builtins`
//!   (which already pulls `brainwires-tool-runtime` as a dep).
//! - wanting both with one toggle → depend on this façade.

// ── Runtime re-exports (modules + types) ───────────────────────────────────

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

// ── Builtin re-exports (modules + types) ───────────────────────────────────

#[cfg(feature = "calendar")]
pub use brainwires_tool_builtins::calendar;

#[cfg(feature = "system")]
pub use brainwires_tool_builtins::system;

#[cfg(feature = "interpreters")]
pub use brainwires_tool_builtins::interpreters;

pub use brainwires_tool_builtins::BuiltinToolExecutor;

#[cfg(feature = "native")]
pub use brainwires_tool_builtins::{BashTool, FileOpsTool, GitTool, SearchTool, WebTool};

#[cfg(feature = "interpreters")]
pub use brainwires_tool_builtins::CodeExecTool;

#[cfg(feature = "rag")]
pub use brainwires_tool_builtins::SemanticSearchTool;

#[cfg(feature = "email")]
pub use brainwires_tool_builtins::{EmailConfig, EmailProvider, EmailSource, EmailTool, gmail_push};

#[cfg(feature = "calendar")]
pub use brainwires_tool_builtins::CalendarTool;

#[cfg(feature = "browser")]
pub use brainwires_tool_builtins::BrowserTool;

/// Re-export of the [`brainwires_tool_builtins::registry_with_builtins`]
/// helper — builds a [`ToolRegistry`] pre-populated with every concrete
/// builtin gated on by the active feature set.
pub use brainwires_tool_builtins::registry_with_builtins;
