/**
 * Core A2A message types: Role, Part, FileContent, Message, Artifact.
 *
 * Serialization rules:
 * - Fields are camelCase in JSON
 * - `Part` is a discriminated union on the `kind` field
 * - `FileContent` is an untagged union (has `bytes` or `uri`)
 * - Optional fields are omitted when undefined
 */

// deno-lint-ignore-file no-explicit-any

/** Sender role in A2A communication. */
export type Role = "user" | "agent";

/**
 * A single unit of communication content (discriminated union on `kind`).
 *
 * JSON shape:
 * - `{ kind: "text", text: "...", metadata?: {...} }`
 * - `{ kind: "file", file: FileContent, metadata?: {...} }`
 * - `{ kind: "data", data: any, metadata?: {...} }`
 */
export type Part = TextPart | FilePart | DataPart;

export interface TextPart {
  kind: "text";
  text: string;
  metadata?: Record<string, unknown>;
}

export interface FilePart {
  kind: "file";
  file: FileContent;
  metadata?: Record<string, unknown>;
}

export interface DataPart {
  kind: "data";
  data: unknown;
  metadata?: Record<string, unknown>;
}

/**
 * File content -- either inline bytes or a URI reference.
 * Untagged union: distinguished by presence of `bytes` vs `uri`.
 */
export type FileContent = FileContentBytes | FileContentUri;

export interface FileContentBytes {
  /** Base64-encoded file bytes. */
  bytes: string;
  /** MIME type of the file. */
  mimeType?: string;
  /** File name. */
  name?: string;
}

export interface FileContentUri {
  /** URI pointing to the file. */
  uri: string;
  /** MIME type of the file. */
  mimeType?: string;
  /** File name. */
  name?: string;
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
  /** Discriminator field (always "message"). */
  kind: string;
}

/** Create a new user message with text content. */
export function createUserMessage(text: string): Message {
  return {
    messageId: crypto.randomUUID(),
    role: "user",
    parts: [{ kind: "text", text }],
    kind: "message",
  };
}

/** Create a new agent message with text content. */
export function createAgentMessage(text: string): Message {
  return {
    messageId: crypto.randomUUID(),
    role: "agent",
    parts: [{ kind: "text", text }],
    kind: "message",
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
