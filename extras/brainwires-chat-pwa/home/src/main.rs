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

    /// Add an exact-match origin to the CORS allow-list.
    ///
    /// Repeatable. Typical production wiring is one entry — the chat
    /// PWA's URL (e.g. `https://chat.example.com`). When this flag is
    /// supplied, the default dev-origin allow-list (`localhost:8080`,
    /// `127.0.0.1:8080`, ...) is discarded.
    #[arg(long = "cors-origin", value_name = "URL")]
    cors_origin: Vec<String>,

    /// Allow any origin (`Access-Control-Allow-Origin: *`). **Dev only.**
    ///
    /// Wide-open CORS in production lets any web page in any tab
    /// preflight-poke your home daemon. Use `--cors-origin` instead.
    #[arg(
        long = "cors-permissive",
        env = "BRAINWIRES_HOME_CORS_PERMISSIVE",
        default_value_t = false
    )]
    cors_permissive: bool,

    /// Cloudflare Calls TURN key id (M7). Find it in the Cloudflare dashboard
    /// under `Calls → TURN keys`. When set together with `--cf-turn-token`,
    /// `POST /signal/session` mints a short-lived TURN credential per session.
    /// Without it, the daemon hands the PWA STUN-only fallbacks (which fail
    /// on cellular symmetric NAT — see `README.md` § TURN credentials).
    #[arg(long = "cf-turn-key-id", env = "CF_TURN_KEY_ID", value_name = "ID")]
    cf_turn_key_id: Option<String>,

    /// Cloudflare **Calls** API token (M7).
    ///
    /// **NOT** a Cloudflare Tunnel token — different product, different page
    /// in the dashboard. The PWA never sees this token; minting is server-
    /// side and credentials returned to the PWA are scoped to one session.
    #[arg(long = "cf-turn-token", env = "CF_TURN_TOKEN", value_name = "TOKEN")]
    cf_turn_token: Option<String>,

    /// Lifetime of minted TURN credentials in seconds. Default 600 (10 min).
    /// Floored at 60 s by the daemon — anything shorter is more likely to
    /// expire mid-handshake than to be useful.
    #[arg(long = "turn-ttl", env = "TURN_TTL", default_value_t = 600)]
    turn_ttl: u32,

    /// `tracing-subscriber` env-filter directive.
    #[arg(long, env = "RUST_LOG", default_value = "brainwires_home=info,info")]
    log: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt().with_env_filter(args.log).init();

    let mut builder = HomeServer::builder().bind(args.bind);
    if args.cors_permissive {
        if !args.cors_origin.is_empty() {
            tracing::warn!(
                "--cors-permissive supersedes --cors-origin; ignoring the explicit allow-list",
            );
        }
        tracing::warn!(
            "CORS is permissive (any origin); this is intended for dev only — \
             use --cors-origin <URL> in production"
        );
        builder = builder.cors_permissive();
    } else {
        for origin in &args.cors_origin {
            builder = builder.cors_allow_origin(origin.clone());
        }
        if args.cors_origin.is_empty() {
            tracing::info!(
                "no --cors-origin flags; defaulting to chat-PWA dev origins (localhost:8080 etc.)"
            );
        } else {
            tracing::info!(origins = ?args.cors_origin, "CORS allow-list configured");
        }
    }

    // M7: TURN credential minting. Both flags must be set; if only one is,
    // log loudly and fall back to STUN-only — silently dropping the half-
    // configured TURN would mask an obvious user mistake.
    match (args.cf_turn_key_id.as_deref(), args.cf_turn_token.as_deref()) {
        (Some(key_id), Some(token)) => {
            builder = builder
                .with_cloudflare_turn(key_id, token)
                .with_turn_ttl(args.turn_ttl);
            tracing::info!(
                ttl_secs = args.turn_ttl,
                "Cloudflare Calls TURN minting enabled"
            );
        }
        (Some(_), None) | (None, Some(_)) => {
            tracing::warn!(
                "--cf-turn-key-id and --cf-turn-token must be set together; \
                 falling back to STUN-only"
            );
        }
        (None, None) => {
            tracing::info!("no TURN config; serving STUN-only ICE servers");
        }
    }

    let server = builder.build()?;
    tracing::info!(addr = %args.bind, "brainwires-home: starting");
    server.serve().await
}
