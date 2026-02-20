use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::types::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// Transport layer for MCP communication
#[derive(Debug)]
pub enum Transport {
    Stdio(StdioTransport),
}

impl Transport {
    /// Send a JSON-RPC request
    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<()> {
        match self {
            Transport::Stdio(transport) => transport.send_request(request).await,
        }
    }

    /// Receive a JSON-RPC response
    pub async fn receive_response(&mut self) -> Result<JsonRpcResponse> {
        match self {
            Transport::Stdio(transport) => transport.receive_response().await,
        }
    }

    /// Receive any JSON-RPC message (response or notification)
    /// This is used for bidirectional communication where servers can send notifications
    pub async fn receive_message(&mut self) -> Result<JsonRpcMessage> {
        match self {
            Transport::Stdio(transport) => transport.receive_message().await,
        }
    }

    /// Close the transport
    pub async fn close(&mut self) -> Result<()> {
        match self {
            Transport::Stdio(transport) => transport.close().await,
        }
    }
}

/// Stdio transport for communicating with MCP servers via stdin/stdout
#[derive(Debug)]
pub struct StdioTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    child: Arc<Mutex<Child>>,
}

impl StdioTransport {
    /// Create a new stdio transport by spawning a command
    pub async fn new(command: &str, args: &[String]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .context(format!("Failed to spawn MCP server: {}", command))?;

        let stdin = child
            .stdin
            .take()
            .context("Failed to get stdin handle")?;

        let stdout = child
            .stdout
            .take()
            .context("Failed to get stdout handle")?;

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            child: Arc::new(Mutex::new(child)),
        })
    }

    /// Send a JSON-RPC request via stdin
    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<()> {
        let json = serde_json::to_string(request)
            .context("Failed to serialize JSON-RPC request")?;

        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(json.as_bytes())
            .await
            .context("Failed to write to stdin")?;
        stdin
            .write_all(b"\n")
            .await
            .context("Failed to write newline")?;
        stdin.flush().await.context("Failed to flush stdin")?;

        Ok(())
    }

    /// Receive a JSON-RPC response from stdout
    pub async fn receive_response(&mut self) -> Result<JsonRpcResponse> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();

        stdout
            .read_line(&mut line)
            .await
            .context("Failed to read from stdout")?;

        if line.is_empty() {
            anyhow::bail!("EOF reached, server closed");
        }

        serde_json::from_str(&line).context("Failed to parse JSON-RPC response")
    }

    /// Receive any JSON-RPC message from stdout (response or notification)
    /// Discriminates based on presence of "id" field:
    /// - If "id" is present and not null: Response
    /// - If "id" is missing or null: Notification
    pub async fn receive_message(&mut self) -> Result<JsonRpcMessage> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();

        match stdout.read_line(&mut line).await {
            Ok(0) => {
                // EOF - server closed connection
                anyhow::bail!("MCP server closed connection (EOF on stdout)");
            }
            Ok(_) => {
                // Successfully read a line
            }
            Err(e) => {
                // Check for specific error types
                let error_msg = if e.kind() == std::io::ErrorKind::BrokenPipe {
                    format!("MCP server process terminated unexpectedly (broken pipe). The server may have crashed during tool execution. Check stderr output for panic messages.")
                } else if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    "MCP server process exited unexpectedly (unexpected EOF)".to_string()
                } else {
                    format!("Failed to read from MCP server stdout: {} (kind: {:?})", e, e.kind())
                };
                anyhow::bail!("{}", error_msg);
            }
        }

        if line.is_empty() {
            anyhow::bail!("MCP server returned empty response");
        }

        // Parse as generic JSON first to check structure
        let value: serde_json::Value =
            serde_json::from_str(&line).context("Failed to parse JSON-RPC message")?;

        // Discriminate based on "id" field
        // Responses have a non-null "id", notifications either lack "id" or have null
        let has_valid_id = value
            .get("id")
            .map(|id| !id.is_null())
            .unwrap_or(false);

        if has_valid_id {
            // This is a response
            let response: JsonRpcResponse = serde_json::from_value(value)
                .context("Failed to parse as JSON-RPC response")?;
            Ok(JsonRpcMessage::Response(response))
        } else {
            // This is a notification
            let notification: JsonRpcNotification = serde_json::from_value(value)
                .context("Failed to parse as JSON-RPC notification")?;
            Ok(JsonRpcMessage::Notification(notification))
        }
    }

    /// Close the transport and kill the child process
    pub async fn close(&mut self) -> Result<()> {
        let mut child = self.child.lock().await;

        child
            .kill()
            .await
            .context("Failed to kill MCP server process")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_stdio_transport_echo() {
        // Test with echo command (simple test)
        let result = StdioTransport::new("echo", &["test".to_string()]).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_rpc_serialization() {
        let request = JsonRpcRequest::new(
            1,
            "initialize".to_string(),
            Some(json!({"test": "value"})),
        )
        .unwrap();

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("jsonrpc"));
        assert!(json.contains("2.0"));
        assert!(json.contains("initialize"));
    }
}
