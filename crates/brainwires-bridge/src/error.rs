use brainwires_mcp::JsonRpcError;

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Unauthorized")]
    Unauthorized,
}

impl BridgeError {
    pub fn to_json_rpc_error(&self) -> JsonRpcError {
        match self {
            BridgeError::ParseError(msg) => JsonRpcError {
                code: -32700,
                message: msg.clone(),
                data: None,
            },
            BridgeError::MethodNotFound(method) => JsonRpcError {
                code: -32601,
                message: format!("Method not found: {method}"),
                data: None,
            },
            BridgeError::InvalidParams(msg) => JsonRpcError {
                code: -32602,
                message: msg.clone(),
                data: None,
            },
            BridgeError::Internal(err) => JsonRpcError {
                code: -32603,
                message: err.to_string(),
                data: None,
            },
            BridgeError::Transport(msg) => JsonRpcError {
                code: -32000,
                message: format!("Transport error: {msg}"),
                data: None,
            },
            BridgeError::ToolNotFound(name) => JsonRpcError {
                code: -32001,
                message: format!("Tool not found: {name}"),
                data: None,
            },
            BridgeError::RateLimited => JsonRpcError {
                code: -32002,
                message: "Rate limited".to_string(),
                data: None,
            },
            BridgeError::Unauthorized => JsonRpcError {
                code: -32003,
                message: "Unauthorized".to_string(),
                data: None,
            },
        }
    }
}
