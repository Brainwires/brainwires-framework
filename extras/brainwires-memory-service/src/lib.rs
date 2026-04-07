//! # brainwires-memory-service
//!
//! A Mem0-compatible memory REST API server for Brainwires agents.
//!
//! Provides persistent, per-user memory storage that any agent (or Mem0 SDK
//! client) can read and write via HTTP.
//!
//! ## API surface
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | `POST` | `/v1/memories` | Add one or more memories |
//! | `GET` | `/v1/memories` | List memories (filterable by user/agent/session) |
//! | `GET` | `/v1/memories/{id}` | Get a single memory |
//! | `PATCH` | `/v1/memories/{id}` | Update memory content |
//! | `DELETE` | `/v1/memories/{id}` | Delete a memory |
//! | `DELETE` | `/v1/memories?user_id=…` | Delete all memories for a user |
//! | `POST` | `/v1/memories/search` | Substring / semantic search |
//! | `GET` | `/health` | Health check |

pub mod routes;
pub mod store;
pub mod types;

use axum::{Router, routing};
use store::MemoryStore;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Shared application state injected into every route handler.
#[derive(Clone)]
pub struct AppState {
    /// The underlying memory store.
    pub store: MemoryStore,
}

/// Build the Axum application router.
pub fn build_app(store: MemoryStore) -> Router {
    let state = AppState { store };

    Router::new()
        .route("/health", routing::get(routes::health))
        .route(
            "/v1/memories",
            routing::post(routes::add_memory)
                .get(routes::list_memories)
                .delete(routes::delete_all_memories),
        )
        .route("/v1/memories/search", routing::post(routes::search_memories))
        .route(
            "/v1/memories/{id}",
            routing::get(routes::get_memory)
                .patch(routes::update_memory)
                .delete(routes::delete_memory),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
