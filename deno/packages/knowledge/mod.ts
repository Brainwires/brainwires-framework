/**
 * @module @brainwires/knowledge
 *
 * Knowledge layer (BrainClient + entity/relationship/thought graph + BKS/PKS)
 * for the Brainwires Agent Framework.
 *
 * In v0.11.0 prompting moved to `@brainwires/prompting`, and RAG +
 * code_analysis moved to `@brainwires/rag` (to mirror Rust's
 * `brainwires-prompting` + `brainwires-rag` extraction). Both are re-exported
 * here through the 0.11.x window as a back-compat shim; remove these imports
 * by 0.12.0.
 */

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

// ── Transitional re-exports (moved to dedicated packages in v0.11.0) ──────
export * from "@brainwires/prompting";
export * from "@brainwires/rag";
