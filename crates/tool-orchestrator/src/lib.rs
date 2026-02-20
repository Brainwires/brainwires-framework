//! Tool Orchestrator - Rhai-based tool orchestration for AI agents
//!
//! Implements Anthropic's "Programmatic Tool Calling" pattern for token-efficient
//! tool orchestration. Instead of sequential tool calls, AI writes Rhai scripts
//! that orchestrate multiple tools, returning only the final result.
//!
//! ## Features
//!
//! This crate supports multiple build targets via feature flags:
//!
//! - **`native`** (default) - Thread-safe Rust library with `Arc`/`Mutex`
//! - **`wasm`** - WebAssembly bindings for browser/Node.js via `wasm-bindgen`
//!
//! ## Benefits
//!
//! - **37% token reduction** - intermediate results don't pollute context
//! - **Parallel execution** - multiple tools in one pass
//! - **Complex orchestration** - loops, conditionals, data processing
//!
//! ## Example (Native)
//!
//! ```ignore
//! use tool_orchestrator::{ToolOrchestrator, ExecutionLimits};
//!
//! let mut orchestrator = ToolOrchestrator::new();
//! orchestrator.register_executor("greet", |input| {
//!     Ok(format!("Hello, {}!", input.as_str().unwrap_or("world")))
//! });
//!
//! let result = orchestrator.execute(
//!     r#"greet("Claude")"#,
//!     ExecutionLimits::default()
//! )?;
//!
//! assert_eq!(result.output, "Hello, Claude!");
//! ```
//!
//! ## Example (WASM)
//!
//! ```javascript
//! import { WasmOrchestrator, ExecutionLimits } from 'tool-orchestrator';
//!
//! const orchestrator = new WasmOrchestrator();
//! orchestrator.register_tool('greet', (input) => {
//!     const name = JSON.parse(input);
//!     return `Hello, ${name}!`;
//! });
//!
//! const result = orchestrator.execute(
//!     'greet("Claude")',
//!     ExecutionLimits.quick()
//! );
//!
//! console.log(result.output); // "Hello, Claude!"
//! ```

// Require either native or wasm feature (but not both)
#[cfg(not(any(feature = "native", feature = "wasm")))]
compile_error!(
    "Either the `native` or `wasm` feature must be enabled. \
     Add `features = [\"native\"]` to your Cargo.toml dependency or use `--features native`."
);

#[cfg(all(feature = "native", feature = "wasm"))]
compile_error!(
    "The `native` and `wasm` features are mutually exclusive. \
     Use `--features native` for Rust library or `--no-default-features --features wasm` for WASM."
);

// Core modules (always available)
pub mod engine;
pub mod sandbox;
pub mod types;

// Re-export core types
pub use engine::{dynamic_to_json, ToolExecutor, ToolOrchestrator};
pub use sandbox::{
    ExecutionLimits,
    // Default limit constants
    DEFAULT_MAX_ARRAY_SIZE, DEFAULT_MAX_MAP_SIZE, DEFAULT_MAX_OPERATIONS, DEFAULT_MAX_STRING_SIZE,
    DEFAULT_MAX_TOOL_CALLS, DEFAULT_TIMEOUT_MS,
    // Profile constants
    EXTENDED_MAX_OPERATIONS, EXTENDED_MAX_TOOL_CALLS, EXTENDED_TIMEOUT_MS, QUICK_MAX_OPERATIONS,
    QUICK_MAX_TOOL_CALLS, QUICK_TIMEOUT_MS,
};
pub use types::{OrchestratorError, OrchestratorResult, ToolCall};

// WASM module (only when wasm feature is enabled)
#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub use wasm::{ExecutionLimits as WasmExecutionLimits, WasmOrchestrator};
