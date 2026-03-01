use anyhow::Result;
use async_trait::async_trait;
use brainwires_mcp::{CallToolResult, InitializeParams, ServerCapabilities, ServerInfo};
use serde_json::Value;

use crate::connection::RequestContext;
use crate::registry::McpToolDef;

#[async_trait]
pub trait McpHandler: Send + Sync + 'static {
    fn server_info(&self) -> ServerInfo;
    fn capabilities(&self) -> ServerCapabilities;
    fn list_tools(&self) -> Vec<McpToolDef>;
    async fn call_tool(
        &self,
        name: &str,
        args: Value,
        ctx: &RequestContext,
    ) -> Result<CallToolResult>;

    async fn on_initialize(&self, _params: &InitializeParams) -> Result<()> {
        Ok(())
    }

    async fn on_shutdown(&self) -> Result<()> {
        Ok(())
    }
}
