use async_trait::async_trait;
use brainwires_mcp::{JsonRpcError, JsonRpcRequest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::{Middleware, MiddlewareResult};
use crate::connection::RequestContext;

struct RateLimitBucket {
    tokens: f64,
    last_refill: Instant,
}

pub struct RateLimitMiddleware {
    max_requests_per_second: f64,
    per_tool_limits: HashMap<String, f64>,
    buckets: Arc<Mutex<HashMap<String, RateLimitBucket>>>,
}

impl RateLimitMiddleware {
    pub fn new(max_requests_per_second: f64) -> Self {
        Self {
            max_requests_per_second,
            per_tool_limits: HashMap::new(),
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_tool_limit(mut self, tool_name: &str, limit: f64) -> Self {
        self.per_tool_limits.insert(tool_name.to_string(), limit);
        self
    }

    fn get_limit(&self, key: &str) -> f64 {
        self.per_tool_limits
            .get(key)
            .copied()
            .unwrap_or(self.max_requests_per_second)
    }
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    async fn process_request(
        &self,
        request: &JsonRpcRequest,
        _ctx: &mut RequestContext,
    ) -> MiddlewareResult {
        // Only rate-limit tools/call
        if request.method != "tools/call" {
            return MiddlewareResult::Continue;
        }

        let tool_name = request
            .params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown");

        let limit = self.get_limit(tool_name);
        let key = format!("tool:{tool_name}");

        let mut buckets = self.buckets.lock().await;
        let bucket = buckets.entry(key).or_insert(RateLimitBucket {
            tokens: limit,
            last_refill: Instant::now(),
        });

        // Token bucket refill
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * limit).min(limit);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            MiddlewareResult::Continue
        } else {
            MiddlewareResult::Reject(JsonRpcError {
                code: -32002,
                message: format!("Rate limited: too many requests for tool '{tool_name}'"),
                data: None,
            })
        }
    }
}
