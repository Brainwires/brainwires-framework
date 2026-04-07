use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::sync::mpsc;

use brainwires_network::channels::Channel;
use brainwires_whatsapp_channel::config::WhatsAppConfig;
use brainwires_whatsapp_channel::event_handler::{WebhookState, build_router};
use brainwires_whatsapp_channel::gateway_client::GatewayClient;
use brainwires_whatsapp_channel::mcp_server::WhatsAppMcpServer;
use brainwires_whatsapp_channel::whatsapp::WhatsAppChannel;

/// Brainwires WhatsApp Channel Adapter
#[derive(Parser)]
#[command(name = "brainclaw-mcp-whatsapp")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "WhatsApp Business channel adapter for the Brainwires gateway")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the WhatsApp adapter (default mode).
    Serve {
        /// Meta Graph API access token.
        #[arg(long, env = "WHATSAPP_TOKEN")]
        token: String,

        /// WhatsApp phone number ID from the Meta Business dashboard.
        #[arg(long, env = "WHATSAPP_PHONE_NUMBER_ID")]
        phone_number_id: String,

        /// Webhook verify token (used to verify Meta's GET challenge).
        #[arg(long, env = "WHATSAPP_VERIFY_TOKEN")]
        verify_token: String,

        /// Port for the local webhook server (Meta POSTs inbound messages here).
        #[arg(long, default_value_t = 8090, env = "WHATSAPP_WEBHOOK_PORT")]
        webhook_port: u16,

        /// WebSocket URL of the brainwires-gateway.
        #[arg(long, default_value = "ws://127.0.0.1:18789/ws", env = "GATEWAY_URL")]
        gateway_url: String,

        /// Optional auth token for the gateway handshake.
        #[arg(long, env = "GATEWAY_TOKEN")]
        gateway_token: Option<String>,

        /// Optional Meta app secret for X-Hub-Signature-256 validation.
        #[arg(long, env = "WHATSAPP_APP_SECRET")]
        app_secret: Option<String>,

        /// Also start the MCP server on stdio.
        #[arg(long, default_value_t = false)]
        mcp: bool,
    },
    /// Show version information.
    Version,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Version) => {
            println!("brainclaw-mcp-whatsapp v{}", env!("CARGO_PKG_VERSION"));
        }
        Some(Commands::Serve {
            token,
            phone_number_id,
            verify_token,
            webhook_port,
            gateway_url,
            gateway_token,
            app_secret,
            mcp,
        }) => {
            let config = WhatsAppConfig {
                token,
                phone_number_id,
                verify_token,
                webhook_port,
                gateway_url,
                gateway_token,
                app_secret,
            };
            run_adapter(config, mcp).await?;
        }
        None => {
            eprintln!("No subcommand given. Use `serve` to start or `version` for info.");
            eprintln!("Run with --help for usage details.");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_adapter(config: WhatsAppConfig, enable_mcp: bool) -> Result<()> {
    tracing::info!("Starting Brainwires WhatsApp adapter");

    // Event channel — webhook → gateway
    let (event_tx, event_rx) = mpsc::channel(512);

    // Build WhatsApp channel
    let channel = Arc::new(WhatsAppChannel::new(
        config.token.clone(),
        config.phone_number_id.clone(),
    ));

    let capabilities = channel.capabilities();

    // Optionally start MCP server on stdio
    if enable_mcp {
        let mcp_channel = Arc::clone(&channel);
        tokio::spawn(async move {
            if let Err(e) = WhatsAppMcpServer::serve_stdio(mcp_channel).await {
                tracing::error!("MCP server error: {:#}", e);
            }
        });
    }

    // Start Axum webhook server
    let webhook_state = Arc::new(WebhookState {
        event_tx,
        verify_token: config.verify_token.clone(),
        app_secret: config.app_secret.clone(),
        phone_number_id: config.phone_number_id.clone(),
    });
    let router = build_router(webhook_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.webhook_port));
    tracing::info!(port = config.webhook_port, "Webhook server listening");

    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind webhook server: {}", e);
                return;
            }
        };
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("Webhook server error: {}", e);
        }
    });

    // Connect to gateway
    let gw_token = config.gateway_token.clone().unwrap_or_default();
    let gw_channel = Arc::clone(&channel);
    let gw_url = config.gateway_url.clone();

    tokio::spawn(async move {
        match GatewayClient::connect(&gw_url, &gw_token, capabilities).await {
            Ok(gw_client) => {
                if let Err(e) = gw_client.run(event_rx, gw_channel).await {
                    tracing::error!("Gateway client error: {:#}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to gateway: {:#}", e);
                tracing::info!("Running in webhook-only mode (no gateway)");
                let mut rx = event_rx;
                while rx.recv().await.is_some() {}
            }
        }
    });

    // Block until interrupted
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down WhatsApp adapter");
    Ok(())
}
