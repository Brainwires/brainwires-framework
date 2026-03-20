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
