use anyhow::Result;
use clap::{Parser, Subcommand};

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
        Some(Commands::Serve) | None => {
            serve(cli).await?;
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
    println!(
        "  Tools:        {} enabled",
        config.tools.enabled.len()
    );
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
