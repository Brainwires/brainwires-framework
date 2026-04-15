import { assertEquals } from "@std/assert/equals";
import { assert } from "@std/assert/assert";

import {
  createSkill,
  createSkillMetadata,
  executionMode,
  explicitMatch,
  getMetadataValue,
  hasToolRestrictions,
  inlineResult,
  isResultError,
  isScript,
  isToolAllowed,
  keywordMatch,
  parseExecutionMode,
  runsAsSubagent,
  scriptResult,
  semanticMatch,
  subagentResult,
} from "./skills_metadata.ts";

Deno.test("SkillSource display values", () => {
  // SkillSource is a string literal type, so values are their own display
  assertEquals("personal" as const, "personal");
  assertEquals("project" as const, "project");
  assertEquals("builtin" as const, "builtin");
});

Deno.test("parseExecutionMode parses known modes", () => {
  assertEquals(parseExecutionMode("inline"), "inline");
  assertEquals(parseExecutionMode("subagent"), "subagent");
  assertEquals(parseExecutionMode("script"), "script");
  assertEquals(parseExecutionMode("SUBAGENT"), "subagent");
  assertEquals(parseExecutionMode("unknown"), "inline");
});

Deno.test("createSkillMetadata sets defaults", () => {
  const metadata = createSkillMetadata(
    "test-skill",
    "A test skill for unit testing",
  );

  assertEquals(metadata.name, "test-skill");
  assertEquals(metadata.description, "A test skill for unit testing");
  assertEquals(metadata["allowed-tools"], undefined);
  assertEquals(metadata.license, undefined);
  assertEquals(metadata.model, undefined);
  assertEquals(metadata.source, "personal");
});

Deno.test("SkillMetadata with source", () => {
  const metadata = createSkillMetadata("test", "desc");
  metadata.source = "project";
  assertEquals(metadata.source, "project");
});

Deno.test("SkillMetadata tool restrictions", () => {
  const metadata = createSkillMetadata("test", "desc");

  // No restrictions
  assert(!hasToolRestrictions(metadata));
  assert(isToolAllowed(metadata, "any_tool"));

  // With restrictions
  metadata["allowed-tools"] = ["Read", "Grep"];
  assert(hasToolRestrictions(metadata));
  assert(isToolAllowed(metadata, "Read"));
  assert(isToolAllowed(metadata, "Grep"));
  assert(!isToolAllowed(metadata, "Write"));
});

Deno.test("SkillMetadata execution mode from metadata map", () => {
  const metadata = createSkillMetadata("test", "desc");

  // Default (no metadata)
  assertEquals(executionMode(metadata), "inline");

  // With execution metadata
  metadata.metadata = { execution: "subagent" };
  assertEquals(executionMode(metadata), "subagent");
});

Deno.test("Skill creation", () => {
  const metadata = createSkillMetadata("review-pr", "Reviews pull requests");
  const skill = createSkill(metadata, "# Review Instructions\n\nDo the review.");

  assertEquals(skill.metadata.name, "review-pr");
  assertEquals(skill.metadata.description, "Reviews pull requests");
  assert(skill.instructions.includes("Review Instructions"));
  assertEquals(skill.executionMode, "inline");
});

Deno.test("Skill execution mode helpers", () => {
  const metadata = createSkillMetadata("test", "desc");

  const inlineSkill = createSkill(metadata, "instructions");
  assert(!runsAsSubagent(inlineSkill));
  assert(!isScript(inlineSkill));

  const subMeta = createSkillMetadata("test", "desc");
  subMeta.metadata = { execution: "subagent" };
  const subagentSkill = createSkill(subMeta, "instructions");
  assert(runsAsSubagent(subagentSkill));
  assert(!isScript(subagentSkill));

  const scriptMeta = createSkillMetadata("test", "desc");
  scriptMeta.metadata = { execution: "script" };
  const scriptSkill = createSkill(scriptMeta, "instructions");
  assert(!runsAsSubagent(scriptSkill));
  assert(isScript(scriptSkill));
});

Deno.test("SkillResult types", () => {
  const inline = inlineResult("instructions");
  assert(!isResultError(inline));

  const subagent = subagentResult("agent-123");
  assert(!isResultError(subagent));

  const scriptOk = scriptResult("output", false);
  assert(!isResultError(scriptOk));

  const scriptErr = scriptResult("error", true);
  assert(isResultError(scriptErr));
});

Deno.test("SkillMatch creation", () => {
  const semantic = semanticMatch("review-pr", 0.85);
  assertEquals(semantic.source, "semantic");
  assertEquals(semantic.confidence, 0.85);

  const keyword = keywordMatch("commit", 0.6);
  assertEquals(keyword.source, "keyword");

  const explicit = explicitMatch("explain-code");
  assertEquals(explicit.source, "explicit");
  assertEquals(explicit.confidence, 1.0);
});

Deno.test("getMetadataValue", () => {
  const metadata = createSkillMetadata("test", "desc");
  assertEquals(getMetadataValue(metadata, "category"), undefined);

  metadata.metadata = { category: "testing" };
  assertEquals(getMetadataValue(metadata, "category"), "testing");
  assertEquals(getMetadataValue(metadata, "nonexistent"), undefined);
});
