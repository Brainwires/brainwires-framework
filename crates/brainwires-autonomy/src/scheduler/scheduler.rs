//! Main scheduler loop for managing recurring autonomous tasks.

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::watch;

use super::cron_trigger::CronTrigger;
use super::task_schedule::{FailurePolicy, ScheduledTask, ScheduledTaskResult, ScheduledTaskType};
use crate::config::SchedulerConfig;

/// Scheduler that manages multiple cron-based autonomous tasks.
pub struct AutonomyScheduler {
    config: SchedulerConfig,
    triggers: Vec<CronTrigger>,
    failure_counts: HashMap<String, u32>,
    disabled_tasks: HashMap<String, bool>,
}

impl AutonomyScheduler {
    /// Create a new scheduler from configuration.
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            triggers: Vec::new(),
            failure_counts: HashMap::new(),
            disabled_tasks: HashMap::new(),
        }
    }

    /// Add a scheduled task.
    pub fn add_task(&mut self, task: ScheduledTask) -> anyhow::Result<()> {
        if !task.enabled {
            tracing::info!("Skipping disabled task: {}", task.name);
            return Ok(());
        }
        let trigger = CronTrigger::new(task)?;
        self.triggers.push(trigger);
        Ok(())
    }

    /// Add multiple tasks from configuration.
    pub fn add_tasks_from_config(&mut self) -> anyhow::Result<()> {
        let defs = self.config.tasks.clone();
        for def in defs {
            let task = ScheduledTask {
                id: def.id,
                name: def.name,
                cron_expression: def.cron_expression,
                task_type: ScheduledTaskType::CustomCommand {
                    cmd: def.task_type.clone(),
                    args: Vec::new(),
                    working_dir: ".".to_string(),
                },
                enabled: def.enabled,
                max_runtime_secs: def.max_runtime_secs,
                on_failure: FailurePolicy::default(),
            };
            self.add_task(task)?;
        }
        Ok(())
    }

    /// Run the scheduler loop until cancelled.
    pub async fn run(&mut self, mut cancel: watch::Receiver<bool>) {
        tracing::info!("Scheduler started with {} tasks", self.triggers.len());

        loop {
            if *cancel.borrow() {
                tracing::info!("Scheduler cancelled");
                break;
            }

            // Find the next task to fire
            let mut earliest: Option<(usize, std::time::Duration)> = None;
            for (i, trigger) in self.triggers.iter().enumerate() {
                if self.disabled_tasks.get(trigger.task_id()).copied().unwrap_or(false) {
                    continue;
                }
                if let Some(dur) = trigger.duration_until_next() {
                    match &earliest {
                        None => earliest = Some((i, dur)),
                        Some((_, existing_dur)) if dur < *existing_dur => {
                            earliest = Some((i, dur));
                        }
                        _ => {}
                    }
                }
            }

            let (trigger_idx, wait_dur) = match earliest {
                Some(v) => v,
                None => {
                    // No tasks to run, sleep and check again
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => continue,
                        _ = cancel.changed() => break,
                    }
                }
            };

            // Wait until the next fire time or cancellation
            tokio::select! {
                _ = tokio::time::sleep(wait_dur) => {},
                _ = cancel.changed() => break,
            }

            // Execute the task
            let task = self.triggers[trigger_idx].task().clone();
            tracing::info!("Firing scheduled task: {} ({})", task.name, task.id);

            let result = self.execute_task(&task).await;

            match &result {
                Ok(r) if r.success => {
                    tracing::info!("Task {} completed in {:.1}s", task.id, r.duration_secs);
                    self.failure_counts.remove(&task.id);
                }
                Ok(r) => {
                    tracing::warn!(
                        "Task {} failed: {}",
                        task.id,
                        r.error.as_deref().unwrap_or("unknown")
                    );
                    self.handle_failure(&task);
                }
                Err(e) => {
                    tracing::error!("Task {} execution error: {e}", task.id);
                    self.handle_failure(&task);
                }
            }
        }

        tracing::info!("Scheduler stopped");
    }

    async fn execute_task(&self, task: &ScheduledTask) -> anyhow::Result<ScheduledTaskResult> {
        let start = Instant::now();

        let result = match &task.task_type {
            ScheduledTaskType::CustomCommand {
                cmd,
                args,
                working_dir,
            } => {
                let timeout = std::time::Duration::from_secs(task.max_runtime_secs);
                let output = tokio::time::timeout(
                    timeout,
                    tokio::process::Command::new(cmd)
                        .args(args)
                        .current_dir(working_dir)
                        .output(),
                )
                .await;

                match output {
                    Ok(Ok(o)) if o.status.success() => ScheduledTaskResult {
                        task_id: task.id.clone(),
                        success: true,
                        summary: format!("Command succeeded: {cmd}"),
                        duration_secs: start.elapsed().as_secs_f64(),
                        error: None,
                    },
                    Ok(Ok(o)) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        ScheduledTaskResult {
                            task_id: task.id.clone(),
                            success: false,
                            summary: format!("Command failed: {cmd}"),
                            duration_secs: start.elapsed().as_secs_f64(),
                            error: Some(stderr.to_string()),
                        }
                    }
                    Ok(Err(e)) => ScheduledTaskResult {
                        task_id: task.id.clone(),
                        success: false,
                        summary: format!("Command execution error: {cmd}"),
                        duration_secs: start.elapsed().as_secs_f64(),
                        error: Some(e.to_string()),
                    },
                    Err(_) => ScheduledTaskResult {
                        task_id: task.id.clone(),
                        success: false,
                        summary: format!("Task timed out after {}s", task.max_runtime_secs),
                        duration_secs: start.elapsed().as_secs_f64(),
                        error: Some("Execution timeout".to_string()),
                    },
                }
            }
            ScheduledTaskType::CodeQualityCheck { repo_path } => {
                self.run_code_quality(task, repo_path, start).await
            }
            ScheduledTaskType::SecurityAudit { repo_path } => {
                self.run_security_audit(task, repo_path, start).await
            }
            ScheduledTaskType::DependencyUpdate { repo_path } => {
                self.run_dependency_update(task, repo_path, start).await
            }
            ScheduledTaskType::SelfImprove { repo_path } => {
                // Self-improve is wired at a higher level; here we just log
                ScheduledTaskResult {
                    task_id: task.id.clone(),
                    success: true,
                    summary: format!("Self-improve scheduled for {repo_path}"),
                    duration_secs: start.elapsed().as_secs_f64(),
                    error: None,
                }
            }
        };

        Ok(result)
    }

    async fn run_code_quality(
        &self,
        task: &ScheduledTask,
        repo_path: &str,
        start: Instant,
    ) -> ScheduledTaskResult {
        let output = tokio::process::Command::new("cargo")
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .current_dir(repo_path)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => ScheduledTaskResult {
                task_id: task.id.clone(),
                success: true,
                summary: "Code quality check passed".to_string(),
                duration_secs: start.elapsed().as_secs_f64(),
                error: None,
            },
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                ScheduledTaskResult {
                    task_id: task.id.clone(),
                    success: false,
                    summary: "Code quality issues found".to_string(),
                    duration_secs: start.elapsed().as_secs_f64(),
                    error: Some(stderr.chars().take(1000).collect()),
                }
            }
            Err(e) => ScheduledTaskResult {
                task_id: task.id.clone(),
                success: false,
                summary: "Failed to run clippy".to_string(),
                duration_secs: start.elapsed().as_secs_f64(),
                error: Some(e.to_string()),
            },
        }
    }

    async fn run_security_audit(
        &self,
        task: &ScheduledTask,
        repo_path: &str,
        start: Instant,
    ) -> ScheduledTaskResult {
        let output = tokio::process::Command::new("cargo")
            .args(["audit"])
            .current_dir(repo_path)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => ScheduledTaskResult {
                task_id: task.id.clone(),
                success: true,
                summary: "Security audit passed".to_string(),
                duration_secs: start.elapsed().as_secs_f64(),
                error: None,
            },
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                ScheduledTaskResult {
                    task_id: task.id.clone(),
                    success: false,
                    summary: "Security vulnerabilities found".to_string(),
                    duration_secs: start.elapsed().as_secs_f64(),
                    error: Some(stdout.chars().take(1000).collect()),
                }
            }
            Err(e) => ScheduledTaskResult {
                task_id: task.id.clone(),
                success: false,
                summary: "Failed to run cargo audit".to_string(),
                duration_secs: start.elapsed().as_secs_f64(),
                error: Some(e.to_string()),
            },
        }
    }

    async fn run_dependency_update(
        &self,
        task: &ScheduledTask,
        repo_path: &str,
        start: Instant,
    ) -> ScheduledTaskResult {
        let output = tokio::process::Command::new("cargo")
            .args(["outdated", "--root-deps-only"])
            .current_dir(repo_path)
            .output()
            .await;

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                ScheduledTaskResult {
                    task_id: task.id.clone(),
                    success: true,
                    summary: if stdout.trim().is_empty() {
                        "All dependencies up to date".to_string()
                    } else {
                        format!("Outdated dependencies found:\n{}", stdout.chars().take(500).collect::<String>())
                    },
                    duration_secs: start.elapsed().as_secs_f64(),
                    error: None,
                }
            }
            Err(e) => ScheduledTaskResult {
                task_id: task.id.clone(),
                success: false,
                summary: "Failed to check dependencies".to_string(),
                duration_secs: start.elapsed().as_secs_f64(),
                error: Some(e.to_string()),
            },
        }
    }

    fn handle_failure(&mut self, task: &ScheduledTask) {
        let count = self.failure_counts.entry(task.id.clone()).or_insert(0);
        *count += 1;

        match &task.on_failure {
            FailurePolicy::Ignore => {}
            FailurePolicy::Retry { max_retries, .. } => {
                if *count >= *max_retries {
                    tracing::warn!(
                        "Task {} exceeded max retries ({}), disabling",
                        task.id,
                        max_retries
                    );
                    self.disabled_tasks.insert(task.id.clone(), true);
                }
            }
            FailurePolicy::Disable => {
                tracing::warn!("Task {} failed, disabling per policy", task.id);
                self.disabled_tasks.insert(task.id.clone(), true);
            }
            FailurePolicy::Escalate => {
                tracing::error!(
                    "Task {} failed, escalating for human attention",
                    task.id
                );
                self.disabled_tasks.insert(task.id.clone(), true);
            }
        }
    }

    /// Get the number of active (non-disabled) tasks.
    pub fn active_task_count(&self) -> usize {
        self.triggers
            .iter()
            .filter(|t| !self.disabled_tasks.get(t.task_id()).copied().unwrap_or(false))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::task_schedule::ScheduledTaskType;

    #[test]
    fn scheduler_new_with_default_config() {
        let config = SchedulerConfig::default();
        let scheduler = AutonomyScheduler::new(config);
        assert_eq!(scheduler.active_task_count(), 0);
    }

    #[test]
    fn scheduler_add_task() {
        let config = SchedulerConfig::default();
        let mut scheduler = AutonomyScheduler::new(config);
        let task = ScheduledTask::new(
            "test".to_string(),
            "Test".to_string(),
            "0 * * * *".to_string(),
            ScheduledTaskType::CodeQualityCheck {
                repo_path: ".".to_string(),
            },
        );
        scheduler.add_task(task).unwrap();
        assert_eq!(scheduler.active_task_count(), 1);
    }

    #[test]
    fn scheduler_skips_disabled_tasks() {
        let config = SchedulerConfig::default();
        let mut scheduler = AutonomyScheduler::new(config);
        let mut task = ScheduledTask::new(
            "test".to_string(),
            "Test".to_string(),
            "0 * * * *".to_string(),
            ScheduledTaskType::CodeQualityCheck {
                repo_path: ".".to_string(),
            },
        );
        task.enabled = false;
        scheduler.add_task(task).unwrap();
        assert_eq!(scheduler.active_task_count(), 0);
    }
}
