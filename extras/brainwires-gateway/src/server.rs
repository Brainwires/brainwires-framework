//! Axum server setup and route definitions for the gateway.

use std::sync::Arc;

use anyhow::Result;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use tokio::net::TcpListener;

use crate::admin;
use crate::channel_registry::ChannelRegistry;
use crate::config::GatewayConfig;
use crate::router::MessageRouter;
use crate::session::SessionManager;
use crate::state::AppState;
use crate::webhook;
use crate::ws_handler;

/// The gateway server.
pub struct Gateway {
    config: GatewayConfig,
}

impl Gateway {
    /// Create a new gateway with the given configuration.
    pub fn new(config: GatewayConfig) -> Self {
        Self { config }
    }

    /// Build and run the gateway server.
    ///
    /// This method blocks until the server is shut down.
    pub async fn run(&self) -> Result<()> {
        let sessions = Arc::new(SessionManager::new());
        let channels = Arc::new(ChannelRegistry::new());
        let router = Arc::new(MessageRouter::new(
            Arc::clone(&sessions),
            Arc::clone(&channels),
        ));

        let state = AppState {
            config: Arc::new(self.config.clone()),
            sessions,
            channels,
            router,
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
async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_handler::handle_ws_connection(socket, state))
}
