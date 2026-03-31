//! Checkpoint Commands
//!
//! Handlers for checkpoint management commands.

use anyhow::Result;

use crate::commands::executor::CommandAction;
use crate::types::agent::AgentContext;
use crate::utils::checkpoint::CheckpointManager;
use crate::utils::conversation::ConversationManager;
use crate::utils::logger::Logger;

/// Handle checkpoint-related command actions
pub async fn handle_checkpoint_action(
    action: CommandAction,
    model_id: &str,
    context: &mut AgentContext,
    conversation_manager: &mut ConversationManager,
    checkpoint_manager: &CheckpointManager,
) -> Result<bool> {
    match action {
        CommandAction::CreateCheckpoint(name) => {
            let messages = conversation_manager.get_messages().to_vec();
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("model".to_string(), model_id.to_string());

            match checkpoint_manager
                .create_checkpoint(
                    name.clone(),
                    conversation_manager.conversation_id().to_string(),
                    messages,
                    metadata,
                )
                .await
            {
                Ok(checkpoint_id) => {
                    let display_name = name.unwrap_or_else(|| checkpoint_id[..8].to_string());
                    Logger::info(format!("Created checkpoint: {}", display_name));
                    println!(
                        "{}\n",
                        console::style(format!("Checkpoint created: {}", display_name)).green()
                    );
                }
                Err(e) => {
                    Logger::error(format!("Failed to create checkpoint: {}", e));
                    println!("{}: {}\n", console::style("Error").red().bold(), e);
                }
            }
            Ok(true)
        }
        CommandAction::RestoreCheckpoint(checkpoint_id) => {
            match checkpoint_manager.restore_checkpoint(&checkpoint_id).await {
                Ok(checkpoint) => {
                    let mut new_manager = ConversationManager::new(128000);
                    new_manager.set_model(model_id.to_string());
                    new_manager.set_conversation_id(checkpoint.conversation_id.clone());
                    for msg in &checkpoint.messages {
                        new_manager.add_message(msg.clone());
                    }
                    *conversation_manager = new_manager;
                    context.conversation_history = conversation_manager.get_messages().to_vec();

                    let display_name = checkpoint
                        .name
                        .unwrap_or_else(|| checkpoint.id[..8].to_string());
                    Logger::info(format!("Restored checkpoint: {}", display_name));
                    println!(
                        "{}\n",
                        console::style(format!("Restored checkpoint: {}", display_name)).green()
                    );
                }
                Err(e) => {
                    Logger::error(format!("Failed to restore checkpoint: {}", e));
                    println!("{}: {}\n", console::style("Error").red().bold(), e);
                }
            }
            Ok(true)
        }
        CommandAction::ListCheckpoints => {
            match checkpoint_manager
                .list_checkpoints(conversation_manager.conversation_id())
                .await
            {
                Ok(checkpoints) => {
                    if checkpoints.is_empty() {
                        println!("{}\n", console::style("No checkpoints found").yellow());
                    } else {
                        println!("{}\n", console::style("Checkpoints:").cyan().bold());
                        for (i, checkpoint) in checkpoints.iter().enumerate() {
                            let name = checkpoint.name.as_deref().unwrap_or("Unnamed");
                            let created =
                                chrono::DateTime::from_timestamp(checkpoint.created_at, 0)
                                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                    .unwrap_or_else(|| "Unknown".to_string());
                            println!(
                                "  {}. {} - {} messages ({})",
                                i + 1,
                                console::style(name).green(),
                                checkpoint.messages.len(),
                                console::style(created).dim()
                            );
                            println!("     ID: {}", console::style(&checkpoint.id[..8]).dim());
                        }
                        println!();
                    }
                }
                Err(e) => {
                    Logger::error(format!("Failed to list checkpoints: {}", e));
                    println!("{}: {}\n", console::style("Error").red().bold(), e);
                }
            }
            Ok(true)
        }
        _ => Ok(true),
    }
}
