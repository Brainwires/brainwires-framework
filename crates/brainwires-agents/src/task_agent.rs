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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use brainwires_core::{
    estimate_tokens_from_size, ChatOptions, ChatResponse, ContentBlock, Message, MessageContent,
    Provider, Role, Task, ToolContext, ToolResult, ToolUse,
};

use crate::communication::AgentMessage;
use crate::context::AgentContext;
use crate::file_locks::LockType;
use crate::validation_loop::{format_validation_feedback, run_validation, ValidationConfig};

/// Runtime status of a task agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAgentStatus {
    /// Agent is idle, waiting to be started.
    Idle,
    /// Agent is actively working on something.
    Working(String),
    /// Agent is blocked waiting for a file lock.
    WaitingForLock(String),
    /// Agent execution is paused.
    Paused(String),
    /// Agent completed the task successfully.
    Completed(String),
    /// Agent failed to complete the task.
    Failed(String),
}

impl std::fmt::Display for TaskAgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskAgentStatus::Idle => write!(f, "Idle"),
            TaskAgentStatus::Working(desc) => write!(f, "Working: {}", desc),
            TaskAgentStatus::WaitingForLock(path) => write!(f, "Waiting for lock: {}", path),
            TaskAgentStatus::Paused(reason) => write!(f, "Paused: {}", reason),
            TaskAgentStatus::Completed(summary) => write!(f, "Completed: {}", summary),
            TaskAgentStatus::Failed(error) => write!(f, "Failed: {}", error),
        }
    }
}

/// Result of a completed task agent execution.
#[derive(Debug, Clone)]
pub struct TaskAgentResult {
    /// The agent's unique ID.
    pub agent_id: String,
    /// The task ID that was executed.
    pub task_id: String,
    /// Whether the task completed successfully.
    pub success: bool,
    /// Completion summary or error description.
    pub summary: String,
    /// Number of provider call iterations used.
    pub iterations: u32,
}

/// Configuration for a task agent.
#[derive(Debug, Clone)]
pub struct TaskAgentConfig {
    /// Maximum provider call iterations before the agent is forced to fail.
    ///
    /// Default: 100 (high default to avoid artificial limits on complex tasks).
    pub max_iterations: u32,

    /// Override the system prompt.
    ///
    /// When `None`, [`crate::system_prompts::reasoning_agent_prompt`] is used.
    pub system_prompt: Option<String>,

    /// Temperature for AI calls (0.0 – 1.0).
    pub temperature: f32,

    /// Maximum tokens for a single AI response.
    pub max_tokens: u32,

    /// Quality checks to run before accepting completion.
    ///
    /// Set to `None` to disable validation entirely (useful in tests).
    pub validation_config: Option<ValidationConfig>,
}

impl Default for TaskAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            system_prompt: None,
            temperature: 0.7,
            max_tokens: 4096,
            validation_config: Some(ValidationConfig::default()),
        }
    }
}

/// Autonomous task agent that runs a provider + tool loop until completion.
///
/// Create with [`TaskAgent::new`], then call [`TaskAgent::execute`] (or spawn
/// it on a background task with [`spawn_task_agent`]).
pub struct TaskAgent {
    /// Unique agent ID.
    pub id: String,
    /// Task being executed (mutated as iterations progress).
    task: Arc<RwLock<Task>>,
    /// AI provider for chat completions.
    provider: Arc<dyn Provider>,
    /// Shared environment context.
    context: Arc<AgentContext>,
    /// Agent configuration.
    config: TaskAgentConfig,
    /// Current status (observable from outside the agent).
    status: Arc<RwLock<TaskAgentStatus>>,
    /// Conversation history (internal — grows each iteration).
    conversation_history: Arc<RwLock<Vec<Message>>>,
}

impl TaskAgent {
    /// Create a new task agent.
    ///
    /// The agent starts in [`TaskAgentStatus::Idle`] and does not begin
    /// execution until [`execute`][Self::execute] is called.
    pub fn new(
        id: String,
        task: Task,
        provider: Arc<dyn Provider>,
        context: Arc<AgentContext>,
        config: TaskAgentConfig,
    ) -> Self {
        Self {
            id,
            task: Arc::new(RwLock::new(task)),
            provider,
            context,
            config,
            status: Arc::new(RwLock::new(TaskAgentStatus::Idle)),
            conversation_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the agent's unique ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the current status.
    pub async fn status(&self) -> TaskAgentStatus {
        self.status.read().await.clone()
    }

    /// Get a snapshot of the task.
    pub async fn task(&self) -> Task {
        self.task.read().await.clone()
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    async fn set_status(&self, status: TaskAgentStatus) {
        *self.status.write().await = status.clone();
        let _ = self
            .context
            .communication_hub
            .broadcast(
                self.id.clone(),
                AgentMessage::StatusUpdate {
                    agent_id: self.id.clone(),
                    status: status.to_string(),
                    details: None,
                },
            )
            .await;
    }

    /// Returns `true` for tool names that operate on a specific file.
    fn is_file_operation(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "read_file" | "write_file" | "edit_file" | "append_to_file" | "delete_file"
        )
    }

    /// Extract the file path from a tool use's input, if present.
    fn extract_file_path(tool_use: &ToolUse) -> Option<PathBuf> {
        let path_str = tool_use
            .input
            .get("file_path")
            .or_else(|| tool_use.input.get("path"))
            .and_then(|v| v.as_str())?;
        Some(PathBuf::from(path_str))
    }

    /// Determine whether a tool requires a file lock, and what kind.
    fn get_lock_requirement(tool_use: &ToolUse) -> Option<(String, LockType)> {
        let path = tool_use
            .input
            .get("path")
            .or_else(|| tool_use.input.get("file_path"))
            .and_then(|v| v.as_str())?;

        let lock_type = match tool_use.name.as_str() {
            "read_file" | "list_directory" | "search_code" => LockType::Read,
            "write_file" | "edit_file" | "patch_file" | "delete_file" | "create_directory" => {
                LockType::Write
            }
            _ => return None,
        };
        Some((path.to_string(), lock_type))
    }

    /// Extract all `ToolUse` blocks from a provider message.
    fn extract_tool_uses(message: &Message) -> Vec<ToolUse> {
        match &message.content {
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => Some(ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    }),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        }
    }

    /// Build the `Message` that wraps a tool result in the conversation.
    fn tool_result_message(result: &ToolResult) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: result.tool_use_id.clone(),
                content: result.content.clone(),
                is_error: Some(result.is_error),
            }]),
            name: None,
            metadata: None,
        }
    }

    /// Call the AI provider with the current conversation state.
    async fn call_provider(&self) -> Result<ChatResponse> {
        let history = self.conversation_history.read().await.clone();
        let tools = self.context.tool_executor.available_tools();

        let system_prompt = self.config.system_prompt.clone().unwrap_or_else(|| {
            crate::system_prompts::reasoning_agent_prompt(
                &self.id,
                &self.context.working_directory,
            )
        });

        let options = ChatOptions {
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            top_p: None,
            stop: None,
            system: Some(system_prompt),
        };

        self.provider
            .chat(&history, Some(&tools), &options)
            .await
    }

    /// Run validation checks and, if they pass, finalise the task.
    ///
    /// Returns `Some(result)` when the agent should stop (validation passed),
    /// `None` when validation failed and the loop should continue so the agent
    /// can self-correct.
    async fn attempt_validated_completion(
        &self,
        message_text: &str,
    ) -> Result<Option<TaskAgentResult>> {
        let task_id = self.task.read().await.id.clone();

        if let Some(ref validation_config) = self.config.validation_config {
            if validation_config.enabled {
                tracing::info!(
                    agent_id = %self.id,
                    "running validation before completion"
                );

                let working_set_files = {
                    let ws = self.context.working_set.read().await;
                    ws.file_paths()
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                };

                let mut config_with_ws = validation_config.clone();
                config_with_ws.working_set_files = working_set_files;

                match run_validation(&config_with_ws).await {
                    Ok(result) if !result.passed => {
                        tracing::warn!(
                            agent_id = %self.id,
                            issues = result.issues.len(),
                            "validation failed, continuing loop"
                        );
                        let feedback = format_validation_feedback(&result);
                        self.conversation_history
                            .write()
                            .await
                            .push(Message::user(feedback));
                        return Ok(None);
                    }
                    Ok(_) => {
                        tracing::info!(agent_id = %self.id, "validation passed");
                    }
                    Err(e) => {
                        // Validation infrastructure error — proceed anyway.
                        tracing::error!(agent_id = %self.id, "validation error: {}", e);
                    }
                }
            }
        }

        // Finalise the task.
        self.task.write().await.complete(message_text);
        self.set_status(TaskAgentStatus::Completed(message_text.to_string()))
            .await;

        let _ = self
            .context
            .communication_hub
            .broadcast(
                self.id.clone(),
                AgentMessage::TaskResult {
                    task_id: task_id.clone(),
                    success: true,
                    result: message_text.to_string(),
                },
            )
            .await;

        let _ = self
            .context
            .communication_hub
            .unregister_agent(&self.id)
            .await;
        self.context
            .file_lock_manager
            .release_all_locks(&self.id)
            .await;

        let iterations = self.task.read().await.iterations;

        Ok(Some(TaskAgentResult {
            agent_id: self.id.clone(),
            task_id,
            success: true,
            summary: message_text.to_string(),
            iterations,
        }))
    }

    // ── Public execution entry point ─────────────────────────────────────────

    /// Execute the task to completion, returning the result.
    ///
    /// Blocks the calling async task until the agent finishes. Use
    /// [`spawn_task_agent`] to run the agent on a Tokio background task.
    pub async fn execute(&self) -> Result<TaskAgentResult> {
        let task_id = self.task.read().await.id.clone();
        let task_description = self.task.read().await.description.clone();

        tracing::info!(
            agent_id = %self.id,
            task_id = %task_id,
            "TaskAgent starting execution"
        );

        // Register with the communication hub.
        if !self
            .context
            .communication_hub
            .is_registered(&self.id)
            .await
        {
            self.context
                .communication_hub
                .register_agent(self.id.clone())
                .await?;
        }

        self.task.write().await.start();
        self.set_status(TaskAgentStatus::Working(task_description.clone()))
            .await;

        // Seed conversation with the task description as the first user message.
        self.conversation_history
            .write()
            .await
            .push(Message::user(task_description.clone()));

        let mut iterations = 0u32;
        let tool_context = ToolContext {
            working_directory: self.context.working_directory.clone(),
            user_id: None,
            metadata: HashMap::new(),
            capabilities: None,
        };

        loop {
            iterations += 1;
            self.task.write().await.increment_iteration();

            tracing::debug!(
                agent_id = %self.id,
                iteration = iterations,
                max = self.config.max_iterations,
                "iteration starting"
            );

            // ── Iteration limit ──────────────────────────────────────────────
            if iterations >= self.config.max_iterations {
                let error = format!(
                    "Agent {} exceeded maximum iterations ({})",
                    self.id, self.config.max_iterations
                );
                tracing::error!(agent_id = %self.id, %error);

                self.task.write().await.fail(&error);
                self.set_status(TaskAgentStatus::Failed(error.clone()))
                    .await;

                let _ = self
                    .context
                    .communication_hub
                    .broadcast(
                        self.id.clone(),
                        AgentMessage::TaskResult {
                            task_id: task_id.clone(),
                            success: false,
                            result: error.clone(),
                        },
                    )
                    .await;

                let _ = self
                    .context
                    .communication_hub
                    .unregister_agent(&self.id)
                    .await;
                self.context
                    .file_lock_manager
                    .release_all_locks(&self.id)
                    .await;

                return Ok(TaskAgentResult {
                    agent_id: self.id.clone(),
                    task_id,
                    success: false,
                    summary: error,
                    iterations,
                });
            }

            // ── Incoming messages (non-blocking) ────────────────────────────
            if let Some(envelope) = self
                .context
                .communication_hub
                .try_receive_message(&self.id)
                .await
            {
                if let AgentMessage::HelpResponse {
                    request_id,
                    response,
                } = envelope.message
                {
                    self.conversation_history
                        .write()
                        .await
                        .push(Message::user(format!(
                            "Response to help request {}: {}",
                            request_id, response
                        )));
                }
            }

            // ── Call provider ───────────────────────────────────────────────
            let response = self.call_provider().await?;

            let is_done = response
                .finish_reason
                .as_deref()
                .is_some_and(|r| r == "end_turn" || r == "stop");

            // ── Completion path ─────────────────────────────────────────────
            if is_done {
                let text = response
                    .message
                    .text()
                    .unwrap_or("Task completed")
                    .to_string();
                if let Some(result) = self.attempt_validated_completion(&text).await? {
                    return Ok(result);
                }
                continue; // Validation failed — let the agent self-correct.
            }

            // ── Tool execution path ─────────────────────────────────────────
            let tool_uses = Self::extract_tool_uses(&response.message);

            if tool_uses.is_empty() {
                // No tools and no explicit completion signal — treat as done.
                let text = response
                    .message
                    .text()
                    .unwrap_or("Task completed")
                    .to_string();
                if let Some(result) = self.attempt_validated_completion(&text).await? {
                    return Ok(result);
                }
                continue;
            }

            // Record the assistant's tool-use message in conversation history.
            self.conversation_history
                .write()
                .await
                .push(response.message.clone());

            for tool_use in &tool_uses {
                tracing::debug!(
                    agent_id = %self.id,
                    tool = %tool_use.name,
                    "executing tool"
                );

                let tool_result = if let Some((path, lock_type)) =
                    Self::get_lock_requirement(tool_use)
                {
                    self.set_status(TaskAgentStatus::WaitingForLock(path.clone()))
                        .await;

                    match self
                        .context
                        .file_lock_manager
                        .acquire_lock(&self.id, &path, lock_type)
                        .await
                    {
                        Ok(_guard) => {
                            self.set_status(TaskAgentStatus::Working(format!(
                                "Executing {}",
                                tool_use.name
                            )))
                            .await;
                            match self
                                .context
                                .tool_executor
                                .execute(tool_use, &tool_context)
                                .await
                            {
                                Ok(r) => r,
                                Err(e) => ToolResult::error(
                                    tool_use.id.clone(),
                                    format!("Tool execution failed: {}", e),
                                ),
                            }
                            // _guard dropped here — lock released.
                        }
                        Err(e) => {
                            tracing::warn!(
                                agent_id = %self.id,
                                path = %path,
                                "failed to acquire lock: {}",
                                e
                            );
                            ToolResult::error(
                                tool_use.id.clone(),
                                format!("Could not acquire file lock: {}", e),
                            )
                        }
                    }
                } else {
                    self.set_status(TaskAgentStatus::Working(format!(
                        "Executing {}",
                        tool_use.name
                    )))
                    .await;
                    match self
                        .context
                        .tool_executor
                        .execute(tool_use, &tool_context)
                        .await
                    {
                        Ok(r) => r,
                        Err(e) => ToolResult::error(
                            tool_use.id.clone(),
                            format!("Tool execution failed: {}", e),
                        ),
                    }
                };

                // Track file in working set for file-write operations.
                if !tool_result.is_error && Self::is_file_operation(&tool_use.name) {
                    if let Some(fp) = Self::extract_file_path(tool_use) {
                        let tokens = estimate_tokens_from_size(
                            std::fs::metadata(&fp)
                                .ok()
                                .map(|m| m.len())
                                .unwrap_or(0),
                        );
                        self.context.working_set.write().await.add(fp, tokens);
                    }
                }

                self.conversation_history
                    .write()
                    .await
                    .push(Self::tool_result_message(&tool_result));
            }
        }
    }
}

/// Spawn a task agent on a Tokio background task.
///
/// Returns a [`JoinHandle`][tokio::task::JoinHandle] that resolves to the
/// agent's [`TaskAgentResult`] when execution finishes.
pub fn spawn_task_agent(
    agent: Arc<TaskAgent>,
) -> tokio::task::JoinHandle<Result<TaskAgentResult>> {
    tokio::spawn(async move { agent.execute().await })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::communication::CommunicationHub;
    use crate::context::AgentContext;
    use crate::file_locks::FileLockManager;
    use async_trait::async_trait;
    use brainwires_core::{ChatResponse, StreamChunk, Tool, ToolContext, ToolResult, ToolUse, Usage};
    use brainwires_tools::ToolExecutor;
    use futures::stream::BoxStream;

    // ── Mock provider ──────────────────────────────────────────────────────

    struct MockProvider {
        responses: std::sync::Mutex<Vec<ChatResponse>>,
    }

    impl MockProvider {
        fn single(text: &str) -> Self {
            Self {
                responses: std::sync::Mutex::new(vec![ChatResponse {
                    message: Message::assistant(text),
                    finish_reason: Some("stop".to_string()),
                    usage: Usage::default(),
                }]),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: &[Message],
            _tools: Option<&[Tool]>,
            _options: &ChatOptions,
        ) -> Result<ChatResponse> {
            let mut guard = self.responses.lock().unwrap();
            if guard.is_empty() {
                anyhow::bail!("no more mock responses")
            }
            Ok(guard.remove(0))
        }

        fn stream_chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: Option<&'a [Tool]>,
            _options: &'a ChatOptions,
        ) -> BoxStream<'a, Result<StreamChunk>> {
            unimplemented!()
        }
    }

    // ── Mock tool executor ─────────────────────────────────────────────────

    struct NoOpExecutor;

    #[async_trait]
    impl ToolExecutor for NoOpExecutor {
        async fn execute(&self, tool_use: &ToolUse, _ctx: &ToolContext) -> Result<ToolResult> {
            Ok(ToolResult::success(tool_use.id.clone(), "ok".to_string()))
        }

        fn available_tools(&self) -> Vec<Tool> {
            vec![]
        }
    }

    fn make_context() -> Arc<AgentContext> {
        Arc::new(AgentContext::new(
            "/tmp",
            Arc::new(NoOpExecutor),
            Arc::new(CommunicationHub::new()),
            Arc::new(FileLockManager::new()),
        ))
    }

    // ── Tests ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_creation() {
        let task = Task::new("t-1", "Do something");
        let agent = TaskAgent::new(
            "agent-1".to_string(),
            task,
            Arc::new(MockProvider::single("done")),
            make_context(),
            TaskAgentConfig::default(),
        );
        assert_eq!(agent.id(), "agent-1");
        assert_eq!(agent.status().await, TaskAgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_execution_completes() {
        let task = Task::new("t-1", "Simple task");
        let agent = Arc::new(TaskAgent::new(
            "agent-1".to_string(),
            task,
            Arc::new(MockProvider::single("Task completed successfully")),
            make_context(),
            TaskAgentConfig {
                validation_config: None,
                ..Default::default()
            },
        ));

        let result = agent.execute().await.unwrap();
        assert!(result.success);
        assert_eq!(result.agent_id, "agent-1");
        assert_eq!(result.task_id, "t-1");
        assert_eq!(result.iterations, 1);
    }

    #[tokio::test]
    async fn test_spawn_task_agent() {
        let task = Task::new("t-1", "Background task");
        let agent = Arc::new(TaskAgent::new(
            "agent-1".to_string(),
            task,
            Arc::new(MockProvider::single("done")),
            make_context(),
            TaskAgentConfig {
                validation_config: None,
                ..Default::default()
            },
        ));

        let handle = spawn_task_agent(agent);
        let result = handle.await.unwrap().unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_status_display() {
        assert_eq!(TaskAgentStatus::Idle.to_string(), "Idle");
        assert_eq!(
            TaskAgentStatus::Working("reading".to_string()).to_string(),
            "Working: reading"
        );
        assert_eq!(
            TaskAgentStatus::Failed("oops".to_string()).to_string(),
            "Failed: oops"
        );
    }
}
