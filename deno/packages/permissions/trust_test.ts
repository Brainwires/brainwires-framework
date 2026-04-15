/**
 * Tests for trust.ts — mirrors Rust tests in trust.rs
 */
import { assertEquals, assert } from "@std/assert";
import {
  trustLevelFromScore,
  compareTrustLevels,
  createTrustFactor,
  createSystemTrustFactor,
  trustFactorRecordSuccess,
  trustFactorRecordViolation,
  defaultViolationCounts,
  recordViolation,
  violationsTotalPenalty,
  TrustManager,
} from "./trust.ts";

Deno.test("trustLevelFromScore", () => {
  assertEquals(trustLevelFromScore(0.95), "high");
  assertEquals(trustLevelFromScore(0.9), "high");
  assertEquals(trustLevelFromScore(0.85), "medium");
  assertEquals(trustLevelFromScore(0.7), "medium");
  assertEquals(trustLevelFromScore(0.5), "low");
  assertEquals(trustLevelFromScore(0.4), "low");
  assertEquals(trustLevelFromScore(0.3), "untrusted");
  assertEquals(trustLevelFromScore(0.0), "untrusted");
});

Deno.test("trust factor - success increases score", () => {
  const factor = createTrustFactor("test-agent");
  for (let i = 0; i < 10; i++) {
    trustFactorRecordSuccess(factor);
  }
  assert(factor.score > 0.5);
  assertEquals(factor.successful_ops, 10);
  assertEquals(factor.total_ops, 10);
});

Deno.test("trust factor - violations decrease score", () => {
  const factor = createTrustFactor("test-agent");
  for (let i = 0; i < 10; i++) {
    trustFactorRecordSuccess(factor);
  }
  const initialScore = factor.score;
  trustFactorRecordViolation(factor, "major");
  assert(factor.score < initialScore);
});

Deno.test("trust factor - critical violation drops score hard", () => {
  const factor = createTrustFactor("test-agent");
  trustFactorRecordViolation(factor, "critical");
  assert(factor.score < 0.4);
  assertEquals(factor.level, "untrusted");
});

Deno.test("system agent always trusted", () => {
  const factor = createSystemTrustFactor("system-agent");
  trustFactorRecordViolation(factor, "critical");
  assertEquals(factor.level, "system");
  assertEquals(factor.score, 1.0);
});

Deno.test("trust manager - basic operations", () => {
  const manager = TrustManager.inMemory();
  manager.recordSuccess("agent-1");
  manager.recordSuccess("agent-1");
  manager.recordViolation("agent-2", "minor");

  assert(compareTrustLevels(manager.getTrustLevel("agent-1"), "low") >= 0);

  const stats = manager.statistics();
  assertEquals(stats.total_agents, 2);
});

Deno.test("violation counts", () => {
  const counts = defaultViolationCounts();
  recordViolation(counts, "minor");
  recordViolation(counts, "major");
  recordViolation(counts, "critical");

  assertEquals(counts.minor, 1);
  assertEquals(counts.major, 1);
  assertEquals(counts.critical, 1);

  const penalty = violationsTotalPenalty(counts);
  assert(penalty > 0.2);
});

Deno.test("trust level ordering", () => {
  assert(compareTrustLevels("system", "high") > 0);
  assert(compareTrustLevels("high", "medium") > 0);
  assert(compareTrustLevels("medium", "low") > 0);
  assert(compareTrustLevels("low", "untrusted") > 0);
});

Deno.test("reset trust", () => {
  const manager = TrustManager.inMemory();
  for (let i = 0; i < 20; i++) {
    manager.recordSuccess("agent-1");
  }
  manager.recordViolation("agent-1", "critical");
  manager.reset("agent-1");

  const factor = manager.get("agent-1")!;
  assertEquals(factor.score, 0.5);
  assertEquals(factor.level, "low");
  assertEquals(factor.successful_ops, 0);
});
