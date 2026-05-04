#![deny(missing_docs)]
//! Brainwires Agents - Agent orchestration, coordination, and lifecycle management
//!
//! This crate provides the multi-agent infrastructure for autonomous task execution:
//!
//! ## Core Components
//! - **CommunicationHub** - Inter-agent messaging bus with 50+ message types
//! - **FileLockManager** - File access coordination with deadlock detection
//! - **ResourceLockManager** - Scoped resource locking with heartbeat-based liveness
//! - **OperationTracker** - Operation tracking with heartbeat-based liveness checking
//! - **ValidationLoop** - Quality checks before agent completion (Bug #5 prevention)
//! - **TaskManager** - Hierarchical task decomposition and dependency tracking
//! - **TaskQueue** - Priority-based task scheduling with dependency awareness
//!
//! ## Coordination Patterns
//! - **ContractNet** - Bidding protocol for agent negotiation
//! - **Saga** - Compensating transactions for distributed operations
//! - **OptimisticConcurrency** - Optimistic locking with version-based conflict detection
//! - **WaitQueue** - Queue-based coordination primitives
//! - **MarketAllocation** - Market-based task allocation
//! - **ThreeStateModel** - State snapshots for rollback support
//!
//! ## Analysis & Validation
//! - **ResourceChecker** - Conflict detection and resolution
//! - **ValidationAgent** - Rule-based validation
//! - **Confidence** - Response confidence scoring
//! - **WorktreeManager** - Git worktree management for agent isolation
//!
//! ## Feature Flags
//! - `tools` - Enable validation tool integration (check_duplicates, verify_build, check_syntax)

// Re-export core types
pub use brainwires_core;

// Re-export the tool runtime for ToolExecutor / ToolRegistry trait surface.
pub use brainwires_tool_runtime;

// ── Chat agent (ready-to-use completion loop) ────────────────────────────────

pub mod chat_agent;

// ── Summarization (LLM-powered history compaction) ───────────────────────────

pub mod summarization;

// ── Personas (pluggable system-prompt assembly) ──────────────────────────────

pub mod personas;

// ── Agent loop hooks ─────────────────────────────────────────────────────────

pub mod agent_hooks;

// ── Agent runtime ────────────────────────────────────────────────────────────

pub mod runtime;

// ── Concrete agent implementation ────────────────────────────────────────────

pub mod context;
pub mod cycle_orchestrator;
pub mod execution_graph;
pub mod judge_agent;
pub mod planner_agent;
pub mod pool;
pub mod roles;
pub mod system_prompts;
pub mod task_agent;
pub mod validator_agent;

// ── Core components ──────────────────────────────────────────────────────────

pub mod communication;
// `confidence` moved to `brainwires-core` in Phase 11a. Existing imports of
// `brainwires_agent::ResponseConfidence` continue to work via the re-export
// below; new code should reach for `brainwires_core::confidence::*` directly.
// The shim is removed in Phase 11g.
pub use brainwires_core::confidence;
pub mod file_locks;
pub mod operation_tracker;
pub mod resource_locks;
pub mod task_manager;
pub mod task_queue;
pub mod validation_loop;

// ── Coordination patterns ────────────────────────────────────────────────────

pub mod contract_net;
pub mod market_allocation;
pub mod optimistic;
pub mod saga;
pub mod state_model;
pub mod wait_queue;

// ── Access control ─────────────────────────────────────────────────────────

pub mod access_control;

// ── Agent management (lifecycle trait + MCP tool registry) ─────────────────
//
// Moved out of brainwires-network in Phase 2 — both modules import only
// brainwires_core/serde/anyhow/async_trait, so they belong here with the
// rest of the agent-runtime surface.

/// Agent lifecycle management — `AgentManager` trait + `SpawnConfig`.
pub mod agent_manager;
/// Pre-built MCP tools for agent operations — `AgentToolRegistry`.
pub mod agent_tools;

pub use agent_manager::{AgentInfo, AgentManager, AgentResult, SpawnConfig};
pub use agent_tools::AgentToolRegistry;

// ── Git coordination ───────────────────────────────────────────────────────

pub mod git_coordination;

// ── Plan execution ─────────────────────────────────────────────────────────

pub mod plan_executor;

// ── Task orchestration ────────────────────────────────────────────────────────

pub mod task_orchestrator;

// ── Workflow graph builder ───────────────────────────────────────────────────

pub mod workflow;

// ── OpenTelemetry export ─────────────────────────────────────────────────────
#[cfg(feature = "otel")]
pub mod otel;

// ── Evaluation framework (merged from brainwires-eval) ──────────────────────
#[cfg(feature = "eval")]
pub mod eval;

// MDAP — extracted to its own brainwires-mdap crate in Phase 11b.

// ── SEAL: Self-Evolving Agentic Learning ─────────────────────────────────
#[cfg(feature = "seal")]
pub mod seal;

// Skills — extracted to its own brainwires-skills crate in Phase 11c.

// ── Analysis & validation ────────────────────────────────────────────────────

pub mod resource_checker;
pub mod validation_agent;
#[cfg(feature = "native")]
pub mod worktree;

// ── Re-exports ───────────────────────────────────────────────────────────────

// Chat agent
pub use chat_agent::ChatAgent;

// Agent loop hooks
pub use agent_hooks::{
    AgentLifecycleHooks, ConversationView, DefaultDelegationHandler, DelegationRequest,
    DelegationResult, IterationContext, IterationDecision, ToolDecision,
};

// Agent runtime
pub use runtime::{AgentExecutionResult, AgentRuntime, run_agent_loop};

// Core components
pub use communication::{
    AgentMessage, CommunicationHub, ConflictInfo, ConflictType, GitOperationType,
};
// Re-export the confidence types from core under their pre-Phase-11 paths.
pub use brainwires_core::confidence::{
    ConfidenceFactors, ResponseConfidence, extract_confidence, quick_confidence_check,
};
pub use file_locks::{FileLockManager, LockType};
pub use operation_tracker::OperationTracker;
pub use resource_checker::{ConflictCheck, ResourceChecker};
pub use resource_locks::{
    ResourceLockGuard, ResourceLockManager, ResourceScope, ResourceType as ResourceLockType,
};
pub use task_manager::{TaskManager, format_duration_secs};
pub use task_queue::TaskQueue;
pub use validation_loop::*;
#[cfg(feature = "native")]
pub use worktree::WorktreeManager;

// Access control
pub use access_control::{AccessControlManager, ContentionStrategy, LockBundle, LockPersistence};

// Git coordination
pub use git_coordination::{
    GitCoordinator, GitLockRequirements, GitOperationLocks, GitOperationRunner,
    get_lock_requirements, git_tools,
};

// Plan execution
pub use plan_executor::{
    ExecutionApprovalMode, ExecutionProgress, PlanExecutionConfig, PlanExecutionStatus,
    PlanExecutorAgent,
};

// Task orchestration
pub use task_orchestrator::{
    FailurePolicy, OrchestrationResult, TaskOrchestrator, TaskOrchestratorConfig, TaskSpec,
};

// Workflow graph builder
pub use workflow::{WorkflowBuilder, WorkflowContext, WorkflowResult};

// Coordination patterns
pub use contract_net::ContractNetManager;
pub use market_allocation::MarketAllocator;
pub use optimistic::OptimisticController;
pub use saga::SagaExecutor;
pub use state_model::{StateModelProposedOperation, StateSnapshot, ThreeStateModel};
pub use wait_queue::WaitQueue;

// Concrete agent types
pub use brainwires_tool_runtime::{PreHookDecision, ToolPreHook};
pub use context::AgentContext;
pub use execution_graph::{ExecutionGraph, RunTelemetry, StepNode, ToolCallRecord};
pub use pool::{AgentPool, AgentPoolStats};
pub use system_prompts::{
    AgentPromptKind, build_agent_prompt, judge_agent_prompt, mdap_microagent_prompt,
    planner_agent_prompt, reasoning_agent_prompt, simple_agent_prompt,
};

// SEAL re-exports
#[cfg(feature = "seal")]
pub use seal::{
    CoreferenceResolver, DialogState, LearningCoordinator as SealLearningCoordinator, QueryCore,
    QueryCoreExtractor, ReflectionModule, SealConfig, SealProcessingResult, SealProcessor,
};

// Cycle orchestration
pub use cycle_orchestrator::{
    CycleOrchestrator, CycleOrchestratorConfig, CycleOrchestratorResult, CycleRecord, MergeStrategy,
};
pub use judge_agent::{
    JudgeAgent, JudgeAgentConfig, JudgeContext, JudgeVerdict, MergeStatus, WorkerResult,
};
pub use planner_agent::{
    DynamicTaskPriority, DynamicTaskSpec, PlannerAgent, PlannerAgentConfig, PlannerOutput,
    SubPlannerRequest,
};
pub use task_agent::{
    FailureCategory, TaskAgent, TaskAgentConfig, TaskAgentResult, TaskAgentStatus, spawn_task_agent,
};
pub use validator_agent::{
    ValidatorAgent, ValidatorAgentConfig, ValidatorAgentResult, ValidatorAgentStatus,
    spawn_validator_agent,
};

/// Prelude module for convenient imports
pub mod prelude {
    // Chat agent
    pub use super::chat_agent::ChatAgent;

    // Agent loop hooks
    pub use super::agent_hooks::{
        AgentLifecycleHooks, ConversationView, DefaultDelegationHandler, DelegationRequest,
        DelegationResult, IterationContext, IterationDecision, ToolDecision,
    };

    // Concrete agent types
    pub use super::context::AgentContext;
    pub use super::execution_graph::{ExecutionGraph, RunTelemetry, StepNode, ToolCallRecord};
    pub use super::pool::{AgentPool, AgentPoolStats};
    pub use super::task_agent::{
        FailureCategory, TaskAgent, TaskAgentConfig, TaskAgentResult, TaskAgentStatus,
    };
    pub use super::validator_agent::{
        ValidatorAgent, ValidatorAgentConfig, ValidatorAgentResult, ValidatorAgentStatus,
    };
    pub use brainwires_tool_runtime::{PreHookDecision, ToolPreHook};

    // Core components
    pub use super::communication::{AgentMessage, CommunicationHub, ConflictInfo, ConflictType};
    pub use brainwires_core::confidence::{ConfidenceFactors, ResponseConfidence};
    pub use super::file_locks::{FileLockManager, LockType};
    pub use super::operation_tracker::OperationTracker;
    pub use super::resource_checker::{ConflictCheck, ResourceChecker};
    pub use super::resource_locks::{ResourceLockManager, ResourceScope};
    pub use super::state_model::{StateSnapshot, ThreeStateModel};
    pub use super::task_manager::{TaskManager, format_duration_secs};
    pub use super::task_queue::TaskQueue;
    pub use super::validation_loop::{
        ValidationCheck, ValidationConfig, ValidationIssue, ValidationResult,
    };
    #[cfg(feature = "native")]
    pub use super::worktree::WorktreeManager;

    // Access control
    pub use super::access_control::{AccessControlManager, ContentionStrategy, LockPersistence};

    // Git coordination
    pub use super::git_coordination::{GitCoordinator, git_tools};

    // Plan execution
    pub use super::plan_executor::{ExecutionApprovalMode, PlanExecutionConfig, PlanExecutorAgent};

    // Task orchestration
    pub use super::task_orchestrator::{FailurePolicy, TaskOrchestrator, TaskOrchestratorConfig};

    // Workflow graph builder
    pub use super::workflow::{WorkflowBuilder, WorkflowContext, WorkflowResult};

    // Coordination patterns
    pub use super::contract_net::ContractNetManager;
    pub use super::market_allocation::MarketAllocator;
    pub use super::optimistic::OptimisticController;
    pub use super::saga::SagaExecutor;
    pub use super::wait_queue::WaitQueue;

    // Cycle orchestration
    pub use super::cycle_orchestrator::{
        CycleOrchestrator, CycleOrchestratorConfig, CycleOrchestratorResult, MergeStrategy,
    };
    pub use super::judge_agent::{JudgeAgent, JudgeAgentConfig, JudgeVerdict, MergeStatus};
    pub use super::planner_agent::{
        DynamicTaskSpec, PlannerAgent, PlannerAgentConfig, PlannerOutput,
    };
}
