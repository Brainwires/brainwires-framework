//! Shared application state for the gateway.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::channel_registry::ChannelRegistry;
use crate::config::GatewayConfig;
use crate::router::MessageRouter;
use crate::session::SessionManager;

/// Shared application state, passed to all axum handlers via Extension.
#[derive(Clone)]
pub struct AppState {
    /// Gateway configuration.
    pub config: Arc<GatewayConfig>,
    /// Session manager for user-to-agent mapping.
    pub sessions: Arc<SessionManager>,
    /// Registry of connected channel adapters.
    pub channels: Arc<ChannelRegistry>,
    /// Message router for inbound/outbound message routing.
    pub router: Arc<MessageRouter>,
    /// When the gateway was started.
    pub start_time: DateTime<Utc>,
}
