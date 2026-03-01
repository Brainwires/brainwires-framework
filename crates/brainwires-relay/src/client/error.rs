#[derive(Debug, thiserror::Error)]
pub enum RelayClientError {
    #[error("Failed to spawn relay process: {0}")]
    SpawnFailed(#[source] std::io::Error),
    #[error("Relay process exited unexpectedly")]
    ProcessExited,
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc { code: i32, message: String },
    #[error("Timeout after {0} seconds")]
    Timeout(u64),
    #[error("Not initialized - call initialize() first")]
    NotInitialized,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
