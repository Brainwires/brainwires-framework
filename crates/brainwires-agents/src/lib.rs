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

// Re-export brainwires-tool-system for ToolExecutor trait
pub use brainwires_tool_system;

// ── Agent loop hooks ─────────────────────────────────────────────────────────

pub mod agent_hooks;

// ── Agent runtime ────────────────────────────────────────────────────────────

pub mod runtime;

// ── Concrete agent implementation ────────────────────────────────────────────

pub mod context;
pub mod execution_graph;
pub mod pool;
pub mod system_prompts;
pub mod task_agent;
pub mod validator_agent;

// ── Core components ──────────────────────────────────────────────────────────

pub mod communication;
pub mod confidence;
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

// ── Git coordination ───────────────────────────────────────────────────────

pub mod git_coordination;

// ── Plan execution ─────────────────────────────────────────────────────────

pub mod plan_executor;

// ── Task orchestration ────────────────────────────────────────────────────────

pub mod task_orchestrator;

// ── Workflow graph builder ───────────────────────────────────────────────────

pub mod workflow;

// ── Reasoning (merged from brainwires-reasoning) ────────────────────────────
#[cfg(feature = "reasoning")]
pub mod reasoning;

// ── OpenTelemetry export ─────────────────────────────────────────────────────
#[cfg(feature = "otel")]
pub mod otel;

// ── Evaluation framework (merged from brainwires-eval) ──────────────────────
#[cfg(feature = "eval")]
pub mod eval;

// ── Analysis & validation ────────────────────────────────────────────────────

pub mod resource_checker;
pub mod validation_agent;
#[cfg(feature = "native")]
pub mod worktree;

// ── Re-exports ───────────────────────────────────────────────────────────────

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
pub use confidence::{
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
pub use brainwires_tool_system::{PreHookDecision, ToolPreHook};
pub use context::AgentContext;
pub use execution_graph::{ExecutionGraph, RunTelemetry, StepNode, ToolCallRecord};
pub use pool::{AgentPool, AgentPoolStats};
pub use system_prompts::{reasoning_agent_prompt, simple_agent_prompt};
pub use task_agent::{
    FailureCategory, TaskAgent, TaskAgentConfig, TaskAgentResult, TaskAgentStatus, spawn_task_agent,
};
pub use validator_agent::{
    ValidatorAgent, ValidatorAgentConfig, ValidatorAgentResult, ValidatorAgentStatus,
    spawn_validator_agent,
};

/// Prelude module for convenient imports
pub mod prelude {
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
    pub use brainwires_tool_system::{PreHookDecision, ToolPreHook};

    // Core components
    pub use super::communication::{AgentMessage, CommunicationHub, ConflictInfo, ConflictType};
    pub use super::confidence::{ConfidenceFactors, ResponseConfidence};
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
}
