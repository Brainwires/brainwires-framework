/**
 * Tests for InMemoryStorageBackend CRUD and vector search.
 */

import { assertEquals, assertRejects } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { InMemoryStorageBackend } from "./memory_backend.ts";
import {
  type FieldDef,
  FieldTypes,
  FieldValues,
  Filters,
  recordGet,
  fieldValueAsStr,
  fieldValueAsI64,
  requiredField,
  optionalField,
} from "./types.ts";

function testSchema(): FieldDef[] {
  return [
    requiredField("id", FieldTypes.Utf8),
    requiredField("name", FieldTypes.Utf8),
    optionalField("score", FieldTypes.Int64),
    requiredField("vector", FieldTypes.Vector(3)),
  ];
}

Deno.test("InMemoryStorageBackend - ensureTable is idempotent", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("test", testSchema());
  await backend.ensureTable("test", testSchema()); // second call is a no-op
});

Deno.test("InMemoryStorageBackend - insert and query", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("Alice")],
      ["score", FieldValues.Int64(100)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("Bob")],
      ["score", FieldValues.Int64(200)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
  ]);

  // Query all
  const all = await backend.query("items");
  assertEquals(all.length, 2);

  // Query with filter
  const filtered = await backend.query(
    "items",
    Filters.Eq("id", FieldValues.Utf8("1")),
  );
  assertEquals(filtered.length, 1);
  assertEquals(fieldValueAsStr(recordGet(filtered[0], "name")!), "Alice");

  // Query with limit
  const limited = await backend.query("items", undefined, 1);
  assertEquals(limited.length, 1);
});

Deno.test("InMemoryStorageBackend - delete", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("Alice")],
      ["score", FieldValues.Int64(100)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("Bob")],
      ["score", FieldValues.Int64(200)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
  ]);

  await backend.delete("items", Filters.Eq("id", FieldValues.Utf8("1")));
  const remaining = await backend.query("items");
  assertEquals(remaining.length, 1);
  assertEquals(fieldValueAsStr(recordGet(remaining[0], "name")!), "Bob");
});

Deno.test("InMemoryStorageBackend - count", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("Alice")],
      ["score", FieldValues.Int64(100)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("Bob")],
      ["score", FieldValues.Int64(200)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
  ]);

  assertEquals(await backend.count("items"), 2);
  assertEquals(await backend.count("items", Filters.Eq("id", FieldValues.Utf8("1"))), 1);
});

Deno.test("InMemoryStorageBackend - vector search returns sorted by similarity", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("East")],
      ["score", FieldValues.Int64(0)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("North")],
      ["score", FieldValues.Int64(0)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
    [
      ["id", FieldValues.Utf8("3")],
      ["name", FieldValues.Utf8("NorthEast")],
      ["score", FieldValues.Int64(0)],
      ["vector", FieldValues.Vector([0.7, 0.7, 0])],
    ],
  ]);

  // Search for vector closest to [1, 0, 0]
  const results = await backend.vectorSearch("items", "vector", [1, 0, 0], 3);
  assertEquals(results.length, 3);
  // First result should be "East" (exact match)
  assertEquals(fieldValueAsStr(recordGet(results[0].record, "name")!), "East");
  assertEquals(results[0].score, 1.0); // exact cosine match
  // Scores should be descending
  for (let i = 1; i < results.length; i++) {
    assertEquals(results[i].score <= results[i - 1].score, true);
  }
});

Deno.test("InMemoryStorageBackend - vector search with filter", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("East")],
      ["score", FieldValues.Int64(100)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("North")],
      ["score", FieldValues.Int64(200)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
  ]);

  // Filter to only id=2, then search for [1,0,0] -- should return North
  const results = await backend.vectorSearch(
    "items",
    "vector",
    [1, 0, 0],
    10,
    Filters.Eq("id", FieldValues.Utf8("2")),
  );
  assertEquals(results.length, 1);
  assertEquals(fieldValueAsStr(recordGet(results[0].record, "name")!), "North");
});

Deno.test("InMemoryStorageBackend - And/Or filters", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("Alice")],
      ["score", FieldValues.Int64(100)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("Bob")],
      ["score", FieldValues.Int64(200)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
    [
      ["id", FieldValues.Utf8("3")],
      ["name", FieldValues.Utf8("Charlie")],
      ["score", FieldValues.Int64(300)],
      ["vector", FieldValues.Vector([0, 0, 1])],
    ],
  ]);

  // Or: id=1 or id=3
  const orResults = await backend.query(
    "items",
    Filters.Or([
      Filters.Eq("id", FieldValues.Utf8("1")),
      Filters.Eq("id", FieldValues.Utf8("3")),
    ]),
  );
  assertEquals(orResults.length, 2);

  // And: score > 100 and score < 300
  const andResults = await backend.query(
    "items",
    Filters.And([
      Filters.Gt("score", FieldValues.Int64(100)),
      Filters.Lt("score", FieldValues.Int64(300)),
    ]),
  );
  assertEquals(andResults.length, 1);
  assertEquals(fieldValueAsStr(recordGet(andResults[0], "name")!), "Bob");
});

Deno.test("InMemoryStorageBackend - insert to nonexistent table throws", async () => {
  const backend = new InMemoryStorageBackend();
  await assertRejects(
    () => backend.insert("missing", []),
    Error,
    "does not exist",
  );
});

Deno.test("InMemoryStorageBackend - NotNull and IsNull filters", async () => {
  const backend = new InMemoryStorageBackend();
  await backend.ensureTable("items", testSchema());

  await backend.insert("items", [
    [
      ["id", FieldValues.Utf8("1")],
      ["name", FieldValues.Utf8("Alice")],
      ["score", FieldValues.Int64(100)],
      ["vector", FieldValues.Vector([1, 0, 0])],
    ],
    [
      ["id", FieldValues.Utf8("2")],
      ["name", FieldValues.Utf8("Bob")],
      ["score", FieldValues.Int64(null)],
      ["vector", FieldValues.Vector([0, 1, 0])],
    ],
  ]);

  const notNull = await backend.query("items", Filters.NotNull("score"));
  assertEquals(notNull.length, 1);

  const isNull = await backend.query("items", Filters.IsNull("score"));
  assertEquals(isNull.length, 1);
});
