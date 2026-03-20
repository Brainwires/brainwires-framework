/**
 * @module transport/traits
 *
 * Server transport interface for MCP communication.
 * Equivalent to Rust's `ServerTransport` trait.
 */

/**
 * Interface for server-side MCP transport.
 * Equivalent to Rust `ServerTransport` trait.
 */
export interface ServerTransport {
  /** Read the next JSON-RPC request, or null on EOF. */
  readRequest(): Promise<string | null>;

  /** Write a JSON-RPC response. */
  writeResponse(response: string): Promise<void>;
}
