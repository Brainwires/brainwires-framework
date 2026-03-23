/**
 * Cycle Orchestrator - Plan -> Work -> Judge loop.
 *
 * Implements the Planner-Worker-Judge pattern for scaling multi-agent
 * coding tasks. Each cycle:
 *
 * 1. **Plan**: A PlannerAgent explores the codebase and creates tasks
 * 2. **Work**: Workers execute tasks independently
 * 3. **Merge**: Worker branches are merged in dependency order
 * 4. **Judge**: A JudgeAgent evaluates results and decides next steps
 *
 * This module provides the configuration types and data structures.
 * The actual orchestration loop requires a Provider and ToolExecutor
 * which are wired up at a higher level.
 *
 * @module
 */

import type { PlannerAgentConfig, PlannerOutput } from "./planner_agent.ts";
import type { JudgeAgentConfig, JudgeVerdict, WorkerResult } from "./judge_agent.ts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Strategy for merging worker branches back into the target. */
export type MergeStrategy = "sequential" | "rebase_sequential";

/** What to do when a worker fails. */
export type FailurePolicy = "continue_on_failure" | "halt_on_failure";

/** Configuration for the cycle orchestrator. */
export interface CycleOrchestratorConfig {
  /** Maximum number of Plan->Work->Judge cycles. Default: 5. */
  maxCycles: number;
  /** Maximum concurrent workers. Default: 5. */
  maxWorkers: number;
  /** Planner agent configuration. */
  plannerConfig: PlannerAgentConfig;
  /** Judge agent configuration. */
  judgeConfig: JudgeAgentConfig;
  /** Whether to automatically merge worker branches. Default: true. */
  autoMerge: boolean;
  /** Branch merge strategy. */
  mergeStrategy: MergeStrategy;
  /** What to do when a worker fails. */
  failurePolicy: FailurePolicy;
}

/** Default cycle orchestrator config. */
export function defaultCycleOrchestratorConfig(): CycleOrchestratorConfig {
  return {
    maxCycles: 5,
    maxWorkers: 5,
    plannerConfig: {
      maxIterations: 20,
      maxTasks: 15,
      maxSubPlanners: 3,
      planningDepth: 2,
      temperature: 0.7,
      maxTokens: 4096,
    },
    judgeConfig: {
      maxIterations: 15,
      inspectFiles: true,
      inspectDiffs: true,
      temperature: 0.3,
      maxTokens: 4096,
    },
    autoMerge: true,
    mergeStrategy: "sequential",
    failurePolicy: "continue_on_failure",
  };
}

/** Record of a single Plan -> Work -> Judge cycle. */
export interface CycleRecord {
  /** Cycle number (0-indexed). */
  cycleNumber: number;
  /** What the planner produced. */
  plannerOutput: PlannerOutput;
  /** Results from all workers. */
  workerResults: WorkerResult[];
  /** The judge's verdict. */
  verdict: JudgeVerdict;
  /** Wall-clock duration of the entire cycle in seconds. */
  durationSecs: number;
}

/** Final result of the orchestration. */
export interface CycleOrchestratorResult {
  /** Whether the goal was achieved. */
  success: boolean;
  /** Number of cycles used. */
  cyclesUsed: number;
  /** Total tasks completed across all cycles. */
  totalTasksCompleted: number;
  /** Total tasks failed across all cycles. */
  totalTasksFailed: number;
  /** The final verdict from the last judge. */
  finalVerdict: JudgeVerdict;
  /** History of all cycles. */
  cycleHistory: CycleRecord[];
}
