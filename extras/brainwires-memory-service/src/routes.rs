//! Axum route handlers for the Mem0-compatible memory REST API.
//!
//! ## Endpoints
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | `POST` | `/v1/memories` | Add one or more memories |
//! | `GET` | `/v1/memories` | List memories (with filters) |
//! | `GET` | `/v1/memories/{id}` | Get a single memory |
//! | `PATCH` | `/v1/memories/{id}` | Update memory content |
//! | `DELETE` | `/v1/memories/{id}` | Delete a memory |
//! | `DELETE` | `/v1/memories` | Delete all memories for a user |
//! | `POST` | `/v1/memories/search` | Semantic / substring search |
//! | `GET` | `/health` | Health check |

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    types::{
        AddMemoryRequest, AddMemoryResponse, ListMemoriesQuery, ListMemoriesResponse,
        MemoryResult, MessageResponse, SearchMemoriesRequest, SearchMemoriesResponse,
        UpdateMemoryRequest,
    },
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn internal_error(e: anyhow::Error) -> (StatusCode, Json<MessageResponse>) {
    tracing::error!("internal error: {e:#}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(MessageResponse { message: e.to_string() }),
    )
}

fn not_found(id: Uuid) -> (StatusCode, Json<MessageResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(MessageResponse { message: format!("Memory {id} not found") }),
    )
}

// ── Health ────────────────────────────────────────────────────────────────────

/// `GET /health`
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

// ── Add memory ────────────────────────────────────────────────────────────────

/// `POST /v1/memories`
pub async fn add_memory(
    State(state): State<AppState>,
    Json(req): Json<AddMemoryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    // Determine what to store: prefer explicit `memory` field, otherwise
    // concatenate assistant message content from `messages`.
    let contents: Vec<String> = if let Some(direct) = req.memory {
        vec![direct]
    } else {
        req.messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| m.content.clone())
            .collect()
    };

    if contents.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(MessageResponse { message: "No memory content provided".to_string() }),
        ));
    }

    let mut results = Vec::with_capacity(contents.len());
    for content in contents {
        let m = state
            .store
            .add(
                &req.user_id,
                req.agent_id.as_deref(),
                req.session_id.as_deref(),
                &content,
                &req.metadata,
            )
            .map_err(internal_error)?;

        results.push(MemoryResult { memory: m.memory, event: "add".to_string(), id: m.id });
    }

    Ok((StatusCode::CREATED, Json(AddMemoryResponse { results })))
}

// ── List memories ─────────────────────────────────────────────────────────────

/// `GET /v1/memories`
pub async fn list_memories(
    State(state): State<AppState>,
    Query(query): Query<ListMemoriesQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    let page = query.page;
    let page_size = query.page_size;
    let (results, total) = state.store.list(&query).map_err(internal_error)?;

    Ok(Json(ListMemoriesResponse { results, total, page, page_size }))
}

// ── Get memory ────────────────────────────────────────────────────────────────

/// `GET /v1/memories/{id}`
pub async fn get_memory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    match state.store.get(id).map_err(internal_error)? {
        Some(m) => Ok(Json(m)),
        None => Err(not_found(id)),
    }
}

// ── Update memory ─────────────────────────────────────────────────────────────

/// `PATCH /v1/memories/{id}`
pub async fn update_memory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMemoryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    match state.store.update(id, &req.memory).map_err(internal_error)? {
        Some(m) => Ok(Json(m)),
        None => Err(not_found(id)),
    }
}

// ── Delete memory ─────────────────────────────────────────────────────────────

/// `DELETE /v1/memories/{id}`
pub async fn delete_memory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    if state.store.delete(id).map_err(internal_error)? {
        Ok(Json(MessageResponse { message: format!("Memory {id} deleted") }))
    } else {
        Err(not_found(id))
    }
}

// ── Delete all ────────────────────────────────────────────────────────────────

/// `DELETE /v1/memories?user_id={user_id}`
pub async fn delete_all_memories(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    let user_id = params.get("user_id").ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(MessageResponse { message: "user_id query parameter is required".to_string() }),
        )
    })?;

    let count = state.store.delete_all_for_user(user_id).map_err(internal_error)?;
    Ok(Json(MessageResponse {
        message: format!("Deleted {count} memories for user {user_id}"),
    }))
}

// ── Search ────────────────────────────────────────────────────────────────────

/// `POST /v1/memories/search`
pub async fn search_memories(
    State(state): State<AppState>,
    Json(req): Json<SearchMemoriesRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<MessageResponse>)> {
    let results = state.store.search(&req).map_err(internal_error)?;
    Ok(Json(SearchMemoriesResponse { results }))
}
