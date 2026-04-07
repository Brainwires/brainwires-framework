//! WebChat channel handler — serves a built-in chat UI and handles WebSocket
//! connections directly, without requiring an external channel adapter.

use axum::extract::ws::{Message, WebSocket};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use brainwires_network::channels::ChannelCapabilities;
use brainwires_network::channels::events::ChannelEvent;

use crate::channel_registry::ConnectedChannel;
use crate::state::AppState;

/// Serve the static WebChat HTML page at `GET /chat`.
pub async fn serve_webchat_page() -> impl IntoResponse {
    Html(include_str!("../static/webchat.html"))
}

/// Serve the admin UI HTML page at `GET /admin/ui` (or configured admin path + `/ui`).
pub async fn serve_admin_ui_page() -> impl IntoResponse {
    Html(include_str!("../static/admin_ui.html"))
}

/// Handle a WebSocket upgrade for the WebChat channel at `GET /chat/ws`.
pub async fn webchat_ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_webchat_connection(socket, state))
}

/// Handle a WebChat WebSocket connection.
///
/// Unlike the normal `ws_handler`, this does not require an external channel
/// adapter handshake. Instead it:
///
/// 1. Auto-generates a unique webchat user identity.
/// 2. Registers as a "webchat" channel in the [`ChannelRegistry`].
/// 3. Spawns read/write loops that bridge between the browser and the
///    gateway's [`InboundHandler`].
/// 4. Cleans up on disconnect.
async fn handle_webchat_connection(ws: WebSocket, state: AppState) {
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4().to_string();

    tracing::info!(
        channel_id = %channel_id,
        user_id = %user_id,
        "WebChat client connected"
    );

    // Create an mpsc channel for outbound messages (gateway -> browser).
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<String>(256);

    // Register as a "webchat" channel in the registry.
    let connected = ConnectedChannel {
        id: channel_id,
        channel_type: "webchat".to_string(),
        capabilities: ChannelCapabilities::RICH_TEXT | ChannelCapabilities::TYPING_INDICATOR,
        connected_at: Utc::now(),
        last_heartbeat: Utc::now(),
        message_tx: outbound_tx,
    };
    state.channels.register(connected);

    // Split the WebSocket.
    let (mut ws_sender, mut ws_receiver) = ws.split();

    // Writer task: forward outbound messages to the browser.
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = outbound_rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Read loop: receive messages from the browser, parse as ChannelEvent,
    // and route through the inbound handler.
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<ChannelEvent>(&text) {
                    Ok(event) => {
                        let router = state.router.clone();
                        let cid = channel_id;
                        tokio::spawn(async move {
                            if let Err(e) = router.handle_inbound(cid, &event).await {
                                tracing::error!(
                                    channel_id = %cid,
                                    error = %e,
                                    "WebChat: failed to handle inbound event"
                                );
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            channel_id = %channel_id,
                            error = %e,
                            "WebChat: failed to deserialize event"
                        );
                    }
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!(channel_id = %channel_id, "WebChat client sent close frame");
                break;
            }
            Ok(Message::Ping(_)) => {
                state.channels.touch_heartbeat(&channel_id);
            }
            Ok(_) => {
                // Binary or Pong — ignore.
            }
            Err(e) => {
                tracing::warn!(
                    channel_id = %channel_id,
                    error = %e,
                    "WebChat: WebSocket read error"
                );
                break;
            }
        }
    }

    // Cleanup on disconnect.
    writer_handle.abort();
    state.channels.unregister(&channel_id);

    tracing::info!(
        channel_id = %channel_id,
        "WebChat client disconnected"
    );
}

#[cfg(test)]
mod tests {
    /// Verify the static HTML page is non-empty and contains expected content.
    #[test]
    fn webchat_html_is_embedded() {
        let html = include_str!("../static/webchat.html");
        assert!(!html.is_empty());
        assert!(html.contains("BrainClaw Chat"));
        assert!(html.contains("/chat/ws"));
    }

    /// Verify the admin UI HTML is non-empty and contains expected content.
    #[test]
    fn admin_ui_html_is_embedded() {
        let html = include_str!("../static/admin_ui.html");
        assert!(!html.is_empty());
        assert!(html.contains("BrainClaw Admin"));
        assert!(html.contains("adminBase"));
    }

    /// Verify the config flag defaults to true.
    #[test]
    fn webchat_enabled_default() {
        let config = crate::config::GatewayConfig::default();
        assert!(config.webchat_enabled);
    }
}
