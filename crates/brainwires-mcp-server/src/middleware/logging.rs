use async_trait::async_trait;
use brainwires_mcp::{JsonRpcRequest, JsonRpcResponse};

use super::{Middleware, MiddlewareResult};
use crate::connection::RequestContext;

/// Middleware that logs all requests and responses.
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    /// Create a new logging middleware.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(
        &self,
        request: &JsonRpcRequest,
        _ctx: &mut RequestContext,
    ) -> MiddlewareResult {
        tracing::debug!(
            method = %request.method,
            id = %request.id,
            "MCP request received"
        );
        MiddlewareResult::Continue
    }

    async fn process_response(&self, response: &mut JsonRpcResponse, _ctx: &RequestContext) {
        if response.error.is_some() {
            tracing::warn!(
                id = %response.id,
                error = ?response.error,
                "MCP response with error"
            );
        } else {
            tracing::debug!(
                id = %response.id,
                "MCP response sent"
            );
        }
    }
}
