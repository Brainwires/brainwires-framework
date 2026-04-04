//! TaskAgent - Autonomous agent that executes a task in a loop using AI + tools
//!
//! Each `TaskAgent` owns its conversation history and calls the AI provider
//! repeatedly, executing tool requests and running validation before it
//! signals completion.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use brainwires_agents::{AgentContext, TaskAgent, TaskAgentConfig, TaskAgentResult};
//! use brainwires_core::Task;
//!
//! let context = Arc::new(AgentContext::new(
//!     "/my/project",
//!     Arc::new(my_executor),
//!     Arc::clone(&hub),
//!     Arc::clone(&lock_manager),
//! ));
//!
//! let agent = Arc::new(TaskAgent::new(
//!     "agent-1".to_string(),
//!     Task::new("task-1", "Refactor src/lib.rs"),
//!     Arc::clone(&provider),
//!     Arc::clone(&context),
//!     TaskAgentConfig::default(),
//! ));
//!
//! let result: TaskAgentResult = agent.execute().await?;
//! ```

mod agent;
mod spawn;
mod types;

#[cfg(test)]
mod tests;

// ── Public re-exports ────────────────────────────────────────────────────────

pub use agent::TaskAgent;
pub use spawn::spawn_task_agent;
pub use types::{
    FailureCategory, LoopDetectionConfig, TaskAgentConfig, TaskAgentResult, TaskAgentStatus,
};
