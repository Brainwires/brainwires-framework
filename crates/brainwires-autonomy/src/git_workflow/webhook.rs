//! Axum webhook server with HMAC signature verification.

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use tokio::sync::{RwLock, mpsc};

use super::forge::RepoRef;
use super::trigger::WorkflowEvent;

/// Webhook server state.
struct WebhookState {
    secret: Option<String>,
    tx: mpsc::Sender<WorkflowEvent>,
    /// Track issues currently being investigated to prevent duplicates.
    active_investigations: RwLock<HashSet<String>>,
}

/// Axum-based webhook server for receiving Git forge events.
pub struct WebhookServer {
    listen_addr: String,
    port: u16,
    secret: Option<String>,
}

impl WebhookServer {
    /// Create a new webhook server with the given listen address, port, and optional secret.
    pub fn new(listen_addr: String, port: u16, secret: Option<String>) -> Self {
        Self {
            listen_addr,
            port,
            secret,
        }
    }

    /// Start the webhook server and emit events to the given channel.
    pub async fn run(self, tx: mpsc::Sender<WorkflowEvent>) -> anyhow::Result<()> {
        let state = Arc::new(WebhookState {
            secret: self.secret,
            tx,
            active_investigations: RwLock::new(HashSet::new()),
        });

        let app = Router::new()
            .route("/health", get(health))
            .route("/webhook", post(handle_webhook))
            .with_state(state);

        let addr = format!("{}:{}", self.listen_addr, self.port);
        tracing::info!("Webhook server listening on {addr}");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

async fn handle_webhook(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Verify HMAC signature if secret is configured
    if let Some(ref secret) = state.secret {
        let signature = headers
            .get("x-hub-signature-256")
            .or_else(|| headers.get("x-hub-signature"))
            .and_then(|v| v.to_str().ok());

        match signature {
            Some(sig) => {
                if !verify_signature(secret, &body, sig) {
                    tracing::warn!("Webhook signature verification failed");
                    return StatusCode::UNAUTHORIZED;
                }
            }
            None => {
                tracing::warn!("Webhook missing signature header");
                return StatusCode::UNAUTHORIZED;
            }
        }
    }

    // Parse event type
    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let payload: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to parse webhook payload: {e}");
            return StatusCode::BAD_REQUEST;
        }
    };

    let event = match parse_github_event(event_type, &payload) {
        Some(e) => e,
        None => {
            tracing::debug!("Ignoring unhandled event type: {event_type}");
            return StatusCode::OK;
        }
    };

    // Check for duplicate investigations
    let key = event_key(&event);
    if let Some(key) = &key {
        let active = state.active_investigations.read().await;
        if active.contains(key) {
            tracing::info!("Skipping duplicate investigation for {key}");
            return StatusCode::OK;
        }
    }

    if let Some(key) = &key {
        state
            .active_investigations
            .write()
            .await
            .insert(key.clone());
    }

    if let Err(e) = state.tx.send(event).await {
        tracing::error!("Failed to send webhook event: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use subtle::ConstantTimeEq;

    // Try SHA-256 first (x-hub-signature-256)
    if let Some(hex_sig) = signature.strip_prefix("sha256=") {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(body);
        let expected = mac.finalize().into_bytes();
        let expected_hex = hex::encode(expected);
        return expected_hex.as_bytes().ct_eq(hex_sig.as_bytes()).into();
    }

    // Fallback to SHA-1 (x-hub-signature)
    if let Some(hex_sig) = signature.strip_prefix("sha1=") {
        use sha1::Sha1;
        let mut mac =
            Hmac::<Sha1>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(body);
        let expected = mac.finalize().into_bytes();
        let expected_hex = hex::encode(expected);
        return expected_hex.as_bytes().ct_eq(hex_sig.as_bytes()).into();
    }

    false
}

fn parse_github_event(event_type: &str, payload: &serde_json::Value) -> Option<WorkflowEvent> {
    let repo = RepoRef {
        owner: payload["repository"]["owner"]["login"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        name: payload["repository"]["name"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    };

    match event_type {
        "issues" if payload["action"].as_str() == Some("opened") => {
            let issue = parse_issue(payload)?;
            Some(WorkflowEvent::IssueOpened { issue, repo })
        }
        "issue_comment" if payload["action"].as_str() == Some("created") => {
            let issue = parse_issue(&payload["issue"])?;
            let comment = super::forge::Comment {
                id: payload["comment"]["id"].to_string(),
                author: payload["comment"]["user"]["login"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                body: payload["comment"]["body"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            };
            Some(WorkflowEvent::IssueCommented {
                issue,
                comment,
                repo,
            })
        }
        "push" => {
            let branch = payload["ref"]
                .as_str()
                .unwrap_or("")
                .trim_start_matches("refs/heads/")
                .to_string();
            let commits = payload["commits"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|c| super::forge::CommitRef {
                            sha: c["id"].as_str().unwrap_or("").to_string(),
                            message: c["message"].as_str().unwrap_or("").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(WorkflowEvent::PushReceived {
                branch,
                commits,
                repo,
            })
        }
        _ => None,
    }
}

fn parse_issue(payload: &serde_json::Value) -> Option<super::forge::Issue> {
    Some(super::forge::Issue {
        id: payload["id"].to_string(),
        number: payload["number"].as_u64()?,
        title: payload["title"].as_str().unwrap_or("").to_string(),
        body: payload["body"].as_str().unwrap_or("").to_string(),
        labels: payload["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        author: payload["user"]["login"].as_str().unwrap_or("").to_string(),
        url: payload["html_url"].as_str().unwrap_or("").to_string(),
    })
}

fn event_key(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::IssueOpened { issue, repo } => {
            Some(format!("{}#{}", repo.full_name(), issue.number))
        }
        _ => None,
    }
}
