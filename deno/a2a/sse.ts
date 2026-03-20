/**
 * SSE (Server-Sent Events) stream parser for A2A.
 *
 * Handles both JSON-RPC envelope and raw REST modes.
 * Buffer is capped at 16 MiB to prevent unbounded memory growth.
 * Parses StreamResponse wrapper format per A2A v1.0.
 */

import { A2aError } from "./error.ts";
import type { JsonRpcResponse } from "./jsonrpc.ts";
import type { StreamResponse } from "./streaming.ts";

/** Maximum SSE buffer size (16 MB). */
const MAX_SSE_BUFFER_SIZE = 16 * 1024 * 1024;

/**
 * Extract concatenated `data:` payload from an SSE frame.
 * Per the SSE spec, multiple `data:` lines are concatenated with newlines.
 * Lines starting with `:` are comments and ignored.
 */
function extractSseData(frame: string): string | null {
  const dataParts: string[] = [];
  for (const rawLine of frame.split("\n")) {
    const line = rawLine.replace(/\r$/, "");
    if (line.startsWith("data:")) {
      const value = line.slice(5);
      dataParts.push(value.startsWith(" ") ? value.slice(1) : value);
    }
    // Ignore event:, id:, retry:, and comments (:)
  }
  if (dataParts.length === 0) return null;
  const payload = dataParts.join("\n");
  return payload.length === 0 ? null : payload;
}

/**
 * Parse an SSE frame expecting a JSON-RPC envelope.
 * Extracts the `result` field as a StreamResponse.
 */
function parseSseFrameJsonRpc(frame: string): StreamResponse | A2aError {
  const data = extractSseData(frame);
  if (data === null) {
    return A2aError.parseError("SSE frame contains no data field");
  }

  let resp: JsonRpcResponse;
  try {
    resp = JSON.parse(data) as JsonRpcResponse;
  } catch (e) {
    return A2aError.parseError(String(e));
  }

  if (resp.error) {
    return A2aError.fromJSON(
      resp.error as { code: number; message: string; data?: unknown },
    );
  }

  if (resp.result !== undefined) {
    return resp.result as StreamResponse;
  }

  return A2aError.parseError(
    "JSON-RPC response has neither result nor error",
  );
}

/**
 * Parse an SSE frame expecting raw StreamResponse JSON (REST mode).
 */
function parseSseFrameRest(frame: string): StreamResponse | A2aError {
  const data = extractSseData(frame);
  if (data === null) {
    return A2aError.parseError("SSE frame contains no data field");
  }

  try {
    return JSON.parse(data) as StreamResponse;
  } catch (e) {
    return A2aError.parseError(String(e));
  }
}

/**
 * Parse an SSE byte stream incrementally, yielding StreamResponse values.
 *
 * @param body - ReadableStream from a fetch() response
 * @param mode - "jsonrpc" expects JSON-RPC envelope; "rest" expects raw events
 */
export async function* parseSseStream(
  body: ReadableStream<Uint8Array>,
  mode: "jsonrpc" | "rest" = "jsonrpc",
): AsyncIterable<StreamResponse> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      if (buffer.length > MAX_SSE_BUFFER_SIZE) {
        throw A2aError.internal("SSE stream buffer exceeded maximum size");
      }

      // Process complete SSE frames (delimited by \n\n)
      let pos: number;
      while ((pos = buffer.indexOf("\n\n")) !== -1) {
        const frame = buffer.slice(0, pos);
        buffer = buffer.slice(pos + 2);

        const result = mode === "jsonrpc"
          ? parseSseFrameJsonRpc(frame)
          : parseSseFrameRest(frame);

        if (result instanceof A2aError) {
          throw result;
        }
        yield result;
      }
    }

    // Process any remaining data in the buffer
    if (buffer.trim().length > 0) {
      const result = mode === "jsonrpc"
        ? parseSseFrameJsonRpc(buffer)
        : parseSseFrameRest(buffer);

      if (result instanceof A2aError) {
        throw result;
      }
      yield result;
    }
  } finally {
    reader.releaseLock();
  }
}
