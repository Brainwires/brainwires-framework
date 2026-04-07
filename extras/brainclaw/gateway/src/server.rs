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
use crate::openai_compat;
use crate::channel_registry::ChannelRegistry;
use crate::config::GatewayConfig;
use crate::cron::CronStore;
use crate::identity::UserIdentityStore;
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
    /// Optional pre-built session manager to share with the handler.
    shared_sessions: Option<Arc<SessionManager>>,
    /// Optional pre-built channel registry to share with the handler.
    shared_channels: Option<Arc<ChannelRegistry>>,
    /// Optional LLM provider for the OpenAI-compatible API endpoint.
    openai_provider: Option<Arc<dyn brainwires_core::Provider>>,
    /// Optional directory for serving TTS audio files at `/audio/<filename>`.
    audio_dir: Option<std::path::PathBuf>,
    /// Optional cron store to expose the admin cron API.
    cron_store: Option<Arc<CronStore>>,
    /// Optional pre-created metrics instance to share with the handler.
    shared_metrics: Option<Arc<MetricsCollector>>,
    /// Optional cross-channel identity store.
    identity_store: Option<Arc<UserIdentityStore>>,
}

impl Gateway {
    /// Create a new gateway with the given configuration.
    ///
    /// Uses the default [`MessageRouter`] for inbound event handling.
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            custom_handler: None,
            shared_sessions: None,
            shared_channels: None,
            openai_provider: None,
            audio_dir: None,
            cron_store: None,
            shared_metrics: None,
            identity_store: None,
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
            shared_sessions: None,
            shared_channels: None,
            openai_provider: None,
            audio_dir: None,
            cron_store: None,
            shared_metrics: None,
            identity_store: None,
        }
    }

    /// Attach a cross-channel user identity store.
    ///
    /// When set, the admin API exposes `/admin/identity` endpoints for
    /// linking and unlinking platform user identities, and the `AppState`
    /// carries the store so downstream consumers can access it.
    pub fn with_identity_store(mut self, store: Arc<UserIdentityStore>) -> Self {
        self.identity_store = Some(store);
        self
    }

    /// Share a pre-created metrics collector so the handler and the admin API
    /// reference the same counters.
    ///
    /// When not set, the gateway creates its own `MetricsCollector` internally.
    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.shared_metrics = Some(metrics);
        self
    }

    /// Attach a cron store to expose admin cron API endpoints.
    ///
    /// When set and `admin_enabled` is true, the gateway exposes
    /// `GET/POST/DELETE /admin/cron` and `GET/DELETE /admin/cron/:id`.
    pub fn with_cron_store(mut self, store: Arc<CronStore>) -> Self {
        self.cron_store = Some(store);
        self
    }

    /// Serve TTS audio files at `/audio/<filename>`.
    ///
    /// When set, files in `dir` are served at `/audio/<filename>`.
    /// The TTS processor writes audio files here; channel adapters send the URL.
    pub fn with_audio_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.audio_dir = Some(dir);
        self
    }

    /// Attach an LLM provider to expose the OpenAI-compatible API endpoint.
    ///
    /// When set, the gateway exposes `/v1/chat/completions`, `/v1/models`,
    /// and `/v1/embeddings` endpoints that proxy requests to this provider.
    pub fn with_openai_provider(
        mut self,
        provider: Arc<dyn brainwires_core::Provider>,
    ) -> Self {
        self.openai_provider = Some(provider);
        self
    }

    /// Share pre-built session manager and channel registry with the gateway.
    ///
    /// When set, the gateway uses these instances in `AppState` so that the
    /// custom handler and the WS/admin routes all reference the same objects.
    /// This is required when using `with_handler` with an `AgentInboundHandler`
    /// that was constructed with specific `Arc<SessionManager>` /
    /// `Arc<ChannelRegistry>` instances.
    pub fn with_shared_state(
        mut self,
        sessions: Arc<SessionManager>,
        channels: Arc<ChannelRegistry>,
    ) -> Self {
        self.shared_sessions = Some(sessions);
        self.shared_channels = Some(channels);
        self
    }

    /// Build and run the gateway server.
    ///
    /// This method blocks until the server is shut down.
    pub async fn run(&self) -> Result<()> {
        let sessions = self
            .shared_sessions
            .clone()
            .unwrap_or_else(|| Arc::new(SessionManager::new()));
        let channels = self
            .shared_channels
            .clone()
            .unwrap_or_else(|| Arc::new(ChannelRegistry::new()));

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

        let metrics = self.shared_metrics.clone().unwrap_or_else(|| {
            let m = MetricsCollector::new();
            #[cfg(feature = "telemetry")]
            let m = {
                use brainwires_telemetry::{AnalyticsCollector, SqliteAnalyticsSink};
                match SqliteAnalyticsSink::new_default() {
                    Ok(sink) => {
                        let collector = std::sync::Arc::new(AnalyticsCollector::new(vec![Box::new(sink)]));
                        m.with_analytics(collector)
                    }
                    Err(e) => {
                        tracing::warn!("[analytics] Failed to open analytics database: {e}");
                        m
                    }
                }
            };
            Arc::new(m)
        });

        let state = AppState {
            config: Arc::new(self.config.clone()),
            sessions,
            channels,
            router,
            sanitizer,
            rate_limiter,
            audit: Arc::new(AuditLogger::new()),
            metrics,
            start_time: Utc::now(),
            openai_provider: self.openai_provider.clone(),
            audio_dir: self.audio_dir.clone(),
            cron_store: self.cron_store.clone(),
            identity_store: self.identity_store.clone(),
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
                &format!("{}/ui", admin_prefix),
                get(webchat::serve_admin_ui_page),
            )
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
            )
            .route(
                &format!("{}/metrics", admin_prefix),
                get(admin::get_metrics),
            );

        // Cron admin API (only when a cron store is wired in)
        if state.cron_store.is_some() {
            app = app
                .route(
                    &format!("{}/cron", admin_prefix),
                    get(admin::list_cron_jobs).post(admin::create_cron_job),
                )
                .route(
                    &format!("{}/cron/:id", admin_prefix),
                    get(admin::get_cron_job)
                        .put(admin::update_cron_job)
                        .delete(admin::delete_cron_job),
                );
        }

        // Identity admin API (only when an identity store is wired in)
        if state.identity_store.is_some() {
            app = app
                .route(
                    &format!("{}/identity", admin_prefix),
                    get(admin::list_identities),
                )
                .route(
                    &format!("{}/identity/link", admin_prefix),
                    post(admin::link_identities),
                )
                .route(
                    &format!("{}/identity/unlink", admin_prefix),
                    axum::routing::delete(admin::unlink_identity),
                );
        }
    }

    // OpenAI-compatible API endpoint (always enabled when provider is configured)
    if state.openai_provider.is_some() {
        app = app
            .route("/v1/models", get(openai_compat::list_models))
            .route("/v1/chat/completions", post(openai_compat::chat_completions))
            .route("/v1/embeddings", post(openai_compat::embeddings));
        tracing::debug!("OpenAI-compatible API endpoint enabled at /v1/");
    }

    // Audio file serving endpoint (TTS output)
    if state.audio_dir.is_some() {
        app = app.route("/audio/{filename}", get(serve_audio_file));
        tracing::debug!("Audio file serving enabled at /audio/");
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

/// Serve TTS-generated audio files at `/audio/<filename>`.
///
/// Reads the file from `state.audio_dir/<filename>`, guesses the MIME type
/// from the extension, and returns the raw bytes with an appropriate
/// `Content-Type` header.  Returns 404 if not found or 403 if the path
/// contains directory traversal.
async fn serve_audio_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    use axum::http::{header, StatusCode};

    // Reject any path that tries to escape the audio directory
    if filename.contains('/') || filename.contains("..") {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    let audio_dir = match &state.audio_dir {
        Some(d) => d,
        None => return (StatusCode::NOT_FOUND, "Not found").into_response(),
    };

    let path = audio_dir.join(&filename);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mime = if filename.ends_with(".mp3") {
                "audio/mpeg"
            } else if filename.ends_with(".opus") {
                "audio/opus"
            } else if filename.ends_with(".flac") {
                "audio/flac"
            } else {
                "audio/wav"
            };
            (
                [(header::CONTENT_TYPE, mime)],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
