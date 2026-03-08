//! JSON-RPC over HTTP+SSE transport.

use std::pin::Pin;

use futures::Stream;
use url::Url;

use crate::error::A2aError;
use crate::jsonrpc::{JsonRpcRequest, JsonRpcResponse, RequestId};
use crate::streaming::StreamEvent;

/// JSON-RPC transport client.
pub struct JsonRpcTransport {
    base_url: Url,
    client: reqwest::Client,
    request_counter: std::sync::atomic::AtomicI64,
}

impl JsonRpcTransport {
    /// Create a new transport pointing at the given base URL.
    pub fn new(base_url: Url, client: reqwest::Client) -> Self {
        Self {
            base_url,
            client,
            request_counter: std::sync::atomic::AtomicI64::new(1),
        }
    }

    fn next_id(&self) -> RequestId {
        let id = self
            .request_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        RequestId::Number(id)
    }

    /// Send a JSON-RPC request and get the response.
    pub async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, A2aError> {
        let id = self.next_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(params),
            id: id.clone(),
        };

        let resp = self
            .client
            .post(self.base_url.as_str())
            .json(&request)
            .send()
            .await
            .map_err(|e| A2aError::internal(format!("HTTP request failed: {e}")))?;

        let rpc_resp: JsonRpcResponse = resp
            .json()
            .await
            .map_err(|e| A2aError::internal(format!("Failed to parse response: {e}")))?;

        if let Some(err) = rpc_resp.error {
            return Err(err);
        }

        rpc_resp.result.ok_or_else(|| A2aError::internal("Empty result"))
    }

    /// Send a JSON-RPC request and stream SSE responses.
    pub fn call_stream(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, A2aError>> + Send>> {
        let id = self.next_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(params),
            id,
        };
        let client = self.client.clone();
        let url = self.base_url.clone();

        Box::pin(async_stream::stream! {
            let resp = match client
                .post(url.as_str())
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(A2aError::internal(format!("HTTP request failed: {e}")));
                    return;
                }
            };

            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    yield Err(A2aError::internal(format!("Failed to read response: {e}")));
                    return;
                }
            };

            use futures::StreamExt;
            let mut stream = std::pin::pin!(crate::client::sse::parse_sse_stream(text));
            while let Some(item) = stream.next().await {
                yield item;
            }
        })
    }
}
