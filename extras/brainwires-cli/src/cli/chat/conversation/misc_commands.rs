//! Miscellaneous Commands
//!
//! Handlers for various other commands (model, exec, templates, tasks, etc.).

use anyhow::Result;

use crate::commands::executor::CommandAction;
use crate::types::agent::AgentContext;
use crate::utils::conversation::ConversationManager;
use crate::utils::logger::Logger;

/// Handle miscellaneous command actions
pub async fn handle_misc_action(
    action: CommandAction,
    _context: &mut AgentContext,
    _conversation_manager: &mut ConversationManager,
) -> Result<bool> {
    match action {
        CommandAction::SwitchModel(new_model) => {
            use crate::config::{ConfigManager, ConfigUpdates};

            match ConfigManager::new() {
                Ok(mut config_manager) => {
                    config_manager.update(ConfigUpdates {
                        model: Some(new_model.clone()),
                        ..Default::default()
                    });
                    if let Err(e) = config_manager.save() {
                        Logger::warn(&format!("Failed to persist model to config: {}", e));
                    }
                }
                Err(e) => {
                    Logger::warn(&format!("Failed to load config for model update: {}", e));
                }
            }

            Logger::info(&format!("Model switched to: {}", new_model));
            println!("{}\n", console::style(format!("Model switched to: {} (saved to config)", new_model)).green());
            println!("{}", console::style("Note: Restart the chat session to use the new model.").dim());
            Ok(true)
        }
        CommandAction::Exit => {
            Logger::info("Exit command received");
            println!("{}", console::style("Goodbye!").green());
            Ok(false)
        }
        CommandAction::SetApprovalMode(mode) => {
            Logger::info(&format!("Approval mode set to: {}", mode));
            println!("{}\n", console::style(format!("Approval mode set to: {}", mode)).green());
            println!("Note: Approval mode feature is currently a placeholder. Full implementation coming soon.");
            Ok(true)
        }
        CommandAction::ExecCommand(command) => {
            use std::process::Command as StdCommand;

            println!("{}", console::style(format!("Executing: {}", command)).cyan());
            Logger::info(&format!("Executing command: {}", command));

            match StdCommand::new("sh").arg("-c").arg(&command).status() {
                Ok(status) => {
                    let exit_code = status.code().unwrap_or(-1);
                    println!("{} (exit code: {})\n",
                        console::style("Command completed").green(), exit_code);
                }
                Err(e) => {
                    println!("{}: {}\n", console::style("Error").red().bold(), e);
                }
            }
            Ok(true)
        }
        CommandAction::ShowShellHistory => {
            println!("{}\n",
                console::style("Shell history viewer is only available in TUI mode").yellow());
            Ok(true)
        }
        CommandAction::ShowTasks => {
            println!("{}\n",
                console::style("Task view is only available in TUI mode (--tui).").yellow());
            Ok(true)
        }
        CommandAction::TaskComplete(_) |
        CommandAction::TaskSkip(_, _) |
        CommandAction::TaskAdd(_) |
        CommandAction::TaskStart(_) |
        CommandAction::TaskBlock(_, _) |
        CommandAction::TaskDepends(_, _) |
        CommandAction::TaskReady |
        CommandAction::TaskTime(_) |
        CommandAction::TaskList |
        CommandAction::ExecutePlan(_, _) => {
            println!("{}\n",
                console::style("Task commands are only available in TUI mode (--tui).").yellow());
            Ok(true)
        }
        CommandAction::ListTemplates => {
            use crate::storage::TemplateStore;

            match TemplateStore::with_default_dir() {
                Ok(store) => {
                    match store.list() {
                        Ok(templates) => {
                            if templates.is_empty() {
                                println!("{}\n",
                                    console::style("No templates saved. Save a template with /template:save <name>").yellow());
                            } else {
                                println!("{}\n", console::style("Templates:").cyan().bold());
                                for template in &templates {
                                    let vars_info = if template.variables.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" [vars: {}]", template.variables.join(", "))
                                    };
                                    println!("  {} - {}{}",
                                        console::style(&template.name).green(),
                                        template.description,
                                        console::style(vars_info).dim()
                                    );
                                    println!("    ID: {}", console::style(&template.template_id[..8]).dim());
                                }
                                println!();
                            }
                        }
                        Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
                    }
                }
                Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
            }
            Ok(true)
        }
        CommandAction::SaveTemplate(_, _) => {
            println!("{}\n",
                console::style("Template save requires an active plan. Use TUI mode (--tui) for full template support.").yellow());
            Ok(true)
        }
        CommandAction::ShowTemplate(name) => {
            use crate::storage::TemplateStore;

            match TemplateStore::with_default_dir() {
                Ok(store) => {
                    match store.get_by_name(&name) {
                        Ok(Some(template)) => {
                            println!("{}\n", console::style("Template Details:").cyan().bold());
                            println!("  Name: {}", console::style(&template.name).green());
                            println!("  ID: {}", template.template_id);
                            println!("  Description: {}", template.description);
                            if !template.variables.is_empty() {
                                println!("  Variables: {}", template.variables.join(", "));
                            }
                            println!("  Used: {} times\n", template.usage_count);
                            println!("{}", console::style("---").dim());
                            println!("\n{}\n", template.content);
                        }
                        Ok(None) => println!("{}\n",
                            console::style(format!("Template not found: {}", name)).yellow()),
                        Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
                    }
                }
                Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
            }
            Ok(true)
        }
        CommandAction::UseTemplate(_, _) => {
            println!("{}\n",
                console::style("Template use is only available in TUI mode (--tui).").yellow());
            Ok(true)
        }
        CommandAction::DeleteTemplate(name) => {
            use crate::storage::TemplateStore;

            match TemplateStore::with_default_dir() {
                Ok(store) => {
                    match store.get_by_name(&name) {
                        Ok(Some(template)) => {
                            match store.delete(&template.template_id) {
                                Ok(true) => println!("{}\n",
                                    console::style(format!("Template '{}' deleted.", template.name)).green()),
                                Ok(false) => println!("{}\n",
                                    console::style(format!("Template '{}' not found.", name)).yellow()),
                                Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
                            }
                        }
                        Ok(None) => println!("{}\n",
                            console::style(format!("Template not found: {}", name)).yellow()),
                        Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
                    }
                }
                Err(e) => println!("{}: {}\n", console::style("Error").red().bold(), e),
            }
            Ok(true)
        }
        CommandAction::SetPromptModeAsk => {
            println!("{}\n", console::style("Switched to Ask mode (read-only)").blue());
            Ok(true)
        }
        CommandAction::SetPromptModeEdit => {
            println!("{}\n", console::style("Switched to Edit mode (full tools)").cyan());
            Ok(true)
        }
        _ => Ok(true),
    }
}
