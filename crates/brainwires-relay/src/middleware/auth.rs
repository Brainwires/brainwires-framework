use async_trait::async_trait;
use brainwires_mcp::{JsonRpcError, JsonRpcRequest};

use super::{Middleware, MiddlewareResult};
use crate::connection::RequestContext;

pub struct AuthMiddleware {
    token: String,
}

impl AuthMiddleware {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn process_request(
        &self,
        request: &JsonRpcRequest,
        ctx: &mut RequestContext,
    ) -> MiddlewareResult {
        // Skip auth for initialize - clients haven't authenticated yet
        if request.method == "initialize" {
            return MiddlewareResult::Continue;
        }

        // Check for token in metadata (set during initialize)
        if let Some(serde_json::Value::String(token)) = ctx.get_metadata("auth_token") {
            if token == &self.token {
                return MiddlewareResult::Continue;
            }
        }

        // Check params for auth token
        if let Some(params) = &request.params {
            if let Some(token) = params.get("_auth_token").and_then(|v| v.as_str()) {
                if token == self.token {
                    ctx.set_metadata(
                        "auth_token".to_string(),
                        serde_json::Value::String(token.to_string()),
                    );
                    return MiddlewareResult::Continue;
                }
            }
        }

        MiddlewareResult::Reject(JsonRpcError {
            code: -32003,
            message: "Unauthorized: invalid or missing auth token".to_string(),
            data: None,
        })
    }
}
