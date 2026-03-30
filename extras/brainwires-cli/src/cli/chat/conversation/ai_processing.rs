//! AI Processing
//!
//! Handles AI response processing and streaming.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};
use std::sync::Arc;

use crate::agents::OrchestratorAgent;
use crate::cli::chat::streaming::process_chat_stream;
use crate::mdap::MdapConfig;
use crate::types::agent::{AgentContext, PermissionMode};
use crate::types::message::{Message, MessageContent, Role};
use crate::utils::conversation::ConversationManager;
use crate::utils::logger::Logger;

/// Process AI response
pub async fn process_ai_response(
    provider_instance: &Arc<dyn crate::providers::Provider>,
    context: &mut AgentContext,
    conversation_manager: &mut ConversationManager,
    model_id: &str,
    input: &str,
    quiet: bool,
    format: &str,
) -> Result<()> {
    // Show thinking indicator (unless quiet)
    let spinner = if !quiet {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message("Thinking...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(spinner)
    } else {
        None
    };

    // Get enhanced context with auto-injected relevant history
    let mut enhanced_messages = conversation_manager
        .get_enhanced_context(input)
        .await
        .unwrap_or_else(|e| {
            Logger::debug(&format!("Enhanced context failed, using raw: {}", e));
            conversation_manager.get_messages().to_vec()
        });

    // Inject working set files as a system message if non-empty
    if let Some(working_set_context) = context.working_set.build_context_injection() {
        let ws_system_msg = Message {
            role: Role::System,
            content: MessageContent::Text(working_set_context),
            name: None,
            metadata: None,
        };
        // Insert after system messages but before conversation
        let insert_pos = enhanced_messages.iter()
            .take_while(|m| m.role == Role::System)
            .count();
        enhanced_messages.insert(insert_pos, ws_system_msg);

        // Increment turn counter for working set (tracks access freshness)
        context.working_set.next_turn();
    }

    // Update context with enhanced messages for the API call
    context.conversation_history = enhanced_messages;

    // Process stream with tool execution support
    let response_text = process_chat_stream(
        provider_instance,
        context,
        &spinner,
        model_id,
        Some(conversation_manager.conversation_id().to_string()),
    ).await;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    match response_text {
        Ok(text) => {
            // Print assistant response based on format
            match format {
                "plain" => {
                    println!("{}", text);
                }
                "json" => {
                    // JSON format handled at exit
                }
                _ => {
                    // Full format (default): formatted with label and typing effect
                    if !quiet {
                        print!("\n{}: ", console::style("Assistant").green().bold());
                        io::stdout().flush()?;

                        // Print with typing effect
                        for chunk in text.chars() {
                            print!("{}", chunk);
                            io::stdout().flush()?;
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        println!("\n");
                    } else {
                        println!("{}", text);
                    }
                }
            }

            // Add assistant response to conversation manager
            let assistant_message = Message {
                role: Role::Assistant,
                content: MessageContent::Text(text.clone()),
                name: None,
                metadata: None,
            };
            conversation_manager.add_message(assistant_message);

            // Auto-save after each exchange
            if let Err(e) = conversation_manager.save_to_db().await {
                Logger::warn(&format!("Failed to auto-save conversation: {}", e));
            }
        }
        Err(e) => {
            Logger::error(&format!("Error: {}", e));
            println!(
                "\n{}: {}\n",
                console::style("Error").red().bold(),
                e
            );
        }
    }

    Ok(())
}

/// Process AI response with MDAP mode for high reliability
pub async fn process_ai_response_mdap(
    provider_instance: &Arc<dyn crate::providers::Provider>,
    context: &mut AgentContext,
    conversation_manager: &mut ConversationManager,
    _model_id: &str,
    input: &str,
    quiet: bool,
    format: &str,
    mdap_config: &MdapConfig,
) -> Result<()> {
    // Show thinking indicator with MDAP info
    let spinner = if !quiet {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(format!("MDAP Processing (k={})...", mdap_config.k));
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(spinner)
    } else {
        None
    };

    // Get enhanced context with auto-injected relevant history
    let enhanced_messages = conversation_manager
        .get_enhanced_context(input)
        .await
        .unwrap_or_else(|e| {
            Logger::debug(&format!("Enhanced context failed, using raw: {}", e));
            conversation_manager.get_messages().to_vec()
        });

    // Update context with enhanced messages
    context.conversation_history = enhanced_messages;

    // Create orchestrator and execute with MDAP
    let mut orchestrator = OrchestratorAgent::new(provider_instance.clone(), PermissionMode::Auto);

    let result = orchestrator.execute_mdap(input, context, mdap_config.clone()).await;

    if let Some(s) = &spinner {
        s.finish_and_clear();
    }

    match result {
        Ok((response, metrics)) => {
            // Print assistant response based on format
            match format {
                "plain" => {
                    println!("{}", response.message);
                }
                "json" => {
                    let output = serde_json::json!({
                        "response": response.message,
                        "mdap": {
                            "success": metrics.final_success,
                            "steps_completed": metrics.completed_steps,
                            "total_samples": metrics.total_samples,
                            "red_flagged": metrics.red_flagged_samples,
                            "cost_usd": metrics.actual_cost_usd,
                            "time_seconds": metrics.total_time_seconds,
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                }
                _ => {
                    // Full format (default)
                    if !quiet {
                        print!("\n{}: ", console::style("Assistant").green().bold());
                        io::stdout().flush()?;

                        // Print with typing effect
                        for chunk in response.message.chars() {
                            print!("{}", chunk);
                            io::stdout().flush()?;
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        println!();

                        // Show MDAP metrics summary
                        println!("\n{}", console::style("MDAP Metrics:").cyan().dim());
                        println!("{}\n", console::style(metrics.summary()).dim());
                    } else {
                        println!("{}", response.message);
                    }
                }
            }

            // Add assistant response to conversation manager
            let assistant_message = Message {
                role: Role::Assistant,
                content: MessageContent::Text(response.message.clone()),
                name: None,
                metadata: None,
            };
            conversation_manager.add_message(assistant_message);

            // Auto-save after each exchange
            if let Err(e) = conversation_manager.save_to_db().await {
                Logger::warn(&format!("Failed to auto-save conversation: {}", e));
            }
        }
        Err(e) => {
            Logger::error(&format!("MDAP Error: {}", e));
            println!(
                "\n{}: {}\n",
                console::style("MDAP Error").red().bold(),
                e
            );
        }
    }

    Ok(())
}
