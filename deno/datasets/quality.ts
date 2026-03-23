/**
 * Data quality validation, statistics, and deduplication.
 * Equivalent to Rust's `brainwires_datasets::quality` module.
 */

import type { PreferencePair, TrainingExample, TrainingRole } from "./types.ts";
import { exampleTokens, messageTokens, pairTokens } from "./types.ts";

// -- Validation types ---------------------------------------------------------

/** Validation issue severity. */
export type IssueSeverity = "error" | "warning";

/** A single validation issue found in a dataset example. */
export interface ValidationIssue {
  /** ID of the example where the issue was found. */
  exampleId: string;
  /** Severity of the issue. */
  severity: IssueSeverity;
  /** Human-readable description of the issue. */
  message: string;
  /** Optional suggestion for how to fix the issue. */
  suggestion?: string;
}

/** Result of validating a dataset. */
export interface ValidationReport {
  /** All issues found during validation. */
  issues: ValidationIssue[];
  /** Total number of examples validated. */
  totalExamples: number;
  /** Number of examples that passed without errors. */
  validExamples: number;
}

/** Check if a report has any error-level issues. */
export function reportHasErrors(report: ValidationReport): boolean {
  return report.issues.some((i) => i.severity === "error");
}

/** Count error-level issues. */
export function reportErrorCount(report: ValidationReport): number {
  return report.issues.filter((i) => i.severity === "error").length;
}

/** Count warning-level issues. */
export function reportWarningCount(report: ValidationReport): number {
  return report.issues.filter((i) => i.severity === "warning").length;
}

// -- Validator config ---------------------------------------------------------

/** Configuration for dataset validation. */
export interface ValidatorConfig {
  /** Minimum messages per example. Default 2. */
  minMessages: number;
  /** Maximum messages per example. Default 1000. */
  maxMessages: number;
  /** Maximum tokens per example (estimated). Default 32768. */
  maxTokens: number;
  /** Require the last message to be from assistant. Default true. */
  requireAssistantLast: boolean;
  /** Require a system message. Default false. */
  requireSystemMessage: boolean;
  /** Reject empty content. Default true. */
  rejectEmptyContent: boolean;
  /** Require alternating user/assistant turns after system. Default false. */
  requireAlternatingTurns: boolean;
}

/** Default validator configuration. */
export function defaultValidatorConfig(): ValidatorConfig {
  return {
    minMessages: 2,
    maxMessages: 1000,
    maxTokens: 32768,
    requireAssistantLast: true,
    requireSystemMessage: false,
    rejectEmptyContent: true,
    requireAlternatingTurns: false,
  };
}

// -- DataValidator ------------------------------------------------------------

/** Validates training examples against configurable rules. */
export class DataValidator {
  readonly config: ValidatorConfig;

  constructor(config?: Partial<ValidatorConfig>) {
    this.config = { ...defaultValidatorConfig(), ...config };
  }

  /** Validate a single training example. */
  validateExample(example: TrainingExample): ValidationIssue[] {
    const issues: ValidationIssue[] = [];
    const id = example.id;

    // Check message count
    if (example.messages.length < this.config.minMessages) {
      issues.push({
        exampleId: id,
        severity: "error",
        message:
          `Too few messages: ${example.messages.length} (min: ${this.config.minMessages})`,
      });
    }

    if (example.messages.length > this.config.maxMessages) {
      issues.push({
        exampleId: id,
        severity: "warning",
        message:
          `Too many messages: ${example.messages.length} (max: ${this.config.maxMessages})`,
      });
    }

    // Check token count
    const tokens = exampleTokens(example);
    if (tokens > this.config.maxTokens) {
      issues.push({
        exampleId: id,
        severity: "warning",
        message:
          `Estimated tokens (${tokens}) exceeds max (${this.config.maxTokens})`,
      });
    }

    // Check system message requirement
    if (
      this.config.requireSystemMessage &&
      !example.messages.some((m) => m.role === "system")
    ) {
      issues.push({
        exampleId: id,
        severity: "warning",
        message: "Missing system message",
      });
    }

    // Check last message is assistant
    if (this.config.requireAssistantLast) {
      const last = example.messages[example.messages.length - 1];
      if (!last || last.role !== "assistant") {
        issues.push({
          exampleId: id,
          severity: "error",
          message: "Last message must be from assistant",
        });
      }
    }

    // Check empty content
    if (this.config.rejectEmptyContent) {
      for (let i = 0; i < example.messages.length; i++) {
        const msg = example.messages[i];
        if (msg.content.trim() === "" && !msg.tool_calls) {
          issues.push({
            exampleId: id,
            severity: "error",
            message: `Message ${i} has empty content`,
          });
        }
      }
    }

    // Check alternating turns
    if (this.config.requireAlternatingTurns) {
      const nonSystem = example.messages.filter(
        (m) => m.role !== "system" && m.role !== "tool",
      );
      for (let i = 1; i < nonSystem.length; i++) {
        if (nonSystem[i].role === nonSystem[i - 1].role) {
          issues.push({
            exampleId: id,
            severity: "warning",
            message:
              `Consecutive ${nonSystem[i].role} messages (expected alternating)`,
          });
          break;
        }
      }
    }

    return issues;
  }

  /** Validate a preference pair. */
  validatePreference(pair: PreferencePair): ValidationIssue[] {
    const issues: ValidationIssue[] = [];
    const id = pair.id;

    if (pair.prompt.length === 0) {
      issues.push({
        exampleId: id,
        severity: "error",
        message: "Preference pair has empty prompt",
        suggestion: "Add at least one prompt message",
      });
    }

    if (pair.chosen.length === 0) {
      issues.push({
        exampleId: id,
        severity: "error",
        message: "Preference pair has empty chosen response",
        suggestion: "Add at least one chosen response message",
      });
    }

    if (pair.rejected.length === 0) {
      issues.push({
        exampleId: id,
        severity: "error",
        message: "Preference pair has empty rejected response",
        suggestion: "Add at least one rejected response message",
      });
    }

    // Check empty content
    if (this.config.rejectEmptyContent) {
      const checkMsgs = (msgs: { content: string }[], label: string) => {
        for (let i = 0; i < msgs.length; i++) {
          if (msgs[i].content.trim() === "") {
            issues.push({
              exampleId: id,
              severity: "error",
              message: `${label} message ${i} has empty content`,
            });
          }
        }
      };
      checkMsgs(pair.prompt, "Prompt");
      checkMsgs(pair.chosen, "Chosen");
      checkMsgs(pair.rejected, "Rejected");
    }

    // Warn if chosen == rejected
    if (pair.chosen.length > 0 && pair.rejected.length > 0) {
      const chosenText = pair.chosen.map((m) => m.content).join("");
      const rejectedText = pair.rejected.map((m) => m.content).join("");

      if (chosenText === rejectedText) {
        issues.push({
          exampleId: id,
          severity: "warning",
          message: "Chosen and rejected responses are identical",
          suggestion: "Ensure chosen and rejected responses differ",
        });
      }

      // Warn if length ratio > 10x
      const chosenLen = Math.max(chosenText.length, 1);
      const rejectedLen = Math.max(rejectedText.length, 1);
      const ratio = Math.max(chosenLen, rejectedLen) /
        Math.min(chosenLen, rejectedLen);
      if (ratio > 10.0) {
        issues.push({
          exampleId: id,
          severity: "warning",
          message:
            `Length ratio between chosen and rejected is ${ratio.toFixed(1)}x (>10x)`,
          suggestion:
            "Large length differences may indicate data quality issues",
        });
      }
    }

    // Token count check
    const tokens = pairTokens(pair);
    if (tokens > this.config.maxTokens) {
      issues.push({
        exampleId: id,
        severity: "warning",
        message:
          `Estimated tokens (${tokens}) exceeds max (${this.config.maxTokens})`,
      });
    }

    return issues;
  }

  /** Validate a full dataset, producing a report. */
  validateDataset(examples: TrainingExample[]): ValidationReport {
    const allIssues: ValidationIssue[] = [];
    let validCount = 0;

    for (const example of examples) {
      const issues = this.validateExample(example);
      if (issues.every((i) => i.severity !== "error")) {
        validCount++;
      }
      allIssues.push(...issues);
    }

    return {
      issues: allIssues,
      totalExamples: examples.length,
      validExamples: validCount,
    };
  }

  /** Validate a full preference dataset, producing a report. */
  validatePreferenceDataset(pairs: PreferencePair[]): ValidationReport {
    const allIssues: ValidationIssue[] = [];
    let validCount = 0;

    for (const pair of pairs) {
      const issues = this.validatePreference(pair);
      if (issues.every((i) => i.severity !== "error")) {
        validCount++;
      }
      allIssues.push(...issues);
    }

    return {
      issues: allIssues,
      totalExamples: pairs.length,
      validExamples: validCount,
    };
  }
}

// -- Statistics ---------------------------------------------------------------

/** Message counts broken down by role. */
export interface RoleCounts {
  system: number;
  user: number;
  assistant: number;
  tool: number;
}

/** Statistics about a training dataset. */
export interface DatasetStats {
  totalExamples: number;
  totalMessages: number;
  totalEstimatedTokens: number;
  avgMessagesPerExample: number;
  avgTokensPerExample: number;
  minTokens: number;
  maxTokens: number;
  examplesWithSystem: number;
  roleCounts: RoleCounts;
}

/** Compute statistics for a set of training examples. */
export function computeStats(examples: TrainingExample[]): DatasetStats {
  if (examples.length === 0) {
    return {
      totalExamples: 0,
      totalMessages: 0,
      totalEstimatedTokens: 0,
      avgMessagesPerExample: 0,
      avgTokensPerExample: 0,
      minTokens: 0,
      maxTokens: 0,
      examplesWithSystem: 0,
      roleCounts: { system: 0, user: 0, assistant: 0, tool: 0 },
    };
  }

  let totalMessages = 0;
  let totalTokens = 0;
  let minTokens = Infinity;
  let maxTokens = 0;
  let examplesWithSystem = 0;
  const roleCounts: RoleCounts = { system: 0, user: 0, assistant: 0, tool: 0 };

  for (const example of examples) {
    const tokens = exampleTokens(example);
    totalMessages += example.messages.length;
    totalTokens += tokens;
    minTokens = Math.min(minTokens, tokens);
    maxTokens = Math.max(maxTokens, tokens);

    if (example.messages.some((m) => m.role === "system")) {
      examplesWithSystem++;
    }

    for (const msg of example.messages) {
      roleCounts[msg.role]++;
    }
  }

  return {
    totalExamples: examples.length,
    totalMessages,
    totalEstimatedTokens: totalTokens,
    avgMessagesPerExample: totalMessages / examples.length,
    avgTokensPerExample: totalTokens / examples.length,
    minTokens,
    maxTokens,
    examplesWithSystem,
    roleCounts,
  };
}

// -- Deduplication ------------------------------------------------------------

/**
 * Exact deduplication by content hash (remove exact duplicates only).
 * Returns [deduplicated items, number removed].
 */
export function exactDedup(
  examples: TrainingExample[],
): [TrainingExample[], number] {
  const seen = new Set<string>();
  const deduped: TrainingExample[] = [];
  let removed = 0;

  for (const example of examples) {
    const hash = example.messages
      .map((m) => `${m.role}:${m.content}`)
      .join("|");

    if (!seen.has(hash)) {
      seen.add(hash);
      deduped.push(example);
    } else {
      removed++;
    }
  }

  return [deduped, removed];
}

/**
 * Exact deduplication for preference pairs.
 * Returns [deduplicated pairs, number removed].
 */
export function exactDedupPreferences(
  pairs: PreferencePair[],
): [PreferencePair[], number] {
  const seen = new Set<string>();
  const deduped: PreferencePair[] = [];
  let removed = 0;

  for (const pair of pairs) {
    const parts = [
      ...pair.prompt.map((m) => `${m.role}:${m.content}`),
      "||chosen:",
      ...pair.chosen.map((m) => m.content),
      "||rejected:",
      ...pair.rejected.map((m) => m.content),
    ];
    const hash = parts.join("|");

    if (!seen.has(hash)) {
      seen.add(hash);
      deduped.push(pair);
    } else {
      removed++;
    }
  }

  return [deduped, removed];
}
