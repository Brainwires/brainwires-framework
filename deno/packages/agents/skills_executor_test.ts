import { assertEquals } from "@std/assert/equals";
import { assert } from "@std/assert/assert";

import {
  createSkillMetadata,
  type Skill,
} from "./skills_metadata.ts";
import { SkillRegistry } from "./skills_registry.ts";
import { SkillExecutor } from "./skills_executor.ts";

function createAvailableTools(): string[] {
  return ["Read", "Write", "Grep", "git_diff"];
}

function createTestSkill(): Skill {
  const metadata = createSkillMetadata("test-skill", "A test skill");
  metadata["allowed-tools"] = ["Read", "Grep"];

  return {
    metadata,
    instructions: "Do the test with {{arg1}}",
    executionMode: "inline",
  };
}

Deno.test("SkillExecutor execute inline", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill();
  const result = executor.execute(skill, { arg1: "value1" });

  assertEquals(result.type, "inline");
  if (result.type === "inline") {
    assert(result.instructions.includes("test-skill"));
    assert(result.instructions.includes("value1"));
  }
});

Deno.test("SkillExecutor execute subagent", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill();
  skill.executionMode = "subagent";

  const result = executor.execute(skill, {});

  assertEquals(result.type, "subagent");
  if (result.type === "subagent") {
    assert(result.agentId.startsWith("skill-test-skill-"));
  }
});

Deno.test("SkillExecutor execute script", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill();
  skill.executionMode = "script";
  skill.instructions = "let x = 1; x + 1";

  const result = executor.execute(skill, {});

  assertEquals(result.type, "script");
  if (result.type === "script") {
    assert(!result.isError);
    assert(result.output.includes("let x = 1"));
  }
});

Deno.test("SkillExecutor filterAllowedTools with restrictions", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill(); // allowed: Read, Grep
  const available = createAvailableTools();

  const filtered = executor.filterAllowedTools(skill, available);

  assertEquals(filtered.length, 2);
  assert(filtered.includes("Read"));
  assert(filtered.includes("Grep"));
  assert(!filtered.includes("Write"));
  assert(!filtered.includes("git_diff"));
});

Deno.test("SkillExecutor filterAllowedTools no restrictions", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill();
  skill.metadata["allowed-tools"] = undefined;

  const available = createAvailableTools();
  const filtered = executor.filterAllowedTools(skill, available);

  assertEquals(filtered.length, 4);
});

Deno.test("SkillExecutor prepareSubagent", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill();
  skill.executionMode = "subagent";

  const available = createAvailableTools();
  const prepared = executor.prepareSubagent(skill, available, {
    arg1: "test_value",
  });

  assert(prepared.taskDescription.includes("test_value"));
  assert(prepared.systemPrompt.includes("test-skill"));
  assertEquals(prepared.allowedToolNames.length, 2);
  assert(prepared.allowedToolNames.includes("Read"));
  assert(prepared.allowedToolNames.includes("Grep"));
});

Deno.test("SkillExecutor prepareScript", () => {
  const registry = new SkillRegistry();
  const executor = new SkillExecutor(registry);

  const skill = createTestSkill();
  skill.executionMode = "script";
  skill.instructions = "let result = {{value}}; result";

  const available = createAvailableTools();
  const prepared = executor.prepareScript(skill, available, { value: "42" });

  assert(prepared.scriptContent.includes("let result = 42"));
  assertEquals(prepared.skillName, "test-skill");
  assertEquals(prepared.allowedToolNames.length, 2);
});

Deno.test("SkillExecutor getExecutionMode", () => {
  const registry = new SkillRegistry();
  const metadata = createSkillMetadata("inline-skill", "An inline skill");
  registry.register(metadata);

  const subMeta = createSkillMetadata("sub-skill", "A subagent skill");
  subMeta.metadata = { execution: "subagent" };
  registry.register(subMeta);

  const executor = new SkillExecutor(registry);

  assertEquals(executor.getExecutionMode("inline-skill"), "inline");
  assertEquals(executor.getExecutionMode("sub-skill"), "subagent");
});
