//! Security middleware for the gateway.
//!
//! Provides message sanitization, WebSocket origin validation, and rate limiting.

pub mod origin;
pub mod rate_limit;
pub mod sanitizer;

pub use origin::OriginValidator;
pub use rate_limit::RateLimiter;
pub use sanitizer::MessageSanitizer;
