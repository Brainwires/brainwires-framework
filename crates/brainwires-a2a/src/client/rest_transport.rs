//! HTTP/REST transport.

use std::pin::Pin;

use futures::Stream;
use url::Url;

use crate::error::A2aError;
use crate::streaming::StreamEvent;

/// REST transport client.
pub struct RestTransport {
    base_url: Url,
    client: reqwest::Client,
}

impl RestTransport {
    /// Create a new REST transport.
    pub fn new(base_url: Url, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.as_str().trim_end_matches('/'), path)
    }

    /// POST with JSON body, return JSON.
    pub async fn post(
        &self,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<serde_json::Value, A2aError> {
        let resp = self
            .client
            .post(&self.url(path))
            .json(body)
            .send()
            .await
            .map_err(|e| A2aError::internal(format!("REST request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(A2aError::internal(format!(
                "REST error: {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| A2aError::internal(format!("Failed to parse REST response: {e}")))
    }

    /// GET, return JSON.
    pub async fn get(&self, path: &str) -> Result<serde_json::Value, A2aError> {
        let resp = self
            .client
            .get(&self.url(path))
            .send()
            .await
            .map_err(|e| A2aError::internal(format!("REST request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(A2aError::internal(format!(
                "REST error: {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| A2aError::internal(format!("Failed to parse REST response: {e}")))
    }

    /// DELETE, return nothing.
    pub async fn delete(&self, path: &str) -> Result<(), A2aError> {
        let resp = self
            .client
            .delete(&self.url(path))
            .send()
            .await
            .map_err(|e| A2aError::internal(format!("REST DELETE failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(A2aError::internal(format!(
                "REST DELETE error: {}",
                resp.status()
            )));
        }

        Ok(())
    }

    /// POST returning SSE stream.
    pub fn post_stream(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, A2aError>> + Send>> {
        let url = self.url(path);
        let client = self.client.clone();

        Box::pin(async_stream::stream! {
            let resp = match client
                .post(&url)
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(A2aError::internal(format!("REST stream request failed: {e}")));
                    return;
                }
            };

            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    yield Err(A2aError::internal(format!("Failed to read stream: {e}")));
                    return;
                }
            };

            // Parse as JSON array of StreamEvents (REST streaming response)
            if let Ok(events) = serde_json::from_str::<Vec<StreamEvent>>(&text) {
                for event in events {
                    yield Ok(event);
                }
            } else {
                // Try SSE format
                use futures::StreamExt;
                let mut stream = std::pin::pin!(crate::client::sse::parse_sse_stream(text));
                while let Some(item) = stream.next().await {
                    yield item;
                }
            }
        })
    }

    /// GET returning SSE stream.
    pub fn get_stream(
        &self,
        path: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, A2aError>> + Send>> {
        let url = self.url(path);
        let client = self.client.clone();

        Box::pin(async_stream::stream! {
            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield Err(A2aError::internal(format!("REST stream GET failed: {e}")));
                    return;
                }
            };

            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    yield Err(A2aError::internal(format!("Failed to read stream: {e}")));
                    return;
                }
            };

            if let Ok(events) = serde_json::from_str::<Vec<StreamEvent>>(&text) {
                for event in events {
                    yield Ok(event);
                }
            } else {
                use futures::StreamExt;
                let mut stream = std::pin::pin!(crate::client::sse::parse_sse_stream(text));
                while let Some(item) = stream.next().await {
                    yield item;
                }
            }
        })
    }
}
