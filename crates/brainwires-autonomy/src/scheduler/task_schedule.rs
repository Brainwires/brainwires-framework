//! Scheduled task definitions.

use serde::{Deserialize, Serialize};

/// Type of scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScheduledTaskType {
    /// Run a self-improvement cycle.
    SelfImprove {
        /// Repository path to improve.
        repo_path: String,
    },
    /// Run code quality checks (clippy, formatting, etc.).
    CodeQualityCheck {
        /// Repository path to check.
        repo_path: String,
    },
    /// Check for dependency updates.
    DependencyUpdate {
        /// Repository path.
        repo_path: String,
    },
    /// Run a security audit (cargo audit, npm audit, etc.).
    SecurityAudit {
        /// Repository path.
        repo_path: String,
    },
    /// Execute a custom command.
    CustomCommand {
        /// Command to run.
        cmd: String,
        /// Command arguments.
        args: Vec<String>,
        /// Working directory.
        working_dir: String,
    },
}

/// Policy for handling task failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum FailurePolicy {
    /// Ignore the failure and continue scheduling.
    Ignore,
    /// Retry with exponential backoff.
    Retry {
        /// Maximum retry attempts.
        max_retries: u32,
        /// Initial backoff in seconds.
        backoff_secs: u64,
    },
    /// Disable the task after failure.
    Disable,
    /// Escalate for human attention.
    Escalate,
}

impl Default for FailurePolicy {
    fn default() -> Self {
        Self::Retry {
            max_retries: 3,
            backoff_secs: 60,
        }
    }
}

/// A scheduled autonomous task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Unique task identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Cron expression (e.g., "0 */6 * * *" for every 6 hours).
    pub cron_expression: String,
    /// Type of task to run.
    pub task_type: ScheduledTaskType,
    /// Whether this task is enabled.
    pub enabled: bool,
    /// Maximum runtime in seconds before the task is killed.
    pub max_runtime_secs: u64,
    /// Policy for handling task failures.
    pub on_failure: FailurePolicy,
}

impl ScheduledTask {
    /// Create a new scheduled task with default settings.
    pub fn new(
        id: String,
        name: String,
        cron_expression: String,
        task_type: ScheduledTaskType,
    ) -> Self {
        Self {
            id,
            name,
            cron_expression,
            task_type,
            enabled: true,
            max_runtime_secs: 3600,
            on_failure: FailurePolicy::default(),
        }
    }
}

/// Result of a scheduled task execution.
#[derive(Debug, Clone)]
pub struct ScheduledTaskResult {
    /// Task that was executed.
    pub task_id: String,
    /// Whether the task succeeded.
    pub success: bool,
    /// Summary of what happened.
    pub summary: String,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Error message if failed.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_task_new_has_sane_defaults() {
        let task = ScheduledTask::new(
            "test".to_string(),
            "Test Task".to_string(),
            "0 * * * *".to_string(),
            ScheduledTaskType::CodeQualityCheck {
                repo_path: ".".to_string(),
            },
        );
        assert!(task.enabled);
        assert_eq!(task.max_runtime_secs, 3600);
    }

    #[test]
    fn scheduled_task_serialization_roundtrip() {
        let task = ScheduledTask::new(
            "test".to_string(),
            "Test".to_string(),
            "0 0 * * *".to_string(),
            ScheduledTaskType::SecurityAudit {
                repo_path: "/tmp".to_string(),
            },
        );
        let json = serde_json::to_string(&task).unwrap();
        let deserialized: ScheduledTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test");
    }
}
