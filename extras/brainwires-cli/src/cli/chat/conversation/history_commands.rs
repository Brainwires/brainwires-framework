//! History Commands
//!
//! Handlers for conversation history management commands.

use anyhow::Result;

use crate::commands::executor::CommandAction;
use crate::types::agent::AgentContext;
use crate::types::message::{Message, MessageContent, Role};
use crate::utils::conversation::ConversationManager;
use crate::utils::logger::Logger;

/// Handle history-related command actions
pub async fn handle_history_action(
    action: CommandAction,
    model_id: &str,
    context: &mut AgentContext,
    conversation_manager: &mut ConversationManager,
    cleared_conversation_manager: &mut Option<ConversationManager>,
    message_store: &crate::storage::MessageStore,
) -> Result<bool> {
    match action {
        CommandAction::ClearHistory => {
            *cleared_conversation_manager = Some(conversation_manager.snapshot());
            context.conversation_history.clear();
            *conversation_manager = ConversationManager::new(128000);
            conversation_manager.set_model(model_id.to_string());
            Logger::info("Conversation history cleared");
            println!("{}\n", console::style("Conversation cleared (use /resume to restore)").green());
            Ok(true)
        }
        CommandAction::ResumeHistory(conversation_id) => {
            if let Some(conv_id) = conversation_id {
                Logger::info(&format!("Loading conversation: {}", conv_id));
                match message_store.get_by_conversation(&conv_id).await {
                    Ok(message_metadata) => {
                        conversation_manager.clear();
                        for msg_meta in message_metadata {
                            let role = match msg_meta.role.as_str() {
                                "user" => Role::User,
                                "assistant" => Role::Assistant,
                                "system" => Role::System,
                                "tool" => Role::Tool,
                                _ => Role::User,
                            };
                            let message = Message {
                                role,
                                content: MessageContent::Text(msg_meta.content.clone()),
                                name: None,
                                metadata: None,
                            };
                            conversation_manager.add_message(message);
                        }
                        Logger::info(&format!("Loaded {} messages from conversation {}",
                            conversation_manager.get_messages().len(), conv_id));
                        println!("{}\n", console::style(format!("Loaded conversation: {} ({} messages)",
                            &conv_id[..8.min(conv_id.len())],
                            conversation_manager.get_messages().len())).green());
                    }
                    Err(e) => {
                        println!("{}\n", console::style(format!("Failed to load conversation: {}", e)).red());
                    }
                }
            } else if let Some(cleared) = cleared_conversation_manager.take() {
                *conversation_manager = cleared;
                Logger::info("Conversation history resumed");
                println!("{}\n", console::style("Conversation resumed").green());
            } else {
                println!("{}\n", console::style("No cleared conversation to resume").yellow());
            }
            Ok(true)
        }
        CommandAction::Rewind(steps) => {
            let messages = conversation_manager.get_messages();
            let remove_count = (steps * 2).min(messages.len());
            let keep_count = messages.len().saturating_sub(remove_count);

            let mut new_manager = ConversationManager::new(128000);
            new_manager.set_model(model_id.to_string());
            for msg in &messages[..keep_count] {
                new_manager.add_message(msg.clone());
            }
            *conversation_manager = new_manager;
            context.conversation_history = conversation_manager.get_messages().to_vec();
            Logger::info(&format!("Rewound {} steps", steps));
            println!("{}\n", console::style(format!("Rewound {} steps", steps)).green());
            Ok(true)
        }
        CommandAction::ShowStatus => {
            println!("Session ID: {}", conversation_manager.conversation_id());
            println!("Model: {}", model_id);
            println!("Messages: {}", conversation_manager.get_messages().len());
            println!();
            Ok(true)
        }
        _ => Ok(true),
    }
}
