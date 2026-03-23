/**
 * Coordination patterns for multi-agent systems.
 *
 * @module
 */

export {
  type AwardedContract,
  bidScore,
  bidScoreWeighted,
  bidTimeRemaining,
  type BidEvaluationStrategy,
  type ContractMessage,
  ContractNetManager,
  ContractParticipant,
  type ContractTaskStatus,
  defaultTaskRequirements,
  isBiddingOpen,
  type TaskAnnouncement,
  type TaskBid,
  type TaskRequirements,
} from "./contract_net.ts";

export {
  type Checkpoint,
  type CompensableOperation,
  CompensationReport,
  type CompensationOutcome,
  type CompensationStatus,
  createCheckpoint,
  failureResult,
  type FileState,
  type GitCheckpoint,
  isCompensable,
  type OperationResult,
  SagaExecutor,
  type SagaOperationType,
  type SagaStatus,
  successResult,
} from "./saga.ts";

export {
  type CommitResult,
  commitVersion,
  type ConflictRecord,
  isCommitSuccess,
  isTokenStale,
  type MergeStrategy,
  type OptimisticConflict,
  type OptimisticConflictDetails,
  OptimisticController,
  type OptimisticStats,
  type OptimisticToken,
  type Resolution,
  type ResolutionStrategy,
  type ResourceVersion,
} from "./optimistic.ts";

// Market-based allocation
export {
  type AgentBudget,
  type AllocationRecord,
  type AllocationResult,
  bidScore as marketBidScore,
  calculateUrgency,
  createBid,
  createBudget,
  type CurrentHolder,
  defaultPricingStrategy,
  defaultUrgencyContext,
  effectivePriority,
  isAllocated,
  MarketAllocator,
  type MarketStats,
  type MarketStatus,
  type PricingStrategy,
  replenishBudget,
  type ResourceBid,
  type UrgencyContext,
} from "./market.ts";

// Three-State Model
export {
  type ApplicationChange,
  ApplicationState,
  defaultGitState,
  DependencyState,
  type DependencyEdge,
  type DependencyStrength,
  type DependencyType,
  type FileStatus,
  type GitState,
  type OperationLog,
  type OperationLogStatus,
  OperationState,
  type ProposedOperation,
  type ResourceNodeType,
  type StateChange,
  type StateSnapshot,
  type StateValidationResult,
  createOperationLog,
  ThreeStateModel,
} from "./three_state.ts";

// Wait Queue
export {
  fileResourceKey,
  type QueueStatus,
  type RemovalReason,
  resourceKey,
  type WaiterInfo,
  WaitQueue,
  type WaitQueueEvent,
  type WaitQueueHandle,
} from "./wait_queue.ts";
