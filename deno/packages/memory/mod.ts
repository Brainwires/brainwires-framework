/**
 * @module @brainwires/memory
 *
 * Tiered-memory orchestration on top of `@brainwires/storage` + `@brainwires/stores`.
 * Equivalent to Rust's `brainwires-memory` crate.
 *
 * Extracted from `@brainwires/storage` in v0.11.0.
 */

export {
  computeMultiFactorScore,
  createTierMetadata,
  defaultTieredMemoryConfig,
  demoteTier,
  type FactType,
  type KeyFact,
  type MemoryAuthority,
  type MemoryTier,
  type MessageSummary,
  type MultiFactorScore,
  parseMemoryAuthority,
  promoteTier,
  recencyFromHours,
  recordAccess,
  retentionScore,
  TieredMemory,
  type TieredMemoryConfig,
  type TieredMemoryStats,
  type TieredSearchResult,
  type TierMetadata,
} from "./tiered_memory.ts";
