import { assertEquals } from "@std/assert";
import { parseNDJSONStream, parseSSEStream } from "./sse.ts";

/** Helper to create a ReadableStream from a string. */
function streamFromString(text: string): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream({
    start(controller) {
      controller.enqueue(encoder.encode(text));
      controller.close();
    },
  });
}

/** Helper to create a ReadableStream that delivers data in chunks. */
function streamFromChunks(chunks: string[]): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream({
    start(controller) {
      for (const chunk of chunks) {
        controller.enqueue(encoder.encode(chunk));
      }
      controller.close();
    },
  });
}

// ---------------------------------------------------------------------------
// SSE stream tests
// ---------------------------------------------------------------------------

Deno.test("parseSSEStream - single event", async () => {
  const stream = streamFromString('data: {"type":"test"}\n\n');
  const events: string[] = [];
  for await (const data of parseSSEStream(stream)) {
    events.push(data);
  }
  assertEquals(events, ['{"type":"test"}']);
});

Deno.test("parseSSEStream - multiple events", async () => {
  const stream = streamFromString(
    'data: {"a":1}\n\ndata: {"b":2}\n\ndata: {"c":3}\n\n',
  );
  const events: string[] = [];
  for await (const data of parseSSEStream(stream)) {
    events.push(data);
  }
  assertEquals(events, ['{"a":1}', '{"b":2}', '{"c":3}']);
});

Deno.test("parseSSEStream - stops at [DONE]", async () => {
  const stream = streamFromString(
    'data: {"a":1}\n\ndata: [DONE]\n\ndata: {"b":2}\n\n',
  );
  const events: string[] = [];
  for await (const data of parseSSEStream(stream)) {
    events.push(data);
  }
  assertEquals(events, ['{"a":1}']);
});

Deno.test("parseSSEStream - handles chunked delivery", async () => {
  const stream = streamFromChunks([
    'data: {"chunk',
    '":1}\n\ndata: {"chunk":2}\n\n',
  ]);
  const events: string[] = [];
  for await (const data of parseSSEStream(stream)) {
    events.push(data);
  }
  assertEquals(events, ['{"chunk":1}', '{"chunk":2}']);
});

Deno.test("parseSSEStream - ignores non-data lines", async () => {
  const stream = streamFromString(
    'event: message\ndata: {"a":1}\n\n',
  );
  const events: string[] = [];
  for await (const data of parseSSEStream(stream)) {
    events.push(data);
  }
  assertEquals(events, ['{"a":1}']);
});

Deno.test("parseSSEStream - empty stream", async () => {
  const stream = streamFromString("");
  const events: string[] = [];
  for await (const data of parseSSEStream(stream)) {
    events.push(data);
  }
  assertEquals(events, []);
});

// ---------------------------------------------------------------------------
// NDJSON stream tests
// ---------------------------------------------------------------------------

Deno.test("parseNDJSONStream - single line", async () => {
  const stream = streamFromString('{"a":1}\n');
  const lines: string[] = [];
  for await (const line of parseNDJSONStream(stream)) {
    lines.push(line);
  }
  assertEquals(lines, ['{"a":1}']);
});

Deno.test("parseNDJSONStream - multiple lines", async () => {
  const stream = streamFromString('{"a":1}\n{"b":2}\n{"c":3}\n');
  const lines: string[] = [];
  for await (const line of parseNDJSONStream(stream)) {
    lines.push(line);
  }
  assertEquals(lines, ['{"a":1}', '{"b":2}', '{"c":3}']);
});

Deno.test("parseNDJSONStream - skips empty lines", async () => {
  const stream = streamFromString('{"a":1}\n\n{"b":2}\n');
  const lines: string[] = [];
  for await (const line of parseNDJSONStream(stream)) {
    lines.push(line);
  }
  assertEquals(lines, ['{"a":1}', '{"b":2}']);
});

Deno.test("parseNDJSONStream - handles chunked delivery", async () => {
  const stream = streamFromChunks([
    '{"chunk',
    '":1}\n{"chunk":2}\n',
  ]);
  const lines: string[] = [];
  for await (const line of parseNDJSONStream(stream)) {
    lines.push(line);
  }
  assertEquals(lines, ['{"chunk":1}', '{"chunk":2}']);
});

Deno.test("parseNDJSONStream - empty stream", async () => {
  const stream = streamFromString("");
  const lines: string[] = [];
  for await (const line of parseNDJSONStream(stream)) {
    lines.push(line);
  }
  assertEquals(lines, []);
});
