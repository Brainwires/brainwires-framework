/**
 * Core training data types (messages, examples, preference pairs).
 * Equivalent to Rust's `brainwires_datasets::types` module.
 */

/** Role in a training conversation. */
export type TrainingRole = "system" | "user" | "assistant" | "tool";

/** A single message in a training conversation. */
export interface TrainingMessage {
  /** Role of the message sender. */
  role: TrainingRole;
  /** Text content of the message. */
  content: string;
  /** Optional tool calls made by the assistant. */
  tool_calls?: unknown[];
  /** ID of the tool call this message responds to. */
  tool_call_id?: string;
  /** Optional name of the sender. */
  name?: string;
}

/** Create a TrainingMessage with the given role and content. */
export function trainingMessage(
  role: TrainingRole,
  content: string,
): TrainingMessage {
  return { role, content };
}

/** Create a system message. */
export function systemMessage(content: string): TrainingMessage {
  return { role: "system", content };
}

/** Create a user message. */
export function userMessage(content: string): TrainingMessage {
  return { role: "user", content };
}

/** Create an assistant message. */
export function assistantMessage(content: string): TrainingMessage {
  return { role: "assistant", content };
}

/** Create a tool response message. */
export function toolMessage(
  content: string,
  toolCallId: string,
): TrainingMessage {
  return { role: "tool", content, tool_call_id: toolCallId };
}

/** Estimated token count for a message (rough: ~4 chars per token). */
export function messageTokens(msg: TrainingMessage): number {
  return Math.floor(msg.content.length / 4) + 1;
}

/** A training example consisting of a multi-turn conversation. */
export interface TrainingExample {
  /** Unique identifier for this example. */
  id: string;
  /** Ordered list of messages in the conversation. */
  messages: TrainingMessage[];
  /** Arbitrary metadata attached to this example. */
  metadata?: Record<string, unknown>;
}

/** Create a new TrainingExample with an auto-generated ID. */
export function trainingExample(
  messages: TrainingMessage[],
  id?: string,
): TrainingExample {
  return {
    id: id ?? crypto.randomUUID(),
    messages,
  };
}

/** Total estimated token count across all messages. */
export function exampleTokens(example: TrainingExample): number {
  return example.messages.reduce((sum, m) => sum + messageTokens(m), 0);
}

/** Check if an example has a system message. */
export function hasSystemMessage(example: TrainingExample): boolean {
  return example.messages.some((m) => m.role === "system");
}

/** Check if the last message is from the assistant. */
export function endsWithAssistant(example: TrainingExample): boolean {
  const last = example.messages[example.messages.length - 1];
  return last?.role === "assistant";
}

/** A preference pair for DPO/ORPO training. */
export interface PreferencePair {
  /** Unique identifier for this preference pair. */
  id: string;
  /** The shared prompt messages. */
  prompt: TrainingMessage[];
  /** The preferred (chosen) response messages. */
  chosen: TrainingMessage[];
  /** The rejected response messages. */
  rejected: TrainingMessage[];
  /** Arbitrary metadata attached to this pair. */
  metadata?: Record<string, unknown>;
}

/** Create a new PreferencePair with an auto-generated ID. */
export function preferencePair(
  prompt: TrainingMessage[],
  chosen: TrainingMessage[],
  rejected: TrainingMessage[],
  id?: string,
): PreferencePair {
  return {
    id: id ?? crypto.randomUUID(),
    prompt,
    chosen,
    rejected,
  };
}

/** Total estimated tokens for prompt + chosen + rejected. */
export function pairTokens(pair: PreferencePair): number {
  const sum = (msgs: TrainingMessage[]) =>
    msgs.reduce((s, m) => s + messageTokens(m), 0);
  return sum(pair.prompt) + sum(pair.chosen) + sum(pair.rejected);
}

/** Supported data formats for import/export. */
export type DataFormat = "openai" | "together" | "alpaca" | "sharegpt" | "chatml";
