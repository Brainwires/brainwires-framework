//! # brainwires-proxy
//!
//! Protocol-agnostic proxy framework for debugging app traffic.
//!
//! Compose transports, middleware, converters, and inspectors to build
//! custom debugging proxies for any protocol.
//!
//! ## Features
//!
//! - **`http`** (default) — HTTP/HTTPS transport via hyper
//! - **`websocket`** — WebSocket transport via tokio-tungstenite
//! - **`tls`** — TLS termination via tokio-rustls
//! - **`inspector-api`** — HTTP query API for captured traffic
//! - **`full`** — All features enabled
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use brainwires_proxy::builder::ProxyBuilder;
//!
//! # async fn example() -> brainwires_proxy::error::ProxyResult<()> {
//! let proxy = ProxyBuilder::new()
//!     .listen_on("127.0.0.1:8080")
//!     .upstream_url("http://localhost:3000")
//!     .build()?;
//!
//! proxy.run().await
//! # }
//! ```

pub mod error;
pub mod config;
pub mod types;
pub mod request_id;

pub mod transport;
pub mod middleware;
pub mod convert;
pub mod inspector;

pub mod proxy;
pub mod builder;

/// Convenience re-exports.
pub mod prelude {
    pub use crate::error::{ProxyError, ProxyResult};
    pub use crate::config::ProxyConfig;
    pub use crate::types::{
        ProxyRequest, ProxyResponse, ProxyBody, TransportKind, FormatId, Extensions,
    };
    pub use crate::request_id::RequestId;
    pub use crate::middleware::{ProxyLayer, LayerAction, MiddlewareStack};
    pub use crate::transport::{TransportListener, TransportConnector};
    pub use crate::convert::{Converter, StreamConverter, ConversionRegistry, FormatDetector};
    pub use crate::builder::ProxyBuilder;
    pub use crate::proxy::ProxyService;
}
