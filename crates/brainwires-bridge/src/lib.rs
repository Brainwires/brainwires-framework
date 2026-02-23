pub mod connection;
pub mod error;
pub mod handler;
pub mod middleware;
pub mod registry;
pub mod server;
pub mod transport;

pub use connection::{ClientInfo, RequestContext};
pub use error::BridgeError;
pub use handler::McpHandler;
pub use middleware::{Middleware, MiddlewareChain, MiddlewareResult};
pub use registry::{McpToolDef, McpToolRegistry, ToolHandler};
pub use server::McpServer;
pub use transport::{ServerTransport, StdioServerTransport};

// Re-export middleware implementations
pub use middleware::auth::AuthMiddleware;
pub use middleware::logging::LoggingMiddleware;
pub use middleware::rate_limit::RateLimitMiddleware;
pub use middleware::tool_filter::ToolFilterMiddleware;
