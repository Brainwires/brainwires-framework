/**
 * In-memory StorageBackend implementation for testing and simple use cases.
 *
 * @module
 */

import type { StorageBackend } from "./traits.ts";
import type {
  FieldDef,
  FieldValue,
  Filter,
  Record,
  ScoredRecord,
} from "./types.ts";
import { recordGet } from "./types.ts";

/**
 * In-memory implementation of StorageBackend.
 *
 * Stores all data in Maps. Useful for testing and lightweight use cases
 * where persistence is not needed.
 */
export class InMemoryStorageBackend implements StorageBackend {
  private tables: Map<string, { schema: FieldDef[]; records: Record[] }> = new Map();

  async ensureTable(tableName: string, schema: FieldDef[]): Promise<void> {
    if (!this.tables.has(tableName)) {
      this.tables.set(tableName, { schema, records: [] });
    }
    await Promise.resolve();
  }

  async insert(tableName: string, records: Record[]): Promise<void> {
    const table = this.tables.get(tableName);
    if (!table) {
      throw new Error(`Table '${tableName}' does not exist`);
    }
    table.records.push(...records);
    await Promise.resolve();
  }

  async query(
    tableName: string,
    filter?: Filter,
    limit?: number,
  ): Promise<Record[]> {
    const table = this.tables.get(tableName);
    if (!table) {
      throw new Error(`Table '${tableName}' does not exist`);
    }

    let results = filter
      ? table.records.filter((r) => matchesFilter(r, filter))
      : [...table.records];

    if (limit !== undefined) {
      results = results.slice(0, limit);
    }

    return await Promise.resolve(results);
  }

  async delete(tableName: string, filter: Filter): Promise<void> {
    const table = this.tables.get(tableName);
    if (!table) {
      throw new Error(`Table '${tableName}' does not exist`);
    }
    table.records = table.records.filter((r) => !matchesFilter(r, filter));
    await Promise.resolve();
  }

  async count(tableName: string, filter?: Filter): Promise<number> {
    const records = await this.query(tableName, filter);
    return records.length;
  }

  async vectorSearch(
    tableName: string,
    vectorColumn: string,
    vector: number[],
    limit: number,
    filter?: Filter,
  ): Promise<ScoredRecord[]> {
    const table = this.tables.get(tableName);
    if (!table) {
      throw new Error(`Table '${tableName}' does not exist`);
    }

    let candidates = filter
      ? table.records.filter((r) => matchesFilter(r, filter))
      : [...table.records];

    // Score each candidate by cosine similarity
    const scored: ScoredRecord[] = candidates
      .map((record) => {
        const fv = recordGet(record, vectorColumn);
        if (!fv || fv.kind !== "Vector" || fv.value.length === 0) {
          return { record, score: 0 };
        }
        const score = cosineSimilarity(vector, fv.value);
        return { record, score };
      })
      .sort((a, b) => b.score - a.score)
      .slice(0, limit);

    return await Promise.resolve(scored);
  }
}

// -- Filter matching --------------------------------------------------------

function matchesFilter(record: Record, filter: Filter): boolean {
  switch (filter.kind) {
    case "Eq":
      return fieldEquals(recordGet(record, filter.field), filter.value);
    case "Ne":
      return !fieldEquals(recordGet(record, filter.field), filter.value);
    case "Lt":
      return fieldCompare(recordGet(record, filter.field), filter.value) < 0;
    case "Lte":
      return fieldCompare(recordGet(record, filter.field), filter.value) <= 0;
    case "Gt":
      return fieldCompare(recordGet(record, filter.field), filter.value) > 0;
    case "Gte":
      return fieldCompare(recordGet(record, filter.field), filter.value) >= 0;
    case "NotNull":
      return !isFieldNull(recordGet(record, filter.field));
    case "IsNull":
      return isFieldNull(recordGet(record, filter.field));
    case "In":
      return filter.values.some((v) =>
        fieldEquals(recordGet(record, filter.field), v)
      );
    case "And":
      return filter.filters.every((f) => matchesFilter(record, f));
    case "Or":
      return filter.filters.some((f) => matchesFilter(record, f));
    case "Raw":
      // Raw filters are not supported in the in-memory backend
      return true;
  }
}

function extractNumericValue(fv: FieldValue | undefined): number | null {
  if (!fv) return null;
  switch (fv.kind) {
    case "Int32":
    case "Int64":
    case "UInt32":
    case "UInt64":
    case "Float32":
    case "Float64":
      return fv.value;
    default:
      return null;
  }
}

function fieldEquals(
  a: FieldValue | undefined,
  b: FieldValue,
): boolean {
  if (!a) return false;
  if (a.kind !== b.kind) return false;

  if (a.kind === "Vector" && b.kind === "Vector") {
    return a.value.length === b.value.length &&
      a.value.every((v, i) => v === b.value[i]);
  }

  return (a as { value: unknown }).value === (b as { value: unknown }).value;
}

function fieldCompare(
  a: FieldValue | undefined,
  b: FieldValue,
): number {
  const aNum = extractNumericValue(a);
  const bNum = extractNumericValue(b);
  if (aNum !== null && bNum !== null) return aNum - bNum;

  if (a && a.kind === "Utf8" && b.kind === "Utf8") {
    const aStr = a.value ?? "";
    const bStr = b.value ?? "";
    return aStr < bStr ? -1 : aStr > bStr ? 1 : 0;
  }

  return 0;
}

function isFieldNull(fv: FieldValue | undefined): boolean {
  if (!fv) return true;
  if (fv.kind === "Vector") return fv.value.length === 0;
  return (fv as { value: unknown }).value === null;
}

// -- Vector math ------------------------------------------------------------

function cosineSimilarity(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length === 0) return 0;

  let dot = 0;
  let normA = 0;
  let normB = 0;

  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }

  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  return denom === 0 ? 0 : dot / denom;
}
