/**
 * Tests for format conversion and detection.
 */

import { assertEquals } from "jsr:@std/assert";
import {
  AlpacaFormat,
  ChatMlFormat,
  detectFormat,
  OpenAiFormat,
  ShareGptFormat,
} from "./format.ts";
import {
  assistantMessage,
  systemMessage,
  trainingExample,
  userMessage,
} from "./types.ts";

// -- OpenAI format tests ------------------------------------------------------

Deno.test("OpenAiFormat - roundtrip", () => {
  const fmt = new OpenAiFormat();
  const example = trainingExample([
    systemMessage("You are helpful"),
    userMessage("Hello"),
    assistantMessage("Hi there!"),
  ]);

  const json = fmt.toJson(example);
  const parsed = fmt.parseJson(json);
  assertEquals(parsed.messages.length, 3);
  assertEquals(parsed.messages[0].role, "system");
  assertEquals(parsed.messages[1].content, "Hello");
  assertEquals(parsed.messages[2].content, "Hi there!");
});

Deno.test("OpenAiFormat - structure", () => {
  const fmt = new OpenAiFormat();
  const example = trainingExample([
    userMessage("Q"),
    assistantMessage("A"),
  ]);

  const json = fmt.toJson(example);
  assertEquals(Array.isArray(json.messages), true);
  assertEquals(json.messages.length, 2);
  assertEquals(json.messages[0].role, "user");
});

// -- ChatML format tests ------------------------------------------------------

Deno.test("ChatMlFormat - roundtrip", () => {
  const fmt = new ChatMlFormat();
  const example = trainingExample([
    systemMessage("You are helpful"),
    userMessage("What is Rust?"),
    assistantMessage("Rust is a systems programming language."),
  ]);

  const json = fmt.toJson(example);
  const text = json.text;
  assertEquals(text.includes("<|im_start|>system"), true);
  assertEquals(text.includes("<|im_start|>user"), true);
  assertEquals(text.includes("<|im_start|>assistant"), true);
  assertEquals(text.includes("<|im_end|>"), true);

  const parsed = fmt.parseJson(json);
  assertEquals(parsed.messages.length, 3);
  assertEquals(parsed.messages[0].role, "system");
  assertEquals(
    parsed.messages[2].content,
    "Rust is a systems programming language.",
  );
});

// -- ShareGPT format tests ----------------------------------------------------

Deno.test("ShareGptFormat - roundtrip", () => {
  const fmt = new ShareGptFormat();
  const example = trainingExample([
    systemMessage("You are helpful"),
    userMessage("Hello"),
    assistantMessage("Hi!"),
  ]);

  const json = fmt.toJson(example);
  assertEquals(json.conversations[0].from, "system");
  assertEquals(json.conversations[1].from, "human");
  assertEquals(json.conversations[2].from, "gpt");

  const parsed = fmt.parseJson(json);
  assertEquals(parsed.messages.length, 3);
  assertEquals(parsed.messages[1].role, "user");
});

Deno.test("ShareGptFormat - alternate roles", () => {
  const fmt = new ShareGptFormat();
  const json = {
    conversations: [
      { from: "user", value: "Hello" },
      { from: "chatgpt", value: "Hi!" },
    ],
  };
  const parsed = fmt.parseJson(json);
  assertEquals(parsed.messages[0].role, "user");
  assertEquals(parsed.messages[1].role, "assistant");
});

// -- Alpaca format tests ------------------------------------------------------

Deno.test("AlpacaFormat - roundtrip", () => {
  const fmt = new AlpacaFormat();
  const example = trainingExample([
    systemMessage("You are a math tutor"),
    userMessage("What is 2+2?"),
    assistantMessage("4"),
  ]);

  const json = fmt.toJson(example);
  assertEquals(json.instruction, "What is 2+2?");
  assertEquals(json.input, "You are a math tutor");
  assertEquals(json.output, "4");

  const parsed = fmt.parseJson(json);
  assertEquals(parsed.messages.length, 3);
  assertEquals(parsed.messages[0].role, "system");
});

Deno.test("AlpacaFormat - no system message", () => {
  const fmt = new AlpacaFormat();
  const example = trainingExample([
    userMessage("Hello"),
    assistantMessage("Hi!"),
  ]);

  const json = fmt.toJson(example);
  assertEquals(json.input, "");

  const parsed = fmt.parseJson(json);
  assertEquals(parsed.messages.length, 2);
});

// -- Detection tests ----------------------------------------------------------

Deno.test("detectFormat - OpenAI", () => {
  assertEquals(detectFormat({ messages: [] }), "openai");
});

Deno.test("detectFormat - Alpaca", () => {
  assertEquals(
    detectFormat({ instruction: "test", output: "result" }),
    "alpaca",
  );
});

Deno.test("detectFormat - ShareGPT", () => {
  assertEquals(detectFormat({ conversations: [] }), "sharegpt");
});

Deno.test("detectFormat - ChatML", () => {
  assertEquals(
    detectFormat({ text: "<|im_start|>user\nHi<|im_end|>" }),
    "chatml",
  );
});

Deno.test("detectFormat - Together (plain text)", () => {
  assertEquals(detectFormat({ text: "some plain text" }), "together");
});

Deno.test("detectFormat - unknown", () => {
  assertEquals(detectFormat({ foo: "bar" }), null);
});
