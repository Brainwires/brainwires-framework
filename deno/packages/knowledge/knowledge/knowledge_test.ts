import { assertEquals, assertExists } from "jsr:@std/assert";
import {
  ALL_THOUGHT_CATEGORIES,
  createThought,
  parseThoughtCategory,
  parseThoughtSource,
} from "./mod.ts";
import type {
  ContradictionEvent,
  Entity,
  ExtractionResult,
  Relationship,
  Thought,
} from "./mod.ts";

// ---------------------------------------------------------------------------
// ThoughtCategory
// ---------------------------------------------------------------------------

Deno.test("ALL_THOUGHT_CATEGORIES has 8 categories", () => {
  assertEquals(ALL_THOUGHT_CATEGORIES.length, 8);
});

Deno.test("parseThoughtCategory round-trips all categories", () => {
  for (const cat of ALL_THOUGHT_CATEGORIES) {
    assertEquals(parseThoughtCategory(cat), cat);
  }
});

Deno.test("parseThoughtCategory handles aliases", () => {
  assertEquals(parseThoughtCategory("meetingnote"), "meeting_note");
  assertEquals(parseThoughtCategory("actionitem"), "action_item");
  assertEquals(parseThoughtCategory("todo"), "action_item");
  assertEquals(parseThoughtCategory("ref"), "reference");
});

Deno.test("parseThoughtCategory defaults to general", () => {
  assertEquals(parseThoughtCategory("unknown"), "general");
});

// ---------------------------------------------------------------------------
// ThoughtSource
// ---------------------------------------------------------------------------

Deno.test("parseThoughtSource parses known values", () => {
  assertEquals(parseThoughtSource("manual"), "manual");
  assertEquals(parseThoughtSource("manual_capture"), "manual");
  assertEquals(parseThoughtSource("conversation"), "conversation");
  assertEquals(parseThoughtSource("conversation_extract"), "conversation");
  assertEquals(parseThoughtSource("import"), "import");
});

Deno.test("parseThoughtSource defaults to manual", () => {
  assertEquals(parseThoughtSource("unknown"), "manual");
});

// ---------------------------------------------------------------------------
// Thought creation
// ---------------------------------------------------------------------------

Deno.test("createThought produces valid defaults", () => {
  const thought = createThought("Test thought");

  assertExists(thought.id);
  assertEquals(thought.content, "Test thought");
  assertEquals(thought.category, "general");
  assertEquals(thought.tags.length, 0);
  assertEquals(thought.source, "manual");
  assertEquals(thought.importance, 0.5);
  assertEquals(thought.deleted, false);
  assertEquals(typeof thought.createdAt, "number");
  assertEquals(typeof thought.updatedAt, "number");
});

Deno.test("createThought generates unique IDs", () => {
  const a = createThought("A");
  const b = createThought("B");
  assertEquals(a.id !== b.id, true);
});

// ---------------------------------------------------------------------------
// Entity type
// ---------------------------------------------------------------------------

Deno.test("Entity interface can be constructed", () => {
  const entity: Entity = {
    name: "main.rs",
    entityType: "file",
    messageIds: ["msg-1"],
    firstSeen: 100,
    lastSeen: 100,
    mentionCount: 1,
  };
  assertEquals(entity.name, "main.rs");
  assertEquals(entity.entityType, "file");
});

// ---------------------------------------------------------------------------
// Relationship types
// ---------------------------------------------------------------------------

Deno.test("Relationship discriminated union covers all variants", () => {
  const relationships: Relationship[] = [
    { kind: "Defines", definer: "A", defined: "B", context: "ctx" },
    { kind: "References", from: "A", to: "B" },
    { kind: "Modifies", modifier: "A", modified: "B", changeType: "edit" },
    { kind: "DependsOn", dependent: "A", dependency: "B" },
    { kind: "Contains", container: "A", contained: "B" },
    { kind: "CoOccurs", entityA: "A", entityB: "B", messageId: "msg-1" },
  ];
  assertEquals(relationships.length, 6);
});

// ---------------------------------------------------------------------------
// ExtractionResult
// ---------------------------------------------------------------------------

Deno.test("ExtractionResult can hold entities and relationships", () => {
  const result: ExtractionResult = {
    entities: [
      ["main.rs", "file"],
      ["process", "function"],
    ],
    relationships: [
      { kind: "Contains", container: "main.rs", contained: "process" },
    ],
  };
  assertEquals(result.entities.length, 2);
  assertEquals(result.relationships.length, 1);
});

// ---------------------------------------------------------------------------
// ContradictionEvent
// ---------------------------------------------------------------------------

Deno.test("ContradictionEvent can represent conflicting definitions", () => {
  const event: ContradictionEvent = {
    kind: "ConflictingDefinition",
    subject: "main::return_type",
    existingContext: "returns i32",
    newContext: "returns String",
  };
  assertEquals(event.kind, "ConflictingDefinition");
  assertEquals(event.subject, "main::return_type");
});
