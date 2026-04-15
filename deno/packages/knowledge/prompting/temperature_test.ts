import { assertEquals } from "@std/assert";
import {
  TemperatureOptimizer,
  createTemperaturePerformance,
  updateTemperaturePerformance,
  temperatureScore,
} from "./temperature.ts";
import { createTaskCluster } from "./cluster.ts";

// ---------------------------------------------------------------------------
// TemperaturePerformance
// ---------------------------------------------------------------------------

Deno.test("TemperaturePerformance: new starts at 0.5/0.5", () => {
  const perf = createTemperaturePerformance();
  assertEquals(perf.successRate, 0.5);
  assertEquals(perf.avgQuality, 0.5);
  assertEquals(perf.sampleCount, 0);
});

Deno.test("TemperaturePerformance: update increases success rate on success", () => {
  const perf = createTemperaturePerformance();
  updateTemperaturePerformance(perf, true, 0.9);
  assertEquals(perf.successRate > 0.5, true);
  assertEquals(perf.sampleCount, 1);
});

Deno.test("TemperaturePerformance: update decreases quality on failure", () => {
  const perf = createTemperaturePerformance();
  updateTemperaturePerformance(perf, true, 0.9);
  updateTemperaturePerformance(perf, false, 0.3);
  assertEquals(perf.sampleCount, 2);
  assertEquals(perf.avgQuality < 0.9, true);
});

Deno.test("TemperaturePerformance: score is 60% success + 40% quality", () => {
  const perf = createTemperaturePerformance();
  perf.successRate = 0.8;
  perf.avgQuality = 0.7;
  const score = temperatureScore(perf);
  assertEquals(Math.abs(score - 0.76) < 0.01, true);
});

// ---------------------------------------------------------------------------
// Default temperature heuristics
// ---------------------------------------------------------------------------

Deno.test("default temperature: logic task -> 0.0", () => {
  const opt = new TemperatureOptimizer();
  const cluster = createTaskCluster({
    id: "logic",
    description: "Boolean logic and reasoning puzzles",
    embedding: [0.5],
    techniques: ["LogicOfThought"],
    exampleTasks: [],
  });
  assertEquals(opt.getDefaultTemperature(cluster), 0.0);
});

Deno.test("default temperature: creative task -> 1.3", () => {
  const opt = new TemperatureOptimizer();
  const cluster = createTaskCluster({
    id: "creative",
    description: "Creative writing and story generation",
    embedding: [0.5],
    techniques: ["RolePlaying"],
    exampleTasks: [],
  });
  assertEquals(opt.getDefaultTemperature(cluster), 1.3);
});

Deno.test("default temperature: numerical task -> 0.2", () => {
  const opt = new TemperatureOptimizer();
  const cluster = createTaskCluster({
    id: "numerical",
    description: "Numerical calculation and math",
    embedding: [0.5],
    techniques: ["ScratchpadPrompting"],
    exampleTasks: [],
  });
  assertEquals(opt.getDefaultTemperature(cluster), 0.2);
});

Deno.test("default temperature: code task -> 0.6", () => {
  const opt = new TemperatureOptimizer();
  const cluster = createTaskCluster({
    id: "code",
    description: "Code implementation and algorithm design",
    embedding: [0.5],
    techniques: ["PlanAndSolve"],
    exampleTasks: [],
  });
  assertEquals(opt.getDefaultTemperature(cluster), 0.6);
});

Deno.test("default temperature: generic task -> 0.7", () => {
  const opt = new TemperatureOptimizer();
  const cluster = createTaskCluster({
    id: "generic",
    description: "General tasks",
    embedding: [0.5],
    techniques: [],
    exampleTasks: [],
  });
  assertEquals(opt.getDefaultTemperature(cluster), 0.7);
});

// ---------------------------------------------------------------------------
// Record and retrieve local optimal
// ---------------------------------------------------------------------------

Deno.test("local optimal after enough samples", () => {
  const opt = new TemperatureOptimizer();

  // Record 10 outcomes for temperature 0.0 (high success)
  for (let i = 0; i < 10; i++) {
    opt.recordOutcome("test_cluster", 0.0, true, 0.9);
  }
  // Record 10 outcomes for temperature 0.6 (low success)
  for (let i = 0; i < 10; i++) {
    opt.recordOutcome("test_cluster", 0.6, false, 0.5);
  }

  assertEquals(opt.getLocalOptimal("test_cluster"), 0.0);
});

Deno.test("min samples requirement: not enough data returns undefined", () => {
  const opt = new TemperatureOptimizer({ minSamples: 5 });

  for (let i = 0; i < 3; i++) {
    opt.recordOutcome("test_cluster", 0.0, true, 0.95);
  }
  assertEquals(opt.getLocalOptimal("test_cluster"), undefined);

  // Add 2 more
  for (let i = 0; i < 2; i++) {
    opt.recordOutcome("test_cluster", 0.0, true, 0.95);
  }
  assertEquals(opt.getLocalOptimal("test_cluster"), 0.0);
});

// ---------------------------------------------------------------------------
// getOptimalTemperature fallback
// ---------------------------------------------------------------------------

Deno.test("getOptimalTemperature falls back to heuristic", () => {
  const opt = new TemperatureOptimizer();
  const cluster = createTaskCluster({
    id: "logic_test",
    description: "Boolean logic problems",
    embedding: [0.5],
    techniques: ["LogicOfThought"],
    exampleTasks: [],
  });

  assertEquals(opt.getOptimalTemperature(cluster), 0.0);
});

Deno.test("getOptimalTemperature uses learned data when available", () => {
  const opt = new TemperatureOptimizer({ minSamples: 3 });
  const cluster = createTaskCluster({
    id: "my_cluster",
    description: "Boolean logic problems",
    embedding: [0.5],
    techniques: [],
    exampleTasks: [],
  });

  // Without data: heuristic (0.0 for logic)
  assertEquals(opt.getOptimalTemperature(cluster), 0.0);

  // Record data showing 0.8 is best
  for (let i = 0; i < 5; i++) {
    opt.recordOutcome("my_cluster", 0.8, true, 0.95);
  }
  for (let i = 0; i < 5; i++) {
    opt.recordOutcome("my_cluster", 0.0, false, 0.3);
  }

  // Now should use learned optimal (0.8)
  assertEquals(opt.getOptimalTemperature(cluster), 0.8);
});

// ---------------------------------------------------------------------------
// getPerformance
// ---------------------------------------------------------------------------

Deno.test("getPerformance returns undefined for unknown", () => {
  const opt = new TemperatureOptimizer();
  assertEquals(opt.getPerformance("unknown", 0.5), undefined);
});

Deno.test("getPerformance returns data after recording", () => {
  const opt = new TemperatureOptimizer();
  opt.recordOutcome("c1", 0.4, true, 0.8);
  const perf = opt.getPerformance("c1", 0.4);
  assertEquals(perf !== undefined, true);
  assertEquals(perf!.sampleCount, 1);
});
