import { assertEquals } from "jsr:@std/assert@1/equals";
import { assert } from "jsr:@std/assert@1/assert";

import { createSkillMetadata } from "./metadata.ts";
import { SkillRegistry } from "./registry.ts";
import { SkillRouter } from "./router.ts";

function createTestRegistry(): SkillRegistry {
  const registry = new SkillRegistry();

  const reviewMeta = createSkillMetadata(
    "review-pr",
    "Reviews pull requests for code quality, security issues, and best practices",
  );
  reviewMeta["allowed-tools"] = ["Read", "Grep"];

  const commitMeta = createSkillMetadata(
    "commit",
    "Creates well-formatted git commits following conventional commit standards",
  );

  const explainMeta = createSkillMetadata(
    "explain-code",
    "Explains code functionality in detail, breaking down complex logic",
  );

  registry.register(reviewMeta);
  registry.register(commitMeta);
  registry.register(explainMeta);

  return registry;
}

Deno.test("SkillRouter creation", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);
  assertEquals(router.minConfidence, 0.5);
});

Deno.test("SkillRouter matchSkills by name", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);

  const matches = router.matchSkills("review my pull request");
  assert(matches.length > 0);
  assert(matches.some((m) => m.skillName === "review-pr"));
});

Deno.test("SkillRouter matchSkills by description", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);

  const matches = router.matchSkills("check code quality");
  assert(matches.length > 0);
  assert(matches.some((m) => m.skillName === "review-pr"));
});

Deno.test("SkillRouter matchSkills commit skill", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);

  const matches = router.matchSkills("create a commit message");
  assert(matches.length > 0);
  assert(matches.some((m) => m.skillName === "commit"));
});

Deno.test("SkillRouter matchSkills empty query", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);

  const matches = router.matchSkills("");
  assertEquals(matches.length, 0);
});

Deno.test("SkillRouter skillExists", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);

  assert(router.skillExists("review-pr"));
  assert(router.skillExists("commit"));
  assert(!router.skillExists("nonexistent"));
});

Deno.test("SkillRouter explicitMatch", () => {
  const registry = createTestRegistry();
  const router = new SkillRouter(registry);

  const m = router.explicitMatch("review-pr");
  assertEquals(m.skillName, "review-pr");
  assertEquals(m.confidence, 1.0);
  assertEquals(m.source, "explicit");
});

Deno.test("SkillRouter formatSuggestions single", () => {
  const registry = new SkillRegistry();
  const router = new SkillRouter(registry);

  const matches = [{ skillName: "review-pr", confidence: 0.8, source: "keyword" as const }];
  const suggestion = router.formatSuggestions(matches);

  assert(suggestion !== undefined);
  assert(suggestion!.includes("/review-pr"));
});

Deno.test("SkillRouter formatSuggestions multiple", () => {
  const registry = new SkillRegistry();
  const router = new SkillRouter(registry);

  const matches = [
    { skillName: "review-pr", confidence: 0.8, source: "keyword" as const },
    { skillName: "commit", confidence: 0.7, source: "keyword" as const },
  ];
  const suggestion = router.formatSuggestions(matches);

  assert(suggestion !== undefined);
  assert(suggestion!.includes("/review-pr"));
  assert(suggestion!.includes("/commit"));
  assert(suggestion!.includes("skills")); // Plural
});

Deno.test("SkillRouter formatSuggestions empty", () => {
  const registry = new SkillRegistry();
  const router = new SkillRouter(registry);

  const suggestion = router.formatSuggestions([]);
  assertEquals(suggestion, undefined);
});
