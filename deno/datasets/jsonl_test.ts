/**
 * Tests for JSONL reader and writer.
 */

import { assertEquals, assertThrows } from "jsr:@std/assert";
import {
  JsonlReader,
  JsonlWriter,
  readJsonl,
  readJsonlPreferences,
  writeJsonl,
  writeJsonlPreferences,
} from "./jsonl.ts";
import type { PreferencePair, TrainingExample } from "./types.ts";
import {
  assistantMessage,
  preferencePair,
  systemMessage,
  trainingExample,
  userMessage,
} from "./types.ts";

const SAMPLE_JSONL = [
  '{"id":"1","messages":[{"role":"user","content":"Hello"},{"role":"assistant","content":"Hi!"}]}',
  '{"id":"2","messages":[{"role":"system","content":"Be helpful"},{"role":"user","content":"Q"},{"role":"assistant","content":"A"}]}',
].join("\n") + "\n";

Deno.test("readJsonl - parse sample JSONL", () => {
  const examples = readJsonl<TrainingExample>(SAMPLE_JSONL);
  assertEquals(examples.length, 2);
  assertEquals(examples[0].messages.length, 2);
  assertEquals(examples[1].messages.length, 3);
});

Deno.test("JsonlReader - iterator protocol", () => {
  const reader = new JsonlReader<TrainingExample>(SAMPLE_JSONL);
  const examples = [...reader];
  assertEquals(examples.length, 2);
});

Deno.test("JsonlReader - skips blank lines", () => {
  const data = [
    '{"id":"1","messages":[{"role":"user","content":"A"},{"role":"assistant","content":"B"}]}',
    "",
    '{"id":"2","messages":[{"role":"user","content":"C"},{"role":"assistant","content":"D"}]}',
  ].join("\n") + "\n";

  const examples = readJsonl<TrainingExample>(data);
  assertEquals(examples.length, 2);
});

Deno.test("JsonlReader - error on invalid JSON", () => {
  assertThrows(
    () => readJsonl("not valid json\n"),
    Error,
    "JSONL parse error",
  );
});

Deno.test("JsonlWriter - write and roundtrip", () => {
  const examples: TrainingExample[] = [
    trainingExample([userMessage("Hello"), assistantMessage("Hi!")], "ex1"),
    trainingExample(
      [systemMessage("Be helpful"), userMessage("Q"), assistantMessage("A")],
      "ex2",
    ),
  ];

  const jsonl = writeJsonl(examples);
  const lines = jsonl.trim().split("\n");
  assertEquals(lines.length, 2);

  const readBack = readJsonl<TrainingExample>(jsonl);
  assertEquals(readBack.length, 2);
  assertEquals(readBack[0].messages.length, 2);
  assertEquals(readBack[1].messages.length, 3);
});

Deno.test("JsonlWriter - count tracking", () => {
  const writer = new JsonlWriter<TrainingExample>();
  writer.write(
    trainingExample([userMessage("Q"), assistantMessage("A")], "1"),
  );
  writer.write(
    trainingExample([userMessage("Q2"), assistantMessage("A2")], "2"),
  );
  assertEquals(writer.count, 2);
});

Deno.test("readJsonlPreferences / writeJsonlPreferences - roundtrip", () => {
  const pairs: PreferencePair[] = [
    preferencePair(
      [userMessage("Q1")],
      [assistantMessage("Good")],
      [assistantMessage("Bad")],
    ),
    preferencePair(
      [userMessage("Q2")],
      [assistantMessage("Yes")],
      [assistantMessage("No")],
    ),
  ];

  const jsonl = writeJsonlPreferences(pairs);
  const readBack = readJsonlPreferences(jsonl);
  assertEquals(readBack.length, 2);
  assertEquals(readBack[0].prompt[0].content, "Q1");
  assertEquals(readBack[1].rejected[0].content, "No");
});

Deno.test("JsonlReader - async iterable", async () => {
  const reader = new JsonlReader<TrainingExample>(SAMPLE_JSONL);
  const examples: TrainingExample[] = [];
  for await (const ex of reader) {
    examples.push(ex);
  }
  assertEquals(examples.length, 2);
});
