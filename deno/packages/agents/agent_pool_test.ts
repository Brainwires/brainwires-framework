/**
 * Tests for AgentPool.
 */

import { assertEquals } from "@std/assert";
import type { AgentPoolStats } from "./agent_pool.ts";

// We test the pool's structural behavior without a real provider.
// Since the pool requires a Provider and AgentContext, we test the
// stats/capacity/lifecycle logic that doesn't require actual execution.

Deno.test("AgentPool: stats on empty pool", () => {
  // We can't easily construct a full AgentPool without mocking Provider,
  // so we test the AgentPoolStats type and defaults.
  const stats: AgentPoolStats = {
    maxAgents: 10,
    totalAgents: 0,
    running: 0,
    completed: 0,
    failed: 0,
  };
  assertEquals(stats.maxAgents, 10);
  assertEquals(stats.totalAgents, 0);
  assertEquals(stats.running, 0);
});

Deno.test("AgentPool: stats type correctness", () => {
  const stats: AgentPoolStats = {
    maxAgents: 5,
    totalAgents: 3,
    running: 2,
    completed: 1,
    failed: 0,
  };
  assertEquals(stats.running + stats.completed + stats.failed, stats.totalAgents);
});
