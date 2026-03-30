//! Test slash command system
//!
//! Run with: cargo run --example test_commands

use brainwires_cli::commands::CommandExecutor;

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("brainwires_cli=info")
        .init();

    println!("=== Testing Slash Command System ===\n");

    // Create executor
    let executor = CommandExecutor::new()?;

    println!("Loaded {} commands\n", executor.registry().list_commands().len());

    // Test /help
    println!("Testing /help:");
    println!("----------------");
    match executor.parse_input("/help") {
        Some((cmd, args)) => {
            match executor.execute(&cmd, &args)? {
                brainwires_cli::commands::executor::CommandResult::Help(lines) => {
                    for line in lines {
                        println!("{}", line);
                    }
                }
                _ => println!("Unexpected result"),
            }
        }
        None => println!("Failed to parse /help"),
    }

    println!("\n\nTesting /model:");
    println!("----------------");
    match executor.parse_input("/model llama-3.3-70b-versatile") {
        Some((cmd, args)) => {
            match executor.execute(&cmd, &args)? {
                brainwires_cli::commands::executor::CommandResult::Action(action) => {
                    println!("Action: {:?}", action);
                }
                _ => println!("Unexpected result"),
            }
        }
        None => println!("Failed to parse /model"),
    }

    // Test custom command if available
    println!("\n\nTesting custom /explain command:");
    println!("----------------");
    if executor.registry().get("explain").is_some() {
        match executor.parse_input("/explain async/await in Rust") {
            Some((cmd, args)) => {
                match executor.execute(&cmd, &args)? {
                    brainwires_cli::commands::executor::CommandResult::Message(msg) => {
                        println!("{}", msg);
                    }
                    _ => println!("Unexpected result"),
                }
            }
            None => println!("Failed to parse /explain"),
        }
    } else {
        println!("Custom /explain command not found (expected at .brainwires/commands/explain.md)");
    }

    Ok(())
}
