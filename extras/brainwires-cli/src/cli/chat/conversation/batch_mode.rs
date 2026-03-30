//! Batch Mode
//!
//! Batch processing mode handler for processing multiple prompts from stdin.

use anyhow::{Context, Result};
use serde_json::json;
use std::io::{self, BufRead};

use crate::auth::SessionManager;
use crate::cli::chat::streaming::process_chat_stream;
use crate::config::ConfigManager;
use crate::providers::ProviderFactory;
use crate::tools::ToolRegistry;
use crate::types::agent::AgentContext;
use crate::types::message::{Message, MessageContent, Role};
use crate::utils::logger::Logger;
use crate::utils::system_prompt::build_system_prompt;

/// Handle batch processing mode
pub async fn handle_batch_mode(
    model: Option<String>,
    _provider: Option<String>,
    system: Option<String>,
    quiet: bool,
    format: &str,
    backend_url_override: Option<String>,
) -> Result<()> {
    // Load configuration and session
    let config_manager = ConfigManager::new()?;
    let session = SessionManager::load()?;

    // Resolve model from config
    let config = config_manager.get();
    let model_id = match model {
        Some(m) => m,
        None => config.model.clone(),
    };

    if !quiet {
        if let Some(ref url) = backend_url_override {
            Logger::info(&format!("Starting batch mode with {} (dev backend: {})", model_id, url));
        } else {
            Logger::info(&format!("Starting batch mode with {} (brainwires)", model_id));
        }
    }

    // Create provider with optional backend URL override
    let factory = ProviderFactory;
    let provider_instance = factory
        .create_with_backend(model_id.clone(), backend_url_override)
        .await
        .context("Failed to create provider. Run: brainwires auth status")?;

    // Read prompts from stdin
    let stdin = io::stdin();
    let mut stdin_reader = stdin.lock();
    let mut results = Vec::new();

    loop {
        let mut line = String::new();
        match stdin_reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let prompt = line.trim();
                if prompt.is_empty() {
                    continue;
                }

                // Process each prompt independently
                let user_id = session.as_ref().map(|s| s.user.user_id.clone());
                let registry = ToolRegistry::with_builtins();
                let mut context = AgentContext {
                    working_directory: std::env::current_dir()?.to_string_lossy().to_string(),
                    user_id,
                    conversation_history: Vec::new(),
                    tools: registry.get_core().into_iter().cloned().collect(),
                    metadata: std::collections::HashMap::new(),
                    working_set: crate::types::WorkingSet::new(),
                    // Use full_access for CLI mode - users expect agents to have write access
                    capabilities: brainwires::permissions::AgentCapabilities::full_access(),
                };

                // Build system message
                let system_prompt = build_system_prompt(system.clone())?;
                let sys_message = Message {
                    role: Role::System,
                    content: MessageContent::Text(system_prompt),
                    name: None,
                    metadata: None,
                };
                context.conversation_history.push(sys_message);

                // Add user prompt
                let user_message = Message {
                    role: Role::User,
                    content: MessageContent::Text(prompt.to_string()),
                    name: None,
                    metadata: None,
                };
                context.conversation_history.push(user_message);

                // Process without spinner in batch mode
                let response_text = process_chat_stream(
                    &provider_instance,
                    &context,
                    &None,
                    &model_id,
                    None,
                ).await;

                match response_text {
                    Ok(text) => {
                        match format {
                            "plain" => {
                                println!("{}", text);
                            }
                            "json" => {
                                results.push(json!({
                                    "prompt": prompt,
                                    "response": text,
                                }));
                            }
                            _ => {
                                // Full format
                                if !quiet {
                                    println!("{}: {}", console::style("Q").cyan(), prompt);
                                    println!("{}: {}\n", console::style("A").green(), text);
                                } else {
                                    println!("{}", text);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if format == "json" {
                            results.push(json!({
                                "prompt": prompt,
                                "error": e.to_string(),
                            }));
                        } else {
                            eprintln!("{}: {}", console::style("Error").red().bold(), e);
                        }
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Output JSON results if in JSON format
    if format == "json" {
        let output = json!({
            "model": model_id,
            "results": results,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    }

    Ok(())
}
