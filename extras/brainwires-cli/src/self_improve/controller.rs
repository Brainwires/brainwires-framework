use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};

use brainwires::agent_network::client::{AgentConfig, AgentNetworkClient};

use super::comparator::{Comparator, ComparisonResult, PathResult};
use super::config::SelfImprovementConfig;
use super::metrics::{SessionMetrics, SessionReport};
use super::safety::{SafetyGuard, SafetyStop};
use super::strategies::ImprovementTask;
use super::task_generator::TaskGenerator;

pub struct CycleResult {
    pub task: ImprovementTask,
    pub direct_result: Option<PathResult>,
    pub bridge_result: Option<PathResult>,
    pub comparison: Option<ComparisonResult>,
    pub committed: bool,
    pub commit_hash: Option<String>,
}

struct WorktreeInfo {
    path: String,
    _branch: String,
}

pub struct SelfImprovementController {
    config: SelfImprovementConfig,
    task_generator: TaskGenerator,
    metrics: SessionMetrics,
    safety: SafetyGuard,
}

impl SelfImprovementController {
    pub fn new(config: SelfImprovementConfig) -> Self {
        let task_generator = TaskGenerator::from_config(&config);
        let safety = SafetyGuard::new(&config);
        Self {
            config,
            task_generator,
            metrics: SessionMetrics::new(),
            safety,
        }
    }

    /// Create a controller with a custom set of strategies instead of the
    /// default strategy registry.  Useful for injecting [`EvalStrategy`] or
    /// other custom strategies directly.
    pub fn new_with_strategies(
        config: SelfImprovementConfig,
        strategies: Vec<Box<dyn super::strategies::ImprovementStrategy>>,
    ) -> Self {
        let task_generator = TaskGenerator::new(strategies);
        let safety = SafetyGuard::new(&config);
        Self {
            config,
            task_generator,
            metrics: SessionMetrics::new(),
            safety,
        }
    }

    pub async fn run(&mut self) -> Result<SessionReport> {
        let start = Instant::now();
        tracing::info!("Starting self-improvement loop");
        tracing::info!("Strategies: {:?}", self.task_generator.strategy_names());

        // Generate tasks
        let tasks = self.task_generator.generate_all(&self.config).await?;
        tracing::info!("Generated {} improvement tasks", tasks.len());

        // Record generation counts
        for task in &tasks {
            self.metrics.record_generated(&task.strategy, 1);
        }

        if self.config.dry_run {
            self.print_dry_run(&tasks);
            return Ok(SessionReport::new(
                self.metrics.clone(),
                start.elapsed(),
                None,
            ));
        }

        let mut stop_reason: Option<SafetyStop> = None;

        for task in tasks {
            // Check safety guards
            if let Err(reason) = self.safety.check_can_continue() {
                tracing::warn!("Safety stop: {reason}");
                stop_reason = Some(reason);
                break;
            }

            tracing::info!(
                "Cycle {}/{}: {} (strategy: {})",
                self.safety.cycles_completed() + 1,
                self.config.max_cycles,
                task.description.chars().take(80).collect::<String>(),
                task.strategy,
            );

            match self.run_cycle(&task).await {
                Ok(result) => {
                    self.metrics.record_attempt(&task.strategy);

                    let success = result.direct_result.as_ref().is_some_and(|r| r.success)
                        || result.bridge_result.as_ref().is_some_and(|r| r.success);

                    if success {
                        let iterations = result
                            .direct_result
                            .as_ref()
                            .map(|r| r.iterations)
                            .unwrap_or(0)
                            .max(
                                result
                                    .bridge_result
                                    .as_ref()
                                    .map(|r| r.iterations)
                                    .unwrap_or(0),
                            );

                        let diff_lines = result
                            .direct_result
                            .as_ref()
                            .map(|r| r.diff_lines)
                            .unwrap_or(0)
                            .max(
                                result
                                    .bridge_result
                                    .as_ref()
                                    .map(|r| r.diff_lines)
                                    .unwrap_or(0),
                            );

                        self.safety.record_success(diff_lines);
                        self.metrics.record_success(&task.strategy, iterations);

                        if let Some(hash) = result.commit_hash {
                            self.metrics.record_commit(hash);
                        }
                    } else {
                        self.safety.record_failure();
                        self.metrics.record_failure(&task.strategy);
                    }

                    if let Some(comparison) = result.comparison {
                        self.metrics.record_comparison(comparison);
                    }
                }
                Err(e) => {
                    tracing::error!("Cycle failed: {e}");
                    self.safety.record_failure();
                    self.metrics.record_failure(&task.strategy);
                }
            }
        }

        let report = SessionReport::new(self.metrics.clone(), start.elapsed(), stop_reason);

        // Save report
        if let Err(e) = report.save("test-results/self-improve") {
            tracing::warn!("Failed to save report: {e}");
        }

        Ok(report)
    }

    async fn run_cycle(&mut self, task: &ImprovementTask) -> Result<CycleResult> {
        let repo_path = std::env::current_dir()?.to_string_lossy().to_string();

        let mut direct_result = None;
        let mut bridge_result = None;

        // Execute direct path
        if !self.config.no_direct {
            tracing::info!("Running direct path...");
            let worktree = self.setup_worktree(task, "direct")?;
            match self.execute_direct_path(task, &worktree.path).await {
                Ok(result) => {
                    if result.success {
                        if self.validate_changes(&worktree.path).await.is_ok() {
                            direct_result = Some(result);
                        } else {
                            direct_result = Some(PathResult::failure(
                                "Validation failed".to_string(),
                                result.duration,
                            ));
                        }
                    } else {
                        direct_result = Some(result);
                    }
                }
                Err(e) => {
                    direct_result =
                        Some(PathResult::failure(e.to_string(), Duration::from_secs(0)));
                }
            }
            self.cleanup_worktree(&worktree)?;
        }

        // Execute bridge path
        if !self.config.no_bridge {
            tracing::info!("Running bridge path...");
            let worktree = self.setup_worktree(task, "bridge")?;
            match self.execute_bridge_path(task, &worktree.path).await {
                Ok(result) => {
                    if result.success {
                        if self.validate_changes(&worktree.path).await.is_ok() {
                            bridge_result = Some(result);
                        } else {
                            bridge_result = Some(PathResult::failure(
                                "Validation failed".to_string(),
                                result.duration,
                            ));
                        }
                    } else {
                        bridge_result = Some(result);
                    }
                }
                Err(e) => {
                    bridge_result =
                        Some(PathResult::failure(e.to_string(), Duration::from_secs(0)));
                }
            }
            self.cleanup_worktree(&worktree)?;
        }

        // Compare results if both paths ran
        let comparison = match (&direct_result, &bridge_result) {
            (Some(d), Some(b)) => Some(Comparator::compare(d, b)),
            _ => None,
        };

        // Commit changes from the successful path
        let mut committed = false;
        let mut commit_hash = None;

        let winning_result = direct_result
            .as_ref()
            .filter(|r| r.success)
            .or(bridge_result.as_ref().filter(|r| r.success));

        if let Some(result) = winning_result {
            if result.diff_lines <= self.config.max_diff_per_task {
                match self.commit_changes(&repo_path, task).await {
                    Ok(hash) => {
                        committed = true;
                        commit_hash = Some(hash);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to commit: {e}");
                    }
                }
            } else {
                tracing::warn!(
                    "Skipping commit: diff too large ({} lines > {} max)",
                    result.diff_lines,
                    self.config.max_diff_per_task
                );
            }
        }

        Ok(CycleResult {
            task: task.clone(),
            direct_result,
            bridge_result,
            comparison,
            committed,
            commit_hash,
        })
    }

    async fn execute_direct_path(
        &self,
        task: &ImprovementTask,
        worktree_path: &str,
    ) -> Result<PathResult> {
        let start = Instant::now();

        // Use ProviderFactory to create a provider
        let model = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let factory = crate::providers::ProviderFactory::new();
        let provider = factory.create(model).await?;

        let agent_task = crate::types::agent::Task::new(
            task.id.clone(),
            format!(
                "{}\n\nContext:\n{}\n\nTarget files: {}",
                task.description,
                task.context,
                task.target_files.join(", ")
            ),
        );

        let hub = Arc::new(crate::agents::CommunicationHub::new());
        let locks = Arc::new(crate::agents::FileLockManager::new());

        let context = crate::types::agent::AgentContext {
            working_directory: worktree_path.to_string(),
            ..Default::default()
        };

        let config = crate::agents::TaskAgentConfig {
            max_iterations: self.config.agent_iterations,
            ..Default::default()
        };

        let agent = crate::agents::TaskAgent::new(
            format!("self-improve-{}", task.id),
            agent_task,
            provider,
            hub,
            locks,
            context,
            config,
        );

        let result = agent.execute().await;
        let elapsed = start.elapsed();

        match result {
            Ok(agent_result) => {
                let diff = get_git_diff(worktree_path).await.unwrap_or_default();
                let diff_lines = diff.lines().count() as u32;

                Ok(PathResult {
                    success: agent_result.success,
                    iterations: agent_result.iterations,
                    diff,
                    diff_lines,
                    duration: elapsed,
                    error: if agent_result.success {
                        None
                    } else {
                        Some(agent_result.summary)
                    },
                })
            }
            Err(e) => Ok(PathResult::failure(e.to_string(), elapsed)),
        }
    }

    async fn execute_bridge_path(
        &self,
        task: &ImprovementTask,
        worktree_path: &str,
    ) -> Result<PathResult> {
        let start = Instant::now();

        let mut client = match AgentNetworkClient::connect("brainwires").await {
            Ok(c) => c,
            Err(e) => {
                return Ok(PathResult::failure(
                    format!("Bridge connect failed: {e}"),
                    start.elapsed(),
                ));
            }
        };

        if let Err(e) = client.initialize().await {
            return Ok(PathResult::failure(
                format!("Bridge initialize failed: {e}"),
                start.elapsed(),
            ));
        }

        let agent_config = AgentConfig {
            max_iterations: Some(self.config.agent_iterations),
            enable_validation: Some(true),
            build_type: Some("cargo".to_string()),
            enable_mdap: None,
            mdap_preset: None,
        };

        let full_description = format!(
            "{}\n\nContext:\n{}\n\nTarget files: {}",
            task.description,
            task.context,
            task.target_files.join(", ")
        );

        let agent_id = match client
            .spawn_agent(&full_description, worktree_path, agent_config)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                let _ = client.shutdown().await;
                return Ok(PathResult::failure(
                    format!("Bridge spawn failed: {e}"),
                    start.elapsed(),
                ));
            }
        };

        let result = match client.await_agent(&agent_id, Some(300)).await {
            Ok(r) => r,
            Err(e) => {
                let _ = client.shutdown().await;
                return Ok(PathResult::failure(
                    format!("Bridge await failed: {e}"),
                    start.elapsed(),
                ));
            }
        };

        let _ = client.shutdown().await;

        let diff = get_git_diff(worktree_path).await.unwrap_or_default();
        let diff_lines = diff.lines().count() as u32;

        Ok(PathResult {
            success: result.success,
            iterations: result.iterations,
            diff,
            diff_lines,
            duration: start.elapsed(),
            error: if result.success {
                None
            } else {
                Some(result.summary)
            },
        })
    }

    async fn validate_changes(&self, worktree_path: &str) -> Result<()> {
        let check = tokio::process::Command::new("cargo")
            .args(["check"])
            .current_dir(worktree_path)
            .output()
            .await?;

        if !check.status.success() {
            let stderr = String::from_utf8_lossy(&check.stderr);
            anyhow::bail!("cargo check failed: {stderr}");
        }

        let test = tokio::process::Command::new("cargo")
            .args(["test", "--", "--no-capture"])
            .current_dir(worktree_path)
            .output()
            .await?;

        if !test.status.success() {
            let stderr = String::from_utf8_lossy(&test.stderr);
            anyhow::bail!("cargo test failed: {stderr}");
        }

        Ok(())
    }

    async fn commit_changes(&self, worktree_path: &str, task: &ImprovementTask) -> Result<String> {
        let add = tokio::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(worktree_path)
            .output()
            .await?;

        if !add.status.success() {
            anyhow::bail!("git add failed");
        }

        let status = tokio::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(worktree_path)
            .output()
            .await?;

        let status_output = String::from_utf8_lossy(&status.stdout);
        if status_output.trim().is_empty() {
            anyhow::bail!("No changes to commit");
        }

        let message = format!(
            "self-improve({}): {}\n\nStrategy: {}\nCategory: {}\nTarget files: {}",
            task.strategy,
            task.description.chars().take(72).collect::<String>(),
            task.strategy,
            task.category,
            task.target_files.join(", ")
        );

        let commit = tokio::process::Command::new("git")
            .args(["commit", "-m", &message])
            .current_dir(worktree_path)
            .output()
            .await?;

        if !commit.status.success() {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            anyhow::bail!("git commit failed: {stderr}");
        }

        let hash = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(worktree_path)
            .output()
            .await?;

        let hash = String::from_utf8_lossy(&hash.stdout).trim().to_string();
        Ok(hash)
    }

    fn setup_worktree(&self, task: &ImprovementTask, suffix: &str) -> Result<WorktreeInfo> {
        let branch = format!(
            "{}{}_{}_{}",
            self.config.branch_prefix, task.strategy, task.id, suffix
        );

        // For now, use the current directory
        // In production, this would use `git worktree add`
        Ok(WorktreeInfo {
            path: std::env::current_dir()?.to_string_lossy().to_string(),
            _branch: branch,
        })
    }

    fn cleanup_worktree(&self, _info: &WorktreeInfo) -> Result<()> {
        Ok(())
    }

    fn print_dry_run(&self, tasks: &[ImprovementTask]) {
        println!("\n=== Self-Improvement Dry Run ===\n");
        println!("Found {} tasks:\n", tasks.len());
        for (i, task) in tasks.iter().enumerate() {
            println!(
                "  {}. [{}] [P{}] {}",
                i + 1,
                task.strategy,
                task.priority,
                task.description.chars().take(100).collect::<String>()
            );
            if !task.target_files.is_empty() {
                println!("     Files: {}", task.target_files.join(", "));
            }
            println!("     Est. diff: ~{} lines", task.estimated_diff_lines);
            println!();
        }
        println!("Config:");
        println!("  Max cycles: {}", self.config.max_cycles);
        println!("  Max budget: ${:.2}", self.config.max_budget);
        println!("  Agent iterations: {}", self.config.agent_iterations);
        println!(
            "  Max diff per task: {} lines",
            self.config.max_diff_per_task
        );
        println!(
            "  Bridge path: {}",
            if self.config.no_bridge {
                "disabled"
            } else {
                "enabled"
            }
        );
        println!(
            "  Direct path: {}",
            if self.config.no_direct {
                "disabled"
            } else {
                "enabled"
            }
        );
    }
}

async fn get_git_diff(path: &str) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(path)
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
