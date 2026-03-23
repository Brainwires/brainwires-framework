/**
 * Tests for sampling utilities.
 */

import { assertEquals } from "jsr:@std/assert";
import { curriculumOrder, sampleN, trainEvalSplit } from "./sampling.ts";
import type { TrainingExample } from "./types.ts";
import {
  assistantMessage,
  exampleTokens,
  trainingExample,
  userMessage,
} from "./types.ts";

function sampleExamples(n: number): TrainingExample[] {
  return Array.from({ length: n }, (_, i) =>
    trainingExample(
      [
        userMessage(`Q${i}: ${"x".repeat(i * 10)}`),
        assistantMessage(`A${i}`),
      ],
      `ex-${i}`,
    )
  );
}

Deno.test("trainEvalSplit - default 90/10 split", () => {
  const examples = sampleExamples(100);
  const result = trainEvalSplit(examples);
  assertEquals(result.train.length, 90);
  assertEquals(result.eval.length, 10);
});

Deno.test("trainEvalSplit - custom ratio", () => {
  const examples = sampleExamples(100);
  const result = trainEvalSplit(examples, { trainRatio: 0.8 });
  assertEquals(result.train.length, 80);
  assertEquals(result.eval.length, 20);
});

Deno.test("trainEvalSplit - no shuffle", () => {
  const examples = sampleExamples(10);
  const result = trainEvalSplit(examples, { shuffle: false });
  // Without shuffle, first 9 should be train
  assertEquals(result.train.length, 9);
  assertEquals(result.eval.length, 1);
  assertEquals(result.train[0].id, "ex-0");
});

Deno.test("curriculumOrder - ascending by token count", () => {
  const examples = sampleExamples(10);
  const ordered = curriculumOrder(examples);
  for (let i = 1; i < ordered.length; i++) {
    const prev = exampleTokens(ordered[i - 1]);
    const curr = exampleTokens(ordered[i]);
    assertEquals(curr >= prev, true, `Token count should be ascending at index ${i}`);
  }
});

Deno.test("sampleN - correct count", () => {
  const examples = sampleExamples(100);
  const sampled = sampleN(examples, 10);
  assertEquals(sampled.length, 10);
});

Deno.test("sampleN - deterministic with same seed", () => {
  const examples = sampleExamples(100);
  const sampled1 = sampleN(examples, 10, 42);
  const sampled2 = sampleN(examples, 10, 42);
  for (let i = 0; i < sampled1.length; i++) {
    assertEquals(sampled1[i].id, sampled2[i].id);
  }
});

Deno.test("sampleN - returns all when n >= length", () => {
  const examples = sampleExamples(5);
  const sampled = sampleN(examples, 100);
  assertEquals(sampled.length, 5);
});

Deno.test("sampleN - different seed gives different results", () => {
  const examples = sampleExamples(100);
  const sampled1 = sampleN(examples, 10, 42);
  const sampled2 = sampleN(examples, 10, 99);
  // Very unlikely to be identical with different seeds
  const allSame = sampled1.every((e, i) => e.id === sampled2[i].id);
  assertEquals(allSame, false);
});
