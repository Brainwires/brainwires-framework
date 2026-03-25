/**
 * MDAP - MAKER voting framework
 *
 * Multi-Dimensional Adaptive Planning system implementing the MAKER paper's
 * approach to reliable agent execution through:
 *
 * - **Voting**: First-to-ahead-by-k consensus algorithm for error correction
 * - **Microagents**: Minimal context single-step agents (m=1 decomposition)
 * - **Decomposition**: Task decomposition strategies (binary recursive, sequential)
 * - **Red Flags**: Output validation and format checking
 * - **Scaling**: Cost/probability estimation and optimization
 * - **Metrics**: Execution metrics collection and reporting
 * - **Composer**: Result composition from subtask outputs
 * - **Tool Intent**: Structured tool calling intent for stateless execution
 */

// Types
export {
  MdapError,
  type CompositionFunction,
  type CompositionHandler,
  type ConfigSummary,
  type DecomposeContext,
  type DecompositionErrorDetails,
  type DecompositionResult,
  type DecompositionStrategy,
  type EarlyStoppingConfig,
  type MdapErrorKind,
  type MdapEstimate,
  type MdapResult,
  type MicroagentConfig,
  type MicroagentProvider,
  type MicroagentResponse,
  type ModelCosts,
  type OutputFormat,
  type RedFlagConfig,
  type RedFlagErrorDetails,
  type RedFlagReason,
  type RedFlagResult,
  type ResponseMetadata,
  type SampledResponse,
  type Subtask,
  type SubtaskMetric,
  type SubtaskOutput,
  type SubtaskOutputWithIntent,
  type ToolCategory,
  type ToolIntent,
  type ToolSchema,
  type VoteResult,
  type VotingErrorDetails,
  type VotingMethod,
  type VotingRoundMetric,
} from "./types.ts";

// Planner: configs, helpers, scaling, metrics, composition, red-flag, tool intent
export {
  // Early stopping presets
  defaultEarlyStopping,
  disabledEarlyStopping,
  aggressiveEarlyStopping,
  conservativeEarlyStopping,

  // Red-flag config presets
  strictRedFlagConfig,
  relaxedRedFlagConfig,

  // Output format helpers
  outputFormatMatches,
  outputFormatDescription,

  // Red-flag validators
  StandardRedFlagValidator,
  AcceptAllValidator,

  // Confidence extraction
  extractResponseConfidence,

  // Subtask helpers
  createAtomicSubtask,
  createSubtask,
  createSubtaskOutput,

  // Microagent config
  defaultMicroagentConfig,

  // Decomposition helpers
  defaultDecomposeContext,
  childContext,
  atomicDecomposition,
  compositeDecomposition,
  validateDecomposition,
  topologicalSort,

  // Composer
  Composer,

  // Scaling laws
  calculateKMin,
  calculatePFull,
  calculateExpectedVotes,
  estimateMdap,
  estimatePerStepSuccess,
  estimateValidResponseRate,
  calculateExpectedCost,
  suggestKForBudget,
  MODEL_COSTS,
  estimateCallCost,

  // Metrics
  MdapMetrics,

  // Tool intent helpers
  toolCategoryContains,
  readOnlyCategories,
  sideEffectCategories,
  toolSchemaToPrompt,
  parseToolIntent,

  // Borda count
  bordaCountWinner,
} from "./planner.ts";

// Voter
export {
  FirstToAheadByKVoter,
  VoterBuilder,
  type RedFlagValidator,
} from "./voter.ts";
