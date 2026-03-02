//! # Brainwires Brain — Open Brain MCP Server
//!
//! Exposes persistent knowledge systems (thoughts, PKS, BKS) to any AI tool
//! via the Model Context Protocol (MCP).
//!
//! ## Dual Purpose
//!
//! - **Library**: Use `BrainClient` directly to capture and search thoughts programmatically
//! - **MCP Server**: Run `BrainMcpServer::serve_stdio()` to expose 7 tools + 5 prompts
//!
//! ## Quick Start
//!
//! ```no_run
//! use brainwires_brain::mcp_server::BrainMcpServer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     BrainMcpServer::serve_stdio().await
//! }
//! ```

pub mod brain_client;
pub mod fact_extractor;
pub mod mcp_server;
pub mod thought;
pub mod types;

// Re-export main types
pub use brain_client::BrainClient;
pub use mcp_server::BrainMcpServer;
pub use thought::{Thought, ThoughtCategory, ThoughtSource};
pub use types::{
    CaptureThoughtRequest, CaptureThoughtResponse, DeleteThoughtRequest, DeleteThoughtResponse,
    GetThoughtRequest, GetThoughtResponse, ListRecentRequest, ListRecentResponse,
    MemoryStatsRequest, MemoryStatsResponse, SearchKnowledgeRequest, SearchKnowledgeResponse,
    SearchMemoryRequest, SearchMemoryResponse,
};
