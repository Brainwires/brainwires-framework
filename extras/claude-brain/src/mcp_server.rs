//! MCP server — exposes context management tools to Claude Code.

use anyhow::{Context, Result};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    tool, tool_handler, tool_router,
};

use crate::config::ClaudeBrainConfig;
use crate::context_manager::ContextManager;

/// Request to recall context from conversation history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct RecallContextRequest {
    /// Natural language query to search conversation history.
    pub query: String,
    /// Maximum results (default: 10).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Minimum relevance score 0.0-1.0 (default: 0.6).
    #[serde(default = "default_min_score")]
    pub min_score: f32,
}

/// Request to capture a thought.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CaptureRequest {
    /// The thought, decision, or insight to persist.
    pub content: String,
    /// Category: decision, insight, preference, action_item, reference, general.
    #[serde(default)]
    pub category: Option<String>,
    /// Tags for organization.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Importance 0.0-1.0 (default: 0.5).
    #[serde(default)]
    pub importance: Option<f32>,
}

/// Request to search memory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchRequest {
    /// Natural language search query.
    pub query: String,
    /// Maximum results (default: 10).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Minimum relevance score (default: 0.6).
    #[serde(default = "default_min_score")]
    pub min_score: f32,
}

/// Request to search knowledge base (PKS/BKS).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct KnowledgeSearchRequest {
    /// Query for PKS/BKS knowledge.
    pub query: String,
    /// Maximum results (default: 10).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Empty request for stats.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct StatsRequest {}

fn default_limit() -> usize {
    10
}
fn default_min_score() -> f32 {
    0.3
}

/// Claude Brain MCP server.
#[derive(Clone)]
pub struct ClaudeBrainMcpServer {
    ctx: std::sync::Arc<ContextManager>,
    tool_router: ToolRouter<Self>,
}

impl ClaudeBrainMcpServer {
    /// Create from config.
    pub async fn new(config: ClaudeBrainConfig) -> Result<Self> {
        let ctx = std::sync::Arc::new(
            ContextManager::new(config)
                .await
                .context("Failed to create ContextManager")?,
        );
        Ok(Self {
            ctx,
            tool_router: Self::tool_router(),
        })
    }

    /// Serve on stdin/stdout.
    pub async fn serve_stdio() -> Result<()> {
        tracing::info!("Starting Claude Brain MCP server");

        let config = ClaudeBrainConfig::load()?;
        let server = Self::new(config)
            .await
            .context("Failed to create Claude Brain MCP server")?;

        let transport = rmcp::transport::io::stdio();
        server.serve(transport).await?.waiting().await?;

        Ok(())
    }
}

// ── Tool definitions ─────────────────────────────────────────────────────

#[tool_router(router = tool_router)]
impl ClaudeBrainMcpServer {
    #[tool(
        description = "Search conversation history for context that may be outside the current window. Use this when you need to recall earlier discussion details, decisions, code snippets, or any information from previous turns or sessions."
    )]
    async fn recall_context(
        &self,
        Parameters(req): Parameters<RecallContextRequest>,
    ) -> Result<String, String> {
        let response = self
            .ctx
            .search_memory(&req.query, req.limit, req.min_score)
            .await
            .map_err(|e| format!("{:#}", e))?;
        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {e}"))
    }

    #[tool(
        description = "Capture a thought, decision, insight, or important context into persistent memory. Automatically categorizes, extracts tags, embeds for semantic search, and extracts knowledge facts."
    )]
    async fn capture_thought(
        &self,
        Parameters(req): Parameters<CaptureRequest>,
    ) -> Result<String, String> {
        use brainwires_knowledge::knowledge::types::CaptureThoughtRequest;

        let mut client = self.ctx.client().lock_owned().await;
        let response = client
            .capture_thought(CaptureThoughtRequest {
                content: req.content,
                category: req.category,
                tags: req.tags,
                importance: req.importance,
                source: Some("claude-brain-mcp".to_string()),
            })
            .await
            .map_err(|e| format!("{:#}", e))?;
        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {e}"))
    }

    #[tool(
        description = "Search across all memory tiers — thoughts, personal facts (PKS), and behavioral knowledge (BKS). Returns results ranked by relevance."
    )]
    async fn search_memory(
        &self,
        Parameters(req): Parameters<SearchRequest>,
    ) -> Result<String, String> {
        let response = self
            .ctx
            .search_memory(&req.query, req.limit, req.min_score)
            .await
            .map_err(|e| format!("{:#}", e))?;
        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {e}"))
    }

    #[tool(
        description = "Query personal knowledge (PKS) facts and behavioral knowledge (BKS) truths. Use this before making choices to check for known preferences."
    )]
    async fn search_knowledge(
        &self,
        Parameters(req): Parameters<KnowledgeSearchRequest>,
    ) -> Result<String, String> {
        let response = self
            .ctx
            .search_knowledge(&req.query, req.limit)
            .await
            .map_err(|e| format!("{:#}", e))?;
        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {e}"))
    }

    #[tool(
        description = "Dashboard of knowledge statistics — thought counts by category, PKS fact counts, BKS truth counts, capture frequency, and top tags."
    )]
    async fn memory_stats(
        &self,
        Parameters(_req): Parameters<StatsRequest>,
    ) -> Result<String, String> {
        let response = self
            .ctx
            .memory_stats()
            .await
            .map_err(|e| format!("{:#}", e))?;
        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization failed: {e}"))
    }
}

// ── ServerHandler ────────────────────────────────────────────────────────

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ClaudeBrainMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info =
            Implementation::new("claude-brain", env!("CARGO_PKG_VERSION"))
                .with_title("Claude Brain — Brainwires Context Management for Claude Code");
        info.instructions = Some(
            "Claude Brain replaces Claude Code's default compaction with Brainwires \
             research-grade context management. Use recall_context to search past \
             conversation history, capture_thought to persist decisions and insights, \
             search_memory for semantic retrieval across all tiers, search_knowledge \
             for PKS/BKS facts, and memory_stats for a dashboard."
                .into(),
        );
        info
    }
}
