//! `brainwires-home` — dial-home daemon for the Brainwires chat PWA.
//!
//! Runs on the user's home machine; the PWA reaches it via WebRTC behind a
//! Cloudflare Tunnel (or equivalent). See `README.md` for the full
//! architecture, endpoints, and pairing flow.
//!
//! This is the **library** surface. The binary in `src/main.rs` is a thin
//! shim that parses CLI flags and calls [`HomeServer::serve`]. Headless
//! integration tests can spin one up via [`HomeServer::builder`] without
//! touching the binary path.

pub mod a2a;
pub mod pairing;
pub mod signaling;
pub mod webrtc;

use anyhow::Result;
use std::net::SocketAddr;

/// Default loopback bind. The daemon expects to live behind a Cloudflare
/// Tunnel (or equivalent reverse tunnel) — listening on a public interface
/// directly is supported for dev only.
pub const DEFAULT_BIND: &str = "127.0.0.1:7878";

/// Top-level handle to the running daemon.
pub struct HomeServer {
    bind: SocketAddr,
}

/// Builder for [`HomeServer`].
///
/// Use [`HomeServer::builder`] to construct one. Phase-2 milestones layer on
/// further setters: `with_task_agent(...)` (M4), `with_turn_minter(...)` (M7),
/// `with_pairing_store(...)` (M8). Today only [`HomeServerBuilder::bind`] is
/// wired up.
pub struct HomeServerBuilder {
    bind: Option<SocketAddr>,
}

impl HomeServer {
    /// Start a new builder.
    pub fn builder() -> HomeServerBuilder {
        HomeServerBuilder { bind: None }
    }

    /// The address the daemon will bind to.
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind
    }

    /// Run the server until it errors or is dropped.
    ///
    /// **M1**: skeleton — emits a warn-level marker so the binary is visibly
    /// inert and exits successfully. M2 wires up the axum router, in-memory
    /// session map, and `/.well-known/agent-card.json` handler.
    pub async fn serve(self) -> Result<()> {
        tracing::warn!(
            addr = %self.bind,
            "brainwires-home: M1 scaffold — no handlers wired yet (axum router lands in M2)",
        );
        Ok(())
    }
}

impl HomeServerBuilder {
    /// Override the bind address. Default: [`DEFAULT_BIND`].
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind = Some(addr);
        self
    }

    /// Materialize the builder into a [`HomeServer`].
    pub fn build(self) -> Result<HomeServer> {
        let bind = self
            .bind
            .unwrap_or_else(|| DEFAULT_BIND.parse().expect("DEFAULT_BIND parses"));
        Ok(HomeServer { bind })
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
}
