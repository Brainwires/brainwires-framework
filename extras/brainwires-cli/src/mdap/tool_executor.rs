//! Microagent Tool Executor
//!
//! This module provides a dedicated tool executor for MDAP microagents that:
//! - Executes tool intents AFTER voting consensus (not during)
//! - Enforces recursion depth limits to prevent infinite loops
//! - Restricts tools to safe categories by default (read-only)
//! - Integrates with the existing permission system
//!
//! # Design
//!
//! The executor wraps the main `ToolExecutor` but adds microagent-specific
//! constraints:
//! - Atomic depth tracking for recursion prevention
//! - Category-based tool allowlists
//! - Restricted trust level for microagent contexts

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;

use super::error::{MdapError, MdapResult};
use super::tool_intent::{ToolCategory, ToolIntent};
use crate::tools::ToolExecutor;
use crate::types::tool::{ToolContext, ToolResult, ToolUse};

/// Configuration for microagent tool execution
#[derive(Clone, Debug)]
pub struct MicroagentToolConfig {
    /// Maximum recursion depth for tool chains (default: 3)
    pub max_depth: u32,
    /// Allowed tool categories (default: read-only)
    pub allowed_categories: HashSet<ToolCategory>,
    /// Whether to log all tool executions
    pub audit_enabled: bool,
    /// Timeout for individual tool executions in milliseconds
    pub timeout_ms: u64,
}

impl Default for MicroagentToolConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            allowed_categories: ToolCategory::read_only_categories(),
            audit_enabled: true,
            timeout_ms: 30_000,
        }
    }
}

impl MicroagentToolConfig {
    /// Create a config that allows all tools (use with caution)
    pub fn permissive() -> Self {
        let mut categories = ToolCategory::read_only_categories();
        categories.extend(ToolCategory::side_effect_categories());
        Self {
            allowed_categories: categories,
            ..Default::default()
        }
    }

    /// Create a strict read-only config
    pub fn read_only() -> Self {
        Self::default()
    }

    /// Add an allowed category
    pub fn allow_category(mut self, category: ToolCategory) -> Self {
        self.allowed_categories.insert(category);
        self
    }

    /// Set maximum recursion depth
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }
}

/// Executes tool intents after voting consensus
///
/// This executor is specifically designed for MDAP microagents and enforces:
/// - Recursion depth limits
/// - Tool category restrictions
/// - Proper permission handling
pub struct MicroagentToolExecutor {
    /// Reference to main tool executor
    tool_executor: Arc<RwLock<ToolExecutor>>,
    /// Configuration
    config: MicroagentToolConfig,
    /// Current recursion depth (atomic for thread safety)
    current_depth: Arc<AtomicU32>,
}

impl MicroagentToolExecutor {
    /// Create a new microagent tool executor
    pub fn new(
        tool_executor: Arc<RwLock<ToolExecutor>>,
        config: MicroagentToolConfig,
    ) -> Self {
        Self {
            tool_executor,
            config,
            current_depth: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Create with default read-only configuration
    pub fn read_only(tool_executor: Arc<RwLock<ToolExecutor>>) -> Self {
        Self::new(tool_executor, MicroagentToolConfig::read_only())
    }

    /// Get current recursion depth
    pub fn current_depth(&self) -> u32 {
        self.current_depth.load(Ordering::SeqCst)
    }

    /// Check if a tool is allowed for microagent execution
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.config.allowed_categories.iter().any(|cat| cat.contains_tool(tool_name))
    }

    /// Get the category for a tool name
    pub fn get_tool_category(&self, tool_name: &str) -> Option<ToolCategory> {
        // Check all known categories
        let all_categories = [
            ToolCategory::FileRead,
            ToolCategory::FileWrite,
            ToolCategory::Search,
            ToolCategory::SemanticSearch,
            ToolCategory::Bash,
            ToolCategory::Git,
            ToolCategory::Web,
            ToolCategory::AgentPool,
            ToolCategory::TaskManager,
            ToolCategory::Mcp,
        ];

        all_categories.into_iter().find(|cat| cat.contains_tool(tool_name))
    }

    /// Validate that a tool can be executed
    fn validate_tool(&self, intent: &ToolIntent) -> MdapResult<()> {
        // Check if tool is allowed
        if !self.is_tool_allowed(&intent.tool_name) {
            let category = self.get_tool_category(&intent.tool_name)
                .map(|c| format!("{:?}", c))
                .unwrap_or_else(|| "Unknown".to_string());

            return Err(MdapError::ToolNotAllowed {
                tool: intent.tool_name.clone(),
                category,
            });
        }

        Ok(())
    }

    /// Execute a tool intent after voting consensus
    ///
    /// This is the main entry point for tool execution. It:
    /// 1. Checks recursion depth
    /// 2. Validates tool is allowed
    /// 3. Executes via main tool executor
    /// 4. Returns result or error
    pub async fn execute_intent(
        &self,
        intent: &ToolIntent,
        context: &ToolContext,
    ) -> MdapResult<ToolResult> {
        // Check recursion depth
        let depth = self.current_depth.fetch_add(1, Ordering::SeqCst);
        if depth >= self.config.max_depth {
            self.current_depth.fetch_sub(1, Ordering::SeqCst);
            return Err(MdapError::ToolRecursionLimit {
                depth,
                max_depth: self.config.max_depth,
            });
        }

        // Validate tool is allowed
        if let Err(e) = self.validate_tool(intent) {
            self.current_depth.fetch_sub(1, Ordering::SeqCst);
            return Err(e);
        }

        // Execute via main tool executor
        let tool_use = ToolUse {
            id: uuid::Uuid::new_v4().to_string(),
            name: intent.tool_name.clone(),
            input: intent.arguments.clone(),
        };

        let result = {
            let executor = self.tool_executor.read().await;
            executor.execute(&tool_use, context).await
        };

        // Decrement depth counter
        self.current_depth.fetch_sub(1, Ordering::SeqCst);

        result.map_err(|e| MdapError::ToolExecutionFailed {
            tool: intent.tool_name.clone(),
            reason: e.to_string(),
        })
    }

    /// Execute multiple tool intents in sequence
    ///
    /// Executes intents one at a time, failing fast on first error.
    pub async fn execute_intents(
        &self,
        intents: &[ToolIntent],
        context: &ToolContext,
    ) -> MdapResult<Vec<ToolResult>> {
        let mut results = Vec::with_capacity(intents.len());

        for intent in intents {
            let result = self.execute_intent(intent, context).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Execute tool intents in parallel where safe
    ///
    /// Only read-only tools can be executed in parallel.
    pub async fn execute_intents_parallel(
        &self,
        intents: &[ToolIntent],
        context: &ToolContext,
    ) -> MdapResult<Vec<ToolResult>> {
        // Check all intents are read-only
        let read_only_cats = ToolCategory::read_only_categories();
        let all_read_only = intents.iter().all(|intent| {
            self.get_tool_category(&intent.tool_name)
                .map(|cat| read_only_cats.contains(&cat))
                .unwrap_or(false)
        });

        if !all_read_only || intents.len() <= 1 {
            // Fall back to sequential execution
            return self.execute_intents(intents, context).await;
        }

        // Execute in parallel
        let futures: Vec<_> = intents
            .iter()
            .map(|intent| self.execute_intent(intent, context))
            .collect();

        let results = futures::future::try_join_all(futures).await?;
        Ok(results)
    }

    /// Create a restricted context for microagent tool execution
    ///
    /// This adds metadata indicating the context is from a microagent,
    /// which can be used by the permission system.
    pub fn create_microagent_context(&self, base_context: &ToolContext) -> ToolContext {
        let mut context = base_context.clone();
        context.metadata.insert(
            "execution_context".to_string(),
            "mdap_microagent".to_string(),
        );
        context.metadata.insert(
            "max_tool_depth".to_string(),
            self.config.max_depth.to_string(),
        );
        context
    }
}

/// Builder for MicroagentToolExecutor
pub struct MicroagentToolExecutorBuilder {
    tool_executor: Option<Arc<RwLock<ToolExecutor>>>,
    config: MicroagentToolConfig,
}

impl Default for MicroagentToolExecutorBuilder {
    fn default() -> Self {
        Self {
            tool_executor: None,
            config: MicroagentToolConfig::default(),
        }
    }
}

impl MicroagentToolExecutorBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tool executor
    pub fn tool_executor(mut self, executor: Arc<RwLock<ToolExecutor>>) -> Self {
        self.tool_executor = Some(executor);
        self
    }

    /// Set configuration
    pub fn config(mut self, config: MicroagentToolConfig) -> Self {
        self.config = config;
        self
    }

    /// Set maximum recursion depth
    pub fn max_depth(mut self, depth: u32) -> Self {
        self.config.max_depth = depth;
        self
    }

    /// Allow a tool category
    pub fn allow_category(mut self, category: ToolCategory) -> Self {
        self.config.allowed_categories.insert(category);
        self
    }

    /// Use read-only configuration
    pub fn read_only(mut self) -> Self {
        self.config = MicroagentToolConfig::read_only();
        self
    }

    /// Use permissive configuration
    pub fn permissive(mut self) -> Self {
        self.config = MicroagentToolConfig::permissive();
        self
    }

    /// Build the executor
    pub fn build(self) -> MdapResult<MicroagentToolExecutor> {
        let tool_executor = self.tool_executor.ok_or_else(|| {
            MdapError::ConfigurationError("Tool executor is required".to_string())
        })?;

        Ok(MicroagentToolExecutor::new(tool_executor, self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::agent::PermissionMode;

    fn create_test_executor() -> Arc<RwLock<ToolExecutor>> {
        Arc::new(RwLock::new(ToolExecutor::new(PermissionMode::Auto)))
    }

    #[test]
    fn test_tool_allowed() {
        let executor = MicroagentToolExecutor::read_only(create_test_executor());

        // Read-only tools should be allowed
        assert!(executor.is_tool_allowed("read_file"));
        assert!(executor.is_tool_allowed("grep"));
        assert!(executor.is_tool_allowed("semantic_search"));

        // Write/dangerous tools should not be allowed
        assert!(!executor.is_tool_allowed("write_file"));
        assert!(!executor.is_tool_allowed("bash"));
        assert!(!executor.is_tool_allowed("git_commit"));
    }

    #[test]
    fn test_category_detection() {
        let executor = MicroagentToolExecutor::read_only(create_test_executor());

        assert_eq!(executor.get_tool_category("read_file"), Some(ToolCategory::FileRead));
        assert_eq!(executor.get_tool_category("write_file"), Some(ToolCategory::FileWrite));
        assert_eq!(executor.get_tool_category("bash"), Some(ToolCategory::Bash));
        assert_eq!(executor.get_tool_category("mcp__test__tool"), Some(ToolCategory::Mcp));
    }

    #[test]
    fn test_config_builder() {
        let config = MicroagentToolConfig::default()
            .allow_category(ToolCategory::Git)
            .with_max_depth(5);

        assert_eq!(config.max_depth, 5);
        assert!(config.allowed_categories.contains(&ToolCategory::Git));
        assert!(config.allowed_categories.contains(&ToolCategory::FileRead));
    }

    #[tokio::test]
    async fn test_recursion_tracking() {
        let executor = MicroagentToolExecutor::new(
            create_test_executor(),
            MicroagentToolConfig::default().with_max_depth(2),
        );

        assert_eq!(executor.current_depth(), 0);

        // Simulating depth tracking (actual execution would require mock tools)
        executor.current_depth.fetch_add(1, Ordering::SeqCst);
        assert_eq!(executor.current_depth(), 1);

        executor.current_depth.fetch_add(1, Ordering::SeqCst);
        assert_eq!(executor.current_depth(), 2);

        executor.current_depth.fetch_sub(2, Ordering::SeqCst);
        assert_eq!(executor.current_depth(), 0);
    }
}
