//! SSE (Server-Sent Events) stream parser.

use bytes::Bytes;
use futures::Stream;

use crate::error::A2aError;
use crate::jsonrpc::JsonRpcResponse;
use crate::streaming::StreamEvent;

/// Parse an SSE response body into a stream of `StreamEvent`.
///
/// Expects lines of the form `data: {...}\n\n` where each data line
/// contains a JSON-RPC response with a `StreamEvent` as the result.
pub fn parse_sse_stream(
    body: String,
) -> impl Stream<Item = Result<StreamEvent, A2aError>> {
    async_stream::stream! {
        for line in body.lines() {
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data: ") {
                match serde_json::from_str::<JsonRpcResponse>(data) {
                    Ok(resp) => {
                        if let Some(err) = resp.error {
                            yield Err(err);
                        } else if let Some(result) = resp.result {
                            match serde_json::from_value::<StreamEvent>(result) {
                                Ok(event) => yield Ok(event),
                                Err(e) => yield Err(A2aError::from(e)),
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(A2aError::parse_error(e.to_string()));
                    }
                }
            }
        }
    }
}

/// Parse SSE from a streaming byte source, yielding events as they arrive.
pub fn parse_sse_bytes(
    data: Bytes,
) -> impl Stream<Item = Result<StreamEvent, A2aError>> {
    let text = String::from_utf8_lossy(&data).to_string();
    parse_sse_stream(text)
}

/// Parse an SSE byte stream incrementally (JSON-RPC envelope).
///
/// Reads chunks from a `reqwest::Response::bytes_stream()`, buffers until
/// complete SSE frames (`\n\n` boundaries) are found, then parses each frame.
/// Handles multi-line `data:` fields per the SSE specification.
pub fn parse_sse_byte_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<StreamEvent, A2aError>> + Send {
    async_stream::stream! {
        use futures::StreamExt;
        let mut pinned = std::pin::pin!(stream);
        let mut buffer = String::new();

        while let Some(chunk) = pinned.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    yield Err(A2aError::internal(format!("Stream read error: {e}")));
                    return;
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE frames (delimited by \n\n)
            while let Some(boundary) = buffer.find("\n\n") {
                let frame = buffer[..boundary].to_string();
                buffer = buffer[boundary + 2..].to_string();

                if let Some(event) = parse_sse_frame_jsonrpc(&frame) {
                    yield event;
                }
            }
        }

        // Process any remaining data in the buffer
        if !buffer.trim().is_empty() {
            if let Some(event) = parse_sse_frame_jsonrpc(&buffer) {
                yield event;
            }
        }
    }
}

/// Parse an SSE byte stream incrementally (raw REST — no JSON-RPC envelope).
///
/// Like `parse_sse_byte_stream` but expects `data:` lines containing raw
/// `StreamEvent` JSON rather than a JSON-RPC response wrapper.
pub fn parse_sse_rest_byte_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> impl Stream<Item = Result<StreamEvent, A2aError>> + Send {
    async_stream::stream! {
        use futures::StreamExt;
        let mut pinned = std::pin::pin!(stream);
        let mut buffer = String::new();

        while let Some(chunk) = pinned.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    yield Err(A2aError::internal(format!("Stream read error: {e}")));
                    return;
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(boundary) = buffer.find("\n\n") {
                let frame = buffer[..boundary].to_string();
                buffer = buffer[boundary + 2..].to_string();

                if let Some(event) = parse_sse_frame_rest(&frame) {
                    yield event;
                }
            }
        }

        if !buffer.trim().is_empty() {
            if let Some(event) = parse_sse_frame_rest(&buffer) {
                yield event;
            }
        }
    }
}

/// Extract the concatenated `data:` payload from an SSE frame.
///
/// Per the SSE spec, multiple `data:` lines are concatenated with newlines.
/// Lines starting with `:` are comments and ignored. `event:`, `id:`, and
/// `retry:` fields are ignored.
fn extract_sse_data(frame: &str) -> Option<String> {
    let mut data_parts: Vec<&str> = Vec::new();

    for line in frame.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(value) = line.strip_prefix("data:") {
            data_parts.push(value.strip_prefix(' ').unwrap_or(value));
        }
        // Ignore event:, id:, retry:, and comments (:)
    }

    if data_parts.is_empty() {
        return None;
    }

    let payload = data_parts.join("\n");
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}

/// Parse an SSE frame with JSON-RPC envelope.
fn parse_sse_frame_jsonrpc(frame: &str) -> Option<Result<StreamEvent, A2aError>> {
    let data = extract_sse_data(frame)?;

    Some(match serde_json::from_str::<JsonRpcResponse>(&data) {
        Ok(resp) => {
            if let Some(err) = resp.error {
                Err(err)
            } else if let Some(result) = resp.result {
                serde_json::from_value::<StreamEvent>(result).map_err(A2aError::from)
            } else {
                return None;
            }
        }
        Err(e) => Err(A2aError::parse_error(e.to_string())),
    })
}

/// Parse an SSE frame with raw StreamEvent JSON (no JSON-RPC envelope).
fn parse_sse_frame_rest(frame: &str) -> Option<Result<StreamEvent, A2aError>> {
    let data = extract_sse_data(frame)?;

    Some(
        serde_json::from_str::<StreamEvent>(&data)
            .map_err(|e| A2aError::parse_error(e.to_string())),
    )
}
