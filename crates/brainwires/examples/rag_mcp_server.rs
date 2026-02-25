//! # RAG MCP Server Example
//!
//! A complete MCP server for semantic code search, built with the brainwires framework.
//! This is the framework equivalent of the standalone `project-rag` server.
//!
//! ## Features
//! - 9 MCP tools (index, query, search, git history, definitions, references, call graph, stats, clear)
//! - 9 MCP prompts (slash commands)
//! - Hybrid search (vector + BM25 keyword)
//! - AST-based code chunking (12 languages)
//! - Incremental indexing with change detection
//!
//! ## Run
//! ```sh
//! cargo run --example rag_mcp_server --features "rag,mcp-server"
//! ```
//!
//! ## Register in Claude Desktop
//! Add to `claude_desktop_config.json`:
//! ```json
//! {
//!   "mcpServers": {
//!     "project-rag": {
//!       "command": "cargo",
//!       "args": ["run", "--example", "rag_mcp_server", "--features", "rag,mcp-server"],
//!       "cwd": "/path/to/brainwires-framework"
//!     }
//!   }
//! }
//! ```

use brainwires::rag::mcp_server::RagMcpServer;

/// Set up a global panic handler that logs panics via tracing and stderr.
///
/// MCP servers communicate over stdio, so panics must never corrupt the
/// transport. This handler captures the panic, logs it, and prints to stderr.
fn setup_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic message".to_string()
        };

        tracing::error!("PANIC at {}: {}\nBacktrace:\n{:?}", location, message, backtrace);
        eprintln!("PANIC at {}: {}\nBacktrace:\n{:?}", location, message, backtrace);
    }));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (logs go to stderr, leaving stdout clean for MCP JSON-RPC)
    tracing_subscriber::fmt::init();

    // Capture panics so they don't corrupt the stdio MCP transport
    setup_panic_handler();

    // Start the RAG MCP server over stdio.
    // This single call creates the RagClient (embedding model, vector DB, BM25 index),
    // registers all 9 tools + 9 prompts, and serves over stdin/stdout.
    if let Err(e) = RagMcpServer::serve_stdio().await {
        tracing::error!("Fatal error in MCP server: {:#}", e);
        eprintln!("Fatal error: {:#}", e);
        std::process::exit(1);
    }

    Ok(())
}
