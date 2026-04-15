//! brainwires-memory — Mem0-compatible memory service.
//!
//! Usage:
//!   brainwires-memory [--host 0.0.0.0] [--port 8765] [--db ./memories.db]

use std::net::SocketAddr;

use anyhow::Context;
use brainwires_memory_service::{build_app, store::MemoryStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "brainwires_memory_service=info,tower_http=debug".to_string()),
        )
        .init();

    let host = std::env::var("MEMORY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("MEMORY_PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse()
        .context("MEMORY_PORT must be a valid port number")?;
    let db_path = std::env::var("MEMORY_DB").unwrap_or_else(|_| {
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("brainwires")
            .join("memories.db")
            .to_string_lossy()
            .to_string()
    });

    tracing::warn!(
        "brainwires-memory-service has no authentication or tenant isolation \
         — intended for local development use only"
    );
    tracing::info!("Opening memory database at {db_path}");
    let store = MemoryStore::open(&db_path)?;
    let app = build_app(store);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    tracing::info!("brainwires-memory listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
