/**
 * @module @brainwires/cognition
 *
 * Unified intelligence layer for the Brainwires Agent Framework.
 *
 * ## Prompting
 * - 15 prompting techniques from the adaptive selection paper
 * - Technique metadata with SEAL quality integration
 *
 * ## Knowledge
 * - BrainClient interface for persistent thought storage
 * - Entity, Relationship, and Thought types
 *
 * ## RAG
 * - RagClient interface for semantic code search
 * - Index, Query, and AdvancedSearch request/response types
 *
 * ## Code Analysis
 * - Regex-based symbol extraction (functions, classes, types, variables)
 * - Call graph generation and reference tracking
 * - Repository map formatting
 * - Supports TypeScript, JavaScript, Python, and Rust
 */

// ── Prompting ─────────────────────────────────────────────────────────────
export {
  ALL_CATEGORIES,
  ALL_COMPLEXITY_LEVELS,
  ALL_TASK_CHARACTERISTICS,
  ALL_TECHNIQUES,
  bestTechnique,
  computeCentroid,
  cosineSimilarity,
  countByComplexity,
  createTaskCluster,
  createTechniqueStats,
  createTemperaturePerformance,
  euclideanDistance,
  getAllTechniqueMetadata,
  getTechniqueMetadata,
  getTechniquesByCategory,
  getTechniquesByComplexity,
  getTechniquesBySealQuality,
  inferRoleAndDomain,
  inferTaskType,
  parseTechniqueId,
  promotableTechniques,
  PromptGenerator,
  PromptingLearningCoordinator,
  statsReliability,
  statsTotalUses,
  TaskClusterManager,
  TECHNIQUE_METADATA,
  techniqueToId,
  TemperatureOptimizer,
  temperatureScore,
  updateClusterSealMetrics,
  updateTechniqueStats,
  updateTemperaturePerformance,
} from "./prompting/mod.ts";

export type {
  ClusterSummary,
  ComplexityLevel,
  GeneratedPrompt,
  PromptingTechnique,
  TaskCharacteristic,
  TaskCluster,
  TaskClusterInit,
  TechniqueCategory,
  TechniqueEffectivenessRecord,
  TechniqueMetadata,
  TechniqueStats,
  TemperaturePerformance,
} from "./prompting/mod.ts";

// ── Knowledge ─────────────────────────────────────────────────────────────
export {
  ALL_THOUGHT_CATEGORIES,
  createThought,
  parseThoughtCategory,
  parseThoughtSource,
} from "./knowledge/mod.ts";

export type {
  BksStats,
  BrainClient,
  CaptureThoughtRequest,
  CaptureThoughtResponse,
  ContradictionEvent,
  ContradictionKind,
  DeleteThoughtRequest,
  DeleteThoughtResponse,
  Entity,
  EntityType,
  ExtractionResult,
  GetThoughtRequest,
  GetThoughtResponse,
  KnowledgeResult,
  ListRecentRequest,
  ListRecentResponse,
  MemorySearchResult,
  MemoryStatsResponse,
  PksStats,
  Relationship,
  SearchKnowledgeRequest,
  SearchKnowledgeResponse,
  SearchMemoryRequest,
  SearchMemoryResponse,
  Thought,
  ThoughtCategory,
  ThoughtSource,
  ThoughtStats,
  ThoughtSummary,
} from "./knowledge/mod.ts";

// ── RAG ───────────────────────────────────────────────────────────────────
export {
  DEFAULT_LIMIT,
  DEFAULT_MAX_FILE_SIZE,
  DEFAULT_MIN_SCORE,
} from "./rag/mod.ts";

export type {
  AdvancedSearchRequest,
  ChunkMetadata,
  ClearResponse,
  GitSearchResult,
  IndexingMode,
  IndexRequest,
  IndexResponse,
  LanguageStats,
  QueryRequest,
  QueryResponse,
  RagClient,
  SearchGitHistoryRequest,
  SearchGitHistoryResponse,
  SearchResult,
  StatisticsResponse,
} from "./rag/mod.ts";

// ── Code Analysis ────────────────────────────────────────────────────────────
export {
  buildCallGraph,
  CallGraph,
  createSymbolId,
  definitionToStorageId,
  determineReferenceKind,
  findReferences,
  referenceToStorageId,
  RepoMap,
  symbolIdToStorageId,
  symbolKindDisplayName,
  visibilityFromKeywords,
} from "./code_analysis/mod.ts";

export type {
  CallEdge,
  CallGraphNode,
  ExtractOptions,
  LanguageStats as CodeAnalysisLanguageStats,
  Reference as CodeAnalysisReference,
  ReferenceKind,
  SymbolId,
  SymbolKind,
  Visibility,
} from "./code_analysis/mod.ts";

// Re-export Definition with a qualified name to avoid ambiguity
export type { Definition as CodeAnalysisDefinition } from "./code_analysis/mod.ts";
