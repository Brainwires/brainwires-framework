/**
 * Database backend implementations for @brainwires/storage.
 *
 * Re-exports all available backends:
 * - PostgresDatabase (pg + pgvector) -- StorageBackend + VectorDatabase
 * - QdrantDatabase (REST API) -- VectorDatabase
 * - SurrealDatabase (SurrealDB SDK) -- StorageBackend
 * @module
 */

export {
  PostgresDatabase,
  type PostgresConfig,
  // SQL helpers exported for testing
  filterToSql,
  buildCreateTable,
  buildInsert,
  buildSelect,
  buildDelete,
  buildCount,
  fieldValueToParam,
} from "./postgres.ts";

export {
  QdrantDatabase,
  // Helpers exported for testing
  buildQdrantFilter,
  buildUpsertBody,
  buildSearchBody,
  parseSearchPoint,
} from "./qdrant.ts";

export {
  SurrealDatabase,
  type SurrealConfig,
  // Helpers exported for testing
  fieldTypeToSurrealQL,
  fieldValueToJson,
  filterToSurrealQL,
  jsonRowToRecord,
} from "./surrealdb.ts";
