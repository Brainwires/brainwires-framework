import { assertEquals, assertExists } from "@std/assert";
import { PromptGenerator, inferRoleAndDomain, inferTaskType } from "./generator.ts";
import { TaskClusterManager, createTaskCluster } from "./cluster.ts";

// ---------------------------------------------------------------------------
// inferRoleAndDomain
// ---------------------------------------------------------------------------

Deno.test("inferRoleAndDomain: code task -> software engineer", () => {
  const [role, domain] = inferRoleAndDomain("Implement a function", "code");
  assertEquals(role, "software engineer");
  assertEquals(domain, "software development");
});

Deno.test("inferRoleAndDomain: calculation task -> mathematician", () => {
  const [role, domain] = inferRoleAndDomain("Calculate prime numbers", "math");
  assertEquals(role, "mathematician");
  assertEquals(domain, "numerical analysis");
});

Deno.test("inferRoleAndDomain: algorithm task -> computer scientist", () => {
  const [role, domain] = inferRoleAndDomain("Optimize the algorithm", "generic");
  assertEquals(role, "computer scientist");
  assertEquals(domain, "algorithms and data structures");
});

Deno.test("inferRoleAndDomain: analysis task -> analyst", () => {
  const [role, domain] = inferRoleAndDomain("Analyze the data", "generic");
  assertEquals(role, "analyst");
  assertEquals(domain, "problem analysis");
});

Deno.test("inferRoleAndDomain: generic task with code cluster -> developer", () => {
  const [role, domain] = inferRoleAndDomain("do something", "code tasks");
  assertEquals(role, "developer");
  assertEquals(domain, "software engineering");
});

Deno.test("inferRoleAndDomain: completely generic -> expert", () => {
  const [role, domain] = inferRoleAndDomain("do something", "generic");
  assertEquals(role, "expert");
  assertEquals(domain, "problem solving");
});

// ---------------------------------------------------------------------------
// inferTaskType
// ---------------------------------------------------------------------------

Deno.test("inferTaskType: calculate -> calculation", () => {
  assertEquals(inferTaskType("Calculate the sum"), "calculation");
});

Deno.test("inferTaskType: implement -> implementation", () => {
  assertEquals(inferTaskType("Implement a class"), "implementation");
});

Deno.test("inferTaskType: analyze -> analysis", () => {
  assertEquals(inferTaskType("Analyze the code"), "analysis");
});

Deno.test("inferTaskType: fix -> debugging", () => {
  assertEquals(inferTaskType("Fix the bug"), "debugging");
});

Deno.test("inferTaskType: generic -> task", () => {
  assertEquals(inferTaskType("Do something"), "task");
});

// ---------------------------------------------------------------------------
// PromptGenerator.generatePrompt
// ---------------------------------------------------------------------------

Deno.test("generatePrompt returns undefined when no clusters", () => {
  const manager = new TaskClusterManager();
  const gen = new PromptGenerator(manager);
  const result = gen.generatePrompt("test", [0.5, 0.5, 0.5]);
  assertEquals(result, undefined);
});

Deno.test("generatePrompt produces valid result with matching cluster", () => {
  const manager = new TaskClusterManager();
  manager.addCluster(
    createTaskCluster({
      id: "test_cluster",
      description: "Code generation tasks",
      embedding: [0.5, 0.5, 0.5],
      techniques: ["RolePlaying", "EmotionPrompting", "ChainOfThought"],
      exampleTasks: ["Write a function"],
    }),
  );

  const gen = new PromptGenerator(manager);
  const result = gen.generatePrompt(
    "Write a function to sort an array",
    [0.5, 0.5, 0.5],
  );

  assertExists(result);
  assertEquals(result.clusterId, "test_cluster");
  assertEquals(result.systemPrompt.length > 0, true);
  assertEquals(result.techniques.length > 0, true);
  assertEquals(result.sealQuality, 0.5);
});

Deno.test("generatePrompt includes task description in output", () => {
  const manager = new TaskClusterManager();
  manager.addCluster(
    createTaskCluster({
      id: "c1",
      description: "Test",
      embedding: [1.0],
      techniques: ["ChainOfThought"],
      exampleTasks: [],
    }),
  );

  const gen = new PromptGenerator(manager);
  const result = gen.generatePrompt("Sort an array", [1.0]);

  assertExists(result);
  assertEquals(result.systemPrompt.includes("Sort an array"), true);
});

// ---------------------------------------------------------------------------
// selectTechniques
// ---------------------------------------------------------------------------

Deno.test("selectTechniques always includes RolePlaying when quality allows", () => {
  const manager = new TaskClusterManager();
  const gen = new PromptGenerator(manager);
  const cluster = createTaskCluster({
    id: "c1",
    description: "Test",
    embedding: [],
    techniques: ["ChainOfThought"],
    exampleTasks: [],
  });

  const techniques = gen.selectTechniques(cluster, 0.5);
  const names = techniques.map((t) => t.technique);
  assertEquals(names.includes("RolePlaying"), true);
});

Deno.test("selectTechniques includes Others only when quality > 0.6", () => {
  const manager = new TaskClusterManager();
  const gen = new PromptGenerator(manager);
  const cluster = createTaskCluster({
    id: "c1",
    description: "Test",
    embedding: [],
    techniques: ["DecomposedPrompting", "HighlightedCoT"],
    exampleTasks: [],
  });

  // Low quality: no Others
  const lowQ = gen.selectTechniques(cluster, 0.4);
  assertEquals(lowQ.filter((t) => t.category === "Others").length, 0);

  // High quality: Others included
  const highQ = gen.selectTechniques(cluster, 0.7);
  assertEquals(highQ.filter((t) => t.category === "Others").length > 0, true);
});

// ---------------------------------------------------------------------------
// composePrompt ordering
// ---------------------------------------------------------------------------

Deno.test("composePrompt orders sections: Role -> Emotion -> Reasoning -> Others -> Task", () => {
  const manager = new TaskClusterManager();
  const gen = new PromptGenerator(manager);

  const result = gen.composePrompt(
    "Test task",
    [
      { technique: "RolePlaying", category: "RoleAssignment", name: "Role Playing", description: "", template: "ROLE", bestFor: [], minSealQuality: 0, complexityLevel: "Simple", bksPromotionEligible: true },
      { technique: "EmotionPrompting", category: "EmotionalStimulus", name: "Emotion", description: "", template: "EMOTION", bestFor: [], minSealQuality: 0, complexityLevel: "Simple", bksPromotionEligible: true },
      { technique: "ChainOfThought", category: "Reasoning", name: "CoT", description: "", template: "REASONING", bestFor: [], minSealQuality: 0, complexityLevel: "Simple", bksPromotionEligible: true },
      { technique: "DecomposedPrompting", category: "Others", name: "Decomposed", description: "", template: "OTHERS", bestFor: [], minSealQuality: 0, complexityLevel: "Simple", bksPromotionEligible: true },
    ],
    "test cluster",
  );

  const roleIdx = result.indexOf("ROLE");
  const emotionIdx = result.indexOf("EMOTION");
  const reasoningIdx = result.indexOf("REASONING");
  const othersIdx = result.indexOf("OTHERS");
  const taskIdx = result.indexOf("# Task");

  assertEquals(roleIdx < emotionIdx, true);
  assertEquals(emotionIdx < reasoningIdx, true);
  assertEquals(reasoningIdx < othersIdx, true);
  assertEquals(othersIdx < taskIdx, true);
});
