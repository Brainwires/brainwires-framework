use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementConfig {
    pub max_cycles: u32,
    pub max_budget: f64,
    pub dry_run: bool,
    pub strategies: Vec<String>,
    pub agent_iterations: u32,
    pub max_diff_per_task: u32,
    pub max_total_diff: u32,
    pub create_prs: bool,
    pub branch_prefix: String,
    pub no_bridge: bool,
    pub no_direct: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub circuit_breaker_threshold: u32,
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self {
            max_cycles: 10,
            max_budget: 10.0,
            dry_run: false,
            strategies: Vec::new(), // empty = all
            agent_iterations: 25,
            max_diff_per_task: 200,
            max_total_diff: 1000,
            create_prs: false,
            branch_prefix: "self-improve/".to_string(),
            no_bridge: false,
            no_direct: false,
            model: None,
            provider: None,
            circuit_breaker_threshold: 3,
        }
    }
}

impl SelfImprovementConfig {
    pub fn is_strategy_enabled(&self, name: &str) -> bool {
        self.strategies.is_empty() || self.strategies.iter().any(|s| s == name)
    }
}

#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub repo_path: String,
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
