#[derive(Debug, thiserror::Error)]
pub enum BridgeClientError {
    #[error("Failed to spawn bridge process: {0}")]
    SpawnFailed(#[source] std::io::Error),
    #[error("Bridge process exited unexpectedly")]
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
