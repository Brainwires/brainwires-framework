use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Specifies which contexts can invoke a tool.
/// Implements Anthropic's `allowed_callers` pattern for programmatic tool calling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCaller {
    /// Tool can be called directly by the AI
    Direct,
    /// Tool can only be called from within code/script execution
    CodeExecution,
}

impl Default for ToolCaller {
    fn default() -> Self {
        Self::Direct
    }
}

/// A tool that can be used by the AI agent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tool {
    /// Name of the tool
    #[serde(default)]
    pub name: String,
    /// Description of what the tool does
    #[serde(default)]
    pub description: String,
    /// Input schema (JSON Schema)
    #[serde(default)]
    pub input_schema: ToolInputSchema,
    /// Whether this tool requires user approval before execution
    #[serde(default)]
    pub requires_approval: bool,
    /// Whether this tool should be deferred from initial context loading.
    #[serde(default)]
    pub defer_loading: bool,
    /// Specifies which contexts can call this tool.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_callers: Vec<ToolCaller>,
    /// Example inputs that teach the AI proper parameter usage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_examples: Vec<Value>,
}

/// JSON Schema for tool input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type", default = "default_schema_type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

fn default_schema_type() -> String {
    "object".to_string()
}

impl Default for ToolInputSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: None,
            required: None,
        }
    }
}

impl ToolInputSchema {
    /// Create a new object schema
    pub fn object(properties: HashMap<String, Value>, required: Vec<String>) -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(required),
        }
    }
}

/// A tool use request from the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Unique ID for this tool use
    pub id: String,
    /// Name of the tool to use
    pub name: String,
    /// Input parameters for the tool
    pub input: Value,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool use this is a result for
    pub tool_use_id: String,
    /// Result content
    pub content: String,
    /// Whether this is an error result
    #[serde(default)]
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful tool result
    pub fn success<S: Into<String>>(tool_use_id: S, content: S) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    /// Create an error tool result
    pub fn error<S: Into<String>>(tool_use_id: S, error: S) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: error.into(),
            is_error: true,
        }
    }
}

/// Execution context for a tool.
///
/// Provides the working directory, optional metadata, and permission capabilities
/// to tool implementations.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current working directory for resolving relative paths
    pub working_directory: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Additional context data (application-specific key-value pairs)
    pub metadata: HashMap<String, String>,
    /// Agent capabilities for permission checks
    pub capabilities: Option<crate::permission::AgentCapabilities>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| ".".to_string()),
            user_id: None,
            metadata: HashMap::new(),
            capabilities: Some(crate::permission::AgentCapabilities::standard_dev()),
        }
    }
}

/// Tool selection mode
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToolMode {
    /// All tools from registry
    Full,
    /// User-selected specific tools (stores tool names)
    Explicit(Vec<String>),
    /// Smart routing based on query analysis (default)
    #[default]
    Smart,
    /// Core tools only
    Core,
    /// No tools enabled
    None,
}

impl ToolMode {
    /// Get a display name for the mode
    pub fn display_name(&self) -> &'static str {
        match self {
            ToolMode::Full => "full",
            ToolMode::Explicit(_) => "explicit",
            ToolMode::Smart => "smart",
            ToolMode::Core => "core",
            ToolMode::None => "none",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("tool-1", "Success!");
        assert!(!result.is_error);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("tool-2", "Failed!");
        assert!(result.is_error);
    }

    #[test]
    fn test_tool_input_schema_object() {
        let mut props = HashMap::new();
        props.insert("name".to_string(), json!({"type": "string"}));
        let schema = ToolInputSchema::object(props, vec!["name".to_string()]);
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_some());
    }
}
