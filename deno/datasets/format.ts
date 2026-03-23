/**
 * Format converters for various fine-tuning providers.
 * Equivalent to Rust's `brainwires_datasets::format` module.
 */

// deno-lint-ignore-file no-explicit-any

import type {
  DataFormat,
  PreferencePair,
  TrainingExample,
  TrainingMessage,
  TrainingRole,
} from "./types.ts";
import {
  preferencePair,
  trainingExample,
  trainingMessage,
} from "./types.ts";

// -- Interface ----------------------------------------------------------------

/** Convert training examples to/from a specific provider format. */
export interface FormatConverter {
  /** Name of this format. */
  name(): string;
  /** Convert a TrainingExample to this format's JSON representation. */
  toJson(example: TrainingExample): any;
  /** Parse this format's JSON back into a TrainingExample. */
  parseJson(value: any): TrainingExample;
}

// -- OpenAI Format ------------------------------------------------------------

/**
 * OpenAI chat fine-tuning JSONL format.
 * Format: `{"messages": [{"role": "...", "content": "..."}]}`
 */
export class OpenAiFormat implements FormatConverter {
  name(): string {
    return "openai";
  }

  toJson(example: TrainingExample): any {
    const messages = example.messages.map((msg) => {
      const obj: any = { role: msg.role, content: msg.content };
      if (msg.tool_calls) obj.tool_calls = msg.tool_calls;
      if (msg.tool_call_id) obj.tool_call_id = msg.tool_call_id;
      if (msg.name) obj.name = msg.name;
      return obj;
    });
    return { messages };
  }

  parseJson(value: any): TrainingExample {
    const messages = value?.messages;
    if (!Array.isArray(messages)) {
      throw new Error("Missing or invalid 'messages' field");
    }
    return trainingExample(
      messages.map((m: any) => parseRole(m.role, m.content ?? "", m)),
    );
  }
}

// -- ChatML Format ------------------------------------------------------------

/**
 * ChatML template format.
 * Format: `{"text": "<|im_start|>system\n...<|im_end|>\n..."}`
 */
export class ChatMlFormat implements FormatConverter {
  name(): string {
    return "chatml";
  }

  toJson(example: TrainingExample): any {
    let text = "";
    for (const msg of example.messages) {
      text += `<|im_start|>${msg.role}\n${msg.content}<|im_end|>\n`;
    }
    return { text };
  }

  parseJson(value: any): TrainingExample {
    const text = value?.text;
    if (typeof text !== "string") {
      throw new Error("Missing 'text' field for ChatML format");
    }
    return trainingExample(parseChatMl(text));
  }
}

function parseChatMl(text: string): TrainingMessage[] {
  const messages: TrainingMessage[] = [];
  let remaining = text;

  while (true) {
    const startIdx = remaining.indexOf("<|im_start|>");
    if (startIdx === -1) break;
    remaining = remaining.slice(startIdx + 12);

    const endIdx = remaining.indexOf("<|im_end|>");
    if (endIdx === -1) throw new Error("Unclosed <|im_start|> tag");

    const block = remaining.slice(0, endIdx);
    const newlinePos = block.indexOf("\n");
    const roleStr = (newlinePos >= 0 ? block.slice(0, newlinePos) : block)
      .trim();
    const content = newlinePos >= 0 ? block.slice(newlinePos + 1).trim() : "";

    const role = toRole(roleStr);
    messages.push(trainingMessage(role, content));
    remaining = remaining.slice(endIdx + 10);
  }

  if (messages.length === 0) {
    throw new Error("No ChatML messages found");
  }
  return messages;
}

// -- ShareGPT Format ----------------------------------------------------------

/**
 * ShareGPT conversation format.
 * Format: `{"conversations": [{"from": "human|gpt|system", "value": "..."}]}`
 */
export class ShareGptFormat implements FormatConverter {
  name(): string {
    return "sharegpt";
  }

  toJson(example: TrainingExample): any {
    const conversations = example.messages.map((msg) => ({
      from: roleToShareGpt(msg.role),
      value: msg.content,
    }));
    return { conversations };
  }

  parseJson(value: any): TrainingExample {
    const conversations = value?.conversations;
    if (!Array.isArray(conversations)) {
      throw new Error("Missing or invalid 'conversations' field");
    }
    return trainingExample(
      conversations.map((c: any) => {
        const role = shareGptToRole(c.from);
        return trainingMessage(role, c.value ?? "");
      }),
    );
  }
}

function roleToShareGpt(role: TrainingRole): string {
  switch (role) {
    case "system":
      return "system";
    case "user":
      return "human";
    case "assistant":
      return "gpt";
    case "tool":
      return "tool";
  }
}

function shareGptToRole(from: string): TrainingRole {
  switch (from) {
    case "system":
      return "system";
    case "human":
    case "user":
      return "user";
    case "gpt":
    case "assistant":
    case "chatgpt":
      return "assistant";
    case "tool":
      return "tool";
    default:
      throw new Error(`Unknown ShareGPT role: ${from}`);
  }
}

// -- Alpaca Format ------------------------------------------------------------

/**
 * Stanford Alpaca instruction-following format.
 * Format: `{"instruction": "...", "input": "...", "output": "..."}`
 */
export class AlpacaFormat implements FormatConverter {
  name(): string {
    return "alpaca";
  }

  toJson(example: TrainingExample): any {
    const system = example.messages.find((m) => m.role === "system")?.content ??
      "";
    const instruction = example.messages.find((m) => m.role === "user")
      ?.content ?? "";
    const output = [...example.messages].reverse().find((m) =>
      m.role === "assistant"
    )?.content ?? "";

    return { instruction, input: system, output };
  }

  parseJson(value: any): TrainingExample {
    const instruction = value?.instruction;
    const input = value?.input ?? "";
    const output = value?.output;

    if (typeof instruction !== "string") {
      throw new Error("Missing 'instruction' field");
    }
    if (typeof output !== "string") {
      throw new Error("Missing 'output' field");
    }

    const messages: TrainingMessage[] = [];
    if (input) {
      messages.push(trainingMessage("system", input));
    }
    messages.push(trainingMessage("user", instruction));
    messages.push(trainingMessage("assistant", output));

    return trainingExample(messages);
  }
}

// -- Detection ----------------------------------------------------------------

/** Auto-detect the format of a JSON value. */
export function detectFormat(value: any): DataFormat | null {
  if (value?.messages) return "openai";
  if (value?.instruction && value?.output) return "alpaca";
  if (value?.conversations) return "sharegpt";
  if (typeof value?.text === "string") {
    if (value.text.includes("<|im_start|>")) return "chatml";
    return "together";
  }
  return null;
}

// -- Helpers ------------------------------------------------------------------

function toRole(str: string): TrainingRole {
  switch (str) {
    case "system":
      return "system";
    case "user":
      return "user";
    case "assistant":
      return "assistant";
    case "tool":
      return "tool";
    default:
      throw new Error(`Unknown role: ${str}`);
  }
}

function parseRole(
  roleStr: string,
  content: string,
  raw: any,
): TrainingMessage {
  const role = toRole(roleStr);
  const msg: TrainingMessage = { role, content };
  if (raw.tool_calls) msg.tool_calls = raw.tool_calls;
  if (raw.tool_call_id) msg.tool_call_id = raw.tool_call_id;
  if (raw.name) msg.name = raw.name;
  return msg;
}
