/**
 * Database backend implementations for @brainwires/storage.
 *
 * Re-exports all available backends:
 * - PostgresDatabase (pg + pgvector) -- StorageBackend + VectorDatabase
 * - QdrantDatabase (REST API) -- VectorDatabase
 * - SurrealDatabase (SurrealDB SDK) -- StorageBackend
 * - PineconeDatabase (REST API) -- VectorDatabase
 * - WeaviateDatabase (REST + GraphQL) -- VectorDatabase
 * - MilvusDatabase (REST API v2) -- VectorDatabase
 * - MySqlDatabase (mysql2) -- StorageBackend
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

export {
  PineconeDatabase,
  // Helpers exported for testing
  buildMetadataFilter as buildPineconeFilter,
  buildUpsertBody as buildPineconeUpsertBody,
  buildQueryBody as buildPineconeQueryBody,
  parseMatch as parsePineconeMatch,
  extractFilePathsFromIds,
} from "./pinecone.ts";

export {
  WeaviateDatabase,
  // Helpers exported for testing
  buildWhereFilter as buildWeaviateWhereFilter,
  buildSearchQuery as buildWeaviateSearchQuery,
  buildAggregateQuery as buildWeaviateAggregateQuery,
  parseWeaviateResult,
  buildBatchObject as buildWeaviateBatchObject,
  deterministicUuid,
} from "./weaviate.ts";

export {
  MilvusDatabase,
  // Helpers exported for testing
  escapeFilterValue as escapeMilvusFilterValue,
  buildFilterExpr as buildMilvusFilterExpr,
  buildSearchBody as buildMilvusSearchBody,
  buildInsertBody as buildMilvusInsertBody,
  parseMilvusResult,
} from "./milvus.ts";

export {
  MySqlDatabase,
  type MySqlConfig,
  // SQL helpers exported for testing
  mapFieldType as mysqlMapFieldType,
  fieldValueToParam as mysqlFieldValueToParam,
  filterToSql as mysqlFilterToSql,
  buildCreateTable as mysqlBuildCreateTable,
  buildInsert as mysqlBuildInsert,
  buildSelect as mysqlBuildSelect,
  buildDelete as mysqlBuildDelete,
  buildCount as mysqlBuildCount,
  cosineSimilarity,
} from "./mysql.ts";
