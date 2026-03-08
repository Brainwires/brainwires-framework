//! SSE (Server-Sent Events) response utilities for streaming.

use std::pin::Pin;

use bytes::Bytes;
use futures::Stream;
use tokio_stream::StreamExt;

use crate::error::A2aError;
use crate::jsonrpc::{JsonRpcResponse, RequestId};
use crate::streaming::StreamEvent;

/// Convert a stream of `StreamEvent` items into an SSE byte stream.
///
/// Each event is serialized as a JSON-RPC response wrapped in an SSE `data:` line.
pub fn stream_to_sse(
    id: RequestId,
    stream: Pin<Box<dyn Stream<Item = Result<StreamEvent, A2aError>> + Send>>,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
    let mapped = stream.map(move |item| {
        let response = match item {
            Ok(event) => {
                let val = serde_json::to_value(&event).unwrap_or(serde_json::Value::Null);
                JsonRpcResponse::success(id.clone(), val)
            }
            Err(e) => JsonRpcResponse::error(id.clone(), e),
        };

        let json = serde_json::to_string(&response).unwrap_or_default();
        let sse_line = format!("data: {json}\n\n");
        Ok(Bytes::from(sse_line))
    });

    Box::pin(mapped)
}
