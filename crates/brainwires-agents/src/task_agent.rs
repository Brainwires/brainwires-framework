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

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use brainwires_core::{
    estimate_tokens_from_size, ChatOptions, ChatResponse, ContentBlock, ContentSource, Message,
    MessageContent, Provider, Role, Task, ToolContext, ToolResult, ToolUse,
};
use brainwires_model_tools::{wrap_with_content_source, PreHookDecision};

use crate::communication::AgentMessage;
use crate::context::AgentContext;
use crate::execution_graph::{ExecutionGraph, RunTelemetry, ToolCallRecord};
use crate::file_locks::LockType;
use crate::validation_loop::{format_validation_feedback, run_validation, ValidationConfig};

/// Tool names whose results originate from external / untrusted sources and
/// must be sanitised before injection into the conversation history.
const EXTERNAL_CONTENT_TOOLS: &[&str] = &[
    "fetch_url",
    "web_fetch",
    "web_search",
    "context_recall",
    "semantic_search",
];

/// Configuration for stuck-agent (loop) detection.
#[derive(Debug, Clone)]
pub struct LoopDetectionConfig {
    /// Consecutive identical tool-name calls that trigger abort. Default: 5.
    pub window_size: usize,
    /// Whether loop detection is active. Default: true.
    pub enabled: bool,
}

impl Default for LoopDetectionConfig {
    fn default() -> Self {
        Self {
            window_size: 5,
            enabled: true,
        }
    }
}

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
    /// Agent is replanning after detecting goal drift or failure.
    Replanning(String),
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
            TaskAgentStatus::Replanning(reason) => write!(f, "Replanning: {}", reason),
            TaskAgentStatus::Completed(summary) => write!(f, "Completed: {}", summary),
            TaskAgentStatus::Failed(error) => write!(f, "Failed: {}", error),
        }
    }
}

/// Classification of why an agent run failed.
///
/// Always `Some` when [`TaskAgentResult::success`] is `false`, always `None`
/// on success.  Enables trend queries and dashboards over failure modes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FailureCategory {
    /// Agent exhausted the allowed iteration count.
    IterationLimitExceeded,
    /// Cumulative token usage exceeded [`TaskAgentConfig::max_total_tokens`].
    TokenBudgetExceeded,
    /// Cumulative cost exceeded [`TaskAgentConfig::max_cost_usd`].
    CostBudgetExceeded,
    /// Wall-clock timeout exceeded [`TaskAgentConfig::timeout_secs`].
    WallClockTimeout,
    /// Loop detection fired — agent was calling the same tool repeatedly.
    LoopDetected,
    /// Replan cycle count exceeded [`TaskAgentConfig::max_replan_attempts`].
    MaxReplanAttemptsExceeded,
    /// File scope whitelist violation (reserved for future hard-stop policy).
    FileScopeViolation,
    /// Validation checks failed and could not be resolved within the
    /// iteration budget.
    ValidationFailed,
    /// An unexpected tool execution error caused abort.
    ToolExecutionError,
    /// Failure cause could not be determined.
    Unknown,
    /// Plan budget check failed before execution started — task was rejected
    /// before any side effects occurred.
    PlanBudgetExceeded,
}

/// Result of a completed task agent execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Number of replan cycles during execution.
    pub replan_count: u32,
    /// True when any budget ceiling caused the stop.
    pub budget_exhausted: bool,
    /// Last meaningful assistant message when stopped early, if any.
    pub partial_output: Option<String>,
    /// Cumulative tokens consumed across all provider calls.
    pub total_tokens_used: u64,
    /// Estimated cost in USD ($0.000003/token conservative estimate).
    pub total_cost_usd: f64,
    /// True when wall-clock timeout caused the stop.
    pub timed_out: bool,
    /// Why the agent failed. `None` on success, always `Some` on failure.
    pub failure_category: Option<FailureCategory>,
    /// Full execution trace (DAG of provider-call steps + tool call records).
    pub execution_graph: ExecutionGraph,
    /// Structured telemetry summary derived from the execution graph.
    pub telemetry: RunTelemetry,
    /// Pre-execution plan produced before the task loop started, if
    /// [`TaskAgentConfig::plan_budget`] was configured.  `None` when planning
    /// was not requested or when the plan could not be parsed.
    pub pre_execution_plan: Option<brainwires_core::SerializablePlan>,
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

    /// Loop detection settings. `None` disables. Default: 5-call window, enabled.
    pub loop_detection: Option<LoopDetectionConfig>,

    /// Inject goal-reminder every N iterations. `None` disables. Default: Some(10).
    pub goal_revalidation_interval: Option<u32>,

    /// Abort after this many REPLAN cycles. Default: 3.
    pub max_replan_attempts: u32,

    /// Abort when cumulative tokens reach this ceiling. Default: None.
    pub max_total_tokens: Option<u64>,

    /// Abort when cumulative cost (USD) reaches this ceiling. Default: None.
    pub max_cost_usd: Option<f64>,

    /// Wall-clock timeout for the entire execute() call, in seconds. Default: None.
    pub timeout_secs: Option<u64>,

    /// Per-agent file scope whitelist.
    ///
    /// When `Some`, the agent receives a scope-violation error for any file
    /// operation targeting a path that is not prefixed by at least one entry
    /// in this list.  When `None`, file access is unrestricted.
    ///
    /// Uses [`Path::starts_with`] for prefix matching, which is
    /// component-aware: `"/src"` allows `"/src/main.rs"` but denies
    /// `"/src_extra/file.txt"`.
    pub allowed_files: Option<Vec<PathBuf>>,

    /// Optional pre-execution budget check.
    ///
    /// When `Some`, the agent asks the provider to produce a structured JSON
    /// plan before starting execution. The plan is validated against the budget
    /// constraints; if any constraint is exceeded the run fails immediately
    /// with [`FailureCategory::PlanBudgetExceeded`] before any file or tool
    /// side-effects occur.
    ///
    /// Set to `None` (the default) to skip the planning phase entirely.
    pub plan_budget: Option<brainwires_core::PlanBudget>,
}

impl Default for TaskAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            system_prompt: None,
            temperature: 0.7,
            max_tokens: 4096,
            validation_config: Some(ValidationConfig::default()),
            loop_detection: Some(LoopDetectionConfig::default()),
            goal_revalidation_interval: Some(10),
            max_replan_attempts: 3,
            max_total_tokens: None,
            max_cost_usd: None,
            timeout_secs: None,
            allowed_files: None,
            plan_budget: None,
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
    /// Internal replan cycle counter.
    replan_count: Arc<RwLock<u32>>,
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
            replan_count: Arc::new(RwLock::new(0)),
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

    /// Returns `true` if `path` is permitted by the file scope whitelist.
    fn is_file_path_allowed(path: &str, allowed: &[PathBuf]) -> bool {
        if allowed.is_empty() {
            return false;
        }
        let candidate = PathBuf::from(path);
        allowed.iter().any(|prefix| candidate.starts_with(prefix))
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
        total_tokens_used: u64,
        total_cost_usd: f64,
        replan_count: u32,
        execution_graph: ExecutionGraph,
        pre_execution_plan: Option<brainwires_core::SerializablePlan>,
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
        let run_ended_at = Utc::now();
        let telemetry =
            RunTelemetry::from_graph(&execution_graph, run_ended_at, true, total_cost_usd);

        Ok(Some(TaskAgentResult {
            agent_id: self.id.clone(),
            task_id,
            success: true,
            summary: message_text.to_string(),
            iterations,
            replan_count,
            budget_exhausted: false,
            partial_output: None,
            total_tokens_used,
            total_cost_usd,
            timed_out: false,
            failure_category: None,
            execution_graph,
            telemetry,
            pre_execution_plan,
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

        // ── Prompt hash + execution graph initialisation ─────────────────────
        let prompt_hash = {
            let system_prompt = self.config.system_prompt.clone().unwrap_or_else(|| {
                crate::system_prompts::reasoning_agent_prompt(
                    &self.id,
                    &self.context.working_directory,
                )
            });
            let mut tool_names: Vec<String> = self
                .context
                .tool_executor
                .available_tools()
                .iter()
                .map(|t| t.name.clone())
                .collect();
            tool_names.sort_unstable();
            let mut hasher = Sha256::new();
            hasher.update(system_prompt.as_bytes());
            for name in &tool_names {
                hasher.update(name.as_bytes());
            }
            hex::encode(hasher.finalize())
        };
        let run_started_at = Utc::now();
        let mut execution_graph = ExecutionGraph::new(prompt_hash, run_started_at);

        // ── Pre-execution planning phase ─────────────────────────────────────
        // When plan_budget is set, ask the model for a structured JSON plan and
        // validate it against the budget before any side effects occur.
        let mut pre_execution_plan: Option<brainwires_core::SerializablePlan> = None;
        if let Some(ref budget) = self.config.plan_budget {
            let planning_msg = Message::user(format!(
                "Before beginning work, produce a JSON execution plan for this task.\n\n\
                 Task: {task_description}\n\n\
                 Reply with ONLY a JSON object in this exact format:\n\
                 {{\"steps\":[{{\"description\":\"short description\",\"tool\":\"tool_name\",\"estimated_tokens\":500}},...]}}\n\n\
                 Estimate 200–2000 tokens per step based on expected complexity. \
                 Do not perform any work yet — only plan.",
            ));
            let planning_options = brainwires_core::ChatOptions {
                temperature: Some(0.1),
                max_tokens: Some(2048),
                top_p: None,
                stop: None,
                system: Some(
                    "You are a planning assistant. Respond only with a valid JSON execution plan.".to_string(),
                ),
            };
            match self.provider.chat(&[planning_msg], None, &planning_options).await {
                Ok(response) => {
                    let plan_text = response.message.text().unwrap_or("").to_string();
                    if let Some(plan) = brainwires_core::SerializablePlan::parse_from_text(
                        task_description.clone(),
                        &plan_text,
                    ) {
                        match budget.check(&plan) {
                            Ok(()) => {
                                tracing::info!(
                                    agent_id = %self.id,
                                    steps = plan.step_count(),
                                    estimated_tokens = plan.total_estimated_tokens(),
                                    "pre-execution plan accepted"
                                );
                                pre_execution_plan = Some(plan);
                            }
                            Err(reason) => {
                                let error = format!(
                                    "Agent {} rejected by plan budget before execution: {}",
                                    self.id, reason
                                );
                                tracing::error!(agent_id = %self.id, %error);
                                self.task.write().await.fail(&error);
                                self.set_status(TaskAgentStatus::Failed(error.clone())).await;
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
                                let run_ended_at = Utc::now();
                                let telemetry = RunTelemetry::from_graph(
                                    &execution_graph,
                                    run_ended_at,
                                    false,
                                    0.0,
                                );
                                return Ok(TaskAgentResult {
                                    agent_id: self.id.clone(),
                                    task_id,
                                    success: false,
                                    summary: error,
                                    iterations: 0,
                                    replan_count: 0,
                                    budget_exhausted: true,
                                    partial_output: None,
                                    total_tokens_used: 0,
                                    total_cost_usd: 0.0,
                                    timed_out: false,
                                    failure_category: Some(FailureCategory::PlanBudgetExceeded),
                                    execution_graph,
                                    telemetry,
                                    pre_execution_plan: None,
                                });
                            }
                        }
                    } else {
                        tracing::warn!(
                            agent_id = %self.id,
                            "could not parse pre-execution plan from model response; \
                             proceeding without budget guard"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        agent_id = %self.id,
                        error = %e,
                        "planning phase provider call failed; proceeding without plan"
                    );
                }
            }
        }

        let mut iterations = 0u32;
        let mut total_tokens_used: u64 = 0;
        let mut total_cost_usd: f64 = 0.0;
        const COST_PER_TOKEN: f64 = 0.000003; // $3/M tokens conservative estimate
        let start_time = std::time::Instant::now();
        let mut recent_tool_names: VecDeque<String> = VecDeque::with_capacity(
            self.config
                .loop_detection
                .as_ref()
                .map(|c| c.window_size)
                .unwrap_or(5),
        );
        let tool_context = ToolContext {
            working_directory: self.context.working_directory.clone(),
            // Each agent run gets its own idempotency registry so that
            // identical write operations within a single run are deduplicated.
            idempotency_registry: Some(brainwires_core::IdempotencyRegistry::new()),
            ..Default::default()
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

            let step_started_at = Utc::now();
            let step_idx = execution_graph.push_step(iterations, step_started_at);

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

                let run_ended_at = Utc::now();
                let telemetry = RunTelemetry::from_graph(
                    &execution_graph,
                    run_ended_at,
                    false,
                    total_cost_usd,
                );
                return Ok(TaskAgentResult {
                    agent_id: self.id.clone(),
                    task_id,
                    success: false,
                    summary: error,
                    iterations,
                    replan_count: *self.replan_count.read().await,
                    budget_exhausted: false,
                    partial_output: None,
                    total_tokens_used,
                    total_cost_usd,
                    timed_out: false,
                    failure_category: Some(FailureCategory::IterationLimitExceeded),
                    execution_graph: execution_graph.clone(),
                    telemetry,
                    pre_execution_plan: pre_execution_plan.clone(),
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

            // ── Budget: timeout ──────────────────────────────────────────────
            if let Some(secs) = self.config.timeout_secs {
                if start_time.elapsed().as_secs() >= secs {
                    let elapsed = start_time.elapsed().as_secs();
                    let partial = self.last_assistant_text().await;
                    let error = format!(
                        "Agent {} timed out after {}s (limit: {}s)",
                        self.id, elapsed, secs
                    );
                    tracing::error!(agent_id = %self.id, %error);
                    self.task.write().await.fail(&error);
                    self.set_status(TaskAgentStatus::Failed(error.clone())).await;
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
                    let _ = self.context.communication_hub.unregister_agent(&self.id).await;
                    self.context.file_lock_manager.release_all_locks(&self.id).await;
                    let run_ended_at = Utc::now();
                    let telemetry = RunTelemetry::from_graph(
                        &execution_graph,
                        run_ended_at,
                        false,
                        total_cost_usd,
                    );
                    return Ok(TaskAgentResult {
                        agent_id: self.id.clone(),
                        task_id,
                        success: false,
                        summary: error,
                        iterations,
                        replan_count: *self.replan_count.read().await,
                        budget_exhausted: false,
                        partial_output: partial,
                        total_tokens_used,
                        total_cost_usd,
                        timed_out: true,
                        failure_category: Some(FailureCategory::WallClockTimeout),
                        execution_graph: execution_graph.clone(),
                        telemetry,
                        pre_execution_plan: pre_execution_plan.clone(),
                    });
                }
            }

            // ── Budget: token ceiling ────────────────────────────────────────
            if let Some(max) = self.config.max_total_tokens {
                if total_tokens_used >= max {
                    let partial = self.last_assistant_text().await;
                    let error = format!(
                        "Agent {} exceeded token budget ({}/{} tokens)",
                        self.id, total_tokens_used, max
                    );
                    tracing::error!(agent_id = %self.id, %error);
                    self.task.write().await.fail(&error);
                    self.set_status(TaskAgentStatus::Failed(error.clone())).await;
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
                    let _ = self.context.communication_hub.unregister_agent(&self.id).await;
                    self.context.file_lock_manager.release_all_locks(&self.id).await;
                    let run_ended_at = Utc::now();
                    let telemetry = RunTelemetry::from_graph(
                        &execution_graph,
                        run_ended_at,
                        false,
                        total_cost_usd,
                    );
                    return Ok(TaskAgentResult {
                        agent_id: self.id.clone(),
                        task_id,
                        success: false,
                        summary: error,
                        iterations,
                        replan_count: *self.replan_count.read().await,
                        budget_exhausted: true,
                        partial_output: partial,
                        total_tokens_used,
                        total_cost_usd,
                        timed_out: false,
                        failure_category: Some(FailureCategory::TokenBudgetExceeded),
                        execution_graph: execution_graph.clone(),
                        telemetry,
                        pre_execution_plan: pre_execution_plan.clone(),
                    });
                }
            }

            // ── Budget: cost ceiling ─────────────────────────────────────────
            if let Some(max) = self.config.max_cost_usd {
                if total_cost_usd >= max {
                    let partial = self.last_assistant_text().await;
                    let error = format!(
                        "Agent {} exceeded cost budget (${:.6}/{:.6} USD)",
                        self.id, total_cost_usd, max
                    );
                    tracing::error!(agent_id = %self.id, %error);
                    self.task.write().await.fail(&error);
                    self.set_status(TaskAgentStatus::Failed(error.clone())).await;
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
                    let _ = self.context.communication_hub.unregister_agent(&self.id).await;
                    self.context.file_lock_manager.release_all_locks(&self.id).await;
                    let run_ended_at = Utc::now();
                    let telemetry = RunTelemetry::from_graph(
                        &execution_graph,
                        run_ended_at,
                        false,
                        total_cost_usd,
                    );
                    return Ok(TaskAgentResult {
                        agent_id: self.id.clone(),
                        task_id,
                        success: false,
                        summary: error,
                        iterations,
                        replan_count: *self.replan_count.read().await,
                        budget_exhausted: true,
                        partial_output: partial,
                        total_tokens_used,
                        total_cost_usd,
                        timed_out: false,
                        failure_category: Some(FailureCategory::CostBudgetExceeded),
                        execution_graph: execution_graph.clone(),
                        telemetry,
                        pre_execution_plan: pre_execution_plan.clone(),
                    });
                }
            }

            // ── Goal re-validation ───────────────────────────────────────────
            if let Some(interval) = self.config.goal_revalidation_interval {
                if interval > 0 && iterations > 1 && (iterations - 1) % interval == 0 {
                    self.conversation_history.write().await.push(Message::user(format!(
                        "GOAL CHECK (iteration {}): Your original task was:\n\n\"{}\"\n\n\
                         Confirm you are still on track. Correct course if you have drifted.",
                        iterations, task_description
                    )));
                }
            }

            // ── Call provider ───────────────────────────────────────────────
            let response = self.call_provider().await?;

            // ── Accumulate token usage ───────────────────────────────────────
            total_tokens_used += response.usage.total_tokens as u64;
            total_cost_usd += response.usage.total_tokens as f64 * COST_PER_TOKEN;

            // ── Finalise step node ───────────────────────────────────────────
            execution_graph.finalize_step(
                step_idx,
                Utc::now(),
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.finish_reason.clone(),
            );

            // ── REPLAN detection ─────────────────────────────────────────────
            {
                let text = response.message.text().unwrap_or("").to_lowercase();
                if text.contains("replan") || text.contains("replanning") {
                    let mut count = self.replan_count.write().await;
                    *count += 1;
                    let c = *count;
                    drop(count);
                    self.set_status(TaskAgentStatus::Replanning(format!(
                        "attempt {}/{}",
                        c, self.config.max_replan_attempts
                    )))
                    .await;
                    if c > self.config.max_replan_attempts {
                        let error = format!(
                            "Agent {} exceeded max replan attempts ({}/{})",
                            self.id, c, self.config.max_replan_attempts
                        );
                        tracing::error!(agent_id = %self.id, %error);
                        self.task.write().await.fail(&error);
                        self.set_status(TaskAgentStatus::Failed(error.clone())).await;
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
                        let _ = self.context.communication_hub.unregister_agent(&self.id).await;
                        self.context.file_lock_manager.release_all_locks(&self.id).await;
                        let run_ended_at = Utc::now();
                        let telemetry = RunTelemetry::from_graph(
                            &execution_graph,
                            run_ended_at,
                            false,
                            total_cost_usd,
                        );
                        return Ok(TaskAgentResult {
                            agent_id: self.id.clone(),
                            task_id,
                            success: false,
                            summary: error,
                            iterations,
                            replan_count: c,
                            budget_exhausted: false,
                            partial_output: None,
                            total_tokens_used,
                            total_cost_usd,
                            timed_out: false,
                            failure_category: Some(FailureCategory::MaxReplanAttemptsExceeded),
                            execution_graph: execution_graph.clone(),
                            telemetry,
                            pre_execution_plan: pre_execution_plan.clone(),
                        });
                    }
                }
            }

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
                if let Some(result) = self
                    .attempt_validated_completion(
                        &text,
                        total_tokens_used,
                        total_cost_usd,
                        *self.replan_count.read().await,
                        execution_graph.clone(),
                        pre_execution_plan.clone(),
                    )
                    .await?
                {
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
                if let Some(result) = self
                    .attempt_validated_completion(
                        &text,
                        total_tokens_used,
                        total_cost_usd,
                        *self.replan_count.read().await,
                        execution_graph.clone(),
                        pre_execution_plan.clone(),
                    )
                    .await?
                {
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

                // ── Pre-execute hook ─────────────────────────────────────────
                if let Some(ref hook) = self.context.pre_execute_hook {
                    match hook.before_execute(tool_use, &tool_context).await {
                        Ok(PreHookDecision::Reject(reason)) => {
                            tracing::warn!(
                                agent_id = %self.id,
                                tool = %tool_use.name,
                                reason = %reason,
                                "tool call rejected by pre-execute hook"
                            );
                            execution_graph.record_tool_call(
                                step_idx,
                                ToolCallRecord {
                                    tool_use_id: tool_use.id.clone(),
                                    tool_name: tool_use.name.clone(),
                                    is_error: true,
                                    executed_at: Utc::now(),
                                },
                            );
                            let rejection =
                                ToolResult::error(tool_use.id.clone(), reason);
                            self.conversation_history
                                .write()
                                .await
                                .push(Self::tool_result_message(&rejection));
                            continue;
                        }
                        Ok(PreHookDecision::Allow) => {}
                        Err(e) => {
                            tracing::error!(
                                agent_id = %self.id,
                                "pre-execute hook error: {}",
                                e
                            );
                        }
                    }
                }

                let tool_result = if let Some((path, lock_type)) =
                    Self::get_lock_requirement(tool_use)
                {
                    // ── File scope whitelist check (Item 3) ──────────────
                    if let Some(ref allowed) = self.config.allowed_files {
                        if !Self::is_file_path_allowed(&path, allowed) {
                            tracing::warn!(
                                agent_id = %self.id,
                                path = %path,
                                "file scope violation"
                            );
                            let result = ToolResult::error(
                                tool_use.id.clone(),
                                format!(
                                    "File scope violation: '{}' is outside allowed paths: {:?}",
                                    path, allowed
                                ),
                            );
                            self.conversation_history
                                .write()
                                .await
                                .push(Self::tool_result_message(&result));
                            continue;
                        }
                    }

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

                // ── Record tool call in execution graph ──────────────────────
                execution_graph.record_tool_call(
                    step_idx,
                    ToolCallRecord {
                        tool_use_id: tool_use.id.clone(),
                        tool_name: tool_use.name.clone(),
                        is_error: tool_result.is_error,
                        executed_at: Utc::now(),
                    },
                );

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

                // Sanitize + wrap external tool results before injecting into
                // conversation history (Items 1 + 2: input sanitization and
                // instruction hierarchy enforcement).
                let final_result = if EXTERNAL_CONTENT_TOOLS
                    .contains(&tool_use.name.as_str())
                    && !tool_result.is_error
                {
                    ToolResult {
                        tool_use_id: tool_result.tool_use_id.clone(),
                        content: wrap_with_content_source(
                            &tool_result.content,
                            ContentSource::ExternalContent,
                        ),
                        is_error: false,
                    }
                } else {
                    tool_result.clone()
                };
                self.conversation_history
                    .write()
                    .await
                    .push(Self::tool_result_message(&final_result));
            }

            // ── Loop detection ───────────────────────────────────────────────
            if let Some(ref ld) = self.config.loop_detection {
                if ld.enabled {
                    for tool_use in &tool_uses {
                        if recent_tool_names.len() == ld.window_size {
                            recent_tool_names.pop_front();
                        }
                        recent_tool_names.push_back(tool_use.name.clone());
                    }
                    if recent_tool_names.len() == ld.window_size
                        && recent_tool_names
                            .iter()
                            .all(|n| n == &recent_tool_names[0])
                    {
                        let stuck = recent_tool_names[0].clone();
                        let error = format!(
                            "Loop detected: '{}' called {} times consecutively. Aborting.",
                            stuck, ld.window_size
                        );
                        tracing::error!(agent_id = %self.id, %error);
                        self.conversation_history.write().await.push(Message::user(format!(
                            "SYSTEM: {error} Stop calling '{stuck}' and summarise progress."
                        )));
                        self.task.write().await.fail(&error);
                        self.set_status(TaskAgentStatus::Failed(error.clone())).await;
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
                        let _ = self.context.communication_hub.unregister_agent(&self.id).await;
                        self.context.file_lock_manager.release_all_locks(&self.id).await;
                        let run_ended_at = Utc::now();
                        let telemetry = RunTelemetry::from_graph(
                            &execution_graph,
                            run_ended_at,
                            false,
                            total_cost_usd,
                        );
                        return Ok(TaskAgentResult {
                            agent_id: self.id.clone(),
                            task_id,
                            success: false,
                            summary: error,
                            iterations,
                            replan_count: *self.replan_count.read().await,
                            budget_exhausted: false,
                            partial_output: None,
                            total_tokens_used,
                            total_cost_usd,
                            timed_out: false,
                            failure_category: Some(FailureCategory::LoopDetected),
                            execution_graph: execution_graph.clone(),
                            telemetry,
                            pre_execution_plan: pre_execution_plan.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Extract the most recent assistant text from conversation history, if any.
    async fn last_assistant_text(&self) -> Option<String> {
        self.conversation_history
            .read()
            .await
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .and_then(|m| m.text())
            .map(|t| t.to_string())
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
    use brainwires_model_tools::ToolExecutor;
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

    #[tokio::test]
    async fn test_result_has_execution_graph() {
        let task = Task::new("t-1", "Simple task");
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

        let result = agent.execute().await.unwrap();
        assert!(result.success);
        // One iteration = one step node
        assert_eq!(result.execution_graph.steps.len(), 1);
        assert_eq!(result.execution_graph.steps[0].iteration, 1);
        // prompt_hash must be non-empty
        assert!(!result.execution_graph.prompt_hash.is_empty());
        // telemetry must match
        assert_eq!(result.telemetry.total_iterations, 1);
        assert!(result.telemetry.success);
        assert_eq!(result.telemetry.prompt_hash, result.execution_graph.prompt_hash);
    }

    #[tokio::test]
    async fn test_pre_execute_hook_reject() {
        use brainwires_model_tools::{PreHookDecision, ToolPreHook};

        struct RejectAll;
        #[async_trait]
        impl ToolPreHook for RejectAll {
            async fn before_execute(
                &self,
                tool_use: &ToolUse,
                _ctx: &ToolContext,
            ) -> anyhow::Result<PreHookDecision> {
                Ok(PreHookDecision::Reject(format!(
                    "rejected: {}",
                    tool_use.name
                )))
            }
        }

        // Provider that requests a tool call on iteration 1, then stops.
        struct ToolThenStop;
        #[async_trait]
        impl Provider for ToolThenStop {
            fn name(&self) -> &str {
                "tool-then-stop"
            }
            async fn chat(
                &self,
                messages: &[Message],
                _tools: Option<&[Tool]>,
                _options: &ChatOptions,
            ) -> Result<ChatResponse> {
                // First call: return a tool use. Subsequent calls: return done.
                let has_tool_result = messages.iter().any(|m| {
                    matches!(&m.content, MessageContent::Blocks(b) if b.iter().any(|cb| matches!(cb, ContentBlock::ToolResult { .. })))
                });
                if has_tool_result {
                    return Ok(ChatResponse {
                        message: Message::assistant("done after hook rejection"),
                        finish_reason: Some("stop".to_string()),
                        usage: Usage::default(),
                    });
                }
                Ok(ChatResponse {
                    message: Message {
                        role: Role::Assistant,
                        content: MessageContent::Blocks(vec![ContentBlock::ToolUse {
                            id: "tu-1".to_string(),
                            name: "bash".to_string(),
                            input: serde_json::json!({"command": "echo hi"}),
                        }]),
                        name: None,
                        metadata: None,
                    },
                    finish_reason: None,
                    usage: Usage::default(),
                })
            }
            fn stream_chat<'a>(
                &'a self,
                _messages: &'a [Message],
                _tools: Option<&'a [Tool]>,
                _options: &'a ChatOptions,
            ) -> futures::stream::BoxStream<'a, Result<brainwires_core::StreamChunk>> {
                unimplemented!()
            }
        }

        let ctx = Arc::new(
            AgentContext::new(
                "/tmp",
                Arc::new(NoOpExecutor),
                Arc::new(CommunicationHub::new()),
                Arc::new(FileLockManager::new()),
            )
            .with_pre_execute_hook(Arc::new(RejectAll)),
        );

        let task = Task::new("t-hook", "test hook rejection");
        let agent = Arc::new(TaskAgent::new(
            "agent-hook".to_string(),
            task,
            Arc::new(ToolThenStop),
            ctx,
            TaskAgentConfig {
                validation_config: None,
                ..Default::default()
            },
        ));

        let result = agent.execute().await.unwrap();
        assert!(result.success);
        // The rejected tool call should appear in the graph as is_error=true
        let rejected: Vec<_> = result
            .execution_graph
            .steps
            .iter()
            .flat_map(|s| s.tool_calls.iter())
            .filter(|tc| tc.is_error)
            .collect();
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].tool_name, "bash");
        // And "bash" should still appear in the tool_sequence
        assert!(result.execution_graph.tool_sequence.contains(&"bash".to_string()));
    }
}
