/**
 * @module transport/stdio
 *
 * Stdio-based server transport for MCP communication.
 * Equivalent to Rust's `StdioServerTransport`.
 */

import type { ServerTransport } from "./traits.ts";

/**
 * Stdio-based server transport (stdin/stdout).
 * Reads newline-delimited JSON from stdin and writes to stdout.
 * Equivalent to Rust `StdioServerTransport`.
 */
export class StdioServerTransport implements ServerTransport {
  private reader: ReadableStreamDefaultReader<string>;
  private buffer = "";
  private encoder = new TextEncoder();
  private done = false;

  constructor() {
    const decoder = new TextDecoderStream();
    Deno.stdin.readable.pipeTo(decoder.writable).catch(() => {
      this.done = true;
    });
    this.reader = decoder.readable.getReader();
  }

  async readRequest(): Promise<string | null> {
    // Check buffer for a complete line
    while (true) {
      const newlineIndex = this.buffer.indexOf("\n");
      if (newlineIndex !== -1) {
        const line = this.buffer.slice(0, newlineIndex).trim();
        this.buffer = this.buffer.slice(newlineIndex + 1);
        if (line.length === 0) {
          return null;
        }
        return line;
      }

      if (this.done) {
        // Drain remaining buffer
        if (this.buffer.trim().length > 0) {
          const line = this.buffer.trim();
          this.buffer = "";
          return line;
        }
        return null;
      }

      const { value, done } = await this.reader.read();
      if (done) {
        this.done = true;
        // Process remaining buffer
        if (this.buffer.trim().length > 0) {
          const line = this.buffer.trim();
          this.buffer = "";
          return line;
        }
        return null;
      }
      this.buffer += value;
    }
  }

  async writeResponse(response: string): Promise<void> {
    const data = this.encoder.encode(response + "\n");
    await Deno.stdout.write(data);
  }
}
