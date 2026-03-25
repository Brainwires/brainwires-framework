/**
 * Tests for ToolFilterMiddleware.
 * Equivalent to Rust tool_filter::tests.
 */

import {
  assertEquals,
  assertFalse,
  assert,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ToolFilterMiddleware } from "./tool_filter.ts";

Deno.test("tool filter - allow list", () => {
  const filter = ToolFilterMiddleware.allowOnly(["agent_spawn", "agent_list"]);
  assert(filter.isAllowed("agent_spawn"));
  assert(filter.isAllowed("agent_list"));
  assertFalse(filter.isAllowed("bash"));
});

Deno.test("tool filter - deny list", () => {
  const filter = ToolFilterMiddleware.deny(["bash", "write_file"]);
  assertFalse(filter.isAllowed("bash"));
  assertFalse(filter.isAllowed("write_file"));
  assert(filter.isAllowed("agent_spawn"));
});
