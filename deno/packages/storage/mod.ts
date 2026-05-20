/**
 * @module @brainwires/storage
 *
 * Backend-agnostic persistent storage substrate for the Brainwires Agent
 * Framework. Equivalent to Rust's `brainwires-storage` crate (post-Phase-9 shape).
 *
 * Provides:
 * - `StorageBackend` and `VectorDatabase` interfaces
 * - `InMemoryStorageBackend` for testing
 * - Embedding provider wrapper
 * - Concrete database adapters: Postgres / MySQL / Qdrant / SurrealDB /
 *   Pinecone / Weaviate / Milvus
 *
 * In v0.11.0 the domain stores moved to `@brainwires/stores` and tiered
 * memory orchestration moved to `@brainwires/memory`. Both are re-exported
 * here through the 0.11.x window as a back-compat shim; remove these imports
 * by 0.12.0.
 */

// -- Core types -------------------------------------------------------------
export {
  type BackendCapabilities,
  defaultCapabilities,
  type FieldDef,
  type FieldType,
  FieldTypes,
  type FieldValue,
  fieldValueAsBool,
  fieldValueAsF32,
  fieldValueAsF64,
  fieldValueAsI32,
  fieldValueAsI64,
  fieldValueAsStr,
  fieldValueAsVector,
  FieldValues,
  type Filter,
  Filters,
  optionalField,
  type Record,
  recordGet,
  requiredField,
  type ScoredRecord,
} from "./types.ts";

// -- Traits / interfaces ----------------------------------------------------
export { type StorageBackend, type VectorDatabase } from "./traits.ts";

// -- In-memory backend ------------------------------------------------------
export { InMemoryStorageBackend } from "./memory_backend.ts";

// -- Embedding provider -----------------------------------------------------
export {
  CachedEmbeddingProvider,
  type EmbeddingProvider,
} from "./embeddings.ts";

// -- Tiered memory (moved to @brainwires/memory in v0.11.0; transitional) ----
export * from "@brainwires/memory";

// -- Database backends ------------------------------------------------------
export {
  MilvusDatabase,
  type MySqlConfig,
  MySqlDatabase,
  PineconeDatabase,
  type PostgresConfig,
  PostgresDatabase,
  QdrantDatabase,
  type SurrealConfig,
  SurrealDatabase,
  WeaviateDatabase,
} from "./backends/mod.ts";

// -- Domain stores (moved to @brainwires/stores in v0.11.0; transitional) ----
export * from "@brainwires/stores";
