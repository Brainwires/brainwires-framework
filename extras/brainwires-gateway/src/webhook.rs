//! Webhook handler for HTTP-based channel integrations.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::Value;

use brainwires_channels::events::ChannelEvent;

use crate::state::AppState;

/// Handle an incoming webhook POST request.
///
/// Parses the JSON payload as a `ChannelEvent` and routes it through the
/// message router. Returns 200 OK on success or an appropriate error.
pub async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    if !state.config.webhook_enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Webhook endpoint is disabled" })),
        );
    }

    // Try to parse the payload as a ChannelEvent
    let event: ChannelEvent = match serde_json::from_value(payload.clone()) {
        Ok(event) => event,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse webhook payload as ChannelEvent");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid payload",
                    "details": e.to_string()
                })),
            );
        }
    };

    // Use a synthetic channel ID of all-zeros for webhook-sourced events
    let webhook_channel_id = uuid::Uuid::nil();

    if let Err(e) = state.router.route_inbound(webhook_channel_id, &event) {
        tracing::error!(error = %e, "Failed to route webhook event");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to process event",
                "details": e.to_string()
            })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok" })),
    )
}
