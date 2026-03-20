/**
 * A2A error types and JSON-RPC error codes.
 */

// ---------------------------------------------------------------------------
// JSON-RPC error codes (spec-defined)
// ---------------------------------------------------------------------------

/** Invalid JSON payload. */
export const JSON_PARSE_ERROR = -32700;
/** Request payload validation error. */
export const INVALID_REQUEST = -32600;
/** Method not found. */
export const METHOD_NOT_FOUND = -32601;
/** Invalid parameters. */
export const INVALID_PARAMS = -32602;
/** Internal error. */
export const INTERNAL_ERROR = -32603;
/** Task not found. */
export const TASK_NOT_FOUND = -32001;
/** Task cannot be canceled. */
export const TASK_NOT_CANCELABLE = -32002;
/** Push notification is not supported. */
export const PUSH_NOT_SUPPORTED = -32003;
/** This operation is not supported. */
export const UNSUPPORTED_OPERATION = -32004;
/** Incompatible content types. */
export const CONTENT_TYPE_NOT_SUPPORTED = -32005;
/** Invalid agent response. */
export const INVALID_AGENT_RESPONSE = -32006;
/** Authenticated Extended Card is not configured. */
export const EXTENDED_CARD_NOT_CONFIGURED = -32007;

// ---------------------------------------------------------------------------
// Error class
// ---------------------------------------------------------------------------

/** A2A protocol error, serializable as a JSON-RPC error object. */
export class A2aError extends Error {
  /** Numeric error code. */
  code: number;
  /** Optional additional data. */
  data?: unknown;

  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = "A2aError";
    this.code = code;
    this.data = data;
  }

  /** Attach extra data to the error (returns `this` for chaining). */
  withData(data: unknown): A2aError {
    this.data = data;
    return this;
  }

  /** Serialize to a plain JSON-RPC error object. */
  toJSON(): { code: number; message: string; data?: unknown } {
    const obj: { code: number; message: string; data?: unknown } = {
      code: this.code,
      message: this.message,
    };
    if (this.data !== undefined) {
      obj.data = this.data;
    }
    return obj;
  }

  /** Create from a plain JSON-RPC error object. */
  static fromJSON(obj: {
    code: number;
    message: string;
    data?: unknown;
  }): A2aError {
    return new A2aError(obj.code, obj.message, obj.data);
  }

  // -------------------------------------------------------------------------
  // Factory methods
  // -------------------------------------------------------------------------

  /** Task not found error. */
  static taskNotFound(taskId: string): A2aError {
    return new A2aError(TASK_NOT_FOUND, `Task not found: ${taskId}`);
  }

  /** Task not cancelable error. */
  static taskNotCancelable(taskId: string): A2aError {
    return new A2aError(
      TASK_NOT_CANCELABLE,
      `Task cannot be canceled: ${taskId}`,
    );
  }

  /** Push notifications not supported error. */
  static pushNotSupported(): A2aError {
    return new A2aError(
      PUSH_NOT_SUPPORTED,
      "Push notifications are not supported",
    );
  }

  /** Unsupported operation error. */
  static unsupportedOperation(detail: string): A2aError {
    return new A2aError(
      UNSUPPORTED_OPERATION,
      `Unsupported operation: ${detail}`,
    );
  }

  /** Content type not supported error. */
  static contentTypeNotSupported(detail: string): A2aError {
    return new A2aError(
      CONTENT_TYPE_NOT_SUPPORTED,
      `Content type not supported: ${detail}`,
    );
  }

  /** Invalid request error. */
  static invalidRequest(detail: string): A2aError {
    return new A2aError(INVALID_REQUEST, detail);
  }

  /** Internal error. */
  static internal(message: string): A2aError {
    return new A2aError(INTERNAL_ERROR, message);
  }

  /** Method not found error. */
  static methodNotFound(method: string): A2aError {
    return new A2aError(METHOD_NOT_FOUND, `Method not found: ${method}`);
  }

  /** Invalid params error. */
  static invalidParams(detail: string): A2aError {
    return new A2aError(INVALID_PARAMS, detail);
  }

  /** Parse error. */
  static parseError(detail: string): A2aError {
    return new A2aError(JSON_PARSE_ERROR, detail);
  }

  /** Extended card not configured. */
  static extendedCardNotConfigured(): A2aError {
    return new A2aError(
      EXTENDED_CARD_NOT_CONFIGURED,
      "Authenticated Extended Card is not configured",
    );
  }
}
