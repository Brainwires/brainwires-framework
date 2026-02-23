use brainwires_mcp::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{json, Value};

use crate::error::BridgeClientError;

pub fn build_initialize_request(id: u64) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(id),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "brainwires-bridge-client",
                "version": "0.1.0"
            }
        })),
    }
}

pub fn build_initialized_notification() -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }))
    .expect("Failed to serialize initialized notification")
}

pub fn build_tools_list_request(id: u64) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(id),
        method: "tools/list".to_string(),
        params: None,
    }
}

pub fn build_tools_call_request(id: u64, name: &str, args: Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(id),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": name,
            "arguments": args
        })),
    }
}

pub fn parse_response(line: &str) -> Result<JsonRpcResponse, BridgeClientError> {
    serde_json::from_str(line).map_err(|e| {
        BridgeClientError::Protocol(format!("Failed to parse response: {e}: {line}"))
    })
}

pub fn extract_result(response: JsonRpcResponse) -> Result<Value, BridgeClientError> {
    if let Some(error) = response.error {
        return Err(BridgeClientError::JsonRpc {
            code: error.code,
            message: error.message,
        });
    }
    Ok(response.result.unwrap_or(json!(null)))
}
