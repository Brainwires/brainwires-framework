import { assertEquals } from "@std/assert/equals";
import { assert } from "@std/assert/assert";

import { createSkillMetadata } from "./skills_metadata.ts";
import { SkillRegistry, truncateDescription } from "./skills_registry.ts";

// --- SkillRegistry ---

Deno.test("SkillRegistry new is empty", () => {
  const registry = new SkillRegistry();
  assert(registry.isEmpty);
  assertEquals(registry.length, 0);
});

Deno.test("SkillRegistry register and contains", () => {
  const registry = new SkillRegistry();
  const metadata = createSkillMetadata("test", "A test skill");

  registry.register(metadata);

  assert(registry.contains("test"));
  assertEquals(registry.length, 1);
});

Deno.test("SkillRegistry getMetadata", () => {
  const registry = new SkillRegistry();
  const metadata = createSkillMetadata("test", "A test skill");

  registry.register(metadata);

  const retrieved = registry.getMetadata("test");
  assert(retrieved !== undefined);
  assertEquals(retrieved!.name, "test");
  assertEquals(retrieved!.description, "A test skill");

  assertEquals(registry.getMetadata("nonexistent"), undefined);
});

Deno.test("SkillRegistry listSkills sorted", () => {
  const registry = new SkillRegistry();
  registry.register(createSkillMetadata("zebra", "Z skill"));
  registry.register(createSkillMetadata("alpha", "A skill"));
  registry.register(createSkillMetadata("beta", "B skill"));

  const names = registry.listSkills();
  assertEquals(names, ["alpha", "beta", "zebra"]);
});

Deno.test("SkillRegistry remove", () => {
  const registry = new SkillRegistry();
  registry.register(createSkillMetadata("test", "A test skill"));
  assert(registry.contains("test"));

  const removed = registry.remove("test");
  assert(removed !== undefined);
  assertEquals(removed!.name, "test");
  assert(!registry.contains("test"));
});

Deno.test("SkillRegistry skillsBySource", () => {
  const registry = new SkillRegistry();

  const personal = createSkillMetadata("personal-skill", "Personal");
  personal.source = "personal";
  registry.register(personal);

  const project = createSkillMetadata("project-skill", "Project");
  project.source = "project";
  registry.register(project);

  assertEquals(registry.skillsBySource("personal").length, 1);
  assertEquals(registry.skillsBySource("project").length, 1);
  assertEquals(registry.skillsBySource("builtin").length, 0);
});

Deno.test("SkillRegistry skillsByCategory", () => {
  const registry = new SkillRegistry();

  const skill1 = createSkillMetadata("skill1", "Desc");
  skill1.metadata = { category: "testing" };
  registry.register(skill1);

  const skill2 = createSkillMetadata("skill2", "Desc");
  registry.register(skill2);

  const testingSkills = registry.skillsByCategory("testing");
  assertEquals(testingSkills.length, 1);
  assertEquals(testingSkills[0].name, "skill1");
});

Deno.test("SkillRegistry discoverFrom with temp dirs", () => {
  const tempDir = Deno.makeTempDirSync();

  const content1 = `---
name: skill-a
description: First skill
---

Do the thing.`;

  const content2 = `---
name: skill-b
description: Second skill
---

Do another thing.`;

  Deno.writeTextFileSync(`${tempDir}/skill-a.md`, content1);
  Deno.writeTextFileSync(`${tempDir}/skill-b.md`, content2);

  const registry = new SkillRegistry();
  registry.discoverFrom([{ path: tempDir, source: "personal" }]);

  assertEquals(registry.length, 2);
  assert(registry.contains("skill-a"));
  assert(registry.contains("skill-b"));

  Deno.removeSync(tempDir, { recursive: true });
});

Deno.test("SkillRegistry subdirectory SKILL.md loading", () => {
  const tempDir = Deno.makeTempDirSync();
  const skillDir = `${tempDir}/my-skill`;
  Deno.mkdirSync(skillDir);

  const content = `---
name: my-skill
description: A skill in a subdirectory
---

Instructions`;

  Deno.writeTextFileSync(`${skillDir}/SKILL.md`, content);

  const registry = new SkillRegistry();
  registry.discoverFrom([{ path: tempDir, source: "project" }]);

  assert(registry.contains("my-skill"));
  assertEquals(registry.getMetadata("my-skill")!.source, "project");

  Deno.removeSync(tempDir, { recursive: true });
});

Deno.test("SkillRegistry project overrides personal", () => {
  const tempDir = Deno.makeTempDirSync();
  const personalDir = `${tempDir}/personal`;
  const projectDir = `${tempDir}/project`;
  Deno.mkdirSync(personalDir);
  Deno.mkdirSync(projectDir);

  const personalContent = `---
name: same-skill
description: Personal version
---

Instructions`;

  const projectContent = `---
name: same-skill
description: Project version
---

Instructions`;

  Deno.writeTextFileSync(`${personalDir}/same-skill.md`, personalContent);
  Deno.writeTextFileSync(`${projectDir}/same-skill.md`, projectContent);

  const registry = new SkillRegistry();
  registry.discoverFrom([
    { path: personalDir, source: "personal" },
    { path: projectDir, source: "project" },
  ]);

  const metadata = registry.getMetadata("same-skill")!;
  assertEquals(metadata.source, "project");
  assertEquals(metadata.description, "Project version");

  Deno.removeSync(tempDir, { recursive: true });
});

Deno.test("SkillRegistry lazy load getSkill", () => {
  const tempDir = Deno.makeTempDirSync();

  const content = `---
name: lazy-skill
description: A lazily loaded skill
---

# lazy-skill Instructions

Do the thing.`;

  Deno.writeTextFileSync(`${tempDir}/lazy-skill.md`, content);

  const registry = new SkillRegistry();
  registry.discoverFrom([{ path: tempDir, source: "personal" }]);

  const skill = registry.getSkill("lazy-skill");
  assertEquals(skill.metadata.name, "lazy-skill");
  assert(skill.instructions.includes("Instructions"));

  Deno.removeSync(tempDir, { recursive: true });
});

Deno.test("SkillRegistry reload picks up new skills", () => {
  const tempDir = Deno.makeTempDirSync();

  const content1 = `---
name: original
description: Original skill
---

Instructions`;

  Deno.writeTextFileSync(`${tempDir}/original.md`, content1);

  const registry = new SkillRegistry();
  registry.discoverFrom([{ path: tempDir, source: "personal" }]);
  assertEquals(registry.length, 1);

  const content2 = `---
name: new-skill
description: New skill
---

Instructions`;

  Deno.writeTextFileSync(`${tempDir}/new-skill.md`, content2);

  registry.reload();
  assertEquals(registry.length, 2);

  Deno.removeSync(tempDir, { recursive: true });
});

Deno.test("SkillRegistry formatSkillList empty", () => {
  const registry = new SkillRegistry();
  const list = registry.formatSkillList();
  assert(list.includes("No skills available"));
});

Deno.test("SkillRegistry formatSkillDetail", () => {
  const registry = new SkillRegistry();
  const metadata = createSkillMetadata("test", "A test skill");
  metadata["allowed-tools"] = ["Read"];
  metadata.model = "claude-sonnet-4";
  registry.register(metadata);

  const detail = registry.formatSkillDetail("test");
  assert(detail.includes("test"));
  assert(detail.includes("A test skill"));
  assert(detail.includes("Read"));
  assert(detail.includes("claude-sonnet-4"));
});

// --- truncateDescription ---

Deno.test("truncateDescription short text", () => {
  assertEquals(truncateDescription("Short", 10), "Short");
});

Deno.test("truncateDescription long text", () => {
  assertEquals(
    truncateDescription("This is a long description", 15),
    "This is a lo...",
  );
});

Deno.test("truncateDescription multiline takes first line", () => {
  assertEquals(
    truncateDescription("Line 1\nLine 2\nLine 3", 100),
    "Line 1",
  );
});
