//! A2A server — serves JSON-RPC, REST, and optionally gRPC.

/// Core handler trait.
pub mod handler;
/// JSON-RPC method dispatch.
pub mod jsonrpc_router;
/// HTTP/REST route handling.
pub mod rest_router;
/// SSE response construction.
pub mod sse_response;
/// gRPC service implementation.
pub mod grpc_service;

pub use handler::A2aHandler;

#[cfg(feature = "grpc-server")]
pub use grpc_service::GrpcBridge;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::error::A2aError;
use crate::jsonrpc::{JsonRpcRequest, RequestId, METHOD_MESSAGE_STREAM, METHOD_TASKS_RESUBSCRIBE};
use crate::params::{SendMessageRequest, SubscribeToTaskRequest};

/// Unified A2A server serving JSON-RPC + REST (HTTP) and optionally gRPC.
pub struct A2aServer<H: A2aHandler> {
    handler: Arc<H>,
    addr: SocketAddr,
    #[cfg(feature = "grpc-server")]
    grpc_addr: Option<SocketAddr>,
}

impl<H: A2aHandler> A2aServer<H> {
    /// Create a new server bound to `addr`.
    pub fn new(handler: H, addr: SocketAddr) -> Self {
        Self {
            handler: Arc::new(handler),
            addr,
            #[cfg(feature = "grpc-server")]
            grpc_addr: None,
        }
    }

    /// Enable gRPC on a separate port.
    #[cfg(feature = "grpc-server")]
    pub fn with_grpc(mut self, grpc_addr: SocketAddr) -> Self {
        self.grpc_addr = Some(grpc_addr);
        self
    }

    /// Run the server (blocks until shutdown).
    pub async fn run(self) -> Result<(), A2aError> {
        use hyper::body::Incoming;
        use hyper::service::service_fn;
        use hyper_util::rt::TokioIo;

        let handler = self.handler.clone();
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| A2aError::internal(format!("Failed to bind: {e}")))?;

        tracing::info!("A2A server listening on {}", self.addr);

        // Optionally spawn gRPC server
        #[cfg(feature = "grpc-server")]
        if let Some(grpc_addr) = self.grpc_addr {
            let grpc_handler = self.handler.clone();
            tokio::spawn(async move {
                let bridge = GrpcBridge::new(grpc_handler);
                let svc = crate::proto::lf_a2a_v1::a2a_service_server::A2aServiceServer::new(bridge);
                tracing::info!("A2A gRPC server listening on {grpc_addr}");
                if let Err(e) = tonic::transport::Server::builder()
                    .add_service(svc)
                    .serve(grpc_addr)
                    .await
                {
                    tracing::error!("gRPC server error: {e}");
                }
            });
        }

        loop {
            let (stream, _peer) = listener
                .accept()
                .await
                .map_err(|e| A2aError::internal(format!("Accept error: {e}")))?;

            let handler = handler.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let svc = service_fn(move |req: hyper::Request<Incoming>| {
                    let handler = handler.clone();
                    async move {
                        handle_http_request(handler, req).await
                    }
                });
                if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                    hyper_util::rt::TokioExecutor::new(),
                )
                .serve_connection(io, svc)
                .await
                {
                    tracing::debug!("Connection error: {e}");
                }
            });
        }
    }
}

#[cfg(feature = "server")]
async fn handle_http_request<H: A2aHandler>(
    handler: Arc<H>,
    req: hyper::Request<hyper::body::Incoming>,
) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, hyper::Error> {
    use http_body_util::BodyExt;

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // Agent card discovery
    if method == hyper::Method::GET && path == "/.well-known/agent-card.json" {
        let card = handler.agent_card();
        let body = serde_json::to_string(card).unwrap_or_default();
        return Ok(hyper::Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(http_body_util::Full::new(bytes::Bytes::from(body)))
            .unwrap());
    }

    // Collect body
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map(|c| c.to_bytes())
        .unwrap_or_default();

    // JSON-RPC: POST to /
    if method == hyper::Method::POST && path == "/" {
        return handle_jsonrpc(&handler, &body_bytes).await;
    }

    // REST routes
    let method_str = method.as_str();
    match rest_router::dispatch_rest(&handler, method_str, &path, &body_bytes).await {
        Ok(rest_router::RestResult::Json(val)) => {
            let body = serde_json::to_string(&val).unwrap_or_default();
            Ok(hyper::Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                .unwrap())
        }
        Ok(rest_router::RestResult::Stream(stream)) => {
            // For simplicity, collect stream events into a JSON array response.
            // A full SSE implementation would use chunked transfer encoding.
            use tokio_stream::StreamExt;
            let mut events = Vec::new();
            let mut stream = stream;
            while let Some(item) = stream.next().await {
                match item {
                    Ok(event) => events.push(serde_json::to_value(&event).unwrap_or_default()),
                    Err(_) => break,
                }
            }
            let body = serde_json::to_string(&events).unwrap_or_default();
            Ok(hyper::Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                .unwrap())
        }
        Err(e) => {
            let body = serde_json::to_string(&e).unwrap_or_default();
            Ok(hyper::Response::builder()
                .status(404)
                .header("Content-Type", "application/json")
                .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                .unwrap())
        }
    }
}

#[cfg(feature = "server")]
async fn handle_jsonrpc<H: A2aHandler>(
    handler: &Arc<H>,
    body: &bytes::Bytes,
) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, hyper::Error> {
    let request: JsonRpcRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => {
            let resp = crate::jsonrpc::JsonRpcResponse::error(
                RequestId::Number(0),
                A2aError::parse_error(e.to_string()),
            );
            let body = serde_json::to_string(&resp).unwrap_or_default();
            return Ok(hyper::Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                .unwrap());
        }
    };

    // Check for streaming methods
    if request.method == METHOD_MESSAGE_STREAM {
        let id = request.id.clone();
        let params = request.params.clone().unwrap_or(serde_json::Value::Null);
        let req: SendMessageRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                let resp = crate::jsonrpc::JsonRpcResponse::error(id, A2aError::from(e));
                let body = serde_json::to_string(&resp).unwrap_or_default();
                return Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                    .unwrap());
            }
        };
        match handler.on_send_streaming_message(req).await {
            Ok(stream) => {
                // Collect SSE stream into response
                use tokio_stream::StreamExt;
                let mut lines = String::new();
                let mut stream = stream;
                while let Some(item) = stream.next().await {
                    let resp = match item {
                        Ok(event) => {
                            let val = serde_json::to_value(&event).unwrap_or_default();
                            crate::jsonrpc::JsonRpcResponse::success(id.clone(), val)
                        }
                        Err(e) => crate::jsonrpc::JsonRpcResponse::error(id.clone(), e),
                    };
                    let json = serde_json::to_string(&resp).unwrap_or_default();
                    lines.push_str(&format!("data: {json}\n\n"));
                }
                return Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .body(http_body_util::Full::new(bytes::Bytes::from(lines)))
                    .unwrap());
            }
            Err(e) => {
                let resp = crate::jsonrpc::JsonRpcResponse::error(id, e);
                let body = serde_json::to_string(&resp).unwrap_or_default();
                return Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                    .unwrap());
            }
        }
    }

    if request.method == METHOD_TASKS_RESUBSCRIBE {
        let id = request.id.clone();
        let params = request.params.clone().unwrap_or(serde_json::Value::Null);
        let req: SubscribeToTaskRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                let resp = crate::jsonrpc::JsonRpcResponse::error(id, A2aError::from(e));
                let body = serde_json::to_string(&resp).unwrap_or_default();
                return Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                    .unwrap());
            }
        };
        match handler.on_subscribe_to_task(req).await {
            Ok(stream) => {
                use tokio_stream::StreamExt;
                let mut lines = String::new();
                let mut stream = stream;
                while let Some(item) = stream.next().await {
                    let resp = match item {
                        Ok(event) => {
                            let val = serde_json::to_value(&event).unwrap_or_default();
                            crate::jsonrpc::JsonRpcResponse::success(id.clone(), val)
                        }
                        Err(e) => crate::jsonrpc::JsonRpcResponse::error(id.clone(), e),
                    };
                    let json = serde_json::to_string(&resp).unwrap_or_default();
                    lines.push_str(&format!("data: {json}\n\n"));
                }
                return Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .body(http_body_util::Full::new(bytes::Bytes::from(lines)))
                    .unwrap());
            }
            Err(e) => {
                let resp = crate::jsonrpc::JsonRpcResponse::error(id, e);
                let body = serde_json::to_string(&resp).unwrap_or_default();
                return Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(http_body_util::Full::new(bytes::Bytes::from(body)))
                    .unwrap());
            }
        }
    }

    // Non-streaming JSON-RPC
    let response = match jsonrpc_router::dispatch(handler, &request).await {
        Ok(Some(resp)) => resp,
        Ok(None) => unreachable!("streaming methods handled above"),
        Err(resp) => resp,
    };

    let body = serde_json::to_string(&response).unwrap_or_default();
    Ok(hyper::Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(bytes::Bytes::from(body)))
        .unwrap())
}
