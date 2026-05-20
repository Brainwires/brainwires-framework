import { assert } from "@std/assert/assert";
import { assertEquals } from "@std/assert/equals";
import {
  confidenceLevel,
  defaultResponseConfidence,
  extractConfidence,
  isHighConfidence,
  isLowConfidence,
  quickConfidenceCheck,
} from "./confidence.ts";
import { type ChatResponse, Message } from "./message.ts";

function makeResponse(
  text: string,
  finishReason: string | undefined,
): ChatResponse {
  return {
    message: Message.assistant(text),
    usage: { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
    finish_reason: finishReason,
  };
}

Deno.test("high-confidence response scores above 0.75", () => {
  const r = makeResponse(
    "The solution is to use a hashmap for O(1) lookup. This will definitely work.",
    "stop",
  );
  const c = extractConfidence(r);
  assert(c.score > 0.75, `expected > 0.75, got ${c.score}`);
  assert(isHighConfidence(c) || c.score >= 0.7);
});

Deno.test("low-confidence hedging response scores below 0.75", () => {
  const r = makeResponse(
    "I'm not sure, but I think maybe this could possibly work. Let me reconsider...",
    "stop",
  );
  const c = extractConfidence(r);
  assert(c.score < 0.75, `expected < 0.75, got ${c.score}`);
  assert(c.factors.pattern_confidence < 0.7);
});

Deno.test("truncated finish_reason lowers completion confidence", () => {
  const r = makeResponse(
    "The answer involves several steps. First, we need to",
    "length",
  );
  const c = extractConfidence(r);
  assert(c.factors.completion_confidence < 0.6);
});

Deno.test("very short response lowers length confidence", () => {
  const r = makeResponse("Yes", "stop");
  const c = extractConfidence(r);
  assert(c.factors.length_confidence < 0.7);
});

Deno.test("quickConfidenceCheck flags obvious low confidence", () => {
  assert(
    quickConfidenceCheck(
      makeResponse("Here is the implementation you need.", "stop"),
    ),
  );
  assert(
    !quickConfidenceCheck(makeResponse("I don't know how to do this.", "stop")),
  );
});

Deno.test("confidenceLevel reports bands", () => {
  assertEquals(
    confidenceLevel({ ...defaultResponseConfidence(), score: 0.9 }),
    "very_high",
  );
  assertEquals(
    confidenceLevel({ ...defaultResponseConfidence(), score: 0.3 }),
    "very_low",
  );
});

Deno.test("isLowConfidence below 0.6", () => {
  assert(isLowConfidence({ ...defaultResponseConfidence(), score: 0.55 }));
  assert(!isLowConfidence({ ...defaultResponseConfidence(), score: 0.65 }));
});
