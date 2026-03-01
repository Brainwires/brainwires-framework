//! Tool Registry - Composable container for tool definitions
//!
//! Provides a `ToolRegistry` that stores tool definitions and supports
//! deferred loading, category filtering, and search.

use brainwires_core::Tool;

/// Tool categories for filtering tools by purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    FileOps,
    Search,
    SemanticSearch,
    Git,
    TaskManager,
    AgentPool,
    Web,
    WebSearch,
    Bash,
    Planning,
    Context,
    Orchestrator,
    CodeExecution,
    SessionTask,
    Validation,
}

/// Composable tool registry - stores and queries tool definitions.
///
/// Unlike the CLI's registry which auto-registers all tools, this registry
/// is empty by default. Callers compose it by registering tools from
/// whichever modules they need.
///
/// # Example
/// ```ignore
/// use brainwires_tooling::{ToolRegistry, BashTool, FileOpsTool, GitTool};
///
/// let mut registry = ToolRegistry::new();
/// registry.register_tools(BashTool::get_tools());
/// registry.register_tools(FileOpsTool::get_tools());
/// registry.register_tools(GitTool::get_tools());
/// ```
pub struct ToolRegistry {
    tools: Vec<Tool>,
}

impl ToolRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self { tools: vec![] }
    }

    /// Create a registry pre-populated with all built-in tools
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();

        // Always-available tools
        registry.register_tools(crate::ToolSearchTool::get_tools());

        // Native-only tools
        #[cfg(feature = "native")]
        {
            registry.register_tools(crate::FileOpsTool::get_tools());
            registry.register_tools(crate::BashTool::get_tools());
            registry.register_tools(crate::GitTool::get_tools());
            registry.register_tools(crate::WebTool::get_tools());
            registry.register_tools(crate::SearchTool::get_tools());
            registry.register_tools(crate::get_validation_tools());
        }

        // Feature-gated tools
        #[cfg(feature = "orchestrator")]
        registry.register_tools(crate::OrchestratorTool::get_tools());

        #[cfg(feature = "interpreters")]
        registry.register_tools(crate::CodeExecTool::get_tools());

        #[cfg(feature = "rag")]
        registry.register_tools(crate::SemanticSearchTool::get_tools());

        registry
    }

    /// Register a single tool
    pub fn register(&mut self, tool: Tool) {
        self.tools.push(tool);
    }

    /// Register multiple tools at once
    pub fn register_tools(&mut self, tools: Vec<Tool>) {
        self.tools.extend(tools);
    }

    /// Get all registered tools
    pub fn get_all(&self) -> &[Tool] {
        &self.tools
    }

    /// Get all tools including additional external tools (e.g., MCP tools)
    pub fn get_all_with_extra(&self, extra: &[Tool]) -> Vec<Tool> {
        let mut all = self.tools.clone();
        all.extend(extra.iter().cloned());
        all
    }

    /// Look up a tool by name
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Get tools that should be loaded initially (defer_loading = false)
    pub fn get_initial_tools(&self) -> Vec<&Tool> {
        self.tools.iter().filter(|t| !t.defer_loading).collect()
    }

    /// Get only deferred tools (defer_loading = true)
    pub fn get_deferred_tools(&self) -> Vec<&Tool> {
        self.tools.iter().filter(|t| t.defer_loading).collect()
    }

    /// Search tools by query string matching name and description
    pub fn search_tools(&self, query: &str) -> Vec<&Tool> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        self.tools
            .iter()
            .filter(|tool| {
                let name_lower = tool.name.to_lowercase();
                let desc_lower = tool.description.to_lowercase();
                query_terms
                    .iter()
                    .any(|term| name_lower.contains(term) || desc_lower.contains(term))
            })
            .collect()
    }

    /// Get tools by category
    pub fn get_by_category(&self, category: ToolCategory) -> Vec<&Tool> {
        let names: &[&str] = match category {
            ToolCategory::FileOps => &[
                "read_file", "write_file", "edit_file", "patch_file",
                "list_directory", "search_files", "delete_file", "create_directory",
            ],
            ToolCategory::Search => &["search_code", "search_files"],
            ToolCategory::SemanticSearch => &[
                "index_codebase", "query_codebase", "search_with_filters",
                "get_rag_statistics", "clear_rag_index", "search_git_history",
            ],
            ToolCategory::Git => &[
                "git_status", "git_diff", "git_log", "git_stage", "git_unstage",
                "git_commit", "git_push", "git_pull", "git_fetch",
                "git_discard", "git_branch",
            ],
            ToolCategory::TaskManager => &[
                "task_create", "task_start", "task_complete", "task_list",
                "task_skip", "task_add", "task_block", "task_depends",
                "task_ready", "task_time",
            ],
            ToolCategory::AgentPool => &[
                "agent_spawn", "agent_status", "agent_list", "agent_stop", "agent_await",
            ],
            ToolCategory::Web => &["fetch_url"],
            ToolCategory::WebSearch => &["web_search", "web_browse", "web_scrape"],
            ToolCategory::Bash => &["execute_command"],
            ToolCategory::Planning => &["plan_task"],
            ToolCategory::Context => &["recall_context"],
            ToolCategory::Orchestrator => &["execute_script"],
            ToolCategory::CodeExecution => &["execute_code"],
            ToolCategory::SessionTask => &["task_list_write"],
            ToolCategory::Validation => &["check_duplicates", "verify_build", "check_syntax"],
        };

        self.tools
            .iter()
            .filter(|t| names.contains(&t.name.as_str()))
            .collect()
    }

    /// Get all tools including MCP tools
    pub fn get_all_with_mcp(&self, mcp_tools: &[Tool]) -> Vec<Tool> {
        self.get_all_with_extra(mcp_tools)
    }

    /// Get core tools for basic project exploration
    pub fn get_core(&self) -> Vec<&Tool> {
        let core_names = [
            "read_file", "write_file", "edit_file", "list_directory",
            "search_code", "execute_command", "git_status", "git_diff",
            "git_log", "git_stage", "git_commit", "search_tools",
            "index_codebase", "query_codebase",
        ];
        self.tools
            .iter()
            .filter(|t| core_names.contains(&t.name.as_str()))
            .collect()
    }

    /// Get primary meta-tools (always available)
    pub fn get_primary(&self) -> Vec<&Tool> {
        let primary_names = ["execute_script", "search_tools"];
        self.tools
            .iter()
            .filter(|t| primary_names.contains(&t.name.as_str()))
            .collect()
    }

    /// Total number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ToolInputSchema;
    use std::collections::HashMap;

    fn make_tool(name: &str, defer: bool) -> Tool {
        Tool {
            name: name.to_string(),
            description: format!("A {} tool", name),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: defer,
            ..Default::default()
        }
    }

    #[test]
    fn test_new_is_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_single() {
        let mut registry = ToolRegistry::new();
        registry.register(make_tool("test_tool", false));
        assert_eq!(registry.len(), 1);
        assert!(registry.get("test_tool").is_some());
    }

    #[test]
    fn test_register_multiple() {
        let mut registry = ToolRegistry::new();
        registry.register_tools(vec![
            make_tool("tool1", false),
            make_tool("tool2", false),
        ]);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_get_by_name() {
        let mut registry = ToolRegistry::new();
        registry.register(make_tool("my_tool", false));

        assert!(registry.get("my_tool").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_initial_vs_deferred() {
        let mut registry = ToolRegistry::new();
        registry.register(make_tool("initial", false));
        registry.register(make_tool("deferred", true));

        assert_eq!(registry.get_initial_tools().len(), 1);
        assert_eq!(registry.get_initial_tools()[0].name, "initial");

        assert_eq!(registry.get_deferred_tools().len(), 1);
        assert_eq!(registry.get_deferred_tools()[0].name, "deferred");
    }

    #[test]
    fn test_search_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Tool {
            name: "read_file".to_string(),
            description: "Read a file from disk".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            ..Default::default()
        });
        registry.register(Tool {
            name: "write_file".to_string(),
            description: "Write content to a file".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            ..Default::default()
        });
        registry.register(Tool {
            name: "execute_command".to_string(),
            description: "Execute a bash command".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            ..Default::default()
        });

        let results = registry.search_tools("file");
        assert_eq!(results.len(), 2);

        let results = registry.search_tools("bash");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_all_with_extra() {
        let mut registry = ToolRegistry::new();
        registry.register(make_tool("builtin", false));

        let extra = vec![make_tool("mcp_tool", false)];
        let all = registry.get_all_with_extra(&extra);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_no_duplicate_names_in_builtins() {
        let registry = ToolRegistry::with_builtins();
        let mut seen = std::collections::HashSet::new();
        for tool in registry.get_all() {
            assert!(
                seen.insert(tool.name.clone()),
                "Duplicate tool name: {}",
                tool.name
            );
        }
    }
}
