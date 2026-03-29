//! Shared application state for the gateway.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use brainwires_core::Provider;

use crate::audit::AuditLogger;
use crate::channel_registry::ChannelRegistry;
use crate::config::GatewayConfig;
use crate::metrics::MetricsCollector;
use crate::middleware::rate_limit::RateLimiter;
use crate::middleware::sanitizer::MessageSanitizer;
use crate::router::InboundHandler;
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
    /// Inbound event handler (trait object for extensibility).
    pub router: Arc<dyn InboundHandler>,
    /// Message sanitizer for inbound/outbound security.
    pub sanitizer: Arc<MessageSanitizer>,
    /// Per-user rate limiter.
    pub rate_limiter: Arc<RateLimiter>,
    /// Audit logger for security events.
    pub audit: Arc<AuditLogger>,
    /// In-memory metrics collector.
    pub metrics: Arc<MetricsCollector>,
    /// When the gateway was started.
    pub start_time: DateTime<Utc>,
    /// Optional LLM provider for the OpenAI-compatible API endpoint.
    pub openai_provider: Option<Arc<dyn Provider>>,
    /// Optional directory for serving TTS audio files at `/audio/<filename>`.
    pub audio_dir: Option<std::path::PathBuf>,
}
