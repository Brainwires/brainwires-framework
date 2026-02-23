use anyhow::Result;
use async_trait::async_trait;
use brainwires_mcp::CallToolResult;
use serde_json::Value;

use crate::connection::RequestContext;
use crate::error::BridgeError;

#[derive(Debug, Clone)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn call(&self, args: Value, ctx: &RequestContext) -> Result<CallToolResult>;
}

struct RegisteredTool {
    def: McpToolDef,
    handler: Box<dyn ToolHandler>,
}

pub struct McpToolRegistry {
    tools: Vec<RegisteredTool>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        input_schema: Value,
        handler: impl ToolHandler + 'static,
    ) {
        self.tools.push(RegisteredTool {
            def: McpToolDef {
                name: name.to_string(),
                description: description.to_string(),
                input_schema,
            },
            handler: Box::new(handler),
        });
    }

    pub fn list_tools(&self) -> Vec<&McpToolDef> {
        self.tools.iter().map(|t| &t.def).collect()
    }

    pub fn list_tool_defs(&self) -> Vec<McpToolDef> {
        self.tools.iter().map(|t| t.def.clone()).collect()
    }

    pub async fn dispatch(
        &self,
        name: &str,
        args: Value,
        ctx: &RequestContext,
    ) -> Result<CallToolResult> {
        for tool in &self.tools {
            if tool.def.name == name {
                return tool.handler.call(args, ctx).await;
            }
        }
        Err(BridgeError::ToolNotFound(name.to_string()).into())
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.def.name == name)
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
    use serde_json::json;

    struct EchoHandler;

    #[async_trait]
    impl ToolHandler for EchoHandler {
        async fn call(&self, _args: Value, _ctx: &RequestContext) -> Result<CallToolResult> {
            Ok(CallToolResult::success(vec![]))
        }
    }

    #[test]
    fn test_registry_register_and_list() {
        let mut registry = McpToolRegistry::new();
        registry.register(
            "echo",
            "Echo tool",
            json!({"type": "object"}),
            EchoHandler,
        );

        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn test_registry_has_tool() {
        let mut registry = McpToolRegistry::new();
        registry.register(
            "test",
            "Test tool",
            json!({"type": "object"}),
            EchoHandler,
        );

        assert!(registry.has_tool("test"));
        assert!(!registry.has_tool("nonexistent"));
    }
}
