/**
 * @module @brainwires/datasets
 *
 * Training data pipelines for the Brainwires Agent Framework.
 * Equivalent to Rust's `brainwires-datasets` crate.
 *
 * Provides JSONL I/O, format conversion, sampling, validation,
 * deduplication, and dataset statistics for fine-tuning workflows.
 */

// Core types
export type {
  DataFormat,
  PreferencePair,
  TrainingExample,
  TrainingMessage,
  TrainingRole,
} from "./types.ts";
export {
  assistantMessage,
  endsWithAssistant,
  exampleTokens,
  hasSystemMessage,
  messageTokens,
  pairTokens,
  preferencePair,
  systemMessage,
  toolMessage,
  trainingExample,
  trainingMessage,
  userMessage,
} from "./types.ts";

// JSONL I/O
export {
  JsonlReader,
  JsonlWriter,
  readJsonl,
  readJsonlFile,
  readJsonlPreferences,
  writeJsonl,
  writeJsonlFile,
  writeJsonlPreferences,
} from "./jsonl.ts";

// Format converters
export {
  AlpacaFormat,
  ChatMlFormat,
  detectFormat,
  OpenAiFormat,
  ShareGptFormat,
} from "./format.ts";
export type { FormatConverter } from "./format.ts";

// Sampling
export {
  curriculumOrder,
  defaultSplitConfig,
  sampleN,
  trainEvalSplit,
} from "./sampling.ts";
export type { SplitConfig, SplitResult } from "./sampling.ts";

// Quality & validation
export {
  computeStats,
  DataValidator,
  defaultValidatorConfig,
  exactDedup,
  exactDedupPreferences,
  reportErrorCount,
  reportHasErrors,
  reportWarningCount,
} from "./quality.ts";
export type {
  DatasetStats,
  IssueSeverity,
  RoleCounts,
  ValidationIssue,
  ValidationReport,
  ValidatorConfig,
} from "./quality.ts";
