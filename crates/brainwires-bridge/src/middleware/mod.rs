pub mod auth;
pub mod logging;
pub mod rate_limit;
pub mod tool_filter;

use anyhow::Result;
use async_trait::async_trait;
use brainwires_mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

use crate::connection::RequestContext;

pub enum MiddlewareResult {
    Continue,
    Reject(JsonRpcError),
}

#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    async fn process_request(
        &self,
        request: &JsonRpcRequest,
        ctx: &mut RequestContext,
    ) -> MiddlewareResult;

    async fn process_response(&self, _response: &mut JsonRpcResponse, _ctx: &RequestContext) {}
}

pub struct MiddlewareChain {
    layers: Vec<Box<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn add(&mut self, middleware: impl Middleware) {
        self.layers.push(Box::new(middleware));
    }

    pub async fn process_request(
        &self,
        request: &JsonRpcRequest,
        ctx: &mut RequestContext,
    ) -> Result<(), JsonRpcError> {
        for layer in &self.layers {
            match layer.process_request(request, ctx).await {
                MiddlewareResult::Continue => continue,
                MiddlewareResult::Reject(err) => return Err(err),
            }
        }
        Ok(())
    }

    pub async fn process_response(&self, response: &mut JsonRpcResponse, ctx: &RequestContext) {
        for layer in &self.layers {
            layer.process_response(response, ctx).await;
        }
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct PassMiddleware;

    #[async_trait]
    impl Middleware for PassMiddleware {
        async fn process_request(
            &self,
            _request: &JsonRpcRequest,
            _ctx: &mut RequestContext,
        ) -> MiddlewareResult {
            MiddlewareResult::Continue
        }
    }

    struct RejectMiddleware;

    #[async_trait]
    impl Middleware for RejectMiddleware {
        async fn process_request(
            &self,
            _request: &JsonRpcRequest,
            _ctx: &mut RequestContext,
        ) -> MiddlewareResult {
            MiddlewareResult::Reject(JsonRpcError {
                code: -32003,
                message: "Rejected".to_string(),
                data: None,
            })
        }
    }

    #[tokio::test]
    async fn test_chain_all_pass() {
        let mut chain = MiddlewareChain::new();
        chain.add(PassMiddleware);
        chain.add(PassMiddleware);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "test".to_string(),
            params: None,
        };
        let mut ctx = RequestContext::new(json!(1));
        assert!(chain.process_request(&request, &mut ctx).await.is_ok());
    }

    #[tokio::test]
    async fn test_chain_reject_stops() {
        let mut chain = MiddlewareChain::new();
        chain.add(PassMiddleware);
        chain.add(RejectMiddleware);
        chain.add(PassMiddleware);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(1),
            method: "test".to_string(),
            params: None,
        };
        let mut ctx = RequestContext::new(json!(1));
        let result = chain.process_request(&request, &mut ctx).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, -32003);
    }
}
