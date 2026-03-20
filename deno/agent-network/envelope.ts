/**
 * @module envelope
 *
 * Message envelope types for agent communication.
 * Equivalent to Rust's `MessageEnvelope`, `MessageTarget`, `Payload`.
 */

/** The target of a message. Equivalent to Rust `MessageTarget`. */
export type MessageTarget =
  | { type: "direct"; id: string }
  | { type: "broadcast" }
  | { type: "topic"; topic: string };

/** The payload of a message. Equivalent to Rust `Payload`. */
export type Payload =
  | { type: "json"; value: unknown }
  | { type: "binary"; data: Uint8Array }
  | { type: "text"; text: string };

/**
 * A message envelope that wraps any payload with routing metadata.
 * Universal message format across all transports.
 * Equivalent to Rust `MessageEnvelope`.
 */
export interface MessageEnvelope {
  /** Unique message identifier. */
  id: string;
  /** The sender's agent identity UUID. */
  sender: string;
  /** Who this message is addressed to. */
  recipient: MessageTarget;
  /** The message payload. */
  payload: Payload;
  /** When the message was created (ISO string). */
  timestamp: string;
  /** Optional time-to-live (hop count). */
  ttl?: number;
  /** Optional correlation ID for request-response patterns. */
  correlationId?: string;
}

/** Create a new direct message envelope. */
export function directEnvelope(
  sender: string,
  recipient: string,
  payload: Payload,
): MessageEnvelope {
  return {
    id: crypto.randomUUID(),
    sender,
    recipient: { type: "direct", id: recipient },
    payload,
    timestamp: new Date().toISOString(),
  };
}

/** Create a new broadcast message envelope. */
export function broadcastEnvelope(
  sender: string,
  payload: Payload,
): MessageEnvelope {
  return {
    id: crypto.randomUUID(),
    sender,
    recipient: { type: "broadcast" },
    payload,
    timestamp: new Date().toISOString(),
  };
}

/** Create a new topic-addressed message envelope. */
export function topicEnvelope(
  sender: string,
  topic: string,
  payload: Payload,
): MessageEnvelope {
  return {
    id: crypto.randomUUID(),
    sender,
    recipient: { type: "topic", topic },
    payload,
    timestamp: new Date().toISOString(),
  };
}

/** Set the TTL on an envelope. Returns a new envelope. */
export function withTtl(
  envelope: MessageEnvelope,
  ttl: number,
): MessageEnvelope {
  return { ...envelope, ttl };
}

/** Set the correlation ID on an envelope. Returns a new envelope. */
export function withCorrelation(
  envelope: MessageEnvelope,
  correlationId: string,
): MessageEnvelope {
  return { ...envelope, correlationId };
}

/** Create a reply envelope to a received message. */
export function replyEnvelope(
  original: MessageEnvelope,
  sender: string,
  payload: Payload,
): MessageEnvelope {
  return {
    id: crypto.randomUUID(),
    sender,
    recipient: { type: "direct", id: original.sender },
    payload,
    timestamp: new Date().toISOString(),
    correlationId: original.id,
  };
}

/** Create a text payload. */
export function textPayload(text: string): Payload {
  return { type: "text", text };
}

/** Create a JSON payload. */
export function jsonPayload(value: unknown): Payload {
  return { type: "json", value };
}

/** Create a binary payload. */
export function binaryPayload(data: Uint8Array): Payload {
  return { type: "binary", data };
}
