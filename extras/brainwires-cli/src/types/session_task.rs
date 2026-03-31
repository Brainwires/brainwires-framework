//! Session Task List - Lightweight, session-specific task tracking
//!
//! This module provides a simple task list for the AI to track multi-step tasks
//! during a conversation. Unlike TaskManager, this is:
//! - Flat (no hierarchy)
//! - In-memory only (no persistence)
//! - Session-specific (cleared when session ends)

use serde::{Deserialize, Serialize};

/// Status for session task items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SessionTaskStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
}

/// A single session task item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTask {
    /// Task description in imperative form (e.g., "Run tests")
    pub content: String,
    /// Current status
    pub status: SessionTaskStatus,
    /// Present continuous form for display (e.g., "Running tests")
    pub active_form: String,
}

impl SessionTask {
    /// Create a new pending task
    pub fn new(content: String, active_form: String) -> Self {
        Self {
            content,
            status: SessionTaskStatus::Pending,
            active_form,
        }
    }

    /// Create a new task with specified status
    pub fn with_status(content: String, status: SessionTaskStatus, active_form: String) -> Self {
        Self {
            content,
            status,
            active_form,
        }
    }
}

/// The session task list (flat, in-memory only)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionTaskList {
    pub tasks: Vec<SessionTask>,
}

impl SessionTaskList {
    /// Create a new empty task list
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Replace the entire list (Claude Code pattern)
    pub fn replace(&mut self, tasks: Vec<SessionTask>) {
        self.tasks = tasks;
    }

    /// Clear all tasks
    pub fn clear(&mut self) {
        self.tasks.clear();
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get the number of tasks
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Get completed task count
    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == SessionTaskStatus::Completed)
            .count()
    }

    /// Get the current in-progress task (if any)
    pub fn current_task(&self) -> Option<&SessionTask> {
        self.tasks
            .iter()
            .find(|t| t.status == SessionTaskStatus::InProgress)
    }

    /// Get summary for status bar display
    ///
    /// Returns format like `[Tasks 2/5: Running tests]` or `[Tasks 2/5]`
    pub fn summary(&self) -> String {
        if self.tasks.is_empty() {
            return String::new();
        }

        let total = self.tasks.len();
        let completed = self.completed_count();

        if let Some(current) = self.current_task() {
            format!("[Tasks {}/{}: {}]", completed, total, current.active_form)
        } else {
            format!("[Tasks {}/{}]", completed, total)
        }
    }

    /// Format for AI response (tool result)
    pub fn format_for_ai(&self) -> String {
        if self.tasks.is_empty() {
            return "No tasks".to_string();
        }

        let mut output = String::new();
        for (i, task) in self.tasks.iter().enumerate() {
            let status_icon = match task.status {
                SessionTaskStatus::Pending => "[ ]",
                SessionTaskStatus::InProgress => "[*]",
                SessionTaskStatus::Completed => "[x]",
            };
            output.push_str(&format!("{} {}. {}\n", status_icon, i + 1, task.content));
        }
        output
    }

    /// Format for sidebar panel display
    ///
    /// Returns a vector of (icon, text, status) tuples for rendering
    pub fn format_for_panel(&self) -> Vec<(String, String, SessionTaskStatus)> {
        self.tasks
            .iter()
            .map(|task| {
                let text = if task.status == SessionTaskStatus::InProgress {
                    // Show active_form for in-progress tasks
                    task.active_form.clone()
                } else {
                    task.content.clone()
                };
                let icon = match task.status {
                    SessionTaskStatus::Pending => " ".to_string(),
                    SessionTaskStatus::InProgress => "▸".to_string(),
                    SessionTaskStatus::Completed => "✓".to_string(),
                };
                (icon, text, task.status)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_task_list() {
        let list = SessionTaskList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_replace_tasks() {
        let mut list = SessionTaskList::new();
        let tasks = vec![
            SessionTask::with_status(
                "Task 1".to_string(),
                SessionTaskStatus::Completed,
                "Completing task 1".to_string(),
            ),
            SessionTask::with_status(
                "Task 2".to_string(),
                SessionTaskStatus::InProgress,
                "Working on task 2".to_string(),
            ),
            SessionTask::new("Task 3".to_string(), "Starting task 3".to_string()),
        ];

        list.replace(tasks);

        assert_eq!(list.len(), 3);
        assert_eq!(list.completed_count(), 1);
        assert!(list.current_task().is_some());
        assert_eq!(list.current_task().unwrap().content, "Task 2");
    }

    #[test]
    fn test_summary_empty() {
        let list = SessionTaskList::new();
        assert_eq!(list.summary(), "");
    }

    #[test]
    fn test_summary_with_in_progress() {
        let mut list = SessionTaskList::new();
        list.replace(vec![
            SessionTask::with_status(
                "Task 1".to_string(),
                SessionTaskStatus::Completed,
                "Task 1".to_string(),
            ),
            SessionTask::with_status(
                "Task 2".to_string(),
                SessionTaskStatus::InProgress,
                "Running tests".to_string(),
            ),
            SessionTask::new("Task 3".to_string(), "Task 3".to_string()),
        ]);

        assert_eq!(list.summary(), "[Tasks 1/3: Running tests]");
    }

    #[test]
    fn test_summary_no_in_progress() {
        let mut list = SessionTaskList::new();
        list.replace(vec![
            SessionTask::with_status(
                "Task 1".to_string(),
                SessionTaskStatus::Completed,
                "Task 1".to_string(),
            ),
            SessionTask::new("Task 2".to_string(), "Task 2".to_string()),
        ]);

        assert_eq!(list.summary(), "[Tasks 1/2]");
    }

    #[test]
    fn test_format_for_ai() {
        let mut list = SessionTaskList::new();
        list.replace(vec![
            SessionTask::with_status(
                "Read file".to_string(),
                SessionTaskStatus::Completed,
                "Reading file".to_string(),
            ),
            SessionTask::with_status(
                "Run tests".to_string(),
                SessionTaskStatus::InProgress,
                "Running tests".to_string(),
            ),
            SessionTask::new("Fix bugs".to_string(), "Fixing bugs".to_string()),
        ]);

        let output = list.format_for_ai();
        assert!(output.contains("[x] 1. Read file"));
        assert!(output.contains("[*] 2. Run tests"));
        assert!(output.contains("[ ] 3. Fix bugs"));
    }

    #[test]
    fn test_format_for_panel() {
        let mut list = SessionTaskList::new();
        list.replace(vec![
            SessionTask::with_status(
                "Read file".to_string(),
                SessionTaskStatus::Completed,
                "Reading file".to_string(),
            ),
            SessionTask::with_status(
                "Run tests".to_string(),
                SessionTaskStatus::InProgress,
                "Running tests".to_string(),
            ),
        ]);

        let panel = list.format_for_panel();
        assert_eq!(panel.len(), 2);
        assert_eq!(panel[0].0, "✓");
        assert_eq!(panel[0].1, "Read file"); // Completed shows content
        assert_eq!(panel[1].0, "▸");
        assert_eq!(panel[1].1, "Running tests"); // In progress shows active_form
    }
}
