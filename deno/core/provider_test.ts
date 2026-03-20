import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ChatOptions } from "./mod.ts";

Deno.test("ChatOptions default values", () => {
  const opts = ChatOptions.new();
  assertEquals(opts.temperature, 0.7);
  assertEquals(opts.max_tokens, 4096);
});

Deno.test("ChatOptions builder", () => {
  const opts = ChatOptions.new()
    .setTemperature(0.5)
    .setMaxTokens(2048)
    .setSystem("Test");
  assertEquals(opts.temperature, 0.5);
  assertEquals(opts.max_tokens, 2048);
  assertEquals(opts.system, "Test");
});

Deno.test("ChatOptions.deterministic", () => {
  const opts = ChatOptions.deterministic(50);
  assertEquals(opts.temperature, 0.0);
  assertEquals(opts.max_tokens, 50);
});

Deno.test("ChatOptions.factual", () => {
  const opts = ChatOptions.factual(200);
  assertEquals(opts.temperature, 0.1);
  assertEquals(opts.max_tokens, 200);
  assertEquals(opts.top_p, 0.9);
});

Deno.test("ChatOptions.creative", () => {
  const opts = ChatOptions.creative(400);
  assertEquals(opts.temperature, 0.3);
  assertEquals(opts.max_tokens, 400);
});
