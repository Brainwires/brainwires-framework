use anyhow::Result;
use lazy_static::lazy_static;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::mcp::{McpToolRegistry, ProgressParams};
use crate::types::tool::{ToolContext, ToolResult};

lazy_static! {
    /// Global MCP tool registry for agent access
    pub static ref MCP_TOOLS: Arc<RwLock<McpToolRegistry>> = Arc::new(RwLock::new(McpToolRegistry::new()));
}

/// MCP Tool executor - bridges MCP tools to our tool system
pub struct McpToolExecutor;

impl McpToolExecutor {
    /// Execute an MCP tool
    pub fn execute(
        tool_use_id: &str,
        tool_name: &str,
        input: &Value,
        _context: &ToolContext,
    ) -> ToolResult {
        // Since execute is called from sync context, we need to use tokio runtime
        let runtime = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    "No tokio runtime available for MCP tool execution".to_string(),
                );
            }
        };

        // Execute async operation
        let result = runtime.block_on(async { Self::execute_async(tool_name, input).await });

        match result {
            Ok(content) => ToolResult::success(tool_use_id.to_string(), content),
            Err(e) => ToolResult::error(tool_use_id.to_string(), e.to_string()),
        }
    }

    /// Async execution of MCP tool
    async fn execute_async(tool_name: &str, input: &Value) -> Result<String> {
        let registry = MCP_TOOLS.read().await;

        // Extract arguments from input
        let arguments = if input.is_null() || input == &serde_json::json!({}) {
            None
        } else {
            Some(input.clone())
        };

        // Execute tool via MCP
        registry.execute_tool(tool_name, arguments).await
    }

    /// Execute an MCP tool asynchronously with progress notifications
    /// This is the preferred method for TUI integration as it provides real progress updates
    pub async fn execute_with_progress<F>(
        tool_name: &str,
        input: &Value,
        progress_callback: F,
    ) -> Result<String>
    where
        F: Fn(ProgressParams) + Send + 'static,
    {
        let registry = MCP_TOOLS.read().await;

        // Extract arguments from input
        let arguments = if input.is_null() || input == &serde_json::json!({}) {
            None
        } else {
            Some(input.clone())
        };

        // Execute tool via MCP with progress notifications
        registry
            .execute_tool_with_progress(tool_name, arguments, progress_callback)
            .await
    }

    /// Get all available MCP tools
    pub async fn get_all_tools() -> Vec<crate::types::tool::Tool> {
        match MCP_TOOLS.read().await.get_all_tools().await {
            Ok(tools) => tools,
            Err(e) => {
                eprintln!("Warning: Failed to fetch MCP tools: {}", e);
                Vec::new()
            }
        }
    }

    /// Register an MCP server for tool discovery
    pub async fn register_server(
        client: Arc<RwLock<crate::mcp::McpClient>>,
        server_name: String,
    ) -> Result<()> {
        MCP_TOOLS
            .write()
            .await
            .register_server(client, server_name)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_name_detection() {
        assert!("mcp_myserver_tool".starts_with("mcp_"));
        assert!(!"regular_tool".starts_with("mcp_"));
    }

    #[tokio::test]
    async fn test_get_all_tools() {
        let tools = McpToolExecutor::get_all_tools().await;
        // Should return empty list when no servers registered
        assert_eq!(tools.len(), 0);
    }
}
