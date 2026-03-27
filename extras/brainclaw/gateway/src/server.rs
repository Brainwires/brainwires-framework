//! Axum server setup and route definitions for the gateway.

use std::sync::Arc;

use anyhow::Result;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use tokio::net::TcpListener;

use crate::admin;
use crate::audit::AuditLogger;
use crate::channel_registry::ChannelRegistry;
use crate::config::GatewayConfig;
use crate::metrics::MetricsCollector;
use crate::middleware::rate_limit::RateLimiter;
use crate::middleware::sanitizer::MessageSanitizer;
use crate::router::{InboundHandler, MessageRouter};
use crate::session::SessionManager;
use crate::state::AppState;
use crate::webchat;
use crate::webhook;
use crate::ws_handler;

/// The gateway server.
pub struct Gateway {
    config: GatewayConfig,
    /// Optional custom inbound handler. When `None`, a default [`MessageRouter`] is used.
    custom_handler: Option<Arc<dyn InboundHandler>>,
}

impl Gateway {
    /// Create a new gateway with the given configuration.
    ///
    /// Uses the default [`MessageRouter`] for inbound event handling.
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            custom_handler: None,
        }
    }

    /// Create a new gateway with a custom inbound handler.
    ///
    /// The provided handler will be used instead of the default [`MessageRouter`]
    /// for processing inbound channel events.
    pub fn with_handler(config: GatewayConfig, handler: Arc<dyn InboundHandler>) -> Self {
        Self {
            config,
            custom_handler: Some(handler),
        }
    }

    /// Build and run the gateway server.
    ///
    /// This method blocks until the server is shut down.
    pub async fn run(&self) -> Result<()> {
        let sessions = Arc::new(SessionManager::new());
        let channels = Arc::new(ChannelRegistry::new());

        let router: Arc<dyn InboundHandler> = match &self.custom_handler {
            Some(handler) => Arc::clone(handler),
            None => Arc::new(MessageRouter::new(
                Arc::clone(&sessions),
                Arc::clone(&channels),
            )),
        };

        let sanitizer = Arc::new(MessageSanitizer::new(
            self.config.strip_system_spoofing,
            self.config.redact_secrets_in_output,
        ));
        let rate_limiter = Arc::new(RateLimiter::new(
            self.config.max_messages_per_minute,
            self.config.max_tool_calls_per_minute,
        ));

        let state = AppState {
            config: Arc::new(self.config.clone()),
            sessions,
            channels,
            router,
            sanitizer,
            rate_limiter,
            audit: Arc::new(AuditLogger::new()),
            metrics: Arc::new(MetricsCollector::new()),
            start_time: Utc::now(),
        };

        let app = build_router(state.clone());

        let bind_addr = self.config.bind_address();
        tracing::info!(address = %bind_addr, "Gateway starting");

        let listener = TcpListener::bind(&bind_addr).await?;
        tracing::info!(address = %bind_addr, "Gateway listening");

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Build the axum Router with all routes.
fn build_router(state: AppState) -> Router {
    let mut app = Router::new()
        // WebSocket endpoint for channel connections
        .route("/ws", get(ws_upgrade));

    // Webhook endpoint (conditionally enabled)
    if state.config.webhook_enabled {
        let webhook_path = state.config.webhook_path.clone();
        app = app.route(&webhook_path, post(webhook::handle_webhook));
    }

    // WebChat endpoints (conditionally enabled)
    if state.config.webchat_enabled {
        app = app
            .route("/chat", get(webchat::serve_webchat_page))
            .route("/chat/ws", get(webchat::webchat_ws_handler));
    }

    // Admin endpoints (conditionally enabled)
    if state.config.admin_enabled {
        let admin_prefix = state.config.admin_path.clone();
        app = app
            .route(
                &format!("{}/health", admin_prefix),
                get(admin::health_check),
            )
            .route(
                &format!("{}/channels", admin_prefix),
                get(admin::list_channels),
            )
            .route(
                &format!("{}/sessions", admin_prefix),
                get(admin::list_sessions),
            )
            .route(
                &format!("{}/broadcast", admin_prefix),
                post(admin::broadcast),
            );
    }

    app.with_state(state)
}

/// Handler for WebSocket upgrade requests at `/ws`.
///
/// Validates the `Origin` header against the configured allow-list before
/// upgrading the connection.
async fn ws_upgrade(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use crate::middleware::OriginValidator;

    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok());

    let validator = OriginValidator::new(state.config.allowed_origins.clone());
    if !validator.validate(origin) {
        tracing::warn!(
            origin = ?origin,
            "WebSocket upgrade rejected: origin not allowed"
        );
        return axum::http::StatusCode::FORBIDDEN.into_response();
    }

    ws.on_upgrade(move |socket| ws_handler::handle_ws_connection(socket, state))
        .into_response()
}
