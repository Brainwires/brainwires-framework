//! Task Agent - Background agent that runs autonomously on a separate task
//!
//! Each TaskAgent has its own context and runs on a separate Tokio task,
//! executing a specific task and reporting results back via the communication hub.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::providers::Provider;
use crate::tools::ToolExecutor;
use crate::types::agent::{AgentContext, PermissionMode, Task};
use crate::types::message::{ChatResponse, ContentBlock, Message, MessageContent, Role};
use crate::types::provider::ChatOptions;
use crate::types::tool::{ToolContext, ToolContextExt, ToolUse};

use super::communication::{AgentMessage, CommunicationHub};
use super::file_locks::{FileLockManager, LockType};

/// Status of a task agent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskAgentStatus {
    /// Agent is idle, not working on anything
    Idle,
    /// Agent is actively working
    Working(String),
    /// Agent is waiting for a file lock
    WaitingForLock(String),
    /// Agent is paused
    Paused(String),
    /// Agent has completed its task
    Completed(String),
    /// Agent has failed
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

/// Result from a task agent execution
#[derive(Debug, Clone)]
pub struct TaskAgentResult {
    /// Agent ID
    pub agent_id: String,
    /// Task ID
    pub task_id: String,
    /// Whether the task completed successfully
    pub success: bool,
    /// Result summary
    pub summary: String,
    /// Number of iterations executed
    pub iterations: u32,
}

/// Configuration for a task agent
#[derive(Debug, Clone)]
pub struct TaskAgentConfig {
    /// Maximum iterations before giving up
    pub max_iterations: u32,
    /// Permission mode for tool execution
    pub permission_mode: PermissionMode,
    /// System prompt for the agent
    pub system_prompt: Option<String>,
    /// Temperature for AI calls
    pub temperature: f32,
    /// Max tokens for AI responses
    pub max_tokens: u32,
    /// Validation configuration (enforced quality checks)
    pub validation_config: Option<super::validation_loop::ValidationConfig>,
    /// MDAP configuration (Massively Decomposed Agentic Processes)
    pub mdap_config: Option<crate::mdap::MdapConfig>,
    /// Analytics collector — emit AgentRun and ToolCall events
    pub analytics_collector: Option<std::sync::Arc<brainwires_analytics::AnalyticsCollector>>,
}

impl Default for TaskAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,  // High default to avoid artificial limits on complex tasks
            permission_mode: PermissionMode::Auto,
            system_prompt: None,
            temperature: 0.7,
            max_tokens: 4096,  // Conservative limit to prevent corruption
            validation_config: Some(super::validation_loop::ValidationConfig::default()),  // Enable validation by default
            mdap_config: None,  // Disabled by default
            analytics_collector: crate::utils::logger::analytics_collector().map(std::sync::Arc::new),
        }
    }
}

/// Task Agent - runs autonomously on a background task
pub struct TaskAgent {
    /// Unique agent ID
    id: String,
    /// The task this agent is working on
    task: Arc<RwLock<Task>>,
    /// AI provider
    provider: Arc<dyn Provider>,
    /// Tool executor
    tool_executor: ToolExecutor,
    /// Communication hub for messaging
    communication_hub: Arc<CommunicationHub>,
    /// File lock manager
    file_lock_manager: Arc<FileLockManager>,
    /// Current status
    status: Arc<RwLock<TaskAgentStatus>>,
    /// Configuration
    config: TaskAgentConfig,
    /// Agent context
    context: Arc<RwLock<AgentContext>>,
}

impl TaskAgent {
    /// Create a new task agent
    pub fn new(
        id: String,
        task: Task,
        provider: Arc<dyn Provider>,
        communication_hub: Arc<CommunicationHub>,
        file_lock_manager: Arc<FileLockManager>,
        context: AgentContext,
        config: TaskAgentConfig,
    ) -> Self {
        Self {
            id,
            task: Arc::new(RwLock::new(task)),
            provider,
            tool_executor: ToolExecutor::new(config.permission_mode),
            communication_hub,
            file_lock_manager,
            status: Arc::new(RwLock::new(TaskAgentStatus::Idle)),
            config,
            context: Arc::new(RwLock::new(context)),
        }
    }

    /// Get the agent ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the current status
    pub async fn status(&self) -> TaskAgentStatus {
        self.status.read().await.clone()
    }

    /// Get the task
    pub async fn task(&self) -> Task {
        self.task.read().await.clone()
    }

    /// Set the status and notify via communication hub
    async fn set_status(&self, status: TaskAgentStatus) {
        *self.status.write().await = status.clone();

        // Send status update
        let _ = self
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

    /// Check if a tool is a file operation that should update working set
    fn is_file_operation(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "read_file" | "write_file" | "edit_file" | "append_to_file" | "delete_file"
        )
    }

    /// Extract file path from tool use
    fn extract_file_path(tool_use: &ToolUse) -> Option<std::path::PathBuf> {
        use std::path::PathBuf;

        // Check different parameter names tools use for file paths
        let path_str = tool_use.input.get("file_path")
            .or_else(|| tool_use.input.get("path"))
            .and_then(|v| v.as_str())?;

        Some(PathBuf::from(path_str))
    }

    /// Execute the task
    pub async fn execute(&self) -> Result<TaskAgentResult> {
        let task_id = {
            let task = self.task.read().await;
            task.id.clone()
        };

        let task_description = {
            let task = self.task.read().await;
            task.description.clone()
        };

        tracing::info!(
            agent_id = %self.id,
            task_id = %task_id,
            task = %task_description,
            "TaskAgent starting execution"
        );

        // Register with communication hub
        if !self.communication_hub.is_registered(&self.id).await {
            self.communication_hub.register_agent(self.id.clone()).await?;
        }

        // Start the task
        {
            let mut task = self.task.write().await;
            task.start();
        }

        self.set_status(TaskAgentStatus::Working(task_description.clone()))
            .await;

        // Add initial user message
        {
            let mut context = self.context.write().await;
            let user_message = Message {
                role: Role::User,
                content: MessageContent::Text(task_description.clone()),
                name: None,
                metadata: None,
            };
            context.conversation_history.push(user_message);
        }

        let mut iterations = 0;

        loop {
            iterations += 1;

            tracing::debug!(
                agent_id = %self.id,
                iteration = iterations,
                max_iterations = self.config.max_iterations,
                "TaskAgent iteration starting"
            );

            // Update task iterations
            {
                let mut task = self.task.write().await;
                task.increment_iteration();
            }

            // Check iteration limit
            if iterations >= self.config.max_iterations {
                let error = format!(
                    "Agent {} exceeded maximum iterations ({})",
                    self.id, self.config.max_iterations
                );

                tracing::error!(
                    agent_id = %self.id,
                    iterations = iterations,
                    max_iterations = self.config.max_iterations,
                    "TaskAgent exceeded max iterations"
                );

                {
                    let mut task = self.task.write().await;
                    task.fail(&error);
                }

                self.set_status(TaskAgentStatus::Failed(error.clone())).await;

                // Send task result
                let _ = self
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

                // Unregister
                let _ = self.communication_hub.unregister_agent(&self.id).await;

                // Release all locks
                self.file_lock_manager.release_all_locks(&self.id).await;

                return Ok(TaskAgentResult {
                    agent_id: self.id.clone(),
                    task_id,
                    success: false,
                    summary: error,
                    iterations,
                });
            }

            // Check for incoming messages (non-blocking)
            if let Some(envelope) = self.communication_hub.try_receive_message(&self.id).await {
                match envelope.message {
                    AgentMessage::HelpResponse { request_id, response } => {
                        // Add help response to context
                        let mut context = self.context.write().await;
                        context.conversation_history.push(Message {
                            role: Role::User,
                            content: MessageContent::Text(format!(
                                "Response to help request {}: {}",
                                request_id, response
                            )),
                            name: None,
                            metadata: None,
                        });
                    }
                    _ => {
                        // Handle other messages as needed
                    }
                }
            }

            // Call the AI provider
            let response = self.call_provider().await?;

            // Check if task is complete
            if let Some(finish_reason) = &response.finish_reason {
                if finish_reason == "end_turn" || finish_reason == "stop" {
                    let message_text = response
                        .message
                        .text()
                        .unwrap_or("Task completed")
                        .to_string();

                    // VALIDATION: Run checks before allowing completion
                    if let Some(validation_attempt) = self.attempt_validated_completion(&message_text).await? {
                        return Ok(validation_attempt);
                    }

                    // Validation failed, continue looping to let agent fix issues
                    continue;
                }
            }

            // Process tool uses
            let tool_uses = self.extract_tool_uses(&response.message);

            if tool_uses.is_empty() {
                // No tool uses, treat as completion
                let message_text = response
                    .message
                    .text()
                    .unwrap_or("Task completed")
                    .to_string();

                // VALIDATION: Run checks before allowing completion
                if let Some(validation_attempt) = self.attempt_validated_completion(&message_text).await? {
                    return Ok(validation_attempt);
                }

                // Validation failed, continue looping to let agent fix issues
                continue;
            }

            // Add assistant message to history
            {
                let mut context = self.context.write().await;
                context.conversation_history.push(response.message.clone());
            }

            // Execute tools
            let tool_context = {
                let context = self.context.read().await;
                ToolContext::from_agent_context(&context)
            };

            for tool_use in tool_uses {
                tracing::debug!("[Agent {}] Processing tool: {}", self.id, tool_use.name);

                // Determine if we need file locks
                let lock_needed = self.get_lock_requirement(&tool_use);
                tracing::debug!("[Agent {}] Lock needed: {:?}", self.id, lock_needed);

                if let Some((path, lock_type)) = lock_needed {
                    // Try to acquire lock
                    tracing::debug!("[Agent {}] Acquiring lock for path: {}", self.id, path);
                    self.set_status(TaskAgentStatus::WaitingForLock(path.clone()))
                        .await;

                    match self
                        .file_lock_manager
                        .acquire_lock(&self.id, &path, lock_type)
                        .await
                    {
                        Ok(_guard) => {
                            tracing::debug!("[Agent {}] Lock acquired, executing {}", self.id, tool_use.name);
                            self.set_status(TaskAgentStatus::Working(format!(
                                "Executing {}",
                                tool_use.name
                            )))
                            .await;

                            // Execute the tool
                            tracing::debug!("[Agent {}] Calling tool_executor.execute for {}", self.id, tool_use.name);
                            let _tool_start = std::time::Instant::now();
                            let result = self.tool_executor.execute(&tool_use, &tool_context).await?;
                            if let Some(ref collector) = self.config.analytics_collector {
                                collector.record(brainwires_analytics::AnalyticsEvent::ToolCall {
                                    session_id: None,
                                    agent_id: Some(self.id.clone()),
                                    tool_name: tool_use.name.clone(),
                                    tool_use_id: tool_use.id.clone(),
                                    is_error: result.is_error,
                                    duration_ms: Some(_tool_start.elapsed().as_millis() as u64),
                                    timestamp: chrono::Utc::now(),
                                });
                            }
                            tracing::debug!("[Agent {}] Tool {} returned: is_error={}", self.id, tool_use.name, result.is_error);

                            // Add tool result to context
                            tracing::debug!("[Agent {}] Acquiring context write lock", self.id);
                            let mut context = self.context.write().await;
                            tracing::debug!("[Agent {}] Context write lock acquired", self.id);
                            context.conversation_history.push(Message {
                                role: Role::User,
                                content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                                    tool_use_id: result.tool_use_id.clone(),
                                    content: result.content.clone(),
                                    is_error: Some(result.is_error),
                                }]),
                                name: None,
                                metadata: None,
                            });
                            tracing::debug!("[Agent {}] Tool result added to context", self.id);

                            // Add file to working set for file operations
                            if !result.is_error && Self::is_file_operation(&tool_use.name) {
                                if let Some(file_path) = Self::extract_file_path(&tool_use) {
                                    let tokens = crate::types::working_set::estimate_tokens_from_size(
                                        std::fs::metadata(&file_path).ok().map(|m| m.len()).unwrap_or(0)
                                    );
                                    let path_display = file_path.display().to_string();
                                    context.working_set.add(file_path, tokens);
                                    tracing::debug!("[Agent {}] Added {} to working set", self.id, path_display);
                                }
                            }

                            // Lock is released when guard is dropped
                        }
                        Err(e) => {
                            tracing::warn!("[Agent {}] Failed to acquire lock: {}", self.id, e);
                            // Could not acquire lock - report as tool error
                            let mut context = self.context.write().await;
                            context.conversation_history.push(Message {
                                role: Role::User,
                                content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                                    tool_use_id: tool_use.id.clone(),
                                    content: format!("Could not acquire file lock: {}", e),
                                    is_error: Some(true),
                                }]),
                                name: None,
                                metadata: None,
                            });
                        }
                    }
                } else {
                    // No lock needed
                    self.set_status(TaskAgentStatus::Working(format!(
                        "Executing {}",
                        tool_use.name
                    )))
                    .await;

                    let _tool_start = std::time::Instant::now();
                    let result = self.tool_executor.execute(&tool_use, &tool_context).await?;
                    if let Some(ref collector) = self.config.analytics_collector {
                        collector.record(brainwires_analytics::AnalyticsEvent::ToolCall {
                            session_id: None,
                            agent_id: Some(self.id.clone()),
                            tool_name: tool_use.name.clone(),
                            tool_use_id: tool_use.id.clone(),
                            is_error: result.is_error,
                            duration_ms: Some(_tool_start.elapsed().as_millis() as u64),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    let mut context = self.context.write().await;
                    context.conversation_history.push(Message {
                        role: Role::User,
                        content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                            tool_use_id: result.tool_use_id.clone(),
                            content: result.content.clone(),
                            is_error: Some(result.is_error),
                        }]),
                        name: None,
                        metadata: None,
                    });

                    // Add file to working set for file operations
                    if !result.is_error && Self::is_file_operation(&tool_use.name) {
                        if let Some(file_path) = Self::extract_file_path(&tool_use) {
                            let tokens = crate::types::working_set::estimate_tokens_from_size(
                                std::fs::metadata(&file_path).ok().map(|m| m.len()).unwrap_or(0)
                            );
                            context.working_set.add(file_path, tokens);
                            tracing::debug!("[Agent {}] Added file to working set", self.id);
                        }
                    }
                }
            }
        }
    }

    /// Attempt to complete task with validation checks
    /// Returns Some(result) if validation passed, None if failed (should retry)
    async fn attempt_validated_completion(&self, message_text: &str) -> Result<Option<TaskAgentResult>> {
        let task_id = {
            let task = self.task.read().await;
            task.id.clone()
        };

        // Check if validation is enabled
        if let Some(ref validation_config) = self.config.validation_config {
            if validation_config.enabled {
                tracing::info!("[Agent {}] Running validation checks before completion...", self.id);

                // Get working set files from context
                let working_set_files = {
                    let context = self.context.read().await;
                    context.working_set.file_paths()
                        .iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect::<Vec<String>>()
                };

                // Update validation config with working set files
                let mut config_with_ws = validation_config.clone();
                config_with_ws.working_set_files = working_set_files;

                tracing::debug!(
                    "[Agent {}] Validating {} working set files",
                    self.id,
                    config_with_ws.working_set_files.len()
                );

                // Run validation
                match super::validation_loop::run_validation(&config_with_ws).await {
                    Ok(validation_result) => {
                        if !validation_result.passed {
                            // Validation failed - inject feedback and continue
                            tracing::warn!(
                                "[Agent {}] Validation failed with {} issues",
                                self.id,
                                validation_result.issues.len()
                            );

                            let feedback = super::validation_loop::format_validation_feedback(&validation_result);

                            // Add validation feedback to conversation history
                            {
                                let mut context = self.context.write().await;
                                context.conversation_history.push(Message {
                                    role: Role::User,
                                    content: MessageContent::Text(feedback),
                                    name: None,
                                    metadata: None,
                                });
                            }

                            // Return None to continue the loop
                            return Ok(None);
                        } else {
                            tracing::info!("[Agent {}] ✓ All validation checks passed!", self.id);
                        }
                    }
                    Err(e) => {
                        tracing::error!("[Agent {}] Validation error: {}", self.id, e);
                        // Continue anyway if validation itself fails
                    }
                }
            }
        }

        // Validation passed or disabled - complete the task
        {
            let mut task = self.task.write().await;
            task.complete(message_text);
        }

        self.set_status(TaskAgentStatus::Completed(message_text.to_string()))
            .await;

        // Send task result
        let _ = self
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

        // Unregister
        let _ = self.communication_hub.unregister_agent(&self.id).await;

        // Release all locks
        self.file_lock_manager.release_all_locks(&self.id).await;

        // Get iterations from context
        let iterations = {
            let task = self.task.read().await;
            task.iterations
        };

        Ok(Some(TaskAgentResult {
            agent_id: self.id.clone(),
            task_id,
            success: true,
            summary: message_text.to_string(),
            iterations,
        }))
    }

    /// Call the AI provider
    async fn call_provider(&self) -> Result<ChatResponse> {
        let context = self.context.read().await;

        let system_prompt = self.config.system_prompt.clone().unwrap_or_else(|| {
            crate::agents::system_prompts::reasoning_agent_prompt(&self.id, &context.working_directory)
        });

        let options = ChatOptions {
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            top_p: None,
            stop: None,
            system: Some(system_prompt),
            model: None,
        };

        self.provider
            .chat(&context.conversation_history, Some(&context.tools), &options)
            .await
    }

    /// Extract tool uses from a message
    fn extract_tool_uses(&self, message: &Message) -> Vec<ToolUse> {
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

    /// Determine if a tool needs a file lock and what type
    fn get_lock_requirement(&self, tool_use: &ToolUse) -> Option<(String, LockType)> {
        let name = tool_use.name.as_str();

        // Extract path from tool input
        let path = tool_use.input.get("path").or_else(|| tool_use.input.get("file_path"));

        if let Some(path_value) = path {
            if let Some(path_str) = path_value.as_str() {
                match name {
                    // Read operations - shared lock
                    "read_file" | "list_directory" | "search_code" => {
                        Some((path_str.to_string(), LockType::Read))
                    }
                    // Write operations - exclusive lock
                    "write_file" | "edit_file" | "patch_file" | "delete_file" | "create_directory" => {
                        Some((path_str.to_string(), LockType::Write))
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Spawn a task agent on a background Tokio task
pub fn spawn_task_agent(agent: Arc<TaskAgent>) -> tokio::task::JoinHandle<Result<TaskAgentResult>> {
    tokio::spawn(async move { agent.execute().await })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::agent::Task;
    use crate::types::message::{ChatResponse, Message, MessageContent, Role, StreamChunk, Usage};
    use crate::types::provider::ChatOptions;
    use crate::types::tool::Tool;
    use async_trait::async_trait;
    use futures::stream::BoxStream;

    /// Mock provider for testing
    struct MockProvider {
        responses: std::sync::Mutex<Vec<ChatResponse>>,
    }

    impl MockProvider {
        fn new(responses: Vec<ChatResponse>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }

        fn single_response(text: &str) -> Self {
            Self::new(vec![ChatResponse {
                message: Message {
                    role: Role::Assistant,
                    content: MessageContent::Text(text.to_string()),
                    name: None,
                    metadata: None,
                },
                finish_reason: Some("stop".to_string()),
                usage: Usage::default(),
            }])
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock-provider"
        }

        async fn chat(
            &self,
            _messages: &[Message],
            _tools: Option<&[Tool]>,
            _options: &ChatOptions,
        ) -> Result<ChatResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                anyhow::bail!("No more mock responses")
            }
            Ok(responses.remove(0))
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

    #[tokio::test]
    async fn test_task_agent_creation() {
        let provider = Arc::new(MockProvider::single_response("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let lock_manager = Arc::new(FileLockManager::new());
        let task = Task::new("task-1", "Test task");
        let context = AgentContext::default();
        let config = TaskAgentConfig::default();

        let agent = TaskAgent::new(
            "agent-1".to_string(),
            task,
            provider,
            hub,
            lock_manager,
            context,
            config,
        );

        assert_eq!(agent.id(), "agent-1");
        assert_eq!(agent.status().await, TaskAgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_task_agent_execution() {
        let provider = Arc::new(MockProvider::single_response("Task completed successfully"));
        let hub = Arc::new(CommunicationHub::new());
        let lock_manager = Arc::new(FileLockManager::new());
        let task = Task::new("task-1", "Test task");
        let context = AgentContext::default();
        let config = TaskAgentConfig {
            validation_config: None,
            ..Default::default()
        };

        let agent = Arc::new(TaskAgent::new(
            "agent-1".to_string(),
            task,
            provider,
            hub,
            lock_manager,
            context,
            config,
        ));

        let result = agent.execute().await.unwrap();

        assert!(result.success);
        assert_eq!(result.agent_id, "agent-1");
        assert_eq!(result.task_id, "task-1");
        assert_eq!(result.iterations, 1);
    }

    #[tokio::test]
    async fn test_task_agent_status() {
        let status = TaskAgentStatus::Working("Processing data".to_string());
        assert_eq!(status.to_string(), "Working: Processing data");

        let status = TaskAgentStatus::Completed("All done".to_string());
        assert_eq!(status.to_string(), "Completed: All done");

        let status = TaskAgentStatus::Failed("Error occurred".to_string());
        assert_eq!(status.to_string(), "Failed: Error occurred");
    }

    #[tokio::test]
    async fn test_spawn_task_agent() {
        let provider = Arc::new(MockProvider::single_response("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let lock_manager = Arc::new(FileLockManager::new());
        let task = Task::new("task-1", "Test task");
        let context = AgentContext::default();
        let config = TaskAgentConfig {
            validation_config: None,
            ..Default::default()
        };

        let agent = Arc::new(TaskAgent::new(
            "agent-1".to_string(),
            task,
            provider,
            hub,
            lock_manager,
            context,
            config,
        ));

        let handle = spawn_task_agent(agent);
        let result = handle.await.unwrap().unwrap();

        assert!(result.success);
    }
}
