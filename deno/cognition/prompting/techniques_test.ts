import { assertEquals, assertExists } from "jsr:@std/assert";
import {
  ALL_CATEGORIES,
  ALL_COMPLEXITY_LEVELS,
  ALL_TASK_CHARACTERISTICS,
  ALL_TECHNIQUES,
  countByComplexity,
  getAllTechniqueMetadata,
  getTechniqueMetadata,
  getTechniquesByCategory,
  getTechniquesByComplexity,
  getTechniquesBySealQuality,
  parseTechniqueId,
  TECHNIQUE_METADATA,
  techniqueToId,
} from "./techniques.ts";
import type {
  ComplexityLevel,
  PromptingTechnique,
  TechniqueCategory,
} from "./techniques.ts";

// ---------------------------------------------------------------------------
// Enum completeness
// ---------------------------------------------------------------------------

Deno.test("ALL_TECHNIQUES contains exactly 15 techniques", () => {
  assertEquals(ALL_TECHNIQUES.length, 15);
});

Deno.test("ALL_CATEGORIES contains 4 categories", () => {
  assertEquals(ALL_CATEGORIES.length, 4);
});

Deno.test("ALL_COMPLEXITY_LEVELS contains 3 levels", () => {
  assertEquals(ALL_COMPLEXITY_LEVELS.length, 3);
});

Deno.test("ALL_TASK_CHARACTERISTICS contains 9 characteristics", () => {
  assertEquals(ALL_TASK_CHARACTERISTICS.length, 9);
});

// ---------------------------------------------------------------------------
// TECHNIQUE_METADATA registry
// ---------------------------------------------------------------------------

Deno.test("TECHNIQUE_METADATA contains all 15 techniques", () => {
  assertEquals(TECHNIQUE_METADATA.size, 15);
});

Deno.test("every technique in ALL_TECHNIQUES has metadata", () => {
  for (const technique of ALL_TECHNIQUES) {
    assertExists(
      TECHNIQUE_METADATA.get(technique),
      `Missing metadata for ${technique}`,
    );
  }
});

Deno.test("metadata technique field matches its key", () => {
  for (const [key, meta] of TECHNIQUE_METADATA) {
    assertEquals(meta.technique, key);
  }
});

// ---------------------------------------------------------------------------
// Category distribution (matches Rust test_library_categories)
// ---------------------------------------------------------------------------

Deno.test("category distribution: 1 RoleAssignment, 2 EmotionalStimulus, 7 Reasoning, 5 Others", () => {
  assertEquals(getTechniquesByCategory("RoleAssignment").length, 1);
  assertEquals(getTechniquesByCategory("EmotionalStimulus").length, 2);
  assertEquals(getTechniquesByCategory("Reasoning").length, 7);
  assertEquals(getTechniquesByCategory("Others").length, 5);
});

// ---------------------------------------------------------------------------
// SEAL quality filtering (matches Rust test_seal_quality_filtering)
// ---------------------------------------------------------------------------

Deno.test("low SEAL quality (0.3) returns only simple techniques", () => {
  const results = getTechniquesBySealQuality(0.3);
  for (const t of results) {
    assertEquals(
      t.minSealQuality <= 0.3,
      true,
      `${t.name} has minSealQuality ${t.minSealQuality} > 0.3`,
    );
  }
});

Deno.test("high SEAL quality (0.9) returns all 15 techniques", () => {
  assertEquals(getTechniquesBySealQuality(0.9).length, 15);
});

// ---------------------------------------------------------------------------
// Technique ID string conversion (matches Rust test_technique_string_conversion)
// ---------------------------------------------------------------------------

Deno.test("techniqueToId converts ChainOfThought to chain_of_thought", () => {
  assertEquals(techniqueToId("ChainOfThought"), "chain_of_thought");
});

Deno.test("parseTechniqueId parses chain_of_thought", () => {
  assertEquals(parseTechniqueId("chain_of_thought"), "ChainOfThought");
});

Deno.test("parseTechniqueId parses CoT abbreviation", () => {
  assertEquals(parseTechniqueId("CoT"), "ChainOfThought");
});

Deno.test("parseTechniqueId returns undefined for unknown", () => {
  assertEquals(parseTechniqueId("unknown_technique"), undefined);
});

Deno.test("round-trip: technique -> id -> technique for all", () => {
  for (const technique of ALL_TECHNIQUES) {
    const id = techniqueToId(technique);
    const parsed = parseTechniqueId(id);
    assertEquals(parsed, technique, `Round-trip failed for ${technique}`);
  }
});

// ---------------------------------------------------------------------------
// Metadata field access (matches Rust test_technique_metadata_creation)
// ---------------------------------------------------------------------------

Deno.test("ChainOfThought metadata has expected fields", () => {
  const meta = getTechniqueMetadata("ChainOfThought");
  assertExists(meta);
  assertEquals(meta.technique, "ChainOfThought");
  assertEquals(meta.category, "Reasoning");
  assertEquals(meta.name, "Chain-of-Thought");
  assertEquals(meta.minSealQuality, 0.0);
  assertEquals(meta.complexityLevel, "Simple");
  assertEquals(meta.bksPromotionEligible, true);
});

// ---------------------------------------------------------------------------
// Complexity level queries
// ---------------------------------------------------------------------------

Deno.test("countByComplexity matches expected distribution", () => {
  const simple = countByComplexity("Simple");
  const moderate = countByComplexity("Moderate");
  const advanced = countByComplexity("Advanced");
  // Total must equal 15
  assertEquals(simple + moderate + advanced, 15);
  // Verify individual counts based on the library data
  assertEquals(simple, 5); // RolePlaying, EmotionPrompting, StressPrompting, ChainOfThought, ScratchpadPrompting
  assertEquals(moderate, 6); // LeastToMost, PlanAndSolve, DecomposedPrompting, IgnoreIrrelevantConditions, HighlightedCoT, AutomaticInformationFiltering
  assertEquals(advanced, 4); // LogicOfThought, ThreadOfThought, SkeletonOfThought, SkillsInContext
});

Deno.test("getTechniquesByComplexity returns correct techniques", () => {
  const advanced = getTechniquesByComplexity("Advanced");
  const names = advanced.map((t) => t.technique).sort();
  assertEquals(names, [
    "LogicOfThought",
    "SkeletonOfThought",
    "SkillsInContext",
    "ThreadOfThought",
  ]);
});

// ---------------------------------------------------------------------------
// getAllTechniqueMetadata
// ---------------------------------------------------------------------------

Deno.test("getAllTechniqueMetadata returns 15 entries", () => {
  assertEquals(getAllTechniqueMetadata().length, 15);
});

// ---------------------------------------------------------------------------
// Template strings are non-empty
// ---------------------------------------------------------------------------

Deno.test("all technique templates are non-empty strings", () => {
  for (const meta of getAllTechniqueMetadata()) {
    assertEquals(
      meta.template.length > 0,
      true,
      `${meta.name} has empty template`,
    );
  }
});

// ---------------------------------------------------------------------------
// bestFor is non-empty for all techniques
// ---------------------------------------------------------------------------

Deno.test("all techniques have at least one bestFor characteristic", () => {
  for (const meta of getAllTechniqueMetadata()) {
    assertEquals(
      meta.bestFor.length > 0,
      true,
      `${meta.name} has empty bestFor`,
    );
  }
});
