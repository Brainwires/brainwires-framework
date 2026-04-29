//! Signaling endpoints — `/signal/*` HTTP for SDP offer/answer + ICE
//! long-poll, and `/.well-known/agent-card.json`. Wired up in M2.
//!
//! State is **in-memory only** (an `Arc<DashMap<String, Arc<SessionState>>>`).
//! The home daemon serves a single user; if it restarts, the PWA re-pairs and
//! mints a new session. M3 wires the offer/answer that flows through these
//! routes into a real `RTCPeerConnection`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use brainwires_a2a::{
    A2A_PROTOCOL_VERSION, AgentCapabilities, AgentCard, AgentInterface, AgentProvider,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

/// Default long-poll wait. The PWA retries on 204.
pub const DEFAULT_LONG_POLL: Duration = Duration::from_secs(25);

/// Default session TTL. Sessions are GC'd after this much idle time.
pub const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);

/// How often the GC sweep runs.
pub const GC_INTERVAL: Duration = Duration::from_secs(60);

/// Crate version, baked at compile time. Surfaced in the AgentCard.
pub const HOME_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------- wire types ----------

/// SDP description sent over signaling. Mirrors the shape `RTCPeerConnection`
/// produces in the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpDesc {
    pub sdp: String,
    /// `"offer"` or `"answer"` — kept as a string so we don't tightly couple
    /// to a Rust enum the browser doesn't share.
    #[serde(rename = "type")]
    pub kind: String,
}

/// One ICE candidate relayed over signaling. `candidate == null` is the
/// end-of-candidates marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub candidate: Option<String>,
    #[serde(rename = "sdpMid", default)]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex", default)]
    pub sdp_m_line_index: Option<u16>,
}

#[derive(Debug, Serialize)]
struct SessionCreatedResponse {
    session_id: String,
    ice_servers: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct IcePollResponse {
    candidates: Vec<IceCandidate>,
    cursor: usize,
}

#[derive(Debug, Deserialize)]
pub struct IceQuery {
    #[serde(default)]
    pub since: usize,
}

// ---------- session state ----------

/// Per-session in-memory state.
pub struct SessionState {
    pub session_id: String,
    pub created_at: Instant,
    pub offer: RwLock<Option<SdpDesc>>,
    pub answer: RwLock<Option<SdpDesc>>,
    pub ice_candidates: RwLock<Vec<IceCandidate>>,
    pub answer_notify: Notify,
    pub ice_notify: Notify,
}

impl SessionState {
    fn new(session_id: String) -> Self {
        Self {
            session_id,
            created_at: Instant::now(),
            offer: RwLock::new(None),
            answer: RwLock::new(None),
            ice_candidates: RwLock::new(Vec::new()),
            answer_notify: Notify::new(),
            ice_notify: Notify::new(),
        }
    }
}

/// Shared application state passed to every handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<DashMap<String, Arc<SessionState>>>,
    pub long_poll_timeout: Duration,
    pub session_ttl: Duration,
}

impl AppState {
    pub fn new(long_poll_timeout: Duration, session_ttl: Duration) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            long_poll_timeout,
            session_ttl,
        }
    }

    /// Drop sessions older than `session_ttl`.
    pub fn gc_expired(&self) {
        let now = Instant::now();
        let ttl = self.session_ttl;
        self.sessions
            .retain(|_, s| now.saturating_duration_since(s.created_at) < ttl);
    }

    /// Spawn a background task that GCs every [`GC_INTERVAL`].
    pub fn spawn_gc(&self) -> tokio::task::JoinHandle<()> {
        let me = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(GC_INTERVAL);
            // First tick fires immediately; skip it.
            tick.tick().await;
            loop {
                tick.tick().await;
                me.gc_expired();
            }
        })
    }
}

// ---------- router ----------

/// Build the axum `Router` for all `/signal/*` + agent-card routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/signal/session", post(create_session))
        .route("/signal/offer/{session}", post(post_offer))
        .route("/signal/answer/{session}", get(get_answer))
        .route("/signal/ice/{session}", post(post_ice).get(get_ice))
        .route("/signal/{session}", axum::routing::delete(delete_session))
        .route("/.well-known/agent-card.json", get(agent_card))
        .with_state(state)
}

// ---------- handlers ----------

async fn create_session(State(state): State<AppState>) -> impl IntoResponse {
    let session_id = Uuid::new_v4().simple().to_string();
    let s = Arc::new(SessionState::new(session_id.clone()));
    state.sessions.insert(session_id.clone(), s);
    (
        StatusCode::OK,
        Json(SessionCreatedResponse {
            session_id,
            ice_servers: Vec::new(),
        }),
    )
}

async fn post_offer(
    State(state): State<AppState>,
    Path(session): Path<String>,
    Json(body): Json<SdpDesc>,
) -> StatusCode {
    let Some(s) = state.sessions.get(&session).map(|e| e.value().clone()) else {
        return StatusCode::NOT_FOUND;
    };
    *s.offer.write().await = Some(body);
    StatusCode::NO_CONTENT
}

async fn get_answer(
    State(state): State<AppState>,
    Path(session): Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    let Some(s) = state.sessions.get(&session).map(|e| e.value().clone()) else {
        return Err(StatusCode::NOT_FOUND);
    };

    // Fast path: answer is already there.
    if let Some(a) = s.answer.read().await.clone() {
        return Ok(Json(a).into_response());
    }

    // Long-poll: wait for a notify or the deadline.
    let notified = s.answer_notify.notified();
    tokio::pin!(notified);
    let outcome = tokio::time::timeout(state.long_poll_timeout, &mut notified).await;
    match outcome {
        Ok(()) => {
            if let Some(a) = s.answer.read().await.clone() {
                Ok(Json(a).into_response())
            } else {
                Ok(StatusCode::NO_CONTENT.into_response())
            }
        }
        Err(_) => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

async fn post_ice(
    State(state): State<AppState>,
    Path(session): Path<String>,
    Json(body): Json<IceCandidate>,
) -> StatusCode {
    let Some(s) = state.sessions.get(&session).map(|e| e.value().clone()) else {
        return StatusCode::NOT_FOUND;
    };
    s.ice_candidates.write().await.push(body);
    s.ice_notify.notify_waiters();
    StatusCode::NO_CONTENT
}

async fn get_ice(
    State(state): State<AppState>,
    Path(session): Path<String>,
    Query(q): Query<IceQuery>,
) -> Result<axum::response::Response, StatusCode> {
    let Some(s) = state.sessions.get(&session).map(|e| e.value().clone()) else {
        return Err(StatusCode::NOT_FOUND);
    };

    // Snapshot fast path.
    {
        let cands = s.ice_candidates.read().await;
        if cands.len() > q.since {
            let slice = cands[q.since..].to_vec();
            let cursor = cands.len();
            return Ok(Json(IcePollResponse {
                candidates: slice,
                cursor,
            })
            .into_response());
        }
    }

    // Long-poll for a new candidate or deadline.
    let notified = s.ice_notify.notified();
    tokio::pin!(notified);
    let _ = tokio::time::timeout(state.long_poll_timeout, &mut notified).await;

    let cands = s.ice_candidates.read().await;
    let from = q.since.min(cands.len());
    let slice = cands[from..].to_vec();
    let cursor = cands.len();
    Ok(Json(IcePollResponse {
        candidates: slice,
        cursor,
    })
    .into_response())
}

async fn delete_session(
    State(state): State<AppState>,
    Path(session): Path<String>,
) -> StatusCode {
    state.sessions.remove(&session);
    StatusCode::NO_CONTENT
}

async fn agent_card() -> impl IntoResponse {
    let card = AgentCard {
        name: "brainwires-home".to_string(),
        description: "Brainwires dial-home daemon: WebRTC peer + A2A JSON-RPC \
                      bridge into the user's local TaskAgent."
            .to_string(),
        version: HOME_VERSION.to_string(),
        // The PWA overrides this with the actual tunnel hostname when it
        // fetches the card; the daemon itself doesn't know its public URL.
        // Per A2A 0.3, `supportedInterfaces[].url` is the canonical service URL.
        supported_interfaces: vec![AgentInterface {
            url: "/".to_string(),
            protocol_binding: "JSONRPC".to_string(),
            tenant: None,
            protocol_version: A2A_PROTOCOL_VERSION.to_string(),
        }],
        capabilities: AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extended_agent_card: Some(false),
            extensions: None,
        },
        skills: Vec::new(),
        default_input_modes: vec!["text".to_string()],
        default_output_modes: vec!["text".to_string()],
        provider: Some(AgentProvider {
            url: "https://brainwires.net".to_string(),
            organization: "Brainwires".to_string(),
        }),
        security_schemes: None,
        security_requirements: None,
        documentation_url: None,
        icon_url: None,
        signatures: None,
    };
    Json(card)
}

// ---------- tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Method, Request};
    use serde_json::Value;
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        AppState::new(Duration::from_millis(200), DEFAULT_SESSION_TTL)
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let body = resp.into_body();
        let bytes = to_bytes(body, 1 << 20).await.expect("collect body");
        serde_json::from_slice(&bytes).expect("body is valid JSON")
    }

    async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
        let body = resp.into_body();
        to_bytes(body, 1 << 20).await.expect("collect body").to_vec()
    }

    fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    fn empty_request(method: Method, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Convenience: open a fresh session and return its id.
    async fn new_session(app: &Router) -> String {
        let resp = app
            .clone()
            .oneshot(empty_request(Method::POST, "/signal/session"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        v["session_id"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_session_create_returns_id_and_empty_ice() {
        let app = router(test_state());
        let resp = app
            .oneshot(empty_request(Method::POST, "/signal/session"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        let id = v["session_id"].as_str().expect("session_id is string");
        // UUID v4 simple form is 32 hex chars.
        assert_eq!(id.len(), 32, "session_id should be 32 hex chars: {id}");
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        let ice = v["ice_servers"].as_array().expect("ice_servers is array");
        assert!(ice.is_empty(), "ice_servers must be empty in M2");
    }

    #[tokio::test]
    async fn test_offer_then_answer_roundtrip() {
        let state = test_state();
        let app = router(state.clone());
        let id = new_session(&app).await;

        // Post an offer.
        let offer = serde_json::json!({ "sdp": "v=0\r\n...offer", "type": "offer" });
        let resp = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                &format!("/signal/offer/{id}"),
                offer.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Sanity: offer landed on session state.
        {
            let s = state.sessions.get(&id).unwrap().value().clone();
            assert_eq!(s.offer.read().await.as_ref().unwrap().kind, "offer");
        }

        // Simulate the home side filling in the answer (M3 will wire this from
        // the WebRTC peer; M2 we poke the state directly to test the route).
        let answer = SdpDesc {
            sdp: "v=0\r\n...answer".to_string(),
            kind: "answer".to_string(),
        };
        {
            let s = state.sessions.get(&id).unwrap().value().clone();
            *s.answer.write().await = Some(answer.clone());
            s.answer_notify.notify_waiters();
        }

        // GET /signal/answer/{id} fast-paths since the answer is set.
        let resp = app
            .clone()
            .oneshot(empty_request(Method::GET, &format!("/signal/answer/{id}")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["sdp"].as_str().unwrap(), "v=0\r\n...answer");
        assert_eq!(v["type"].as_str().unwrap(), "answer");
    }

    #[tokio::test]
    async fn test_answer_long_poll_times_out() {
        let app = router(test_state());
        let id = new_session(&app).await;

        let start = Instant::now();
        let resp = app
            .clone()
            .oneshot(empty_request(Method::GET, &format!("/signal/answer/{id}")))
            .await
            .unwrap();
        let elapsed = start.elapsed();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert!(
            elapsed >= Duration::from_millis(150),
            "long-poll returned too fast: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "long-poll took too long: {elapsed:?}"
        );

        // Unknown session returns 404.
        let resp = app
            .oneshot(empty_request(
                Method::GET,
                "/signal/answer/00000000000000000000000000000000",
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_ice_append_and_get() {
        let app = router(test_state());
        let id = new_session(&app).await;

        let cand_a = serde_json::json!({
            "candidate": "candidate:1 1 UDP 2122260223 192.168.1.10 51234 typ host",
            "sdpMid": "0",
            "sdpMLineIndex": 0
        });
        let cand_b = serde_json::json!({
            "candidate": "candidate:2 1 UDP 2122194687 192.168.1.10 51235 typ host",
            "sdpMid": "0",
            "sdpMLineIndex": 0
        });

        // POST first candidate.
        let resp = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                &format!("/signal/ice/{id}"),
                cand_a.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // GET ?since=0 returns it.
        let resp = app
            .clone()
            .oneshot(empty_request(
                Method::GET,
                &format!("/signal/ice/{id}?since=0"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["cursor"].as_u64().unwrap(), 1);
        let cands = v["candidates"].as_array().unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0]["candidate"], cand_a["candidate"]);

        // POST second.
        let resp = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                &format!("/signal/ice/{id}"),
                cand_b.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // GET ?since=1 returns just the second.
        let resp = app
            .clone()
            .oneshot(empty_request(
                Method::GET,
                &format!("/signal/ice/{id}?since=1"),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["cursor"].as_u64().unwrap(), 2);
        let cands = v["candidates"].as_array().unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0]["candidate"], cand_b["candidate"]);
    }

    #[tokio::test]
    async fn test_delete_session_idempotent() {
        let app = router(test_state());
        let id = new_session(&app).await;

        for _ in 0..2 {
            let resp = app
                .clone()
                .oneshot(empty_request(Method::DELETE, &format!("/signal/{id}")))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        }
    }

    #[tokio::test]
    async fn test_agent_card_returns_valid_json() {
        let app = router(test_state());
        let resp = app
            .oneshot(empty_request(Method::GET, "/.well-known/agent-card.json"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = body_bytes(resp).await;
        let v: Value = serde_json::from_slice(&bytes).expect("agent-card is valid JSON");
        assert_eq!(v["name"].as_str().unwrap(), "brainwires-home");
        // protocolVersion is per-supportedInterface in the A2A AgentCard struct.
        let iface = v["supportedInterfaces"]
            .as_array()
            .expect("supportedInterfaces array")
            .first()
            .expect("at least one interface");
        assert_eq!(iface["protocolVersion"].as_str().unwrap(), "0.3");
        assert_eq!(
            v["capabilities"]["streaming"].as_bool().unwrap(),
            true,
            "streaming capability"
        );
        // version field reflects the crate version.
        assert_eq!(v["version"].as_str().unwrap(), HOME_VERSION);
    }

    #[tokio::test]
    async fn test_gc_expires_old_sessions() {
        let mut state = test_state();
        state.session_ttl = Duration::from_millis(50);
        let app = router(state.clone());
        let id = new_session(&app).await;
        assert!(state.sessions.contains_key(&id));
        tokio::time::sleep(Duration::from_millis(80)).await;
        state.gc_expired();
        assert!(!state.sessions.contains_key(&id), "session should be GC'd");
    }
}
