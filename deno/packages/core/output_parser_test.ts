import { assertEquals } from "@std/assert/equals";
import { assert } from "@std/assert/assert";
import { assertThrows } from "@std/assert";
import {
  extractJson,
  JsonListParser,
  JsonOutputParser,
  RegexOutputParser,
} from "./output_parser.ts";

// ---------------------------------------------------------------------------
// JsonOutputParser
// ---------------------------------------------------------------------------

interface TestStruct {
  name: string;
  value: number;
}

Deno.test("JsonOutputParser: clean JSON", () => {
  const parser = new JsonOutputParser<TestStruct>();
  const result = parser.parse('{"name": "test", "value": 42}');
  assertEquals(result.name, "test");
  assertEquals(result.value, 42);
});

Deno.test("JsonOutputParser: JSON with surrounding prose", () => {
  const parser = new JsonOutputParser<TestStruct>();
  const input = 'Here is the result: {"name": "test", "value": 42} Hope that helps!';
  const result = parser.parse(input);
  assertEquals(result.name, "test");
  assertEquals(result.value, 42);
});

Deno.test("JsonOutputParser: JSON in code fence", () => {
  const parser = new JsonOutputParser<TestStruct>();
  const input = 'Here\'s the JSON:\n```json\n{"name": "test", "value": 42}\n```';
  const result = parser.parse(input);
  assertEquals(result.name, "test");
});

Deno.test("JsonOutputParser: no JSON throws", () => {
  const parser = new JsonOutputParser<TestStruct>();
  assertThrows(
    () => parser.parse("no json here at all"),
    Error,
    "No JSON found",
  );
});

Deno.test("JsonOutputParser: format instructions mention JSON", () => {
  const parser = new JsonOutputParser<TestStruct>();
  const instructions = parser.formatInstructions();
  assert(instructions.includes("JSON"));
});

// ---------------------------------------------------------------------------
// JsonListParser
// ---------------------------------------------------------------------------

Deno.test("JsonListParser: parse array", () => {
  const parser = new JsonListParser<TestStruct>();
  const input = '[{"name": "a", "value": 1}, {"name": "b", "value": 2}]';
  const result = parser.parse(input);
  assertEquals(result.length, 2);
  assertEquals(result[0].name, "a");
  assertEquals(result[1].name, "b");
});

Deno.test("JsonListParser: no JSON throws", () => {
  const parser = new JsonListParser<TestStruct>();
  assertThrows(
    () => parser.parse("no json here"),
    Error,
    "No JSON array found",
  );
});

Deno.test("JsonListParser: format instructions mention array", () => {
  const parser = new JsonListParser<TestStruct>();
  assert(parser.formatInstructions().includes("array"));
});

// ---------------------------------------------------------------------------
// RegexOutputParser
// ---------------------------------------------------------------------------

Deno.test("RegexOutputParser: named capture groups", () => {
  const parser = new RegexOutputParser(
    "sentiment: (?<sentiment>\\w+), score: (?<score>[\\d.]+)",
  );
  const result = parser.parse("The sentiment: positive, score: 0.95 overall");
  assertEquals(result["sentiment"], "positive");
  assertEquals(result["score"], "0.95");
});

Deno.test("RegexOutputParser: no match throws", () => {
  const parser = new RegexOutputParser("(?<x>\\d+)");
  assertThrows(
    () => parser.parse("no digits here"),
    Error,
    "did not match",
  );
});

Deno.test("RegexOutputParser: format instructions contain pattern", () => {
  const parser = new RegexOutputParser("(?<val>\\w+)");
  assert(parser.formatInstructions().includes("(?<val>\\w+)"));
});

Deno.test("RegexOutputParser: invalid regex throws on construction", () => {
  assertThrows(
    () => new RegexOutputParser("[invalid"),
    Error,
    "Invalid regex",
  );
});

// ---------------------------------------------------------------------------
// extractJson helper
// ---------------------------------------------------------------------------

Deno.test("extractJson: array in prose", () => {
  const result = extractJson("Here are the items: [1, 2, 3] done.");
  assertEquals(result, "[1, 2, 3]");
});

Deno.test("extractJson: standalone object", () => {
  const result = extractJson('  {"a": 1}  ');
  assertEquals(result, '{"a": 1}');
});

Deno.test("extractJson: no JSON returns undefined", () => {
  assertEquals(extractJson("just plain text"), undefined);
});
