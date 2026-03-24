/**
 * @module @brainwires/storage
 *
 * Backend-agnostic persistent storage for the Brainwires Agent Framework.
 * Equivalent to Rust's `brainwires-storage` crate.
 *
 * Provides:
 * - StorageBackend and VectorDatabase interfaces
 * - InMemoryStorageBackend for testing
 * - Domain stores: Message, Conversation, Task, Plan, Template
 * - Tiered memory hierarchy (hot/warm/cold)
 * - Embedding provider wrapper
 */

// -- Core types -------------------------------------------------------------
export {
  type FieldType,
  FieldTypes,
  type FieldDef,
  requiredField,
  optionalField,
  type FieldValue,
  FieldValues,
  fieldValueAsStr,
  fieldValueAsI64,
  fieldValueAsI32,
  fieldValueAsF32,
  fieldValueAsF64,
  fieldValueAsBool,
  fieldValueAsVector,
  type Record,
  recordGet,
  type ScoredRecord,
  type Filter,
  Filters,
  type BackendCapabilities,
  defaultCapabilities,
} from "./types.ts";

// -- Traits / interfaces ----------------------------------------------------
export {
  type StorageBackend,
  type VectorDatabase,
} from "./traits.ts";

// -- In-memory backend ------------------------------------------------------
export { InMemoryStorageBackend } from "./memory_backend.ts";

// -- Embedding provider -----------------------------------------------------
export {
  type EmbeddingProvider,
  CachedEmbeddingProvider,
} from "./embeddings.ts";

// -- Tiered memory ----------------------------------------------------------
export {
  type MemoryAuthority,
  parseMemoryAuthority,
  type MemoryTier,
  demoteTier,
  promoteTier,
  type TierMetadata,
  createTierMetadata,
  recordAccess,
  retentionScore,
  type MultiFactorScore,
  computeMultiFactorScore,
  recencyFromHours,
  type MessageSummary,
  type FactType,
  type KeyFact,
  type TieredSearchResult,
  type TieredMemoryConfig,
  defaultTieredMemoryConfig,
  type TieredMemoryStats,
  TieredMemory,
} from "./tiered_memory.ts";

// -- Database backends ------------------------------------------------------
export {
  PostgresDatabase,
  type PostgresConfig,
  QdrantDatabase,
  SurrealDatabase,
  type SurrealConfig,
  PineconeDatabase,
  WeaviateDatabase,
  MilvusDatabase,
  MySqlDatabase,
  type MySqlConfig,
} from "./backends/mod.ts";

// -- Domain stores ----------------------------------------------------------
export {
  // Message store
  type MessageMetadata,
  type MessageStoreI,
  MessageStore,
  InMemoryMessageStore,
  // Conversation store
  type ConversationMetadata,
  type ConversationStoreI,
  ConversationStore,
  InMemoryConversationStore,
  // Task store
  type TaskMetadata,
  type AgentStateMetadata,
  type TaskStoreI,
  type AgentStateStoreI,
  TaskStore,
  InMemoryTaskStore,
  AgentStateStore,
  InMemoryAgentStateStore,
  taskToMetadata,
  metadataToTask,
  // Plan store
  type PlanStoreI,
  PlanStore,
  InMemoryPlanStore,
  // Template store
  type PlanTemplate,
  TemplateStore,
  createTemplate,
  createTemplateFromPlan,
  withCategory,
  withTags,
  instantiateTemplate,
  extractVariables,
  markUsed,
} from "./stores/mod.ts";
