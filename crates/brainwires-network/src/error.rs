use brainwires_mcp::JsonRpcError;

/// Errors that can occur in the agent network layer.
#[derive(Debug, thiserror::Error)]
pub enum AgentNetworkError {
    /// JSON-RPC parse error.
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Requested method does not exist.
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    /// Invalid parameters supplied.
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
    /// Transport-level error.
    #[error("Transport error: {0}")]
    Transport(String),
    /// Requested tool does not exist.
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    /// Request was rate-limited.
    #[error("Rate limited")]
    RateLimited,
    /// Request was not authorized.
    #[error("Unauthorized")]
    Unauthorized,
}

impl AgentNetworkError {
    /// Convert to a JSON-RPC error with the appropriate code.
    pub fn to_json_rpc_error(&self) -> JsonRpcError {
        match self {
            AgentNetworkError::ParseError(msg) => JsonRpcError {
                code: -32700,
                message: msg.clone(),
                data: None,
            },
            AgentNetworkError::MethodNotFound(method) => JsonRpcError {
                code: -32601,
                message: format!("Method not found: {method}"),
                data: None,
            },
            AgentNetworkError::InvalidParams(msg) => JsonRpcError {
                code: -32602,
                message: msg.clone(),
                data: None,
            },
            AgentNetworkError::Internal(err) => JsonRpcError {
                code: -32603,
                message: err.to_string(),
                data: None,
            },
            AgentNetworkError::Transport(msg) => JsonRpcError {
                code: -32000,
                message: format!("Transport error: {msg}"),
                data: None,
            },
            AgentNetworkError::ToolNotFound(name) => JsonRpcError {
                code: -32001,
                message: format!("Tool not found: {name}"),
                data: None,
            },
            AgentNetworkError::RateLimited => JsonRpcError {
                code: -32002,
                message: "Rate limited".to_string(),
                data: None,
            },
            AgentNetworkError::Unauthorized => JsonRpcError {
                code: -32003,
                message: "Unauthorized".to_string(),
                data: None,
            },
        }
    }
}
