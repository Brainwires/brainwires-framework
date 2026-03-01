use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::error::RelayClientError;
use super::protocol;

pub struct RelayClient {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    request_id: AtomicU64,
    initialized: bool,
}

impl RelayClient {
    pub async fn connect(binary_path: &str) -> Result<Self, RelayClientError> {
        Self::connect_with_args(binary_path, &["chat", "--mcp-server"]).await
    }

    pub async fn connect_with_args(
        binary_path: &str,
        args: &[&str],
    ) -> Result<Self, RelayClientError> {
        let mut child = Command::new(binary_path)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(RelayClientError::SpawnFailed)?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| RelayClientError::Protocol("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RelayClientError::Protocol("Failed to capture stdout".to_string()))?;

        Ok(Self {
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
            request_id: AtomicU64::new(1),
            initialized: false,
        })
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, RelayClientError> {
        let id = self.next_id();
        let request = brainwires_mcp::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(id),
            method: method.to_string(),
            params,
        };

        let json = serde_json::to_string(&request)?;
        self.stdin
            .write_all(format!("{json}\n").as_bytes())
            .await
            .map_err(|e| RelayClientError::Io(e))?;
        self.stdin.flush().await.map_err(|e| RelayClientError::Io(e))?;

        // Read response
        let mut line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut line)
            .await
            .map_err(|e| RelayClientError::Io(e))?;

        if bytes == 0 {
            return Err(RelayClientError::ProcessExited);
        }

        let response = protocol::parse_response(line.trim())?;
        protocol::extract_result(response)
    }

    pub async fn initialize(&mut self) -> Result<serde_json::Value, RelayClientError> {
        let id = self.next_id();
        let request = protocol::build_initialize_request(id);

        let json = serde_json::to_string(&request)?;
        self.stdin
            .write_all(format!("{json}\n").as_bytes())
            .await
            .map_err(|e| RelayClientError::Io(e))?;
        self.stdin.flush().await.map_err(|e| RelayClientError::Io(e))?;

        // Read initialize response
        let mut line = String::new();
        let bytes = self
            .stdout
            .read_line(&mut line)
            .await
            .map_err(|e| RelayClientError::Io(e))?;

        if bytes == 0 {
            return Err(RelayClientError::ProcessExited);
        }

        let response = protocol::parse_response(line.trim())?;
        let result = protocol::extract_result(response)?;

        // Send initialized notification
        let notif = protocol::build_initialized_notification();
        self.stdin
            .write_all(format!("{notif}\n").as_bytes())
            .await
            .map_err(|e| RelayClientError::Io(e))?;
        self.stdin.flush().await.map_err(|e| RelayClientError::Io(e))?;

        self.initialized = true;
        Ok(result)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, RelayClientError> {
        if !self.initialized {
            return Err(RelayClientError::NotInitialized);
        }

        self.send_request(
            "tools/call",
            Some(serde_json::json!({
                "name": name,
                "arguments": args
            })),
        )
        .await
    }

    pub async fn list_tools(&mut self) -> Result<serde_json::Value, RelayClientError> {
        if !self.initialized {
            return Err(RelayClientError::NotInitialized);
        }
        self.send_request("tools/list", None).await
    }

    pub async fn shutdown(mut self) -> Result<(), RelayClientError> {
        // Close stdin to signal EOF to the child process
        drop(self.stdin);
        // Wait for child to exit (with timeout)
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.child.wait(),
        )
        .await;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
