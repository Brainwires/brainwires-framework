//! Axum webhook server for receiving inbound WhatsApp messages from Meta.
//!
//! Meta sends:
//! - `GET /webhook` — hub challenge verification
//! - `POST /webhook` — inbound message events

use std::sync::Arc;

use axum::{
    Router,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tokio::sync::mpsc;

use brainwires_channels::ChannelEvent;

use crate::whatsapp::parse_webhook_messages;

/// Shared state for the Axum webhook handlers.
pub struct WebhookState {
    /// Sender for inbound channel events.
    pub event_tx: mpsc::Sender<ChannelEvent>,
    /// Webhook verify token for hub challenge.
    pub verify_token: String,
    /// Optional app secret for HMAC-SHA256 signature validation.
    pub app_secret: Option<String>,
    /// Our phone number ID (used to populate ConversationId.server_id).
    pub phone_number_id: String,
}

/// Query parameters for the GET challenge request.
#[derive(Debug, Deserialize)]
pub struct HubQuery {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// GET /webhook — responds to Meta's hub challenge during webhook registration.
async fn verify_webhook(
    State(state): State<Arc<WebhookState>>,
    Query(params): Query<HubQuery>,
) -> impl IntoResponse {
    if params.mode.as_deref() == Some("subscribe")
        && params.verify_token.as_deref() == Some(state.verify_token.as_str())
    {
        let challenge = params.challenge.unwrap_or_default();
        tracing::info!("Webhook challenge verified");
        (StatusCode::OK, challenge)
    } else {
        tracing::warn!("Webhook challenge verification failed");
        (StatusCode::FORBIDDEN, "Forbidden".to_string())
    }
}

/// POST /webhook — receives inbound messages from Meta.
async fn receive_webhook(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // Validate signature if app_secret is configured
    if let Some(ref secret) = state.app_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("sha256="))
            .unwrap_or("");

        if !verify_signature(secret.as_bytes(), &body, signature) {
            tracing::warn!("Invalid webhook signature; rejecting request");
            return StatusCode::UNAUTHORIZED;
        }
    }

    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse webhook payload");
            return StatusCode::BAD_REQUEST;
        }
    };

    let messages = parse_webhook_messages(&payload, &state.phone_number_id);

    for msg in messages {
        tracing::debug!(
            from = %msg.author,
            "Received WhatsApp message"
        );
        let event = ChannelEvent::MessageReceived(msg);
        if let Err(e) = state.event_tx.try_send(event) {
            tracing::warn!(error = %e, "Failed to forward WhatsApp event to channel");
        }
    }

    StatusCode::OK
}

/// Build the Axum router for the webhook server.
pub fn build_router(state: Arc<WebhookState>) -> Router {
    Router::new()
        .route("/webhook", get(verify_webhook))
        .route("/webhook", post(receive_webhook))
        .with_state(state)
}

/// Verify the `X-Hub-Signature-256` HMAC-SHA256 signature.
fn verify_signature(secret: &[u8], body: &[u8], expected_hex: &str) -> bool {
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret) else {
        return false;
    };
    mac.update(body);
    let result = mac.finalize().into_bytes();
    let computed = hex::encode(result);
    // Constant-time comparison to prevent timing attacks
    computed.len() == expected_hex.len()
        && computed
            .bytes()
            .zip(expected_hex.bytes())
            .all(|(a, b)| a == b)
}
