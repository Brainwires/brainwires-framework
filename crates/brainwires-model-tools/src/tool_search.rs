//! Tool Search - Meta-tool for discovering available tools dynamically

use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

use brainwires_core::{Tool, ToolContext, ToolInputSchema, ToolResult};
use crate::ToolRegistry;

/// Search mode for tool discovery
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    #[default]
    Keyword,
    Regex,
}

pub struct ToolSearchTool;

impl ToolSearchTool {
    pub fn get_tools() -> Vec<Tool> {
        vec![Self::search_tools_tool()]
    }

    fn search_tools_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert("query".to_string(), json!({"type": "string", "description": "Search query to find relevant tools"}));
        properties.insert("mode".to_string(), json!({"type": "string", "enum": ["keyword", "regex"], "description": "Search mode", "default": "keyword"}));
        properties.insert("include_deferred".to_string(), json!({"type": "boolean", "description": "Include deferred tools", "default": true}));
        Tool {
            name: "search_tools".to_string(),
            description: "Search for available tools by name or description.".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["query".to_string()]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    #[tracing::instrument(name = "tool.execute", skip(input, _context, registry), fields(tool_name))]
    pub fn execute(tool_use_id: &str, tool_name: &str, input: &Value, _context: &ToolContext, registry: &ToolRegistry) -> ToolResult {
        let result = match tool_name {
            "search_tools" => Self::search_tools(input, registry),
            _ => Err(anyhow::anyhow!("Unknown tool search tool: {}", tool_name)),
        };
        match result {
            Ok(output) => ToolResult::success(tool_use_id.to_string(), output),
            Err(e) => ToolResult::error(tool_use_id.to_string(), format!("Tool search failed: {}", e)),
        }
    }

    fn search_tools(input: &Value, registry: &ToolRegistry) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input { query: String, #[serde(default)] mode: SearchMode, #[serde(default = "dt")] include_deferred: bool }
        fn dt() -> bool { true }

        let params: Input = serde_json::from_value(input.clone())?;
        if params.mode == SearchMode::Regex && params.query.len() > 200 {
            return Err(anyhow::anyhow!("Regex pattern exceeds maximum length of 200 characters (got {})", params.query.len()));
        }

        let regex = if params.mode == SearchMode::Regex {
            Some(Regex::new(&params.query).map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", params.query, e))?)
        } else { None };

        let query_lower = params.query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let matching_tools: Vec<&Tool> = registry.get_all().iter().filter(|tool| {
            if tool.defer_loading && !params.include_deferred { return false; }
            let search_text = format!("{} {}", tool.name, tool.description);
            match &regex {
                Some(re) => re.is_match(&search_text),
                None => {
                    let name_lower = tool.name.to_lowercase();
                    let desc_lower = tool.description.to_lowercase();
                    query_terms.iter().any(|term| name_lower.contains(term) || desc_lower.contains(term))
                }
            }
        }).collect();

        if matching_tools.is_empty() {
            return Ok(format!("No tools found matching query: \"{}\"", params.query));
        }

        let mut result = format!("Found {} tools matching \"{}\":\n\n", matching_tools.len(), params.query);
        for tool in matching_tools {
            result.push_str(&format!("## {}\n**Description:** {}\n", tool.name, tool.description));
            if let Some(props) = &tool.input_schema.properties {
                result.push_str("**Parameters:**\n");
                for (name, schema) in props {
                    let desc = schema.get("description").and_then(|v| v.as_str()).unwrap_or("No description");
                    let ptype = schema.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    result.push_str(&format!("  - `{}` ({}): {}\n", name, ptype, desc));
                }
            }
            result.push('\n');
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tools() {
        let tools = ToolSearchTool::get_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "search_tools");
    }

    #[test]
    fn test_search_mode_default() {
        let mode = SearchMode::default();
        assert_eq!(mode, SearchMode::Keyword);
    }
}
