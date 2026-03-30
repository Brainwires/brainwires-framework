use anyhow::{Context, Result};
use clap::Subcommand;
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::mcp::{McpClient, McpConfigManager, McpServerConfig};
use crate::utils::logger::Logger;

lazy_static! {
    static ref MCP_CLIENT: Arc<RwLock<McpClient>> = Arc::new(RwLock::new(McpClient::new("brainwires", env!("CARGO_PKG_VERSION"))));
}

#[derive(Subcommand)]
pub enum McpCommands {
    /// List configured MCP servers
    List,
    /// Add a new MCP server
    Add {
        name: String,
        command: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Remove an MCP server
    Remove { name: String },
    /// Connect to an MCP server
    Connect { name: String },
    /// Disconnect from an MCP server
    Disconnect { name: String },
    /// List tools from a connected server
    Tools { server: String },
    /// List resources from a connected server
    Resources { server: String },
    /// List prompts from a connected server
    Prompts { server: String },
}

pub async fn handle_mcp(cmd: McpCommands) -> Result<()> {
    match cmd {
        McpCommands::List => handle_list().await,
        McpCommands::Add { name, command, args } => handle_add(name, command, args).await,
        McpCommands::Remove { name } => handle_remove(name).await,
        McpCommands::Connect { name } => handle_connect(name).await,
        McpCommands::Disconnect { name } => handle_disconnect(name).await,
        McpCommands::Tools { server } => handle_tools(server).await,
        McpCommands::Resources { server } => handle_resources(server).await,
        McpCommands::Prompts { server } => handle_prompts(server).await,
    }
}

async fn handle_list() -> Result<()> {
    let config_manager = McpConfigManager::load()?;
    let servers = config_manager.get_servers();

    if servers.is_empty() {
        println!("{}", console::style("No MCP servers configured").dim());
        println!("\nUse 'brainwires mcp add <name> <command> [args...]' to add a server");
        return Ok(());
    }

    println!("\n{}\n", console::style("Configured MCP Servers:").cyan().bold());

    let client = MCP_CLIENT.read().await;
    for server in servers {
        let connected = client.is_connected(&server.name).await;
        let status = if connected {
            console::style("● Connected").green()
        } else {
            console::style("○ Disconnected").dim()
        };

        println!(
            "{} {} - {} {}",
            status,
            console::style(&server.name).bold(),
            server.command,
            server.args.join(" ")
        );
    }

    println!();
    Ok(())
}

async fn handle_add(name: String, command: String, args: Vec<String>) -> Result<()> {
    let mut config_manager = McpConfigManager::load()?;

    let config = McpServerConfig {
        name: name.clone(),
        command,
        args,
        env: None,
    };

    config_manager.add_server(config)?;

    Logger::info(&format!("Added MCP server: {}", name));
    println!("{} {}", console::style("✓").green(), console::style(format!("Added server '{}'", name)).bold());

    Ok(())
}

async fn handle_remove(name: String) -> Result<()> {
    let mut config_manager = McpConfigManager::load()?;

    // Disconnect if connected
    let client = MCP_CLIENT.read().await;
    if client.is_connected(&name).await {
        drop(client);
        let client_write = MCP_CLIENT.write().await;
        client_write.disconnect(&name).await?;
        Logger::info(&format!("Disconnected from server: {}", name));
    }

    config_manager.remove_server(&name)?;

    Logger::info(&format!("Removed MCP server: {}", name));
    println!("{} {}", console::style("✓").green(), console::style(format!("Removed server '{}'", name)).bold());

    Ok(())
}

async fn handle_connect(name: String) -> Result<()> {
    let config_manager = McpConfigManager::load()?;
    let server_config = config_manager
        .get_server(&name)
        .context(format!("Server '{}' not found in config", name))?;

    println!("{}", console::style(format!("Connecting to '{}'...", name)).dim());

    let client = MCP_CLIENT.write().await;
    client.connect(server_config).await?;

    // Get server info
    let info = client.get_server_info(&name).await?;
    let capabilities = client.get_capabilities(&name).await?;

    Logger::info(&format!("Connected to MCP server: {}", name));
    println!("{} {}", console::style("✓").green(), console::style("Connected").bold());
    println!(
        "  Server: {} v{}",
        console::style(&info.name).cyan(),
        info.version
    );

    if capabilities.tools.is_some() {
        println!("  {} Tools", console::style("✓").green());
    }
    if capabilities.resources.is_some() {
        println!("  {} Resources", console::style("✓").green());
    }
    if capabilities.prompts.is_some() {
        println!("  {} Prompts", console::style("✓").green());
    }

    Ok(())
}

async fn handle_disconnect(name: String) -> Result<()> {
    let client = MCP_CLIENT.write().await;

    if !client.is_connected(&name).await {
        anyhow::bail!("Not connected to server '{}'", name);
    }

    client.disconnect(&name).await?;

    Logger::info(&format!("Disconnected from MCP server: {}", name));
    println!("{} {}", console::style("✓").green(), console::style(format!("Disconnected from '{}'", name)).bold());

    Ok(())
}

async fn handle_tools(server: String) -> Result<()> {
    let client = MCP_CLIENT.read().await;

    if !client.is_connected(&server).await {
        anyhow::bail!("Not connected to server '{}'. Use 'brainwires mcp connect {}'", server, server);
    }

    let tools = client.list_tools(&server).await?;

    if tools.is_empty() {
        println!("{}", console::style("No tools available").dim());
        return Ok(());
    }

    println!("\n{} from '{}':\n", console::style("Tools").cyan().bold(), server);

    for tool in tools {
        println!("{}", console::style(&tool.name).bold());
        if let Some(desc) = &tool.description {
            println!("  {}", console::style(desc.as_ref()).dim());
        }
        println!();
    }

    Ok(())
}

async fn handle_resources(server: String) -> Result<()> {
    let client = MCP_CLIENT.read().await;

    if !client.is_connected(&server).await {
        anyhow::bail!("Not connected to server '{}'. Use 'brainwires mcp connect {}'", server, server);
    }

    let resources = client.list_resources(&server).await?;

    if resources.is_empty() {
        println!("{}", console::style("No resources available").dim());
        return Ok(());
    }

    println!("\n{} from '{}':\n", console::style("Resources").cyan().bold(), server);

    for resource in resources {
        println!("{}", console::style(&resource.name).bold());
        println!("  URI: {}", console::style(&resource.uri).cyan());
        if let Some(desc) = &resource.description {
            println!("  {}", console::style(desc).dim());
        }
        println!();
    }

    Ok(())
}

async fn handle_prompts(server: String) -> Result<()> {
    let client = MCP_CLIENT.read().await;

    if !client.is_connected(&server).await {
        anyhow::bail!("Not connected to server '{}'. Use 'brainwires mcp connect {}'", server, server);
    }

    let prompts = client.list_prompts(&server).await?;

    if prompts.is_empty() {
        println!("{}", console::style("No prompts available").dim());
        return Ok(());
    }

    println!("\n{} from '{}':\n", console::style("Prompts").cyan().bold(), server);

    for prompt in prompts {
        println!("{}", console::style(&prompt.name).bold());
        if let Some(desc) = &prompt.description {
            println!("  {}", console::style(desc).dim());
        }
        if let Some(args) = &prompt.arguments {
            if !args.is_empty() {
                println!("  Arguments:");
                for arg in args {
                    let required = if arg.required.unwrap_or(false) { "(required)" } else { "(optional)" };
                    let arg_desc = arg.description.as_deref().unwrap_or("");
                    println!("    - {} {} - {}", arg.name, console::style(required).dim(), arg_desc);
                }
            }
        }
        println!();
    }

    Ok(())
}
