/**
 * Core A2A message types: Role, Part, Message, Artifact.
 *
 * Serialization rules:
 * - Fields are camelCase in JSON
 * - `Part` must have exactly one of: text, raw, url, data
 * - Optional fields are omitted when undefined
 */

/**
 * Sender role in A2A communication.
 * Uses SCREAMING_SNAKE_CASE per A2A v1.0.
 */
export type Role = "ROLE_UNSPECIFIED" | "ROLE_USER" | "ROLE_AGENT";

/**
 * A single unit of communication content.
 *
 * Exactly one of `text`, `raw`, `url`, or `data` must be set.
 *
 * JSON shape examples:
 * - `{ text: "hello", metadata: {...} }`
 * - `{ raw: "base64...", mediaType: "image/png", filename: "pic.png" }`
 * - `{ url: "https://...", mediaType: "application/pdf" }`
 * - `{ data: { key: "value" }, metadata: {...} }`
 */
export interface Part {
  /** Plain text content. */
  text?: string;
  /** Base64-encoded raw bytes. */
  raw?: string;
  /** URL reference to content. */
  url?: string;
  /** Structured data content. */
  data?: unknown;
  /** MIME type of the content. */
  mediaType?: string;
  /** Filename hint. */
  filename?: string;
  /** Custom metadata. */
  metadata?: Record<string, unknown>;
}

/** A single communication message between client and server. */
export interface Message {
  /** Unique message identifier. */
  messageId: string;
  /** Sender role. */
  role: Role;
  /** Content parts. */
  parts: Part[];
  /** Context identifier (conversation/session). */
  contextId?: string;
  /** Associated task identifier. */
  taskId?: string;
  /** Referenced task identifiers for additional context. */
  referenceTaskIds?: string[];
  /** Custom metadata. */
  metadata?: Record<string, unknown>;
  /** Extension URIs present in this message. */
  extensions?: string[];
}

/** Create a new user message with text content. */
export function createUserMessage(text: string): Message {
  return {
    messageId: crypto.randomUUID(),
    role: "ROLE_USER",
    parts: [{ text }],
  };
}

/** Create a new agent message with text content. */
export function createAgentMessage(text: string): Message {
  return {
    messageId: crypto.randomUUID(),
    role: "ROLE_AGENT",
    parts: [{ text }],
  };
}

/** Task output artifact. */
export interface Artifact {
  /** Unique artifact identifier (unique within a task). */
  artifactId: string;
  /** Human-readable name. */
  name?: string;
  /** Human-readable description. */
  description?: string;
  /** Artifact content parts. */
  parts: Part[];
  /** Custom metadata. */
  metadata?: Record<string, unknown>;
  /** Extension URIs. */
  extensions?: string[];
}
