import { assertEquals, assertThrows } from "@std/assert";
import { assert } from "@std/assert/assert";

import {
  parseMetadataFromContent,
  parseSkillFromContent,
  renderTemplate,
  validateCompatibility,
  validateDescription,
  validateSkillName,
} from "./skills_parser.ts";

// --- validateSkillName ---

Deno.test("validateSkillName accepts valid names", () => {
  validateSkillName("review-pr");
  validateSkillName("commit");
  validateSkillName("explain-code-123");
  validateSkillName("a");
  validateSkillName("a-b-c");
});

Deno.test("validateSkillName rejects empty", () => {
  assertThrows(() => validateSkillName(""));
});

Deno.test("validateSkillName rejects too long", () => {
  assertThrows(() => validateSkillName("a".repeat(65)));
});

Deno.test("validateSkillName rejects uppercase", () => {
  assertThrows(() => validateSkillName("Review-PR"));
});

Deno.test("validateSkillName rejects underscore", () => {
  assertThrows(() => validateSkillName("review_pr"));
});

Deno.test("validateSkillName rejects space", () => {
  assertThrows(() => validateSkillName("review pr"));
});

Deno.test("validateSkillName rejects dot", () => {
  assertThrows(() => validateSkillName("review.pr"));
});

Deno.test("validateSkillName rejects leading hyphen", () => {
  assertThrows(() => validateSkillName("-review"));
});

Deno.test("validateSkillName rejects trailing hyphen", () => {
  assertThrows(() => validateSkillName("review-"));
});

Deno.test("validateSkillName rejects consecutive hyphens", () => {
  assertThrows(() => validateSkillName("review--pr"));
  assertThrows(() => validateSkillName("a--b--c"));
});

// --- validateDescription ---

Deno.test("validateDescription accepts valid descriptions", () => {
  validateDescription("A short description");
  validateDescription("a".repeat(1024));
});

Deno.test("validateDescription rejects empty", () => {
  assertThrows(() => validateDescription(""));
  assertThrows(() => validateDescription("   "));
});

Deno.test("validateDescription rejects too long", () => {
  assertThrows(() => validateDescription("a".repeat(1025)));
});

// --- validateCompatibility ---

Deno.test("validateCompatibility accepts valid values", () => {
  validateCompatibility("Requires git and docker");
  validateCompatibility("a".repeat(500));
});

Deno.test("validateCompatibility rejects empty", () => {
  assertThrows(() => validateCompatibility(""));
  assertThrows(() => validateCompatibility("   "));
});

Deno.test("validateCompatibility rejects too long", () => {
  assertThrows(() => validateCompatibility("a".repeat(501)));
});

// --- parseMetadataFromContent ---

Deno.test("parseMetadataFromContent parses full frontmatter", () => {
  const content = `---
name: test-skill
description: A test skill for testing
allowed-tools:
  - Read
  - Grep
license: MIT
model: claude-sonnet-4
metadata:
  category: testing
  execution: inline
---

# Test Skill Instructions

Do the test thing.`;

  const metadata = parseMetadataFromContent(content, "test.md");

  assertEquals(metadata.name, "test-skill");
  assertEquals(metadata.description, "A test skill for testing");
  assertEquals(metadata["allowed-tools"], ["Read", "Grep"]);
  assertEquals(metadata.license, "MIT");
  assertEquals(metadata.model, "claude-sonnet-4");
  assertEquals(metadata.metadata?.["category"], "testing");
});

Deno.test("parseSkillFromContent parses full skill", () => {
  const content = `---
name: review-pr
description: Reviews pull requests for quality
metadata:
  execution: subagent
---

# PR Review

When reviewing:
1. Check code quality
2. Look for bugs`;

  const skill = parseSkillFromContent(content, "review-pr.md");

  assertEquals(skill.metadata.name, "review-pr");
  assertEquals(skill.executionMode, "subagent");
  assert(skill.instructions.includes("PR Review"));
  assert(skill.instructions.includes("Check code quality"));
});

Deno.test("parseSkillFromContent parses minimal skill", () => {
  const content = `---
name: simple
description: A simple skill
---

Just do the thing.`;

  const skill = parseSkillFromContent(content, "simple.md");

  assertEquals(skill.metadata.name, "simple");
  assertEquals(skill.metadata["allowed-tools"], undefined);
  assertEquals(skill.metadata.license, undefined);
  assertEquals(skill.metadata.model, undefined);
  assertEquals(skill.executionMode, "inline");
  assertEquals(skill.instructions, "Just do the thing.");
});

Deno.test("parseSkillFromContent rejects invalid format", () => {
  assertThrows(() => parseSkillFromContent("No frontmatter here", "invalid.md"));
});

Deno.test("parseSkillFromContent rejects invalid name", () => {
  const content = `---
name: Invalid_Name
description: A skill with invalid name
---

Instructions`;

  assertThrows(() => parseSkillFromContent(content, "invalid.md"));
});

Deno.test("parseMetadataFromContent multiline description", () => {
  const content = `---
name: test
description: |
  A multiline description
  that spans multiple lines
  for better readability.
---

Instructions`;

  const metadata = parseMetadataFromContent(content, "test.md");

  assert(metadata.description.includes("multiline description"));
  assert(metadata.description.includes("spans multiple lines"));
});

Deno.test("parseMetadataFromContent with compatibility", () => {
  const content = `---
name: deploy
description: Deploys the application to production
compatibility: Requires docker, kubectl, and access to the internet
license: MIT
---

# Deploy Instructions

Run the deploy script.`;

  const metadata = parseMetadataFromContent(content, "deploy.md");

  assertEquals(metadata.name, "deploy");
  assertEquals(
    metadata.compatibility,
    "Requires docker, kubectl, and access to the internet",
  );
});

Deno.test("parseMetadataFromContent space-delimited allowed-tools", () => {
  const content = `---
name: git-helper
description: Helps with git operations
allowed-tools: Bash(git:*) Bash(jq:*) Read
---

# Git Helper

Help with git.`;

  const metadata = parseMetadataFromContent(content, "git-helper.md");

  assertEquals(metadata["allowed-tools"], [
    "Bash(git:*)",
    "Bash(jq:*)",
    "Read",
  ]);
});

Deno.test("parseMetadataFromContent YAML list allowed-tools", () => {
  const content = `---
name: reviewer
description: Reviews code
allowed-tools:
  - Read
  - Grep
---

# Reviewer

Review code.`;

  const metadata = parseMetadataFromContent(content, "reviewer.md");

  assertEquals(metadata["allowed-tools"], ["Read", "Grep"]);
});

Deno.test("parseMetadataFromContent rejects consecutive hyphens", () => {
  const content = `---
name: bad--name
description: A skill with consecutive hyphens
---

Instructions`;

  assertThrows(() => parseMetadataFromContent(content, "bad.md"));
});

// --- renderTemplate ---

Deno.test("renderTemplate simple substitution", () => {
  const template = "Hello {{name}}, you are working on {{task}}";
  const result = renderTemplate(template, {
    name: "Claude",
    task: "code review",
  });
  assertEquals(result, "Hello Claude, you are working on code review");
});

Deno.test("renderTemplate missing arg left as-is", () => {
  const template = "Hello {{name}}";
  const result = renderTemplate(template, {});
  assertEquals(result, "Hello {{name}}");
});

Deno.test("renderTemplate conditional with value", () => {
  const template = "Review{{#if pr_number}} PR #{{pr_number}}{{/if}} now";

  const result = renderTemplate(template, { pr_number: "123" });
  assertEquals(result, "Review PR #123 now");
});

Deno.test("renderTemplate conditional with empty value", () => {
  const template = "Review{{#if pr_number}} PR #{{pr_number}}{{/if}} now";

  const result = renderTemplate(template, { pr_number: "" });
  assertEquals(result, "Review now");
});
