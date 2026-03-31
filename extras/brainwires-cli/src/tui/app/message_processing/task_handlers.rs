//! Task Handlers
//!
//! Handles task management command operations.

use super::super::state::{App, TuiMessage};
use crate::agents::format_duration_secs;
use crate::types::agent::{TaskPriority, TaskStatus};

impl App {
    /// Handle show tasks command - displays current task list
    pub(super) async fn handle_show_tasks(&mut self) {
        let content = if self.active_plan.is_some() {
            let tree = {
                let task_mgr = self.task_manager.read().await;
                task_mgr.format_tree().await
            };

            let stats = {
                let task_mgr = self.task_manager.read().await;
                task_mgr.get_stats().await
            };

            format!(
                "Task List ({}/{} completed):\n\n{}",
                stats.completed, stats.total, tree
            )
        } else {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:complete command
    pub(super) async fn handle_task_complete(&mut self, task_id: Option<String>) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.write().await;

            // Get task ID - either provided or find current in-progress task
            let target_id = if let Some(id) = task_id {
                Some(id)
            } else {
                // Find current in-progress task
                let tasks = task_mgr.get_tasks_by_status(TaskStatus::InProgress).await;
                tasks.first().map(|t| t.id.clone())
            };

            match target_id {
                Some(id) => {
                    match task_mgr
                        .complete_task(&id, "Manually completed".to_string())
                        .await
                    {
                        Ok(()) => format!("Task '{}' marked as complete.", id),
                        Err(e) => format!("Failed to complete task: {}", e),
                    }
                }
                None => "No task ID provided and no in-progress task found.".to_string(),
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:skip command
    pub(super) async fn handle_task_skip(
        &mut self,
        task_id: Option<String>,
        reason: Option<String>,
    ) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.write().await;

            // Get task ID - either provided or find current in-progress task
            let target_id = if let Some(id) = task_id {
                Some(id)
            } else {
                let tasks = task_mgr.get_tasks_by_status(TaskStatus::InProgress).await;
                tasks.first().map(|t| t.id.clone())
            };

            match target_id {
                Some(id) => match task_mgr.skip_task(&id, reason.clone()).await {
                    Ok(()) => {
                        let reason_text = reason.map(|r| format!(" ({})", r)).unwrap_or_default();
                        format!("Task '{}' skipped{}.", id, reason_text)
                    }
                    Err(e) => format!("Failed to skip task: {}", e),
                },
                None => "No task ID provided and no in-progress task found.".to_string(),
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:add command
    pub(super) async fn handle_task_add(&mut self, description: String) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.write().await;
            match task_mgr
                .create_task(description.clone(), None, TaskPriority::Normal)
                .await
            {
                Ok(id) => format!("Task added: {} (ID: {})", description, &id[..8]),
                Err(e) => format!("Failed to add task: {}", e),
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:start command
    pub(super) async fn handle_task_start(&mut self, task_id: String) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.write().await;

            // Check if task can start (dependencies complete)
            match task_mgr.can_start(&task_id).await {
                Ok(true) => match task_mgr.start_task(&task_id).await {
                    Ok(()) => format!("Started task '{}'.", task_id),
                    Err(e) => format!("Failed to start task: {}", e),
                },
                Ok(false) => format!(
                    "Task '{}' cannot be started (already completed or failed).",
                    task_id
                ),
                Err(blocking) => format!(
                    "Cannot start task '{}' - blocked by: {}",
                    task_id,
                    blocking.join(", ")
                ),
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:block command
    pub(super) async fn handle_task_block(&mut self, task_id: String, reason: Option<String>) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.write().await;
            match task_mgr.block_task(&task_id, reason.clone()).await {
                Ok(()) => {
                    let reason_text = reason.map(|r| format!(" ({})", r)).unwrap_or_default();
                    format!("Task '{}' blocked{}.", task_id, reason_text)
                }
                Err(e) => format!("Failed to block task: {}", e),
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:depends command
    pub(super) async fn handle_task_depends(&mut self, task_id: String, depends_on: String) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.write().await;
            match task_mgr.add_dependency(&task_id, &depends_on).await {
                Ok(()) => format!("Task '{}' now depends on '{}'.", task_id, depends_on),
                Err(e) => format!("Failed to add dependency: {}", e),
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:ready command
    pub(super) async fn handle_task_ready(&mut self) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.read().await;
            let ready_tasks = task_mgr.get_ready_tasks().await;

            if ready_tasks.is_empty() {
                "No tasks ready to execute.".to_string()
            } else {
                let task_list: Vec<String> = ready_tasks
                    .iter()
                    .map(|t| {
                        format!(
                            "  {} [{}] {}",
                            &t.id[..8],
                            format!("{:?}", t.priority).to_lowercase(),
                            t.description
                        )
                    })
                    .collect();
                format!(
                    "Tasks ready to execute ({}):\n{}",
                    ready_tasks.len(),
                    task_list.join("\n")
                )
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:time command
    pub(super) async fn handle_task_time(&mut self, task_id: Option<String>) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.read().await;

            if let Some(id) = task_id {
                // Show time for specific task
                match task_mgr.get_task_time_info(&id).await {
                    Some(info) => {
                        format!(
                            "Time for task '{}':\n  Status: {:?}\n  Duration: {}",
                            info.description,
                            info.status,
                            info.format_duration()
                        )
                    }
                    None => format!("Task '{}' not found.", id),
                }
            } else {
                // Show time stats for all tasks
                let time_stats = task_mgr.get_time_stats().await;
                let estimate = task_mgr.estimate_remaining_time().await;

                let mut output = format!(
                    "Time Statistics:\n\
                     Total time: {}\n\
                     Completed tasks: {}\n\
                     Average per task: {}",
                    time_stats.format_total(),
                    time_stats.completed_tasks,
                    time_stats.format_average()
                );

                if time_stats.in_progress_tasks > 0 {
                    output.push_str(&format!(
                        "\nCurrent elapsed: {} ({} in progress)",
                        time_stats.format_elapsed(),
                        time_stats.in_progress_tasks
                    ));
                }

                if let Some(est) = estimate {
                    output.push_str(&format!(
                        "\nEstimated remaining: {}",
                        format_duration_secs(est)
                    ));
                }

                output
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle task:list command - enhanced list with IDs and dependencies
    pub(super) async fn handle_task_list(&mut self) {
        let content = if self.active_plan.is_none() {
            "No active plan. Use /plan:activate <id> to set a plan first.".to_string()
        } else {
            let task_mgr = self.task_manager.read().await;
            let tasks = task_mgr.get_all_tasks().await;

            if tasks.is_empty() {
                "No tasks in the current plan.".to_string()
            } else {
                let status_icon = |s: &TaskStatus| match s {
                    TaskStatus::Pending => "○",
                    TaskStatus::InProgress => "◐",
                    TaskStatus::Completed => "●",
                    TaskStatus::Failed => "✗",
                    TaskStatus::Blocked => "◌",
                    TaskStatus::Skipped => "⊘",
                };

                let mut output = String::from("Tasks:\n");
                for task in &tasks {
                    let deps = if task.depends_on.is_empty() {
                        String::new()
                    } else {
                        let short_deps: Vec<&str> = task
                            .depends_on
                            .iter()
                            .map(|d| &d[..8.min(d.len())])
                            .collect();
                        format!(" [deps: {}]", short_deps.join(", "))
                    };

                    let time_info = if let Some(info) = task_mgr.get_task_time_info(&task.id).await
                    {
                        let duration = info.format_duration();
                        if duration != "-" {
                            format!(" [{}]", duration)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    output.push_str(&format!(
                        "  {} {} {}{}{}\n",
                        status_icon(&task.status),
                        &task.id[..8],
                        task.description,
                        deps,
                        time_info
                    ));
                }

                let stats = task_mgr.get_stats().await;
                output.push_str(&format!(
                    "\nSummary: {} total, {} pending, {} in progress, {} completed, {} skipped, {} blocked",
                    stats.total, stats.pending, stats.in_progress, stats.completed, stats.skipped, stats.blocked
                ));

                output
            }
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }
}
