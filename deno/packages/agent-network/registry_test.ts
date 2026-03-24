/**
 * Tests for McpToolRegistry.
 * Equivalent to Rust registry::tests.
 */

import {
  assertEquals,
  assert,
  assertFalse,
  assertRejects,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { McpToolRegistry } from "./registry.ts";
import { RequestContext } from "./server.ts";

const echoHandler = async (
  _args: Record<string, unknown>,
  _ctx: RequestContext,
) => ({
  content: [],
  isError: false,
});

Deno.test("registry - register and list", () => {
  const registry = new McpToolRegistry();
  registry.register("echo", "Echo tool", { type: "object" }, echoHandler);

  const tools = registry.listTools();
  assertEquals(tools.length, 1);
  assertEquals(tools[0].name, "echo");
});

Deno.test("registry - hasTool", () => {
  const registry = new McpToolRegistry();
  registry.register("test", "Test tool", { type: "object" }, echoHandler);

  assert(registry.hasTool("test"));
  assertFalse(registry.hasTool("nonexistent"));
});

Deno.test("registry - dispatch calls handler", async () => {
  const registry = new McpToolRegistry();
  registry.register(
    "greet",
    "Greet tool",
    { type: "object" },
    async (args: Record<string, unknown>) => ({
      content: [{ type: "text" as const, text: `Hello ${args.name}` }],
      isError: false,
    }),
  );

  const ctx = new RequestContext(1);
  const result = await registry.dispatch("greet", { name: "world" }, ctx);
  assertEquals(result.content.length, 1);
});

Deno.test("registry - dispatch unknown tool throws", async () => {
  const registry = new McpToolRegistry();
  const ctx = new RequestContext(1);

  await assertRejects(
    () => registry.dispatch("nonexistent", {}, ctx),
    Error,
    "Tool not found",
  );
});
