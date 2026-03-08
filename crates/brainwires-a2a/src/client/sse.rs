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
