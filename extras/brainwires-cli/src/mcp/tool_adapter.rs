use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::mcp::{JsonRpcNotification, McpClient, McpNotification, McpTool, ProgressParams};
use crate::types::tool::{Tool, ToolInputSchema};

/// MCP Tool Adapter - bridges MCP tools to our internal tool system
pub struct McpToolAdapter {
    client: Arc<RwLock<McpClient>>,
    server_name: String,
}

impl McpToolAdapter {
    pub fn new(client: Arc<RwLock<McpClient>>, server_name: String) -> Self {
        Self {
            client,
            server_name,
        }
    }

    /// Fetch all tools from the MCP server and convert to our Tool format
    pub async fn get_tools(&self) -> Result<Vec<Tool>> {
        let client = self.client.read().await;

        // Check if connected
        if !client.is_connected(&self.server_name).await {
            anyhow::bail!("Not connected to MCP server: {}", self.server_name);
        }

        // Fetch tools from server
        let mcp_tools = client.list_tools(&self.server_name).await?;

        // Convert to our Tool format
        let tools = mcp_tools
            .into_iter()
            .map(|mcp_tool| self.convert_mcp_tool(mcp_tool))
            .collect();

        Ok(tools)
    }

    /// Execute a tool on the MCP server
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<String> {
        let client = self.client.read().await;

        // Check if connected
        if !client.is_connected(&self.server_name).await {
            anyhow::bail!("Not connected to MCP server: {}", self.server_name);
        }

        // Call tool on server
        let result = client
            .call_tool(&self.server_name, tool_name, arguments)
            .await?;

        // Check for errors
        if result.is_error.unwrap_or(false) {
            anyhow::bail!("MCP tool execution failed: {:?}", result.content);
        }

        // Extract text content from result
        // rmcp uses Annotated<RawContent> which is a complex type
        let mut output = String::new();
        for content in result.content {
            use rmcp::model::RawContent;

            match &content.raw {
                RawContent::Text(text_content) => {
                    output.push_str(&text_content.text);
                    output.push('\n');
                }
                RawContent::Image(_) => {
                    output.push_str("[Image content]\n");
                }
                RawContent::Resource(resource) => {
                    output.push_str(&format!("[Embedded Resource]\n"));
                }
                RawContent::Audio(_) => {
                    output.push_str("[Audio content]\n");
                }
                RawContent::ResourceLink(resource) => {
                    output.push_str(&format!("[Resource: {}]\n", resource.name));
                }
            }
        }

        Ok(output.trim().to_string())
    }

    /// Execute a tool on the MCP server with progress notifications
    /// Progress updates are sent through the provided callback
    pub async fn execute_tool_with_progress<F>(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
        progress_callback: F,
    ) -> Result<String>
    where
        F: Fn(ProgressParams) + Send + 'static,
    {
        let client = self.client.read().await;

        // Check if connected
        if !client.is_connected(&self.server_name).await {
            anyhow::bail!("Not connected to MCP server: {}", self.server_name);
        }

        // Create notification channel
        let (notif_tx, mut notif_rx) = mpsc::unbounded_channel::<JsonRpcNotification>();

        // Spawn a task to process notifications and call the progress callback
        let progress_task = tokio::spawn(async move {
            while let Some(notification) = notif_rx.recv().await {
                match McpNotification::from_notification(&notification) {
                    McpNotification::Progress(params) => {
                        progress_callback(params);
                    }
                    McpNotification::Unknown { .. } => {
                        // Ignore unknown notifications
                    }
                }
            }
        });

        // Call tool with notification forwarding
        let result = client
            .call_tool_with_notifications(&self.server_name, tool_name, arguments, Some(notif_tx))
            .await;

        // Ensure progress task is cleaned up
        progress_task.abort();

        // Process result
        let result = result?;

        // Check for errors
        if result.is_error.unwrap_or(false) {
            anyhow::bail!("MCP tool execution failed: {:?}", result.content);
        }

        // Extract text content from result
        let mut output = String::new();
        for content in result.content {
            use rmcp::model::RawContent;

            match &content.raw {
                RawContent::Text(text_content) => {
                    output.push_str(&text_content.text);
                    output.push('\n');
                }
                RawContent::Image(_) => {
                    output.push_str("[Image content]\n");
                }
                RawContent::Resource(_) => {
                    output.push_str("[Embedded Resource]\n");
                }
                RawContent::Audio(_) => {
                    output.push_str("[Audio content]\n");
                }
                RawContent::ResourceLink(resource) => {
                    output.push_str(&format!("[Resource: {}]\n", resource.name));
                }
            }
        }

        Ok(output.trim().to_string())
    }

    /// Convert MCP tool to our internal Tool format
    fn convert_mcp_tool(&self, mcp_tool: McpTool) -> Tool {
        // Convert Arc<JsonObject> to Value for parsing
        // rmcp uses Arc<Map<String, Value>> for input_schema
        let schema_value = serde_json::Value::Object((*mcp_tool.input_schema).clone());

        // Parse the input schema from MCP (it's already a JSON Schema)
        let input_schema = if let Ok(schema_obj) = serde_json::from_value::<ToolInputSchema>(schema_value) {
            schema_obj
        } else {
            // Fallback: create a simple object schema
            ToolInputSchema::object(std::collections::HashMap::new(), vec![])
        };

        // Convert Option<Cow<'_, str>> to String
        let description = mcp_tool.description
            .map(|d| d.to_string())
            .unwrap_or_else(|| format!("MCP tool from {}", self.server_name));

        // Create tool with MCP-prefixed name
        Tool {
            name: format!("mcp_{}_{}", self.server_name, mcp_tool.name),
            description,
            input_schema,
            requires_approval: false, // MCP tools are assumed safe
            ..Default::default()
        }
    }
}

/// Global MCP tool registry for agents
pub struct McpToolRegistry {
    adapters: Arc<RwLock<Vec<Arc<McpToolAdapter>>>>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register an MCP server for tool discovery
    pub async fn register_server(
        &self,
        client: Arc<RwLock<McpClient>>,
        server_name: String,
    ) -> Result<()> {
        let adapter = Arc::new(McpToolAdapter::new(client, server_name));
        self.adapters.write().await.push(adapter);
        Ok(())
    }

    /// Get all tools from all registered MCP servers
    pub async fn get_all_tools(&self) -> Result<Vec<Tool>> {
        let adapters = self.adapters.read().await;
        let mut all_tools = Vec::new();

        for adapter in adapters.iter() {
            match adapter.get_tools().await {
                Ok(tools) => all_tools.extend(tools),
                Err(e) => {
                    eprintln!("Warning: Failed to get tools from MCP server: {}", e);
                }
            }
        }

        Ok(all_tools)
    }

    /// Find adapter for a specific tool name
    pub async fn find_adapter(&self, tool_name: &str) -> Option<Arc<McpToolAdapter>> {
        // Tool name format: mcp_{server}_{tool}
        if !tool_name.starts_with("mcp_") {
            return None;
        }

        let parts: Vec<&str> = tool_name.split('_').collect();
        if parts.len() < 3 {
            return None;
        }

        let server_name = parts[1];
        let adapters = self.adapters.read().await;

        adapters
            .iter()
            .find(|a| a.server_name == server_name)
            .cloned()
    }

    /// Execute an MCP tool
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<String> {
        let adapter = self
            .find_adapter(tool_name)
            .await
            .context("MCP adapter not found for tool")?;

        // Extract actual tool name (remove mcp_{server}_ prefix)
        let parts: Vec<&str> = tool_name.split('_').collect();
        let actual_tool_name = parts[2..].join("_");

        adapter.execute_tool(&actual_tool_name, arguments).await
    }

    /// Execute an MCP tool with progress notifications
    /// Progress updates are sent through the provided callback
    pub async fn execute_tool_with_progress<F>(
        &self,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
        progress_callback: F,
    ) -> Result<String>
    where
        F: Fn(ProgressParams) + Send + 'static,
    {
        let adapter = self
            .find_adapter(tool_name)
            .await
            .context("MCP adapter not found for tool")?;

        // Extract actual tool name (remove mcp_{server}_ prefix)
        let parts: Vec<&str> = tool_name.split('_').collect();
        let actual_tool_name = parts[2..].join("_");

        adapter.execute_tool_with_progress(&actual_tool_name, arguments, progress_callback).await
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name_parsing() {
        let tool_name = "mcp_myserver_read_file";
        let parts: Vec<&str> = tool_name.split('_').collect();

        assert_eq!(parts[0], "mcp");
        assert_eq!(parts[1], "myserver");
        assert_eq!(parts[2..].join("_"), "read_file");
    }

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = McpToolRegistry::new();
        let tools = registry.get_all_tools().await.unwrap();
        assert_eq!(tools.len(), 0);
    }
}
