//! Configuration types for autonomous operations.

use serde::{Deserialize, Serialize};

/// Top-level configuration for the autonomy subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AutonomyConfig {
    /// Self-improvement session configuration.
    #[serde(default)]
    pub self_improve: SelfImprovementConfig,
    /// Safety and budget limits.
    #[serde(default)]
    pub safety: SafetyConfig,
    /// Git workflow configuration.
    #[serde(default)]
    pub git_workflow: GitWorkflowConfig,
}


/// Configuration for self-improvement sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementConfig {
    /// Maximum improvement cycles to run.
    pub max_cycles: u32,
    /// Maximum total cost in USD.
    pub max_budget: f64,
    /// If true, generate tasks but don't execute them.
    pub dry_run: bool,
    /// Enabled strategy names (empty = all).
    pub strategies: Vec<String>,
    /// Max iterations per agent task.
    pub agent_iterations: u32,
    /// Max diff lines per single task.
    pub max_diff_per_task: u32,
    /// Max total diff lines across entire session.
    pub max_total_diff: u32,
    /// Create PRs for committed changes.
    pub create_prs: bool,
    /// Git branch prefix for improvement branches.
    pub branch_prefix: String,
    /// Override model for agent tasks.
    pub model: Option<String>,
    /// Override provider.
    pub provider: Option<String>,
    /// Consecutive failures before circuit breaker trips.
    pub circuit_breaker_threshold: u32,
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self {
            max_cycles: 10,
            max_budget: 10.0,
            dry_run: false,
            strategies: Vec::new(),
            agent_iterations: 25,
            max_diff_per_task: 200,
            max_total_diff: 1000,
            create_prs: false,
            branch_prefix: "self-improve/".to_string(),
            model: None,
            provider: None,
            circuit_breaker_threshold: 3,
        }
    }
}

impl SelfImprovementConfig {
    /// Check if a given strategy name is enabled (empty list = all enabled).
    pub fn is_strategy_enabled(&self, name: &str) -> bool {
        self.strategies.is_empty() || self.strategies.iter().any(|s| s == name)
    }
}

/// Per-strategy configuration passed to strategy task generators.
#[derive(Debug, Clone)]
pub struct StrategyConfig {
    /// Path to the repository root.
    pub repo_path: String,
    /// Maximum tasks to generate per strategy.
    pub max_tasks_per_strategy: usize,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            repo_path: ".".to_string(),
            max_tasks_per_strategy: 5,
        }
    }
}

/// Safety and budget configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Maximum total cost in USD across all operations.
    pub max_total_cost: f64,
    /// Maximum cost per single operation.
    pub max_per_operation_cost: f64,
    /// Maximum daily operations.
    pub max_daily_operations: u32,
    /// Consecutive failure threshold for circuit breaker.
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker cooldown in seconds.
    pub circuit_breaker_cooldown_secs: u64,
    /// Max diff lines per task.
    pub max_diff_per_task: u32,
    /// Max total diff lines per session.
    pub max_total_diff: u32,
    /// Max concurrent agents.
    pub max_concurrent_agents: u32,
    /// Dead man's switch heartbeat timeout in seconds.
    pub heartbeat_timeout_secs: u64,
    /// Allowed path globs for file modifications.
    pub allowed_paths: Vec<String>,
    /// Forbidden path globs (takes precedence over allowed).
    pub forbidden_paths: Vec<String>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_total_cost: 50.0,
            max_per_operation_cost: 5.0,
            max_daily_operations: 100,
            circuit_breaker_threshold: 3,
            circuit_breaker_cooldown_secs: 300,
            max_diff_per_task: 200,
            max_total_diff: 1000,
            max_concurrent_agents: 5,
            heartbeat_timeout_secs: 1800,
            allowed_paths: Vec::new(),
            forbidden_paths: Vec::new(),
        }
    }
}

/// Git workflow pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWorkflowConfig {
    /// Branch prefix for autonomous fix branches.
    pub branch_prefix: String,
    /// Whether to auto-merge PRs when policy allows.
    pub auto_merge: bool,
    /// Default merge method.
    pub merge_method: String,
    /// Minimum investigation confidence to proceed with fix.
    pub min_confidence: f64,
    /// Webhook server configuration.
    #[serde(default)]
    pub webhook: WebhookConfig,
}

impl Default for GitWorkflowConfig {
    fn default() -> Self {
        Self {
            branch_prefix: "autonomy/".to_string(),
            auto_merge: false,
            merge_method: "squash".to_string(),
            min_confidence: 0.7,
            webhook: WebhookConfig::default(),
        }
    }
}

/// Webhook server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Listen address.
    pub listen_addr: String,
    /// Listen port.
    pub port: u16,
    /// Webhook secret for HMAC verification.
    pub secret: Option<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0".to_string(),
            port: 3000,
            secret: None,
        }
    }
}
