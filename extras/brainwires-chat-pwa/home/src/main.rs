//! `brainwires-home` — dial-home daemon for the chat PWA.
//!
//! M1 just parses CLI flags and prints a startup banner. M2 wires the
//! axum router and signaling state behind `HomeServer::serve`.

use anyhow::Result;
use brainwires_home::HomeServer;
use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(version, about = "Brainwires chat-PWA dial-home daemon")]
struct Args {
    /// Bind address for the signaling server.
    ///
    /// The daemon expects to sit behind a Cloudflare Tunnel (or equivalent)
    /// landing on this loopback address. Listening on a public interface
    /// directly is supported for dev only.
    #[arg(long, env = "BRAINWIRES_HOME_BIND", default_value = "127.0.0.1:7878")]
    bind: SocketAddr,

    /// `tracing-subscriber` env-filter directive.
    #[arg(long, env = "RUST_LOG", default_value = "brainwires_home=info,info")]
    log: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt().with_env_filter(args.log).init();

    let server = HomeServer::builder().bind(args.bind).build()?;
    tracing::info!(addr = %args.bind, "brainwires-home: starting");
    server.serve().await
}
