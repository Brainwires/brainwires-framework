/**
 * Tests for quality validation, statistics, and deduplication.
 */

import { assertEquals } from "jsr:@std/assert";
import {
  computeStats,
  DataValidator,
  exactDedup,
  exactDedupPreferences,
  reportHasErrors,
} from "./quality.ts";
import type { PreferencePair, TrainingExample } from "./types.ts";
import {
  assistantMessage,
  preferencePair,
  systemMessage,
  trainingExample,
  userMessage,
} from "./types.ts";

// -- Validation tests ---------------------------------------------------------

Deno.test("DataValidator - valid example passes", () => {
  const validator = new DataValidator();
  const example = trainingExample(
    [userMessage("Hello"), assistantMessage("Hi!")],
    "test",
  );
  const issues = validator.validateExample(example);
  assertEquals(issues.length, 0);
});

Deno.test("DataValidator - too few messages", () => {
  const validator = new DataValidator();
  const example = trainingExample([userMessage("Hello")], "test");
  const issues = validator.validateExample(example);
  assertEquals(issues.some((i) => i.message.includes("Too few")), true);
  assertEquals(
    issues.some((i) => i.message.includes("must be from assistant")),
    true,
  );
});

Deno.test("DataValidator - empty content rejected", () => {
  const validator = new DataValidator();
  const example = trainingExample(
    [userMessage(""), assistantMessage("Hi")],
    "test",
  );
  const issues = validator.validateExample(example);
  assertEquals(issues.some((i) => i.message.includes("empty content")), true);
});

Deno.test("DataValidator - validation report", () => {
  const validator = new DataValidator();
  const examples: TrainingExample[] = [
    trainingExample(
      [userMessage("Q"), assistantMessage("A")],
      "good",
    ),
    trainingExample([userMessage("Q")], "bad"),
  ];
  const report = validator.validateDataset(examples);
  assertEquals(report.totalExamples, 2);
  assertEquals(report.validExamples, 1);
  assertEquals(reportHasErrors(report), true);
});

Deno.test("DataValidator - preference validation: identical", () => {
  const validator = new DataValidator();
  const pair = preferencePair(
    [userMessage("Q")],
    [assistantMessage("Same")],
    [assistantMessage("Same")],
  );
  const issues = validator.validatePreference(pair);
  assertEquals(issues.some((i) => i.message.includes("identical")), true);
});

Deno.test("DataValidator - preference validation: empty prompt", () => {
  const validator = new DataValidator();
  const pair: PreferencePair = {
    id: "test",
    prompt: [],
    chosen: [assistantMessage("Good")],
    rejected: [assistantMessage("Bad")],
  };
  const issues = validator.validatePreference(pair);
  assertEquals(issues.some((i) => i.message.includes("empty prompt")), true);
});

Deno.test("DataValidator - preference validation: empty content", () => {
  const validator = new DataValidator();
  const pair = preferencePair(
    [userMessage("")],
    [assistantMessage("Good")],
    [assistantMessage("Bad")],
  );
  const issues = validator.validatePreference(pair);
  assertEquals(issues.some((i) => i.message.includes("empty content")), true);
});

Deno.test("DataValidator - preference dataset report", () => {
  const validator = new DataValidator();
  const pairs: PreferencePair[] = [
    preferencePair(
      [userMessage("Q")],
      [assistantMessage("Good")],
      [assistantMessage("Bad")],
    ),
    {
      id: "bad",
      prompt: [],
      chosen: [assistantMessage("Good")],
      rejected: [assistantMessage("Bad")],
    },
  ];
  const report = validator.validatePreferenceDataset(pairs);
  assertEquals(report.totalExamples, 2);
  assertEquals(report.validExamples, 1);
});

// -- Statistics tests ---------------------------------------------------------

Deno.test("computeStats - basic statistics", () => {
  const examples: TrainingExample[] = [
    trainingExample(
      [
        systemMessage("Be helpful"),
        userMessage("Hello"),
        assistantMessage("Hi there! How can I help?"),
      ],
      "1",
    ),
    trainingExample(
      [userMessage("What is 2+2?"), assistantMessage("4")],
      "2",
    ),
    trainingExample(
      [
        systemMessage("Expert mode"),
        userMessage("Explain quantum computing"),
        assistantMessage(
          "Quantum computing leverages quantum mechanical phenomena...",
        ),
      ],
      "3",
    ),
  ];

  const stats = computeStats(examples);
  assertEquals(stats.totalExamples, 3);
  assertEquals(stats.totalMessages, 8);
  assertEquals(stats.examplesWithSystem, 2);
  assertEquals(stats.roleCounts.system, 2);
  assertEquals(stats.roleCounts.user, 3);
  assertEquals(stats.roleCounts.assistant, 3);
  assertEquals(stats.avgMessagesPerExample > 2.0, true);
  assertEquals(stats.totalEstimatedTokens > 0, true);
});

Deno.test("computeStats - empty dataset", () => {
  const stats = computeStats([]);
  assertEquals(stats.totalExamples, 0);
  assertEquals(stats.avgTokensPerExample, 0);
});

// -- Deduplication tests ------------------------------------------------------

Deno.test("exactDedup - removes exact duplicates", () => {
  const examples: TrainingExample[] = [
    trainingExample(
      [userMessage("Hello"), assistantMessage("Hi")],
      "1",
    ),
    trainingExample(
      [userMessage("Hello"), assistantMessage("Hi")],
      "2",
    ),
    trainingExample(
      [userMessage("Different"), assistantMessage("Response")],
      "3",
    ),
  ];

  const [deduped, removed] = exactDedup(examples);
  assertEquals(deduped.length, 2);
  assertEquals(removed, 1);
});

Deno.test("exactDedup - empty input", () => {
  const [deduped, removed] = exactDedup([]);
  assertEquals(deduped.length, 0);
  assertEquals(removed, 0);
});

Deno.test("exactDedupPreferences - removes exact duplicates", () => {
  const pairs: PreferencePair[] = [
    preferencePair(
      [userMessage("Q")],
      [assistantMessage("Good")],
      [assistantMessage("Bad")],
    ),
    preferencePair(
      [userMessage("Q")],
      [assistantMessage("Good")],
      [assistantMessage("Bad")],
    ),
    preferencePair(
      [userMessage("Different")],
      [assistantMessage("A")],
      [assistantMessage("B")],
    ),
  ];

  const [deduped, removed] = exactDedupPreferences(pairs);
  assertEquals(deduped.length, 2);
  assertEquals(removed, 1);
});
