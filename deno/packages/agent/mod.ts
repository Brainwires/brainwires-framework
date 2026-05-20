/**
 * @module @brainwires/agents
 *
 * Agent coordination primitives for the Brainwires Agent Framework.
 *
 * In v0.11.0 this package was slimmed to mirror Rust's restructured
 * `brainwires-agent` crate. The LLM-driven workhorses (TaskAgent,
 * ChatAgent, JudgeAgent, PlannerAgent, ValidatorAgent, CycleOrchestrator,
 * PlanExecutorAgent, runtime, system_prompts) moved to
 * `@brainwires/inference`. MDAP / SEAL / Skills / Eval each became their
 * own package. This package now ships only:
 *
 * - **CommunicationHub** — inter-agent messaging bus
 * - **FileLockManager** — file access coordination
 * - **TaskManager / TaskQueue** — hierarchical task decomposition + scheduling
 * - **AgentPool** — pooled task agents
 * - **ExecutionGraph** — telemetry-graph for runs
 * - Coordination patterns: ContractNet, Saga, OptimisticConcurrency,
 *   MarketAllocator, ThreeStateModel, WaitQueue
 *
 * The full pre-0.11.0 surface is preserved through transitional
 * `export *` re-exports from each new package below. These re-exports
 * will be removed in 0.12.0; update imports to the focused packages.
 *
 * @deprecated for `@brainwires/agent`-style imports — renamed in v0.11.0;
 * a `0.10.2` tombstone of this package name re-exports the new one.
 */

// ── Communication ──────────────────────────────────────────────────────
export {
  type AgentMessage,
  CommunicationHub,
  type ConflictInfo,
  type ConflictType,
  type GitOperationType,
  type MessageEnvelope,
  type OperationType,
} from "./communication.ts";

// ── File locks ─────────────────────────────────────────────────────────
export {
  FileLockManager,
  isLockExpired,
  type LockGuard,
  type LockInfo,
  type LockStats,
  lockTimeRemaining,
  type LockType,
} from "./file_locks.ts";

// ── Task manager ───────────────────────────────────────────────────────
export {
  formatDurationSecs,
  TaskManager,
  type TaskStats,
  type TimeStats,
} from "./task_manager.ts";

// ── Task queue ─────────────────────────────────────────────────────────
export { type QueuedTask, TaskQueue } from "./task_queue.ts";

// ── Execution Graph ────────────────────────────────────────────────────
export {
  ExecutionGraph,
  type RunTelemetry,
  type StepNode,
  telemetryFromGraph,
  type ToolCallRecord,
} from "./execution_graph.ts";

// ── Coordination patterns ──────────────────────────────────────────────
export * from "./coordination/mod.ts";

// Confidence types live in `@brainwires/core` since v0.11.0.
export {
  type ConfidenceFactors,
  extractConfidence,
  type ResponseConfidence,
} from "@brainwires/core";

// ── Extracted packages (v0.11.0) ───────────────────────────────────────
// LLM workhorses, MDAP, SEAL, Skills, and Eval each became standalone
// packages. Import from them directly:
//
//   import { TaskAgent, runAgentLoop } from "@brainwires/inference";
//   import { FirstToAheadByKVoter }   from "@brainwires/mdap";
//   import { SealProcessor }          from "@brainwires/seal";
//   import { SkillRegistry }          from "@brainwires/skills";
//   import { RegressionSuite }        from "@brainwires/eval";
//
// `@brainwires/agents` no longer re-exports them — the type-name collisions
// across these surfaces (e.g. `MergeStrategy` exists in both
// coordination/optimistic and inference/cycle_orchestrator) prevent a clean
// wildcard re-export. The `0.10.2` tombstone of `@brainwires/agents`
// published from a release branch covers consumers pinned to `^0.10.x`.
