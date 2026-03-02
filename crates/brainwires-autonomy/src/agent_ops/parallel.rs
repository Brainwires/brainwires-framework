//! Parallel coordinator — fan-out/fan-in for multi-agent task execution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A unit of work in a parallel execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTask {
    /// Unique task identifier.
    pub id: String,
    /// Task description for the agent.
    pub description: String,
    /// Working directory.
    pub working_directory: String,
    /// Task dependencies (must complete before this task starts).
    pub depends_on: Vec<String>,
    /// Maximum iterations for this task.
    pub max_iterations: u32,
}

/// Result from a parallel task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelTaskResult {
    pub task_id: String,
    pub success: bool,
    pub summary: String,
    pub iterations: u32,
    pub cost: f64,
}

/// Status of a parallel execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParallelPlanStatus {
    Pending,
    Running { completed: usize, total: usize },
    Completed { results: Vec<ParallelTaskResult> },
    Failed { reason: String, partial_results: Vec<ParallelTaskResult> },
}

/// Configuration for the parallel coordinator.
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum concurrent agents.
    pub max_concurrent: usize,
    /// Whether to use MDAP voting for task results.
    pub use_mdap: bool,
    /// Fail fast: stop all tasks if any task fails.
    pub fail_fast: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            use_mdap: false,
            fail_fast: false,
        }
    }
}

/// Coordinates parallel execution of multiple agent tasks.
///
/// Handles task dependency resolution, concurrent execution limits,
/// and result aggregation.
pub struct ParallelCoordinator {
    config: ParallelConfig,
    tasks: Vec<ParallelTask>,
    results: HashMap<String, ParallelTaskResult>,
}

impl ParallelCoordinator {
    pub fn new(config: ParallelConfig) -> Self {
        Self {
            config,
            tasks: Vec::new(),
            results: HashMap::new(),
        }
    }

    /// Add a task to the execution plan.
    pub fn add_task(&mut self, task: ParallelTask) {
        self.tasks.push(task);
    }

    /// Get tasks that are ready to execute (all dependencies satisfied).
    pub fn ready_tasks(&self) -> Vec<&ParallelTask> {
        self.tasks
            .iter()
            .filter(|t| {
                !self.results.contains_key(&t.id)
                    && t.depends_on.iter().all(|dep| {
                        self.results.get(dep).is_some_and(|r| r.success)
                    })
            })
            .collect()
    }

    /// Record the result of a completed task.
    pub fn record_result(&mut self, result: ParallelTaskResult) {
        self.results.insert(result.task_id.clone(), result);
    }

    /// Check if all tasks are completed.
    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| self.results.contains_key(&t.id))
    }

    /// Check if any task has failed (relevant for fail-fast mode).
    pub fn has_failure(&self) -> bool {
        self.results.values().any(|r| !r.success)
    }

    /// Get the current plan status.
    pub fn status(&self) -> ParallelPlanStatus {
        if self.results.is_empty() && !self.tasks.is_empty() {
            return ParallelPlanStatus::Pending;
        }

        let completed = self.results.len();
        let total = self.tasks.len();

        if completed < total {
            if self.config.fail_fast && self.has_failure() {
                return ParallelPlanStatus::Failed {
                    reason: "fail-fast: a task failed".to_string(),
                    partial_results: self.results.values().cloned().collect(),
                };
            }
            return ParallelPlanStatus::Running { completed, total };
        }

        let results: Vec<ParallelTaskResult> = self.results.values().cloned().collect();
        if results.iter().all(|r| r.success) {
            ParallelPlanStatus::Completed { results }
        } else {
            ParallelPlanStatus::Failed {
                reason: "one or more tasks failed".to_string(),
                partial_results: results,
            }
        }
    }

    /// Get aggregate statistics.
    pub fn stats(&self) -> ParallelStats {
        let results: Vec<&ParallelTaskResult> = self.results.values().collect();
        ParallelStats {
            total_tasks: self.tasks.len(),
            completed: results.len(),
            succeeded: results.iter().filter(|r| r.success).count(),
            failed: results.iter().filter(|r| !r.success).count(),
            total_iterations: results.iter().map(|r| r.iterations as u64).sum(),
            total_cost: results.iter().map(|r| r.cost).sum(),
        }
    }
}

/// Aggregate statistics for a parallel execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelStats {
    pub total_tasks: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_iterations: u64,
    pub total_cost: f64,
}
