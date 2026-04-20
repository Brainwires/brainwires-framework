use anyhow::Result;
use clap::{Parser, Subcommand};

use brainclaw::doctor::{self, DoctorArgs};
use brainclaw::onboard::{self, OnboardArgs};
use brainclaw::{BrainClaw, BrainClawConfig};

/// BrainClaw — personal AI assistant daemon
#[derive(Parser)]
#[command(name = "brainclaw")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Personal AI assistant daemon built on the Brainwires Framework")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to configuration file
    #[arg(long, global = true)]
    config: Option<String>,

    /// Host address to bind to
    #[arg(long, global = true)]
    host: Option<String>,

    /// Port to listen on
    #[arg(long, global = true)]
    port: Option<u16>,

    /// AI provider (anthropic, openai, google, groq, ollama, etc.)
    #[arg(long, global = true)]
    provider: Option<String>,

    /// Model name
    #[arg(long, global = true)]
    model: Option<String>,

    /// API key for the provider
    #[arg(long, global = true)]
    api_key: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the BrainClaw daemon (default)
    Serve,
    /// Show version information
    Version,
    /// Validate the configuration file
    ConfigCheck,
    /// DM pairing administration (approve/reject peer DMs).
    #[command(subcommand)]
    Pairing(PairingCmd),
    /// Run diagnostic checks across all subsystems.
    Doctor(DoctorArgs),
    /// Interactive setup wizard — writes a ready-to-use `brainclaw.toml`.
    Onboard(OnboardArgs),
}

#[derive(Subcommand)]
enum PairingCmd {
    /// List pending pairing codes.
    Pending,
    /// List approved peers.
    List,
    /// Approve a pending pairing code.
    Approve {
        /// The 6-digit code.
        code: String,
    },
    /// Reject (discard) a pending pairing code.
    Reject {
        /// The 6-digit code.
        code: String,
    },
    /// Revoke a previously-approved peer.
    Revoke {
        /// Channel name (e.g. `discord`).
        channel: String,
        /// Platform user id.
        user: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            show_version();
        }
        Some(Commands::ConfigCheck) => {
            config_check(&cli)?;
        }
        Some(Commands::Pairing(ref cmd)) => {
            pairing_cmd(&cli, cmd).await?;
        }
        Some(Commands::Doctor(ref args)) => {
            let (_results, exit) = doctor::run(cli.config.as_deref(), args).await?;
            if exit != 0 {
                std::process::exit(exit);
            }
        }
        Some(Commands::Onboard(ref args)) => {
            // Allow `--config` at the top-level to stand in for the per-subcommand flag.
            let mut args = args.clone();
            if args.config.is_none() {
                args.config = cli.config.clone();
            }
            onboard::run(&args).await?;
        }
        Some(Commands::Serve) | None => {
            serve(cli).await?;
        }
    }

    Ok(())
}

/// Run a `brainclaw pairing ...` subcommand against the local gateway's
/// admin API.
async fn pairing_cmd(cli: &Cli, cmd: &PairingCmd) -> Result<()> {
    let config = load_config(cli)?;
    let host = cli
        .host
        .clone()
        .unwrap_or_else(|| config.gateway.host.clone());
    let port = cli.port.unwrap_or(config.gateway.port);
    // The daemon listens on `127.0.0.1` by default; when host is `0.0.0.0`
    // we still address it via loopback on this machine.
    let connect_host = if host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        host
    };
    let base = format!("http://{connect_host}:{port}/admin/pairing");
    let token = config.security.admin_token.clone();
    let client = reqwest::Client::new();

    let auth = |req: reqwest::RequestBuilder| match &token {
        Some(t) => req.bearer_auth(t),
        None => req,
    };

    match cmd {
        PairingCmd::Pending => {
            let resp = auth(client.get(format!("{base}/pending")))
                .send()
                .await?
                .error_for_status()?;
            let codes: Vec<serde_json::Value> = resp.json().await?;
            if codes.is_empty() {
                println!("(no pending codes)");
            } else {
                for pc in codes {
                    println!(
                        "{}  {}:{}  {} (expires {})",
                        pc.get("code").and_then(|v| v.as_str()).unwrap_or("?"),
                        pc.get("channel").and_then(|v| v.as_str()).unwrap_or("?"),
                        pc.get("user_id").and_then(|v| v.as_str()).unwrap_or("?"),
                        pc.get("peer_display")
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                        pc.get("expires_at").and_then(|v| v.as_str()).unwrap_or(""),
                    );
                }
            }
        }
        PairingCmd::List => {
            let resp = auth(client.get(format!("{base}/approved")))
                .send()
                .await?
                .error_for_status()?;
            let peers: Vec<String> = resp.json().await?;
            if peers.is_empty() {
                println!("(no approved peers)");
            } else {
                for p in peers {
                    println!("{p}");
                }
            }
        }
        PairingCmd::Approve { code } => {
            let resp = auth(client.post(format!("{base}/approve")))
                .json(&serde_json::json!({ "code": code }))
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = resp.json().await?;
            if body
                .get("approved")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                println!(
                    "approved {}:{}",
                    body.get("channel").and_then(|v| v.as_str()).unwrap_or("?"),
                    body.get("user_id").and_then(|v| v.as_str()).unwrap_or("?"),
                );
            } else {
                let reason = body
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                println!("not approved: {reason}");
            }
        }
        PairingCmd::Reject { code } => {
            let resp = auth(client.post(format!("{base}/reject")))
                .json(&serde_json::json!({ "code": code }))
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = resp.json().await?;
            let rejected = body
                .get("rejected")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if rejected {
                println!("rejected code {code}");
            } else {
                println!("code {code} not found");
            }
        }
        PairingCmd::Revoke { channel, user } => {
            let resp = auth(client.post(format!("{base}/revoke")))
                .json(&serde_json::json!({ "channel": channel, "user_id": user }))
                .send()
                .await?
                .error_for_status()?;
            let body: serde_json::Value = resp.json().await?;
            if body
                .get("revoked")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                println!("revoked {channel}:{user}");
            } else {
                println!("revoke failed for {channel}:{user}");
            }
        }
    }

    Ok(())
}

fn show_version() {
    println!("brainclaw v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Components:");
    println!("  Gateway:     brainwires-gateway (WebSocket + webhook)");
    println!("  Agents:      brainwires-agents (ChatAgent with tool loops)");
    println!("  Providers:   brainwires-providers (Anthropic, OpenAI, Google, etc.)");
    println!("  Tools:       brainwires-tools (bash, files, git, search, web, validation)");
    println!("  Skills:      brainwires-skills (SKILL.md-based extensibility)");
    println!("  Channels:    brainwires-channels (Discord, Telegram, Slack, etc.)");
}

fn config_check(cli: &Cli) -> Result<()> {
    let config = load_config(cli)?;
    config.validate()?;
    println!("Configuration is valid.");
    println!();
    println!("  Provider:     {}", config.provider.default_provider);
    println!(
        "  Model:        {}",
        config
            .provider
            .default_model
            .as_deref()
            .unwrap_or("(provider default)")
    );
    println!(
        "  Listen:       {}:{}",
        config.gateway.host, config.gateway.port
    );
    println!("  Persona:      {}", config.persona.name);
    println!("  Tools:        {} enabled", config.tools.enabled.len());
    println!(
        "  Skills:       {}",
        if config.skills.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    Ok(())
}

async fn serve(cli: Cli) -> Result<()> {
    let mut config = load_config(&cli)?;

    // CLI overrides
    if let Some(ref host) = cli.host {
        config.gateway.host = host.clone();
    }
    if let Some(port) = cli.port {
        config.gateway.port = port;
    }
    if let Some(ref provider) = cli.provider {
        config.provider.default_provider = provider.clone();
    }
    if let Some(ref model) = cli.model {
        config.provider.default_model = Some(model.clone());
    }
    if let Some(ref api_key) = cli.api_key {
        config.provider.api_key = Some(api_key.clone());
    }

    config.validate()?;

    let app = BrainClaw::new(config);
    app.run().await
}

fn load_config(cli: &Cli) -> Result<BrainClawConfig> {
    if let Some(ref path) = cli.config {
        BrainClawConfig::load(path)
    } else {
        BrainClawConfig::load_or_default()
    }
}
