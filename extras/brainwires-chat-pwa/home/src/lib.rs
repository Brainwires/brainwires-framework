//! `brainwires-home` — dial-home daemon for the Brainwires chat PWA.
//!
//! Runs on the user's home machine; the PWA reaches it via WebRTC behind a
//! Cloudflare Tunnel (or equivalent). See `README.md` for the full
//! architecture, endpoints, and pairing flow.
//!
//! This is the **library** surface. The binary in `src/main.rs` is a thin
//! shim that parses CLI flags and calls [`HomeServer::serve`]. Headless
//! integration tests can spin one up via [`HomeServer::builder`] without
//! touching the binary path — and via [`HomeServer::router`] without binding
//! a port at all.

pub mod a2a;
pub mod pairing;
pub mod signaling;
pub mod webrtc;

use anyhow::{Context, Result};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::a2a::A2aBridge;
use crate::signaling::{AppState, DEFAULT_LONG_POLL, DEFAULT_SESSION_TTL};

/// Default loopback bind. The daemon expects to live behind a Cloudflare
/// Tunnel (or equivalent reverse tunnel) — listening on a public interface
/// directly is supported for dev only.
pub const DEFAULT_BIND: &str = "127.0.0.1:7878";

/// Top-level handle to the running daemon.
pub struct HomeServer {
    bind: SocketAddr,
    state: AppState,
}

/// Builder for [`HomeServer`].
///
/// Phase-2 milestones layer on further setters: `with_task_agent(...)` (M4),
/// `with_turn_minter(...)` (M7), `with_pairing_store(...)` (M8). M2 wires
/// [`HomeServerBuilder::bind`], [`HomeServerBuilder::long_poll_timeout`], and
/// [`HomeServerBuilder::session_ttl`].
pub struct HomeServerBuilder {
    bind: Option<SocketAddr>,
    long_poll_timeout: Duration,
    session_ttl: Duration,
    bridge: Option<Arc<A2aBridge>>,
}

impl HomeServer {
    /// Start a new builder.
    pub fn builder() -> HomeServerBuilder {
        HomeServerBuilder {
            bind: None,
            long_poll_timeout: DEFAULT_LONG_POLL,
            session_ttl: DEFAULT_SESSION_TTL,
            bridge: None,
        }
    }

    /// The address the daemon will bind to.
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind
    }

    /// Borrow the shared application state. Useful in tests that want to
    /// poke the in-memory session map directly while exercising the router.
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Build the configured `axum::Router` without binding a port.
    ///
    /// Tests can drive this via `tower::ServiceExt::oneshot`. Production code
    /// goes through [`HomeServer::serve`], which binds and runs it.
    pub fn router(&self) -> Router {
        signaling::router(self.state.clone())
    }

    /// Run the server until it errors or is dropped.
    ///
    /// Binds to [`HomeServer::bind_addr`], spawns the session GC task, and
    /// hands the listener to `axum::serve`. Returns when the server exits.
    pub async fn serve(self) -> Result<()> {
        let _gc = self.state.spawn_gc();
        let app = signaling::router(self.state.clone());
        let listener = tokio::net::TcpListener::bind(self.bind)
            .await
            .with_context(|| format!("bind {}", self.bind))?;
        tracing::info!(
            addr = %self.bind,
            "brainwires-home: signaling server listening (M2)",
        );
        axum::serve(listener, app)
            .await
            .context("axum::serve exited with error")
    }
}

impl HomeServerBuilder {
    /// Override the bind address. Default: [`DEFAULT_BIND`].
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind = Some(addr);
        self
    }

    /// Override the long-poll wait for `/signal/answer` and `/signal/ice`.
    /// Default: 25 s. Tests usually shrink this to ~200 ms.
    pub fn long_poll_timeout(mut self, d: Duration) -> Self {
        self.long_poll_timeout = d;
        self
    }

    /// Override the session TTL. Sessions older than this are GC'd. Default:
    /// 30 minutes.
    pub fn session_ttl(mut self, d: Duration) -> Self {
        self.session_ttl = d;
        self
    }

    /// Attach the [`A2aBridge`] that the WebRTC data-channel loop will
    /// route inbound JSON-RPC frames through (M4).
    ///
    /// Production wiring (real provider, API keys, system prompt, ...)
    /// happens before construction: callers build a [`brainwires_agents::ChatAgent`]
    /// however they like, wrap it in an [`A2aBridge`], then hand it here.
    /// Tests use [`crate::a2a::test_support::echo_chat_agent`] to skip the
    /// network entirely.
    ///
    /// If unset, the daemon still answers `ping` for the M3 smoke-test path
    /// but rejects every other inbound method.
    pub fn with_agent(mut self, bridge: Arc<A2aBridge>) -> Self {
        self.bridge = Some(bridge);
        self
    }

    /// Materialize the builder into a [`HomeServer`].
    pub fn build(self) -> Result<HomeServer> {
        let bind = self
            .bind
            .unwrap_or_else(|| DEFAULT_BIND.parse().expect("DEFAULT_BIND parses"));
        let mut state = AppState::new(self.long_poll_timeout, self.session_ttl);
        if let Some(bridge) = self.bridge {
            state = state.with_bridge(bridge);
        }
        Ok(HomeServer { bind, state })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_defaults_to_loopback_7878() {
        let server = HomeServer::builder().build().expect("build");
        assert_eq!(server.bind_addr().to_string(), "127.0.0.1:7878");
    }

    #[test]
    fn builder_respects_explicit_bind() {
        let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
        let server = HomeServer::builder().bind(addr).build().expect("build");
        assert_eq!(server.bind_addr(), addr);
    }

    #[test]
    fn builder_exposes_router_without_binding() {
        // Constructing the router should never touch the network.
        let server = HomeServer::builder()
            .long_poll_timeout(Duration::from_millis(50))
            .session_ttl(Duration::from_secs(5))
            .build()
            .expect("build");
        let _router: Router = server.router();
    }
}
