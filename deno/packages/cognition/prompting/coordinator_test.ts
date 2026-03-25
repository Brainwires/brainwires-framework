import { assertEquals } from "jsr:@std/assert";
import {
  PromptingLearningCoordinator,
  createTechniqueStats,
  statsReliability,
  statsTotalUses,
  updateTechniqueStats,
  bestTechnique,
  promotableTechniques,
} from "./coordinator.ts";

// ---------------------------------------------------------------------------
// TechniqueStats
// ---------------------------------------------------------------------------

Deno.test("TechniqueStats: update tracks success and failure counts", () => {
  const stats = createTechniqueStats();

  for (let i = 0; i < 5; i++) {
    updateTechniqueStats(stats, true, 10, 0.9);
  }

  assertEquals(stats.successCount, 5);
  assertEquals(stats.failureCount, 0);
  assertEquals(statsReliability(stats), 1.0);
  assertEquals(statsTotalUses(stats), 5);
  assertEquals(stats.avgQuality > 0.7, true);
});

Deno.test("TechniqueStats: reliability is 0 when no uses", () => {
  const stats = createTechniqueStats();
  assertEquals(statsReliability(stats), 0);
});

Deno.test("TechniqueStats: mixed outcomes give partial reliability", () => {
  const stats = createTechniqueStats();
  updateTechniqueStats(stats, true, 5, 0.9);
  updateTechniqueStats(stats, false, 5, 0.3);
  assertEquals(statsReliability(stats), 0.5);
});

// ---------------------------------------------------------------------------
// shouldPromote
// ---------------------------------------------------------------------------

Deno.test("shouldPromote: true when threshold met", () => {
  const coord = new PromptingLearningCoordinator();

  for (let i = 0; i < 6; i++) {
    coord.recordOutcome("test_cluster", ["ChainOfThought"], "test task", true, 5, 0.9);
  }

  assertEquals(coord.shouldPromote("test_cluster", "ChainOfThought"), true);
});

Deno.test("shouldPromote: false when not enough uses", () => {
  const coord = new PromptingLearningCoordinator();

  for (let i = 0; i < 3; i++) {
    coord.recordOutcome("test_cluster", ["ChainOfThought"], "test task", true, 5, 0.9);
  }

  assertEquals(coord.shouldPromote("test_cluster", "ChainOfThought"), false);
});

Deno.test("shouldPromote: false when reliability too low", () => {
  const coord = new PromptingLearningCoordinator();

  // 3 successes + 3 failures = 50% (below 80% threshold)
  for (let i = 0; i < 3; i++) {
    coord.recordOutcome("test_cluster", ["ChainOfThought"], "test task", true, 5, 0.9);
  }
  for (let i = 0; i < 3; i++) {
    coord.recordOutcome("test_cluster", ["ChainOfThought"], "test task", false, 5, 0.5);
  }

  assertEquals(coord.shouldPromote("test_cluster", "ChainOfThought"), false);
});

// ---------------------------------------------------------------------------
// getClusterSummary
// ---------------------------------------------------------------------------

Deno.test("getClusterSummary aggregates multiple techniques", () => {
  const coord = new PromptingLearningCoordinator();

  coord.recordOutcome("test_cluster", ["ChainOfThought"], "task 1", true, 5, 0.9);
  coord.recordOutcome("test_cluster", ["PlanAndSolve"], "task 2", true, 8, 0.85);

  const summary = coord.getClusterSummary("test_cluster");
  assertEquals(summary.clusterId, "test_cluster");
  assertEquals(summary.totalExecutions, 2);
  assertEquals(summary.techniques.size, 2);
});

Deno.test("getClusterSummary returns empty for unknown cluster", () => {
  const coord = new PromptingLearningCoordinator();
  const summary = coord.getClusterSummary("unknown");
  assertEquals(summary.totalExecutions, 0);
  assertEquals(summary.techniques.size, 0);
});

// ---------------------------------------------------------------------------
// getPromotionCandidates
// ---------------------------------------------------------------------------

Deno.test("getPromotionCandidates returns eligible techniques", () => {
  const coord = new PromptingLearningCoordinator();

  // Technique with enough data and high reliability
  for (let i = 0; i < 6; i++) {
    coord.recordOutcome("c1", ["ChainOfThought"], "task", true, 5, 0.9);
  }
  // Technique with too few uses
  for (let i = 0; i < 2; i++) {
    coord.recordOutcome("c1", ["PlanAndSolve"], "task", true, 5, 0.9);
  }

  const candidates = coord.getPromotionCandidates();
  assertEquals(candidates.length, 1);
  assertEquals(candidates[0].technique, "ChainOfThought");
  assertEquals(candidates[0].clusterId, "c1");
});

// ---------------------------------------------------------------------------
// recordOutcome with multiple techniques
// ---------------------------------------------------------------------------

Deno.test("recordOutcome records for each technique separately", () => {
  const coord = new PromptingLearningCoordinator();

  coord.recordOutcome(
    "c1",
    ["ChainOfThought", "RolePlaying"],
    "multi-technique task",
    true,
    5,
    0.9,
  );

  const cotStats = coord.getStats("c1", "ChainOfThought");
  const rpStats = coord.getStats("c1", "RolePlaying");

  assertEquals(cotStats !== undefined, true);
  assertEquals(rpStats !== undefined, true);
  assertEquals(statsTotalUses(cotStats!), 1);
  assertEquals(statsTotalUses(rpStats!), 1);
});

// ---------------------------------------------------------------------------
// getRecentRecords
// ---------------------------------------------------------------------------

Deno.test("getRecentRecords returns most recent entries", () => {
  const coord = new PromptingLearningCoordinator();

  coord.recordOutcome("c1", ["ChainOfThought"], "task A", true, 5, 0.9);
  coord.recordOutcome("c1", ["PlanAndSolve"], "task B", true, 5, 0.8);
  coord.recordOutcome("c1", ["LeastToMost"], "task C", false, 10, 0.4);

  const recent = coord.getRecentRecords(2);
  assertEquals(recent.length, 2);
  assertEquals(recent[0].technique, "PlanAndSolve");
  assertEquals(recent[1].technique, "LeastToMost");
});

// ---------------------------------------------------------------------------
// pruneOldRecords
// ---------------------------------------------------------------------------

Deno.test("pruneOldRecords keeps only specified count", () => {
  const coord = new PromptingLearningCoordinator();

  for (let i = 0; i < 10; i++) {
    coord.recordOutcome("c1", ["ChainOfThought"], `task ${i}`, true, 5, 0.9);
  }

  coord.pruneOldRecords(3);
  assertEquals(coord.getRecentRecords(100).length, 3);
});

// ---------------------------------------------------------------------------
// ClusterSummary utilities
// ---------------------------------------------------------------------------

Deno.test("bestTechnique returns most reliable technique", () => {
  const coord = new PromptingLearningCoordinator();

  // CoT: 5 successes
  for (let i = 0; i < 5; i++) {
    coord.recordOutcome("c1", ["ChainOfThought"], "task", true, 5, 0.9);
  }
  // P&S: 3 successes, 2 failures
  for (let i = 0; i < 3; i++) {
    coord.recordOutcome("c1", ["PlanAndSolve"], "task", true, 8, 0.7);
  }
  for (let i = 0; i < 2; i++) {
    coord.recordOutcome("c1", ["PlanAndSolve"], "task", false, 10, 0.4);
  }

  const summary = coord.getClusterSummary("c1");
  const best = bestTechnique(summary);
  assertEquals(best !== undefined, true);
  assertEquals(best![0], "ChainOfThought");
});

Deno.test("promotableTechniques filters by threshold and min uses", () => {
  const coord = new PromptingLearningCoordinator();

  for (let i = 0; i < 6; i++) {
    coord.recordOutcome("c1", ["ChainOfThought"], "task", true, 5, 0.9);
  }
  for (let i = 0; i < 3; i++) {
    coord.recordOutcome("c1", ["PlanAndSolve"], "task", true, 8, 0.7);
  }

  const summary = coord.getClusterSummary("c1");
  const promotable = promotableTechniques(summary, 0.8, 5);
  assertEquals(promotable.length, 1);
  assertEquals(promotable[0], "ChainOfThought");
});

// ---------------------------------------------------------------------------
// Custom thresholds
// ---------------------------------------------------------------------------

Deno.test("custom thresholds affect promotion eligibility", () => {
  const coord = new PromptingLearningCoordinator({
    promotionThreshold: 0.5,
    minUsesForPromotion: 2,
  });

  coord.recordOutcome("c1", ["ChainOfThought"], "task", true, 5, 0.9);
  coord.recordOutcome("c1", ["ChainOfThought"], "task", true, 5, 0.9);

  assertEquals(coord.shouldPromote("c1", "ChainOfThought"), true);

  const thresholds = coord.getThresholds();
  assertEquals(thresholds.promotionThreshold, 0.5);
  assertEquals(thresholds.minUsesForPromotion, 2);
});
