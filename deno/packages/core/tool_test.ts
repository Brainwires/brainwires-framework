import { assertEquals } from "@std/assert";
import {
  IdempotencyRegistry,
  objectSchema,
  ToolContext,
  toolModeDisplayName,
  ToolResult,
} from "./mod.ts";

Deno.test("ToolResult.success creates non-error result", () => {
  const result = ToolResult.success("tool-1", "Success!");
  assertEquals(result.is_error, false);
});

Deno.test("ToolResult.error creates error result", () => {
  const result = ToolResult.error("tool-2", "Failed!");
  assertEquals(result.is_error, true);
});

Deno.test("objectSchema creates object schema", () => {
  const schema = objectSchema({ name: { type: "string" } }, ["name"]);
  assertEquals(schema.type, "object");
  assertEquals(schema.properties !== undefined, true);
});

Deno.test("IdempotencyRegistry basic operations", () => {
  const registry = new IdempotencyRegistry();
  assertEquals(registry.isEmpty(), true);

  registry.record("key-1", "result-1");
  assertEquals(registry.length, 1);

  const record = registry.get("key-1");
  assertEquals(record?.cached_result, "result-1");
  assertEquals((record?.executed_at ?? 0) > 0, true);

  // Second record call with same key is a no-op (first result wins)
  registry.record("key-1", "result-DIFFERENT");
  assertEquals(registry.get("key-1")?.cached_result, "result-1");
  assertEquals(registry.length, 1);
});

Deno.test("ToolContext default has no registry", () => {
  const ctx = new ToolContext();
  assertEquals(ctx.idempotency_registry, undefined);
});

Deno.test("ToolContext with registry", () => {
  const ctx = new ToolContext().withIdempotencyRegistry();
  assertEquals(ctx.idempotency_registry !== undefined, true);
  assertEquals(ctx.idempotency_registry!.isEmpty(), true);
});

Deno.test("toolModeDisplayName", () => {
  assertEquals(toolModeDisplayName({ type: "full" }), "full");
  assertEquals(toolModeDisplayName({ type: "explicit", tools: ["a"] }), "explicit");
  assertEquals(toolModeDisplayName({ type: "smart" }), "smart");
});
