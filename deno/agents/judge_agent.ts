/**
 * Judge Agent - LLM-powered cycle evaluator.
 *
 * Wraps a TaskAgent with a judge-specific system prompt to evaluate
 * the results of a Plan->Work cycle and produce a {@link JudgeVerdict}
 * that determines what happens next: complete, continue, fresh restart,
 * or abort.
 *
 * @module
 */

import type { DynamicTaskSpec } from "./planner_agent.ts";

// ---------------------------------------------------------------------------
// Verdict types
// ---------------------------------------------------------------------------

/** The judge's decision after evaluating a cycle. */
export type JudgeVerdict =
  | { verdict: "complete"; summary: string }
  | {
      verdict: "continue";
      summary: string;
      additionalTasks: DynamicTaskSpec[];
      retryTasks: string[];
      hints: string[];
    }
  | {
      verdict: "fresh_restart";
      reason: string;
      hints: string[];
      summary: string;
    }
  | { verdict: "abort"; reason: string; summary: string };

/** Get the verdict type string. */
export function verdictType(v: JudgeVerdict): string {
  return v.verdict;
}

/** Get hints from a verdict (empty array for complete/abort). */
export function verdictHints(v: JudgeVerdict): string[] {
  if (v.verdict === "continue" || v.verdict === "fresh_restart") {
    return v.hints;
  }
  return [];
}

// ---------------------------------------------------------------------------
// Merge status
// ---------------------------------------------------------------------------

/** Merge status for a worker's branch. */
export type MergeStatus =
  | { kind: "merged" }
  | { kind: "conflict_resolved" }
  | { kind: "conflict_failed"; message: string }
  | { kind: "not_attempted" };

/** Format merge status as a display string. */
export function formatMergeStatus(status: MergeStatus): string {
  switch (status.kind) {
    case "merged":
      return "merged";
    case "conflict_resolved":
      return "conflict_resolved";
    case "conflict_failed":
      return `conflict_failed: ${status.message}`;
    case "not_attempted":
      return "not_attempted";
  }
}

// ---------------------------------------------------------------------------
// Worker result
// ---------------------------------------------------------------------------

/** Result from a single worker in the cycle. */
export interface WorkerResult {
  taskId: string;
  taskDescription: string;
  success: boolean;
  summary: string;
  iterations: number;
  branchName: string;
  mergeStatus: MergeStatus;
}

// ---------------------------------------------------------------------------
// Judge context
// ---------------------------------------------------------------------------

/** Context provided to the judge for evaluation. */
export interface JudgeContext {
  originalGoal: string;
  cycleNumber: number;
  workerResults: WorkerResult[];
  plannerRationale: string;
  previousVerdicts: JudgeVerdict[];
}

// ---------------------------------------------------------------------------
// Judge agent config
// ---------------------------------------------------------------------------

/** Configuration for the judge agent. */
export interface JudgeAgentConfig {
  maxIterations: number;
  inspectFiles: boolean;
  inspectDiffs: boolean;
  temperature: number;
  maxTokens: number;
}

/** Default judge agent config. */
export function defaultJudgeAgentConfig(): JudgeAgentConfig {
  return {
    maxIterations: 15,
    inspectFiles: true,
    inspectDiffs: true,
    temperature: 0.3,
    maxTokens: 4096,
  };
}

// ---------------------------------------------------------------------------
// System prompt generation
// ---------------------------------------------------------------------------

/** Generate the system prompt for a judge agent. */
export function judgeAgentPrompt(
  agentId: string,
  workingDirectory: string,
): string {
  return `You are a judge agent (ID: ${agentId}).

Working Directory: ${workingDirectory}

# ROLE

You evaluate the results of a Plan->Work cycle. Your job is to determine whether
the original goal has been achieved, partially achieved, or failed -- and decide
what happens next.

# PROCESS

1. **Review** the original goal and planner rationale
2. **Examine** each worker's result (success/failure, summary)
3. **Inspect** files and diffs if needed to verify quality
4. **Decide** on a verdict

# OUTPUT FORMAT

You MUST output a single JSON block wrapped in \`\`\`json fences with exactly this structure:

\`\`\`json
{
  "verdict": "<complete|continue|fresh_restart|abort>",
  "summary": "<brief explanation of your assessment>",
  "additional_tasks": [
    {
      "id": "<unique-id>",
      "description": "<what still needs to be done>",
      "files_involved": ["<file paths>"],
      "depends_on": [],
      "priority": "<urgent|high|normal|low>",
      "estimated_iterations": null
    }
  ],
  "retry_tasks": ["<task_ids that should be retried>"],
  "hints": ["<guidance for the next planner cycle>"],
  "reason": "<detailed reason for fresh_restart or abort>"
}
\`\`\`

# VERDICT TYPES

- **complete**: The goal is fully achieved. All work is correct and merged.
- **continue**: Partial progress. Use \`additional_tasks\` and/or \`retry_tasks\` to specify remaining work.
- **fresh_restart**: Significant drift or tunnel vision detected. Discard current approach and re-plan.
  Include \`hints\` to guide the next planner. Include \`reason\`.
- **abort**: The goal is impossible or a fatal error occurred. Include \`reason\`.

# EVALUATION CRITERIA

1. Does the work actually accomplish the stated goal?
2. Are there any regressions or broken functionality?
3. Is the code quality acceptable (no duplicates, proper structure)?
4. Were all required files created/modified?
5. Do merge conflicts indicate coordination problems?

# AVAILABLE TOOLS

You have access to (READ-ONLY):
- list_directory: See project structure
- read_file: Read file contents
- search_code: Find code patterns
- query_codebase: Semantic search`;
}

// ---------------------------------------------------------------------------
// Task description builder
// ---------------------------------------------------------------------------

/** Build the task description that gives the judge full context. */
export function buildJudgeTaskDescription(ctx: JudgeContext): string {
  let desc = `# Evaluate Cycle ${ctx.cycleNumber} Results\n\n`;
  desc += `## Original Goal\n${ctx.originalGoal}\n\n`;
  desc += `## Planner Rationale\n${ctx.plannerRationale}\n\n`;
  desc += `## Worker Results\n\n`;

  for (let i = 0; i < ctx.workerResults.length; i++) {
    const wr = ctx.workerResults[i];
    desc += `### Worker ${i + 1} (task: ${wr.taskId})\n`;
    desc += `- **Task**: ${wr.taskDescription}\n`;
    desc += `- **Success**: ${wr.success}\n`;
    desc += `- **Summary**: ${wr.summary}\n`;
    desc += `- **Branch**: ${wr.branchName}\n`;
    desc += `- **Merge**: ${formatMergeStatus(wr.mergeStatus)}\n`;
    desc += `- **Iterations**: ${wr.iterations}\n\n`;
  }

  if (ctx.previousVerdicts.length > 0) {
    desc += `## Previous Verdicts\n\n`;
    for (let i = 0; i < ctx.previousVerdicts.length; i++) {
      desc += `- Cycle ${i}: ${verdictType(ctx.previousVerdicts[i])}\n`;
    }
    desc += "\n";
  }

  desc +=
    "## Your Task\n\n" +
    "Evaluate the above results against the original goal. " +
    "Output your verdict as a JSON block. " +
    "If you need to inspect files or diffs for verification, use the available tools first.";

  return desc;
}

// ---------------------------------------------------------------------------
// Verdict parsing
// ---------------------------------------------------------------------------

/** Extract a JSON block from text (searches for ```json fences, then raw JSON). */
export function extractJsonBlock(text: string): string | null {
  // Try ```json ... ``` fences
  const jsonFenceStart = text.indexOf("```json");
  if (jsonFenceStart !== -1) {
    const contentStart = jsonFenceStart + "```json".length;
    const end = text.indexOf("```", contentStart);
    if (end !== -1) {
      return text.slice(contentStart, end).trim();
    }
  }

  // Try ``` ... ``` fences
  const fenceStart = text.indexOf("```");
  if (fenceStart !== -1) {
    const contentStart = fenceStart + "```".length;
    const lineEnd = text.indexOf("\n", contentStart);
    if (lineEnd !== -1) {
      const actualStart = lineEnd + 1;
      const end = text.indexOf("```", actualStart);
      if (end !== -1) {
        const candidate = text.slice(actualStart, end).trim();
        if (candidate.startsWith("{")) return candidate;
      }
    }
  }

  // Try raw JSON
  const braceStart = text.indexOf("{");
  if (braceStart !== -1) {
    let depth = 0;
    let end = braceStart;
    for (let i = braceStart; i < text.length; i++) {
      if (text[i] === "{") depth++;
      else if (text[i] === "}") {
        depth--;
        if (depth === 0) {
          end = i + 1;
          break;
        }
      }
    }
    if (depth === 0 && end > braceStart) {
      return text.slice(braceStart, end);
    }
  }

  return null;
}

/** Parse a judge verdict from text output. */
export function parseVerdict(text: string): JudgeVerdict {
  const jsonStr = extractJsonBlock(text);
  if (!jsonStr) throw new Error("No JSON block found in judge output");

  const raw = JSON.parse(jsonStr);

  switch (raw.verdict) {
    case "complete":
      return { verdict: "complete", summary: raw.summary ?? "" };
    case "continue":
      return {
        verdict: "continue",
        summary: raw.summary ?? "",
        additionalTasks: raw.additional_tasks ?? [],
        retryTasks: raw.retry_tasks ?? [],
        hints: raw.hints ?? [],
      };
    case "fresh_restart":
      return {
        verdict: "fresh_restart",
        reason: raw.reason ?? "",
        hints: raw.hints ?? [],
        summary: raw.summary ?? "",
      };
    case "abort":
      return {
        verdict: "abort",
        reason: raw.reason ?? "",
        summary: raw.summary ?? "",
      };
    default:
      throw new Error(`Unknown verdict type: ${raw.verdict}`);
  }
}
