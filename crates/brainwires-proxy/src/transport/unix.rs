//! Unix domain socket proxy transport.

use crate::error::ProxyResult;
use crate::request_id::RequestId;
use crate::transport::{InboundConnection, TransportListener};
use crate::types::{ProxyBody, ProxyRequest, TransportKind, Extensions};

use http::{Method, Uri};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot, watch};

/// Unix domain socket listener.
pub struct UnixListener {
    path: PathBuf,
    max_read: usize,
}

impl UnixListener {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_read: 10 * 1024 * 1024,
        }
    }

    pub fn with_max_read(mut self, max: usize) -> Self {
        self.max_read = max;
        self
    }
}

#[async_trait::async_trait]
impl TransportListener for UnixListener {
    async fn listen(
        &self,
        tx: mpsc::Sender<InboundConnection>,
        mut shutdown: watch::Receiver<bool>,
    ) -> ProxyResult<()> {
        // Clean up stale socket
        if self.path.exists() {
            std::fs::remove_file(&self.path).ok();
        }

        let listener = tokio::net::UnixListener::bind(&self.path)?;
        tracing::info!(path = %self.path.display(), "Unix socket listener started");

        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let (mut stream, _) = accept?;
                    let tx = tx.clone();
                    let max_read = self.max_read;
                    let path_str = self.path.display().to_string();

                    tokio::spawn(async move {
                        let mut buf = Vec::with_capacity(4096);
                        let mut tmp = vec![0u8; 8192];

                        loop {
                            match stream.read(&mut tmp).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    buf.extend_from_slice(&tmp[..n]);
                                    if buf.len() >= max_read {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!(error = %e, "Unix socket read error");
                                    return;
                                }
                            }
                        }

                        let uri: Uri = format!("unix://{path_str}").parse().unwrap_or_default();
                        let request = ProxyRequest {
                            id: RequestId::new(),
                            method: Method::POST,
                            uri,
                            headers: http::HeaderMap::new(),
                            body: ProxyBody::from(buf),
                            transport: TransportKind::Unix,
                            timestamp: chrono::Utc::now(),
                            extensions: Extensions::new(),
                        };

                        let (resp_tx, resp_rx) = oneshot::channel();
                        if tx.send((request, resp_tx)).await.is_ok() {
                            if let Ok(resp) = resp_rx.await {
                                use tokio::io::AsyncWriteExt;
                                let _ = stream.write_all(resp.body.as_bytes()).await;
                            }
                        }
                    });
                }
                _ = shutdown.changed() => {
                    tracing::info!("Unix socket listener shutting down");
                    break;
                }
            }
        }

        // Cleanup socket file
        std::fs::remove_file(&self.path).ok();
        Ok(())
    }

    fn transport_name(&self) -> &str {
        "unix"
    }
}
