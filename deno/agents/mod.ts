/**
 * @module @brainwires/agents
 *
 * Agent orchestration, coordination, and lifecycle management for the
 * Brainwires Agent Framework. Equivalent to Rust's `brainwires-agents` crate.
 *
 * ## Core Components
 * - **AgentRuntime / runAgentLoop** - Generic execution loop for autonomous agents
 * - **TaskAgent** - Concrete agent implementation with provider + tool loop
 * - **AgentContext** - Environment bundle (tools, hub, locks, working set)
 * - **CommunicationHub** - Inter-agent messaging bus
 * - **FileLockManager** - File access coordination with deadlock detection
 * - **TaskManager** - Hierarchical task decomposition and dependency tracking
 * - **TaskQueue** - Priority-based task scheduling
 * - **ValidationLoop** - Quality checks before agent completion
 * - **PlanExecutorAgent** - Plan execution orchestration
 *
 * ## Coordination Patterns
 * - **ContractNet** - Bidding protocol for agent negotiation
 * - **Saga** - Compensating transactions for distributed operations
 * - **OptimisticConcurrency** - Optimistic locking with conflict detection
 *
 * ## Lifecycle Hooks
 * - **AgentLifecycleHooks** - Granular control over the agent execution loop
 */

// Runtime
export {
  runAgentLoop,
  type AgentExecutionResult,
  type AgentRuntime,
} from "./runtime.ts";

// Context
export { AgentContext, type ToolPreHook } from "./context.ts";

// Task agent
export {
  defaultLoopDetectionConfig,
  defaultTaskAgentConfig,
  formatTaskAgentStatus,
  spawnTaskAgent,
  TaskAgent,
  type FailureCategory,
  type LoopDetectionConfig,
  type TaskAgentConfig,
  type TaskAgentResult,
  type TaskAgentStatus,
} from "./task_agent.ts";

// Communication
export {
  CommunicationHub,
  type AgentMessage,
  type ConflictInfo,
  type ConflictType,
  type GitOperationType,
  type MessageEnvelope,
  type OperationType,
} from "./communication.ts";

// File locks
export {
  FileLockManager,
  isLockExpired,
  lockTimeRemaining,
  type LockGuard,
  type LockInfo,
  type LockStats,
  type LockType,
} from "./file_locks.ts";

// Task manager
export {
  formatDurationSecs,
  TaskManager,
  type TaskStats,
  type TimeStats,
} from "./task_manager.ts";

// Task queue
export { TaskQueue, type QueuedTask } from "./task_queue.ts";

// Validation loop
export {
  defaultValidationConfig,
  disabledValidationConfig,
  formatValidationFeedback,
  runValidation,
  type ValidationCheck,
  type ValidationConfig,
  type ValidationIssue,
  type ValidationResult,
  type ValidationSeverity,
} from "./validation_loop.ts";

// Hooks
export {
  ConversationView,
  defaultDelegationRequest,
  type AgentLifecycleHooks,
  type DelegationRequest,
  type DelegationResult,
  type IterationContext,
  type IterationDecision,
  type ToolDecision,
} from "./hooks.ts";

// Plan executor
export {
  defaultPlanExecutionConfig,
  formatPlanExecutionStatus,
  parseExecutionApprovalMode,
  PlanExecutorAgent,
  type ExecutionApprovalMode,
  type ExecutionProgress,
  type PlanExecutionConfig,
  type PlanExecutionStatus,
} from "./plan_executor.ts";

// Coordination patterns
export {
  // Contract-net
  bidScore,
  bidScoreWeighted,
  bidTimeRemaining,
  ContractNetManager,
  ContractParticipant,
  defaultTaskRequirements,
  isBiddingOpen,
  type AwardedContract,
  type BidEvaluationStrategy,
  type ContractMessage,
  type ContractTaskStatus,
  type TaskAnnouncement,
  type TaskBid,
  type TaskRequirements,

  // Saga
  CompensationReport,
  createCheckpoint,
  failureResult,
  isCompensable,
  SagaExecutor,
  successResult,
  type Checkpoint,
  type CompensableOperation,
  type CompensationOutcome,
  type CompensationStatus,
  type FileState,
  type GitCheckpoint,
  type OperationResult,
  type SagaOperationType,
  type SagaStatus,

  // Optimistic
  commitVersion,
  isCommitSuccess,
  isTokenStale,
  OptimisticController,
  type CommitResult,
  type ConflictRecord,
  type MergeStrategy,
  type OptimisticConflict,
  type OptimisticConflictDetails,
  type OptimisticStats,
  type OptimisticToken,
  type Resolution,
  type ResolutionStrategy,
  type ResourceVersion,
} from "./coordination/mod.ts";
