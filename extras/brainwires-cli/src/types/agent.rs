use super::message::Message;
use super::tool::Tool;
use super::working_set::WorkingSet;
use brainwires::permissions::AgentCapabilities;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export Task types from framework (structurally identical)
pub use brainwires::core::task::{AgentResponse, Task, TaskPriority, TaskStatus};

/// Context for agent execution
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// Current working directory
    pub working_directory: String,
    /// Conversation history
    pub conversation_history: Vec<Message>,
    /// Available tools
    pub tools: Vec<Tool>,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Working set of files currently in context
    pub working_set: WorkingSet,
    /// Agent's granted capabilities (permission system)
    pub capabilities: AgentCapabilities,
}

impl Default for AgentContext {
    fn default() -> Self {
        Self {
            working_directory: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| ".".to_string()),
            conversation_history: Vec::new(),
            tools: Vec::new(),
            user_id: None,
            metadata: HashMap::new(),
            working_set: WorkingSet::new(),
            capabilities: AgentCapabilities::standard_dev(),
        }
    }
}

/// Permission mode for tool execution
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum PermissionMode {
    /// Read-only mode - deny all write operations
    ReadOnly,
    /// Auto mode - approve safe operations, ask for dangerous ones
    #[default]
    Auto,
    /// Full mode - auto-approve all operations
    Full,
}

impl PermissionMode {
    /// Parse from string
    pub fn parse_mode(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "read-only" | "readonly" => Some(Self::ReadOnly),
            "auto" => Some(Self::Auto),
            "full" => Some(Self::Full),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Auto => "auto",
            Self::Full => "full",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_context_default() {
        let context = AgentContext::default();
        assert!(!context.working_directory.is_empty());
        assert!(context.conversation_history.is_empty());
        assert!(context.tools.is_empty());
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("task-1", "Test task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.iterations, 0);

        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);

        task.increment_iteration();
        assert_eq!(task.iterations, 1);

        task.complete("Done!");
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.summary, Some("Done!".to_string()));
    }

    #[test]
    fn test_task_failure() {
        let mut task = Task::new("task-2", "Failing task");
        task.start();
        task.fail("Error occurred");
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.summary.unwrap().contains("Error occurred"));
    }

    #[test]
    fn test_permission_mode_from_str() {
        assert_eq!(
            PermissionMode::parse_mode("read-only"),
            Some(PermissionMode::ReadOnly)
        );
        assert_eq!(
            PermissionMode::parse_mode("readonly"),
            Some(PermissionMode::ReadOnly)
        );
        assert_eq!(
            PermissionMode::parse_mode("auto"),
            Some(PermissionMode::Auto)
        );
        assert_eq!(
            PermissionMode::parse_mode("full"),
            Some(PermissionMode::Full)
        );
        assert_eq!(PermissionMode::parse_mode("invalid"), None);
    }

    #[test]
    fn test_permission_mode_as_str() {
        assert_eq!(PermissionMode::ReadOnly.as_str(), "read-only");
        assert_eq!(PermissionMode::Auto.as_str(), "auto");
        assert_eq!(PermissionMode::Full.as_str(), "full");
    }

    #[test]
    fn test_permission_mode_default() {
        let mode = PermissionMode::default();
        assert_eq!(mode, PermissionMode::Auto);
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new("task-3", "Serializable task");
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("task-3"));
        assert!(json.contains("Serializable task"));

        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "task-3");
    }

    #[test]
    fn test_agent_response() {
        let task = Task::new("task-4", "Response task");
        let response = AgentResponse {
            message: "Completed".to_string(),
            is_complete: true,
            tasks: vec![task],
            iterations: 5,
        };

        assert_eq!(response.message, "Completed");
        assert!(response.is_complete);
        assert_eq!(response.iterations, 5);
        assert_eq!(response.tasks.len(), 1);
    }
}
