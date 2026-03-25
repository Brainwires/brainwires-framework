//! Admin API handlers for gateway monitoring and control.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::channel_registry::ChannelInfo;
use crate::state::AppState;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Server status.
    pub status: String,
    /// Uptime in seconds.
    pub uptime_secs: i64,
    /// Number of connected channels.
    pub channels_connected: usize,
    /// Number of active sessions.
    pub active_sessions: usize,
}

/// Session info for the admin API (serializable summary).
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    /// Session UUID.
    pub id: String,
    /// Platform name.
    pub platform: String,
    /// Platform user ID.
    pub platform_user_id: String,
    /// Display name.
    pub display_name: String,
    /// Agent session ID.
    pub agent_session_id: String,
    /// When the session was created (ISO 8601).
    pub created_at: String,
    /// When the session was last active (ISO 8601).
    pub last_activity: String,
}

/// Request body for the broadcast endpoint.
#[derive(Debug, Deserialize)]
pub struct BroadcastRequest {
    /// Message content to broadcast.
    pub message: String,
    /// Optional: limit to specific channel type (e.g., "discord").
    /// If None, broadcast to all channels.
    pub channel_type: Option<String>,
}

/// GET /admin/health — health check endpoint.
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = Utc::now() - state.start_time;
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime.num_seconds(),
        channels_connected: state.channels.count(),
        active_sessions: state.sessions.count(),
    })
}

/// GET /admin/channels — list all connected channels.
pub async fn list_channels(State(state): State<AppState>) -> Json<Vec<ChannelInfo>> {
    Json(state.channels.list())
}

/// GET /admin/sessions — list all active sessions.
pub async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    let sessions = state
        .sessions
        .list_sessions()
        .into_iter()
        .map(|s| SessionInfo {
            id: s.id.to_string(),
            platform: s.channel_user.platform,
            platform_user_id: s.channel_user.platform_user_id,
            display_name: s.channel_user.display_name,
            agent_session_id: s.agent_session_id,
            created_at: s.created_at.to_rfc3339(),
            last_activity: s.last_activity.to_rfc3339(),
        })
        .collect();

    Json(sessions)
}

/// POST /admin/broadcast — send a message to all (or filtered) channels.
pub async fn broadcast(
    State(state): State<AppState>,
    Json(payload): Json<BroadcastRequest>,
) -> impl IntoResponse {
    let channels = state.channels.list();
    let mut sent = 0usize;
    let mut failed = 0usize;

    for info in &channels {
        // Filter by channel type if specified
        if let Some(ref ct) = payload.channel_type {
            if info.channel_type != *ct {
                continue;
            }
        }

        if let Some(tx) = state.channels.get_sender(&info.id) {
            match tx.try_send(payload.message.clone()) {
                Ok(()) => sent += 1,
                Err(_) => failed += 1,
            }
        } else {
            failed += 1;
        }
    }

    (
        StatusCode::OK,
        Json(json!({
            "sent": sent,
            "failed": failed,
            "total_channels": channels.len()
        })),
    )
}
