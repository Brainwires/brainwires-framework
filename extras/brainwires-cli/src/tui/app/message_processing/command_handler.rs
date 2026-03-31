//! Command Handler
//!
//! Handles slash command parsing and execution.

use super::super::state::{App, AppMode, ApprovalMode, TuiMessage};
use crate::providers::ProviderFactory;
use crate::types::message::{Message, MessageContent, Role};
use anyhow::Result;

impl App {
    /// Handle slash command execution
    /// Returns true if AI processing should be skipped (command was a pure action/help/error)
    /// Returns false if AI should be called (command produced a message to send)
    pub(super) async fn handle_command(
        &mut self,
        cmd_name: String,
        cmd_args: &[String],
        _user_content: String,
    ) -> Result<bool> {
        use crate::commands::executor::CommandResult;

        // Execute slash command
        match self.command_executor.execute(&cmd_name, cmd_args) {
            Ok(CommandResult::Help(lines)) => {
                // Display help as system message
                let help_text = lines.join("\n");
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: help_text,
                    created_at: chrono::Utc::now().timestamp(),
                });
                self.clear_input();
                Ok(true) // Skip AI
            }
            Ok(CommandResult::Action(action)) => {
                self.handle_command_action(action).await?;
                Ok(true) // Skip AI - actions don't need AI response
            }
            Ok(CommandResult::ActionWithMessage(action, msg)) => {
                // Execute the action (e.g., switch prompt mode), then send the message to AI
                self.handle_command_action(action).await?;

                // Add the message as user input for AI processing
                let user_message = TuiMessage {
                    role: "user".to_string(),
                    content: msg.clone(),
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.messages.push(user_message);

                self.conversation_history.push(Message {
                    role: Role::User,
                    content: MessageContent::Text(msg),
                    name: None,
                    metadata: None,
                });
                Ok(false) // Don't skip AI - need to process the message
            }
            Ok(CommandResult::Message(msg)) => {
                // Command produced a message to send to AI
                // Use the expanded message instead of original input
                let user_message = TuiMessage {
                    role: "user".to_string(),
                    content: msg.clone(),
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.messages.push(user_message);

                self.conversation_history.push(Message {
                    role: Role::User,
                    content: MessageContent::Text(msg.clone()),
                    name: None,
                    metadata: None,
                });
                // Continue to AI processing
                Ok(false) // Don't skip AI - need to process the message
            }
            Err(e) => {
                // Display error as system message
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: format!("Command error: {}", e),
                    created_at: chrono::Utc::now().timestamp(),
                });
                self.clear_input();
                Ok(true) // Skip AI
            }
        }
    }

    /// Handle command action, returns true if processing should stop
    pub(super) async fn handle_command_action(
        &mut self,
        action: crate::commands::executor::CommandAction,
    ) -> Result<bool> {
        use crate::commands::executor::CommandAction;

        match action {
            CommandAction::ClearHistory => {
                // Save current state before clearing (for /resume)
                self.cleared_messages = Some(self.messages.clone());
                self.cleared_conversation_history = Some(self.conversation_history.clone());

                // Clear conversation history
                self.messages.clear();
                self.conversation_history.clear();
                // Also clear shell history
                self.shell_history.clear();
                self.selected_shell_index = 0;
                self.shell_viewer_scroll = 0;

                self.status =
                    "Conversation and shell history cleared (use /resume to restore)".to_string();
                self.clear_input();
                Ok(true)
            }
            CommandAction::ResumeHistory(conversation_id) => {
                use super::super::session_management::SessionManagement;
                if let Some(conv_id) = conversation_id {
                    // Load conversation from database by ID
                    match self.load_conversation(&conv_id).await {
                        Ok(()) => {
                            self.status = format!("Loaded conversation: {}", conv_id);
                        }
                        Err(e) => {
                            self.status = format!("Failed to load conversation: {}", e);
                        }
                    }
                } else {
                    // Show session picker to select a conversation
                    match self.conversation_store.list(Some(50)).await {
                        Ok(conversations) => {
                            if conversations.is_empty() {
                                self.status = "No saved conversations found".to_string();
                            } else {
                                // list() already returns sorted by updated_at descending (newest first)
                                self.available_sessions = conversations;
                                self.selected_session_index = 0;
                                self.session_picker_scroll = 0;
                                self.mode = AppMode::SessionPicker;
                                self.status = "Select a conversation to resume (↑/↓ to navigate, Enter to select, Esc to cancel)".to_string();
                            }
                        }
                        Err(e) => {
                            self.status = format!("Failed to load conversations: {}", e);
                        }
                    }
                }
                self.clear_input();
                Ok(true)
            }
            CommandAction::SwitchModel(new_model) => {
                // Switch model by recreating the provider
                match ProviderFactory::new().create(new_model.clone()).await {
                    Ok(new_provider) => {
                        self.provider = new_provider;
                        self.model = new_model.clone();

                        // Persist the model selection to config
                        if let Err(e) = Self::update_config_model(&new_model) {
                            tracing::warn!("Failed to persist model to config: {}", e);
                        }

                        self.status = format!("Model switched to: {}", new_model);
                    }
                    Err(e) => {
                        self.status = format!("Failed to switch model: {}", e);
                    }
                }
                self.clear_input();
                Ok(true)
            }
            CommandAction::ShowStatus => {
                // Show status as system message
                let status_msg = format!(
                    "Session: {}\nModel: {}\nMessages: {}",
                    self.session_id,
                    self.model,
                    self.messages.len()
                );
                self.messages.push(TuiMessage {
                    role: "system".to_string(),
                    content: status_msg,
                    created_at: chrono::Utc::now().timestamp(),
                });
                self.clear_input();
                Ok(true)
            }
            CommandAction::Rewind(steps) => {
                // Rewind conversation
                let remove_count = (steps * 2).min(self.messages.len());
                for _ in 0..remove_count {
                    self.messages.pop();
                    self.conversation_history.pop();
                }
                self.status = format!("Rewound {} steps", steps);
                self.clear_input();
                Ok(true)
            }
            CommandAction::CreateCheckpoint(name) => {
                self.handle_create_checkpoint(name).await?;
                Ok(true)
            }
            CommandAction::RestoreCheckpoint(checkpoint_id) => {
                self.handle_restore_checkpoint(checkpoint_id).await?;
                Ok(true)
            }
            CommandAction::ListCheckpoints => {
                self.handle_list_checkpoints().await?;
                Ok(true)
            }
            CommandAction::Exit => {
                // Exit the application
                self.should_quit = true;
                self.clear_input();
                Ok(true)
            }
            CommandAction::SetApprovalMode(mode) => {
                // Set approval mode
                self.approval_mode = match mode.as_str() {
                    "suggest" => ApprovalMode::Suggest,
                    "auto-edit" => ApprovalMode::AutoEdit,
                    "full-auto" => ApprovalMode::FullAuto,
                    _ => ApprovalMode::Suggest, // Default to safest
                };
                self.status = format!("Approval mode set to: {}", mode);
                self.clear_input();
                Ok(true)
            }
            CommandAction::ExecCommand(command) => {
                // Store command to be executed in main loop where we have terminal access
                self.pending_exec_command = Some(command);
                self.clear_input();
                Ok(true)
            }
            CommandAction::ShowShellHistory => {
                // Open shell history viewer
                self.mode = AppMode::ShellViewer;
                self.selected_shell_index = self.shell_history.len().saturating_sub(1);
                self.shell_viewer_scroll = 0;
                self.clear_input();
                Ok(true)
            }
            CommandAction::OpenHotkeyDialog => {
                // Open hotkey configuration dialog
                use ratatui_interact::components::hotkey_dialog::HotkeyDialogState;
                self.hotkey_dialog_state = Some(HotkeyDialogState::new());
                self.mode = AppMode::HotkeyDialog;
                self.clear_input();
                Ok(true)
            }
            CommandAction::ListPlans(conversation_id) => {
                self.handle_list_plans(conversation_id).await?;
                Ok(true)
            }
            CommandAction::ShowPlan(plan_id) => {
                self.handle_show_plan(plan_id).await?;
                Ok(true)
            }
            CommandAction::DeletePlan(plan_id) => {
                self.handle_delete_plan(plan_id).await?;
                Ok(true)
            }
            CommandAction::ActivatePlan(plan_id) => {
                self.handle_activate_plan(plan_id).await?;
                Ok(true)
            }
            CommandAction::DeactivatePlan => {
                self.handle_deactivate_plan();
                Ok(true)
            }
            CommandAction::PlanStatus => {
                self.handle_plan_status();
                Ok(true)
            }
            CommandAction::PausePlan => {
                self.handle_pause_plan().await?;
                Ok(true)
            }
            CommandAction::ResumePlan(plan_id) => {
                self.handle_resume_plan(plan_id).await?;
                Ok(true)
            }
            CommandAction::ShowTasks => {
                self.handle_show_tasks().await;
                Ok(true)
            }
            CommandAction::TaskComplete(task_id) => {
                self.handle_task_complete(task_id).await;
                Ok(true)
            }
            CommandAction::TaskSkip(task_id, reason) => {
                self.handle_task_skip(task_id, reason).await;
                Ok(true)
            }
            CommandAction::TaskAdd(description) => {
                self.handle_task_add(description).await;
                Ok(true)
            }
            CommandAction::TaskStart(task_id) => {
                self.handle_task_start(task_id).await;
                Ok(true)
            }
            CommandAction::TaskBlock(task_id, reason) => {
                self.handle_task_block(task_id, reason).await;
                Ok(true)
            }
            CommandAction::TaskDepends(task_id, depends_on) => {
                self.handle_task_depends(task_id, depends_on).await;
                Ok(true)
            }
            CommandAction::TaskReady => {
                self.handle_task_ready().await;
                Ok(true)
            }
            CommandAction::TaskTime(task_id) => {
                self.handle_task_time(task_id).await;
                Ok(true)
            }
            CommandAction::TaskList => {
                self.handle_task_list().await;
                Ok(true)
            }
            CommandAction::ExecutePlan(plan_id, mode) => {
                self.handle_execute_plan(plan_id, mode).await?;
                Ok(true)
            }
            CommandAction::ListTemplates => {
                self.handle_list_templates().await;
                Ok(true)
            }
            CommandAction::SaveTemplate(name, description) => {
                self.handle_save_template(name, description).await?;
                Ok(true)
            }
            CommandAction::ShowTemplate(name) => {
                self.handle_show_template(name).await;
                Ok(true)
            }
            CommandAction::UseTemplate(name, vars) => {
                self.handle_use_template(name, vars).await?;
                Ok(true)
            }
            CommandAction::DeleteTemplate(name) => {
                self.handle_delete_template(name).await;
                Ok(true)
            }
            CommandAction::SearchPlans(query) => {
                self.handle_search_plans(query).await;
                Ok(true)
            }
            CommandAction::BranchPlan(name, task) => {
                self.handle_branch_plan(name, task).await?;
                Ok(true)
            }
            CommandAction::MergePlan(plan_id) => {
                self.handle_merge_plan(plan_id).await?;
                Ok(true)
            }
            CommandAction::PlanTree(plan_id) => {
                self.handle_plan_tree(plan_id).await;
                Ok(true)
            }
            // Context/Working Set commands
            CommandAction::ContextShow => {
                self.add_console_message(self.working_set.display());
                Ok(true)
            }
            CommandAction::ContextAdd(path, pinned) => {
                self.handle_context_add(&path, pinned);
                Ok(true)
            }
            CommandAction::ContextRemove(path) => {
                self.handle_context_remove(&path);
                Ok(true)
            }
            CommandAction::ContextPin(path) => {
                self.handle_context_pin(&path);
                Ok(true)
            }
            CommandAction::ContextUnpin(path) => {
                self.handle_context_unpin(&path);
                Ok(true)
            }
            CommandAction::ContextClear(keep_pinned) => {
                self.handle_context_clear(keep_pinned);
                Ok(true)
            }
            // Tool mode commands
            CommandAction::ShowToolMode => {
                self.handle_show_tool_mode();
                Ok(true)
            }
            CommandAction::SetToolMode(mode) => {
                self.handle_set_tool_mode(mode);
                Ok(true)
            }
            CommandAction::OpenToolPicker => {
                self.handle_open_tool_picker();
                Ok(true)
            }
            // MDAP commands
            CommandAction::MdapStatus => {
                self.handle_mdap_status();
                Ok(true)
            }
            CommandAction::MdapEnable => {
                self.handle_mdap_enable();
                Ok(true)
            }
            CommandAction::MdapDisable => {
                self.handle_mdap_disable();
                Ok(true)
            }
            CommandAction::MdapSetK(k) => {
                self.handle_mdap_set_k(k);
                Ok(true)
            }
            CommandAction::MdapSetTarget(target) => {
                self.handle_mdap_set_target(target);
                Ok(true)
            }
            // Knowledge commands
            CommandAction::LearnTruth(rule, rationale) => {
                self.handle_learn_truth(&rule, rationale.as_deref()).await;
                Ok(true)
            }
            CommandAction::KnowledgeStatus => {
                self.handle_knowledge_status().await;
                Ok(true)
            }
            CommandAction::KnowledgeList(category) => {
                self.handle_knowledge_list(category.as_deref()).await;
                Ok(true)
            }
            CommandAction::KnowledgeSearch(query) => {
                self.handle_knowledge_search(&query).await;
                Ok(true)
            }
            CommandAction::KnowledgeSync => {
                self.handle_knowledge_sync().await;
                Ok(true)
            }
            CommandAction::KnowledgeContradict(id, reason) => {
                self.handle_knowledge_contradict(&id, reason.as_deref())
                    .await;
                Ok(true)
            }
            CommandAction::KnowledgeDelete(id) => {
                self.handle_knowledge_delete(&id).await;
                Ok(true)
            }
            // Personal Knowledge System commands
            CommandAction::ProfileShow => {
                self.handle_profile_show().await;
                Ok(true)
            }
            CommandAction::ProfileSet(key, value, local_only) => {
                self.handle_profile_set(&key, &value, local_only).await;
                Ok(true)
            }
            CommandAction::ProfileList(category) => {
                self.handle_profile_list(category.as_deref()).await;
                Ok(true)
            }
            CommandAction::ProfileSearch(query) => {
                self.handle_profile_search(&query).await;
                Ok(true)
            }
            CommandAction::ProfileDelete(id_or_key) => {
                self.handle_profile_delete(&id_or_key).await;
                Ok(true)
            }
            CommandAction::ProfileSync => {
                self.handle_profile_sync().await;
                Ok(true)
            }
            CommandAction::ProfileExport(path) => {
                self.handle_profile_export(path.as_deref()).await;
                Ok(true)
            }
            CommandAction::ProfileImport(path) => {
                self.handle_profile_import(&path).await;
                Ok(true)
            }
            CommandAction::ProfileStats => {
                self.handle_profile_stats().await;
                Ok(true)
            }
            // Multi-Agent System Actions
            CommandAction::ListAgents => {
                self.handle_list_agents().await;
                Ok(true)
            }
            CommandAction::SwitchAgent(session_id) => {
                self.handle_switch_agent(&session_id).await;
                Ok(true)
            }
            CommandAction::SpawnChildAgent(model, reason) => {
                self.handle_spawn_child_agent(model, reason).await;
                Ok(true)
            }
            CommandAction::AgentTree => {
                self.handle_agent_tree().await;
                Ok(true)
            }
            CommandAction::HibernateAgents => {
                self.handle_hibernate_agents().await;
                Ok(true)
            }
            CommandAction::ResumeAgents => {
                self.handle_resume_agents().await;
                Ok(true)
            }
            // Skill commands
            CommandAction::InvokeSkill(name, args) => {
                self.handle_invoke_skill(&name, args).await;
                Ok(true)
            }
            CommandAction::ListSkills => {
                self.handle_list_skills().await;
                Ok(true)
            }
            CommandAction::ShowSkill(name) => {
                self.handle_show_skill(&name).await;
                Ok(true)
            }
            CommandAction::ReloadSkills => {
                self.handle_reload_skills().await;
                Ok(true)
            }
            CommandAction::CreateSkill(name, location) => {
                self.handle_create_skill(&name, location.as_deref()).await;
                Ok(true)
            }
            // Prompt mode commands
            CommandAction::SetPromptModeAsk => {
                self.set_prompt_mode_ask().await?;
                self.clear_input();
                Ok(true)
            }
            CommandAction::SetPromptModeEdit => {
                self.set_prompt_mode_edit().await?;
                self.clear_input();
                Ok(true)
            }
            // Plan mode commands
            CommandAction::EnterPlanMode(focus) => {
                self.enter_plan_mode(focus).await?;
                Ok(true)
            }
            CommandAction::ExitPlanMode => {
                self.exit_plan_mode().await?;
                Ok(true)
            }
            CommandAction::PlanModeStatus => {
                let status = self.plan_mode_status();
                self.add_console_message(status);
                self.clear_input();
                Ok(true)
            }
            CommandAction::ClearPlanMode => {
                self.clear_plan_mode();
                self.clear_input();
                Ok(true)
            }
            CommandAction::ExportPlanMode(path) => {
                // Export plan mode session to file
                if let Some(ref state) = self.plan_mode_state {
                    let output = if let Some(p) = path {
                        p
                    } else {
                        format!("plan-{}.md", state.plan_session_id)
                    };
                    let content = state
                        .messages
                        .iter()
                        .map(|m| format!("## {}\n\n{}\n", m.role, m.content))
                        .collect::<Vec<_>>()
                        .join("\n---\n\n");
                    match std::fs::write(&output, &content) {
                        Ok(_) => {
                            self.add_console_message(format!("Exported plan mode to: {}", output))
                        }
                        Err(e) => self.add_console_message(format!("Failed to export: {}", e)),
                    }
                } else {
                    self.add_console_message("No plan mode session to export".to_string());
                }
                self.clear_input();
                Ok(true)
            }
        }
    }

    /// Handle /mdap (show status)
    fn handle_mdap_status(&mut self) {
        let status_msg = if let Some(ref config) = self.mdap_config {
            format!(
                "MDAP Mode: Enabled\n\n\
                Configuration:\n\
                - Vote margin (k): {}\n\
                - Target success rate: {:.1}%\n\
                - Parallel samples: {}\n\
                - Max samples/subtask: {}\n\n\
                Commands:\n\
                - /mdap off      - Disable MDAP mode\n\
                - /mdap:k <n>    - Set vote margin\n\
                - /mdap:target <rate> - Set target success rate",
                config.k,
                config.target_success_rate * 100.0,
                config.parallel_samples,
                config.max_samples_per_subtask
            )
        } else {
            "MDAP Mode: Disabled\n\n\
            MDAP (Massively Decomposed Agentic Processes) enables high-reliability\n\
            execution through task decomposition and multi-sample voting.\n\n\
            Commands:\n\
            - /mdap on       - Enable MDAP mode\n\
            - /mdap:k <n>    - Set vote margin (1-10, default: 3)\n\
            - /mdap:target <rate> - Set target success rate (default: 0.95)"
                .to_string()
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: status_msg,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /mdap:on
    fn handle_mdap_enable(&mut self) {
        use crate::mdap::MdapConfig;

        if self.mdap_config.is_some() {
            self.add_console_message("ℹ️  MDAP mode is already enabled".to_string());
        } else {
            self.mdap_config = Some(MdapConfig::default());
            self.status = format!("Ready - Model: {} [MDAP] (Ctrl+C to quit)", self.model);
            self.add_console_message("✅ MDAP mode enabled (k=3, target=95%)".to_string());
        }
        self.clear_input();
    }

    /// Handle /mdap:off
    fn handle_mdap_disable(&mut self) {
        if self.mdap_config.is_some() {
            self.mdap_config = None;
            self.status = format!("Ready - Model: {} (Ctrl+C to quit)", self.model);
            self.add_console_message("✅ MDAP mode disabled".to_string());
        } else {
            self.add_console_message("ℹ️  MDAP mode is already disabled".to_string());
        }
        self.clear_input();
    }

    /// Handle /mdap:k
    fn handle_mdap_set_k(&mut self, k: u32) {
        use crate::mdap::MdapConfig;

        if let Some(ref mut config) = self.mdap_config {
            config.k = k;
            self.add_console_message(format!("✅ MDAP vote margin set to k={}", k));
        } else {
            // Enable MDAP with custom k
            let config = MdapConfig {
                k,
                ..Default::default()
            };
            self.mdap_config = Some(config);
            self.status = format!("Ready - Model: {} [MDAP] (Ctrl+C to quit)", self.model);
            self.add_console_message(format!("✅ MDAP mode enabled with k={}", k));
        }
        self.clear_input();
    }

    /// Handle /mdap:target
    fn handle_mdap_set_target(&mut self, target: f64) {
        use crate::mdap::MdapConfig;

        if let Some(ref mut config) = self.mdap_config {
            config.target_success_rate = target;
            self.add_console_message(format!(
                "✅ MDAP target success rate set to {:.1}%",
                target * 100.0
            ));
        } else {
            // Enable MDAP with custom target
            let config = MdapConfig {
                target_success_rate: target,
                ..Default::default()
            };
            self.mdap_config = Some(config);
            self.status = format!("Ready - Model: {} [MDAP] (Ctrl+C to quit)", self.model);
            self.add_console_message(format!(
                "✅ MDAP mode enabled with target={:.1}%",
                target * 100.0
            ));
        }
        self.clear_input();
    }

    /// Handle /context:add command
    fn handle_context_add(&mut self, path: &str, pinned: bool) {
        use crate::types::working_set::estimate_tokens;
        use std::path::PathBuf;

        // Resolve path
        let file_path = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            PathBuf::from(&self.working_directory).join(path)
        };

        // Try to canonicalize
        let file_path = file_path.canonicalize().unwrap_or(file_path);

        if !file_path.exists() {
            self.add_console_message(format!("❌ File not found: {}", file_path.display()));
            return;
        }

        // Read file to estimate tokens
        match std::fs::read_to_string(&file_path) {
            Ok(content) => {
                let tokens = estimate_tokens(&content);
                let file_name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());

                if pinned {
                    self.working_set
                        .add_pinned(file_path.clone(), tokens, Some(&file_name));
                    self.add_console_message(format!(
                        "📌 Added and pinned: {} (~{} tokens)",
                        file_path.display(),
                        tokens
                    ));
                } else {
                    let eviction = self.working_set.add(file_path.clone(), tokens);
                    self.add_console_message(format!(
                        "✅ Added: {} (~{} tokens)",
                        file_path.display(),
                        tokens
                    ));
                    if let Some(reason) = eviction {
                        self.add_console_message(format!("⚠️  {}", reason));
                    }
                }
            }
            Err(e) => {
                self.add_console_message(format!("❌ Failed to read file: {}", e));
            }
        }
    }

    /// Handle /context:remove command
    fn handle_context_remove(&mut self, path: &str) {
        use std::path::PathBuf;

        let file_path = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            PathBuf::from(&self.working_directory).join(path)
        };
        let file_path = file_path.canonicalize().unwrap_or(file_path);

        if self.working_set.remove(&file_path) {
            self.add_console_message(format!("✅ Removed: {}", file_path.display()));
        } else {
            self.add_console_message(format!("⚠️  Not in working set: {}", file_path.display()));
        }
    }

    /// Handle /context:pin command
    fn handle_context_pin(&mut self, path: &str) {
        use std::path::PathBuf;

        let file_path = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            PathBuf::from(&self.working_directory).join(path)
        };
        let file_path = file_path.canonicalize().unwrap_or(file_path);

        if self.working_set.pin(&file_path) {
            self.add_console_message(format!("📌 Pinned: {}", file_path.display()));
        } else {
            self.add_console_message(format!(
                "⚠️  Not in working set: {}. Add it first with /context:add",
                file_path.display()
            ));
        }
    }

    /// Handle /context:unpin command
    fn handle_context_unpin(&mut self, path: &str) {
        use std::path::PathBuf;

        let file_path = if PathBuf::from(path).is_absolute() {
            PathBuf::from(path)
        } else {
            PathBuf::from(&self.working_directory).join(path)
        };
        let file_path = file_path.canonicalize().unwrap_or(file_path);

        if self.working_set.unpin(&file_path) {
            self.add_console_message(format!("✅ Unpinned: {}", file_path.display()));
        } else {
            self.add_console_message(format!("⚠️  Not in working set: {}", file_path.display()));
        }
    }

    /// Handle /context:clear command
    fn handle_context_clear(&mut self, keep_pinned: bool) {
        let count_before = self.working_set.len();
        self.working_set.clear(keep_pinned);
        let count_after = self.working_set.len();
        let removed = count_before - count_after;

        if keep_pinned && count_after > 0 {
            self.add_console_message(format!(
                "✅ Cleared {} file(s), kept {} pinned",
                removed, count_after
            ));
        } else {
            self.add_console_message(format!("✅ Cleared {} file(s) from working set", removed));
        }
    }

    /// Handle /tools (show current mode)
    fn handle_show_tool_mode(&mut self) {
        use crate::tools::ToolRegistry;
        use crate::types::tool::ToolMode;

        let registry = ToolRegistry::with_builtins();
        let builtin_count = registry.get_all().len();
        let mcp_count = self.mcp_tools.len();
        let total = builtin_count + mcp_count;

        let mode_str = match &self.tool_mode {
            ToolMode::Full => format!("full ({} built-in + {} MCP)", builtin_count, mcp_count),
            ToolMode::Explicit(tools) => {
                let builtin = tools.iter().filter(|t| !t.starts_with("mcp_")).count();
                let mcp = tools.iter().filter(|t| t.starts_with("mcp_")).count();
                format!("explicit ({} built-in + {} MCP selected)", builtin, mcp)
            }
            ToolMode::Smart => "smart (auto-select based on query)".to_string(),
            ToolMode::Core => format!("core ({} essential tools)", self.tools.len()),
            ToolMode::None => "none (tools disabled)".to_string(),
        };

        let mcp_servers_str = if self.mcp_connected_servers.is_empty() {
            "none".to_string()
        } else {
            self.mcp_connected_servers.join(", ")
        };

        let msg = format!(
            "Tool Mode: {}\n\n\
            Available modes:\n\
            • /tools full     - All {} tools ({} built-in + {} MCP)\n\
            • /tools explicit - Pick specific tools\n\
            • /tools smart    - Auto-select based on query (default)\n\
            • /tools core     - Core {} tools only\n\
            • /tools none     - Disable all tools\n\n\
            Connected MCP servers: {}",
            mode_str,
            total,
            builtin_count,
            mcp_count,
            registry.get_core().len(),
            mcp_servers_str
        );

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: msg,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /tools <mode> (set tool mode)
    fn handle_set_tool_mode(&mut self, mode: crate::types::tool::ToolMode) {
        use crate::tools::ToolRegistry;
        use crate::types::tool::ToolMode;

        let registry = ToolRegistry::with_builtins();

        self.tools = match &mode {
            ToolMode::Full => {
                // Built-in + MCP tools
                registry.get_all_with_mcp(&self.mcp_tools)
            }
            ToolMode::Explicit(names) => {
                // Include both built-in and MCP tools by name
                let mut tools: Vec<_> = names
                    .iter()
                    .filter_map(|name| registry.get(name).cloned())
                    .collect();
                // Add MCP tools that match
                tools.extend(
                    self.mcp_tools
                        .iter()
                        .filter(|t| names.contains(&t.name))
                        .cloned(),
                );
                tools
            }
            ToolMode::Smart => registry.get_core().into_iter().cloned().collect(),
            ToolMode::Core => registry.get_core().into_iter().cloned().collect(),
            ToolMode::None => vec![],
        };

        let count = self.tools.len();
        let mode_name = mode.display_name();
        self.tool_mode = mode;

        self.status = format!("Tool mode: {} ({} tools)", mode_name, count);
        self.add_console_message(format!(
            "✅ Tool mode set to: {} ({} tools active)",
            mode_name, count
        ));
        self.clear_input();
    }

    /// Handle /tools explicit (open tool picker)
    fn handle_open_tool_picker(&mut self) {
        use crate::tools::{ToolCategory, ToolRegistry};
        use crate::tui::app::state::ToolPickerState;
        use crate::types::tool::ToolMode;
        use std::collections::{HashMap, HashSet};

        let registry = ToolRegistry::with_builtins();

        // Get currently selected tools (if already in explicit mode)
        let selected_names: HashSet<String> = match &self.tool_mode {
            ToolMode::Explicit(names) => names.iter().cloned().collect(),
            _ => HashSet::new(),
        };

        // Build categories with their tools
        let categories = vec![
            ("File Operations", ToolCategory::FileOps),
            ("Search", ToolCategory::Search),
            ("Semantic Search", ToolCategory::SemanticSearch),
            ("Git", ToolCategory::Git),
            ("Web Search", ToolCategory::WebSearch),
            ("Web/HTTP", ToolCategory::Web),
            ("Bash/Shell", ToolCategory::Bash),
            ("Task Manager", ToolCategory::TaskManager),
            ("Agent Pool", ToolCategory::AgentPool),
            ("Planning", ToolCategory::Planning),
            ("Context", ToolCategory::Context),
        ];

        let mut picker_categories = Vec::new();
        for (name, category) in categories {
            let tools: Vec<(String, String, bool)> = registry
                .get_by_category(category)
                .iter()
                .map(|t| {
                    (
                        t.name.clone(),
                        t.description.clone(),
                        selected_names.contains(&t.name),
                    )
                })
                .collect();
            if !tools.is_empty() {
                picker_categories.push((name.to_string(), tools));
            }
        }

        // Add MCP server categories
        // Group MCP tools by server name
        let mut mcp_by_server: HashMap<String, Vec<(String, String, bool)>> = HashMap::new();

        for tool in &self.mcp_tools {
            // Extract server name from mcp_{server}_{tool} format
            if let Some(server_name) = extract_mcp_server_name(&tool.name) {
                let is_selected = selected_names.contains(&tool.name);

                mcp_by_server.entry(server_name.clone()).or_default().push((
                    tool.name.clone(),
                    tool.description.clone(),
                    is_selected,
                ));
            }
        }

        // Add MCP servers as categories with "MCP: " prefix
        let mut server_names: Vec<_> = mcp_by_server.keys().cloned().collect();
        server_names.sort();
        for server_name in server_names {
            if let Some(tools) = mcp_by_server.remove(&server_name)
                && !tools.is_empty()
            {
                picker_categories.push((format!("MCP: {}", server_name), tools));
            }
        }

        self.tool_picker_state = Some(ToolPickerState {
            categories: picker_categories,
            selected_category: 0,
            selected_tool: None,
            scroll: 0,
            filter_query: String::new(),
            collapsed: HashSet::new(),
        });

        self.mode = AppMode::ToolPicker;
        self.status = "Select tools (Space: toggle, Enter: confirm, A: all, N: none, Esc: cancel)"
            .to_string();
        self.clear_input();
    }

    /// Confirm tool selection and apply explicit mode
    pub fn confirm_tool_selection(&mut self) {
        use crate::tools::ToolRegistry;
        use crate::types::tool::ToolMode;

        if let Some(state) = &self.tool_picker_state {
            let selected: Vec<String> = state
                .categories
                .iter()
                .flat_map(|(_, tools)| tools.iter())
                .filter(|(_, _, selected)| *selected)
                .map(|(name, _, _)| name.clone())
                .collect();

            let registry = ToolRegistry::with_builtins();

            // Get built-in tools
            let mut tools: Vec<_> = selected
                .iter()
                .filter_map(|name| registry.get(name).cloned())
                .collect();

            // Add MCP tools that were selected
            tools.extend(
                self.mcp_tools
                    .iter()
                    .filter(|t| selected.contains(&t.name))
                    .cloned(),
            );

            let count = tools.len();
            self.tools = tools;
            self.tool_mode = ToolMode::Explicit(selected);

            self.status = format!("Tool mode: explicit ({} tools selected)", count);
            self.add_console_message(format!("✅ Selected {} tools", count));
        }

        self.tool_picker_state = None;
        self.mode = AppMode::Normal;
    }

    // ==================== Knowledge Commands ====================

    /// Handle /learn command - teach a behavioral truth
    async fn handle_learn_truth(&mut self, rule: &str, rationale: Option<&str>) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::cache::BehavioralKnowledgeCache;
        use brainwires::brain::bks_pks::truth::{BehavioralTruth, TruthCategory, TruthSource};

        // Infer category from the rule text
        let category = if rule.to_lowercase().contains("--") || rule.to_lowercase().contains("flag")
        {
            TruthCategory::CommandUsage
        } else if rule.to_lowercase().contains("instead") || rule.to_lowercase().contains("spawn") {
            TruthCategory::TaskStrategy
        } else if rule.to_lowercase().contains("error") || rule.to_lowercase().contains("fail") {
            TruthCategory::ErrorRecovery
        } else {
            TruthCategory::ToolBehavior
        };

        // Extract context pattern (first few words)
        let context = rule
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");

        // Create the truth
        let truth = BehavioralTruth::new(
            category,
            context,
            rule.to_string(),
            rationale.unwrap_or("Explicitly taught by user").to_string(),
            TruthSource::ExplicitCommand,
            None, // created_by
        );

        // Try to save to cache
        let save_result = match PlatformPaths::knowledge_db() {
            Ok(db_path) => match BehavioralKnowledgeCache::new(&db_path, 100) {
                Ok(mut cache) => cache.add_truth(truth.clone()).map(|_| true),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };

        let msg = match save_result {
            Ok(_) => format!(
                "📚 Learned new behavioral truth:\n\n\
                **Rule:** {}\n\
                **Category:** {:?}\n\
                **Rationale:** {}\n\
                **Confidence:** {:.0}%\n\n\
                This truth will be shared with all Brainwires users once synced.",
                truth.rule,
                truth.category,
                truth.rationale,
                truth.confidence * 100.0
            ),
            Err(e) => format!("❌ Failed to save truth: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: msg,
            created_at: chrono::Utc::now().timestamp(),
        });

        self.clear_input();
    }

    /// Handle /knowledge command - show status
    async fn handle_knowledge_status(&mut self) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::cache::BehavioralKnowledgeCache;

        // Try to load cache to get stats
        let stats_msg = match PlatformPaths::knowledge_db() {
            Ok(db_path) => match BehavioralKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let stats = cache.stats();
                    format!(
                        "📊 Behavioral Knowledge System Status\n\n\
                            Total truths: {}\n\
                            Average confidence: {:.0}%\n\
                            Pending submissions: {}\n\
                            Last sync: {}\n\n\
                            Commands:\n\
                            • /learn <rule>        - Teach a new truth\n\
                            • /knowledge:list      - List all truths\n\
                            • /knowledge:search    - Search truths\n\
                            • /knowledge:sync      - Force sync with server\n\
                            • /knowledge:contradict <id> - Report incorrect truth",
                        stats.total_truths,
                        stats.avg_confidence * 100.0,
                        stats.pending_submissions,
                        if stats.last_sync > 0 {
                            chrono::DateTime::from_timestamp(stats.last_sync, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Never".to_string()
                        }
                    )
                }
                Err(e) => format!("Failed to load knowledge cache: {}", e),
            },
            Err(e) => format!("Failed to get knowledge database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: stats_msg,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /knowledge:list command
    async fn handle_knowledge_list(&mut self, category: Option<&str>) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::cache::BehavioralKnowledgeCache;
        use brainwires::brain::bks_pks::truth::TruthCategory;

        let result = match PlatformPaths::knowledge_db() {
            Ok(db_path) => match BehavioralKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let truths: Vec<_> = if let Some(cat_str) = category {
                        let cat = match cat_str.to_lowercase().as_str() {
                            "command" => TruthCategory::CommandUsage,
                            "strategy" => TruthCategory::TaskStrategy,
                            "tool" => TruthCategory::ToolBehavior,
                            "error" => TruthCategory::ErrorRecovery,
                            "resource" => TruthCategory::ResourceManagement,
                            "pattern" => TruthCategory::PatternAvoidance,
                            _ => {
                                self.add_console_message(format!(
                                    "❌ Invalid category: {}",
                                    cat_str
                                ));
                                return;
                            }
                        };
                        cache.truths_by_category(cat).into_iter().cloned().collect()
                    } else {
                        cache.all_truths().cloned().collect()
                    };

                    if truths.is_empty() {
                        "No learned truths found.".to_string()
                    } else {
                        let mut output = format!("📚 Learned Truths ({} total)\n\n", truths.len());
                        for (i, truth) in truths.iter().take(20).enumerate() {
                            output.push_str(&format!(
                                "{}. **{}** ({:?})\n   {}\n   Confidence: {:.0}% | Uses: {}\n\n",
                                i + 1,
                                &truth.id[..8.min(truth.id.len())],
                                truth.category,
                                truth.rule,
                                truth.confidence * 100.0,
                                truth.reinforcements
                            ));
                        }
                        if truths.len() > 20 {
                            output.push_str(&format!("...and {} more", truths.len() - 20));
                        }
                        output
                    }
                }
                Err(e) => format!("Failed to load knowledge cache: {}", e),
            },
            Err(e) => format!("Failed to get knowledge database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /knowledge:search command
    async fn handle_knowledge_search(&mut self, query: &str) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::cache::BehavioralKnowledgeCache;
        use brainwires::brain::bks_pks::matcher::ContextMatcher;

        let result = match PlatformPaths::knowledge_db() {
            Ok(db_path) => match BehavioralKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let matcher = ContextMatcher::new(0.0, 30, 10);
                    let truths: Vec<_> = cache.all_truths().cloned().collect();
                    let matches = matcher.search(query, truths.iter());

                    if matches.is_empty() {
                        format!("No truths found matching \"{}\"", query)
                    } else {
                        let mut output = format!("🔍 Search Results for \"{}\"\n\n", query);
                        for (i, m) in matches.iter().enumerate() {
                            output.push_str(&format!(
                                    "{}. **{}** ({:?})\n   {}\n   Confidence: {:.0}% | Score: {:.0}%\n\n",
                                    i + 1,
                                    &m.truth.id[..8.min(m.truth.id.len())],
                                    m.truth.category,
                                    m.truth.rule,
                                    m.effective_confidence * 100.0,
                                    m.match_score * 100.0
                                ));
                        }
                        output
                    }
                }
                Err(e) => format!("Failed to load knowledge cache: {}", e),
            },
            Err(e) => format!("Failed to get knowledge database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /knowledge:sync command
    async fn handle_knowledge_sync(&mut self) {
        // For now, just show a message - actual sync would require
        // the HTTP client and backend URL from config
        self.add_console_message("🔄 Syncing with Brainwires server...".to_string());

        // TODO: Implement actual sync when backend endpoints are ready
        self.add_console_message(
            "ℹ️  Server sync not yet implemented - truths stored locally".to_string(),
        );

        self.clear_input();
    }

    /// Handle /knowledge:contradict command
    async fn handle_knowledge_contradict(&mut self, id: &str, reason: Option<&str>) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::cache::BehavioralKnowledgeCache;

        let result = match PlatformPaths::knowledge_db() {
            Ok(db_path) => {
                match BehavioralKnowledgeCache::new(&db_path, 100) {
                    Ok(mut cache) => {
                        // Get and update the truth
                        if let Some(truth) = cache.get_truth_mut(id) {
                            truth.contradict(0.1); // Default EMA alpha
                            let reason_str = reason
                                .map(|r| format!("\nReason: {}", r))
                                .unwrap_or_default();
                            format!(
                                "✅ Contradicted truth: {}{}\nNew confidence: {:.0}%",
                                id,
                                reason_str,
                                truth.confidence * 100.0
                            )
                        } else {
                            format!("❌ Truth not found: {}", id)
                        }
                    }
                    Err(e) => format!("Failed to load knowledge cache: {}", e),
                }
            }
            Err(e) => format!("Failed to get knowledge database path: {}", e),
        };

        self.add_console_message(result);
        self.clear_input();
    }

    /// Handle /knowledge:delete command
    async fn handle_knowledge_delete(&mut self, id: &str) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::cache::BehavioralKnowledgeCache;

        let result = match PlatformPaths::knowledge_db() {
            Ok(db_path) => match BehavioralKnowledgeCache::new(&db_path, 100) {
                Ok(mut cache) => match cache.remove_truth(id) {
                    Ok(true) => format!("✅ Deleted truth: {}", id),
                    Ok(false) => format!("❌ Truth not found: {}", id),
                    Err(e) => format!("❌ Failed to delete truth: {}", e),
                },
                Err(e) => format!("Failed to load knowledge cache: {}", e),
            },
            Err(e) => format!("Failed to get knowledge database path: {}", e),
        };

        self.add_console_message(result);
        self.clear_input();
    }

    // ==================== Personal Knowledge Commands ====================

    /// Handle /profile command - show profile summary
    async fn handle_profile_show(&mut self) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::{PersonalFactMatcher, PersonalKnowledgeCache};

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => match PersonalKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let facts: Vec<_> = cache.all_facts().cloned().collect();
                    if facts.is_empty() {
                        "👤 Your Profile\n\n\
                            No personal facts learned yet.\n\n\
                            Use these commands to build your profile:\n\
                            • /profile:set <key> <value> - Set a fact\n\
                            • /profile:name <your_name>  - Set your name\n\n\
                            The system also learns from conversation patterns like:\n\
                            • \"My name is...\"\n\
                            • \"I prefer...\"\n\
                            • \"I'm working on...\""
                            .to_string()
                    } else {
                        let matcher = PersonalFactMatcher::new(0.0, 30, true);
                        let fact_refs: Vec<_> = facts.iter().collect();
                        matcher.format_profile_summary(&fact_refs)
                    }
                }
                Err(e) => format!("Failed to load profile: {}", e),
            },
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /profile:set command
    async fn handle_profile_set(&mut self, key: &str, value: &str, local_only: bool) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::{
            PersonalFact, PersonalFactCategory, PersonalFactSource, PersonalKnowledgeCache,
        };

        // Infer category from key name
        let category = match key.to_lowercase().as_str() {
            "name" | "role" | "team" | "organization" | "company" => PersonalFactCategory::Identity,
            "timezone" | "limitation" | "restriction" => PersonalFactCategory::Constraint,
            "skill" | "expert" | "proficient" | "knows" => PersonalFactCategory::Capability,
            "project" | "working_on" | "current_task" => PersonalFactCategory::Context,
            _ => PersonalFactCategory::Preference,
        };

        let fact = PersonalFact::new(
            category,
            key.to_string(),
            value.to_string(),
            None,
            PersonalFactSource::ExplicitStatement,
            local_only,
        );

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => match PersonalKnowledgeCache::new(&db_path, 100) {
                Ok(mut cache) => match cache.upsert_fact(fact.clone()) {
                    Ok(_) => {
                        let local_str = if local_only { " (local only)" } else { "" };
                        format!(
                            "✅ Set profile fact{}\n\n\
                                    **{}** = {}\n\
                                    Category: {:?}",
                            local_str, key, value, category
                        )
                    }
                    Err(e) => format!("❌ Failed to save fact: {}", e),
                },
                Err(e) => format!("Failed to load profile: {}", e),
            },
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.add_console_message(result);
        self.clear_input();
    }

    /// Handle /profile:list command
    async fn handle_profile_list(&mut self, category: Option<&str>) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::{PersonalFactCategory, PersonalKnowledgeCache};

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => match PersonalKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let facts: Vec<_> = if let Some(cat_str) = category {
                        let cat = match cat_str.to_lowercase().as_str() {
                            "identity" => PersonalFactCategory::Identity,
                            "preference" => PersonalFactCategory::Preference,
                            "capability" => PersonalFactCategory::Capability,
                            "context" => PersonalFactCategory::Context,
                            "constraint" => PersonalFactCategory::Constraint,
                            "relationship" => PersonalFactCategory::Relationship,
                            _ => {
                                self.add_console_message(format!(
                                    "❌ Invalid category: {}",
                                    cat_str
                                ));
                                return;
                            }
                        };
                        cache.facts_by_category(cat).into_iter().cloned().collect()
                    } else {
                        cache.all_facts().cloned().collect()
                    };

                    if facts.is_empty() {
                        "No personal facts found.".to_string()
                    } else {
                        let mut output = format!("👤 Personal Facts ({} total)\n\n", facts.len());
                        for (i, fact) in facts.iter().take(30).enumerate() {
                            let local_marker = if fact.local_only { " 🔒" } else { "" };
                            output.push_str(&format!(
                                "{}. **{}**{} ({:?})\n   {}\n   Confidence: {:.0}%\n\n",
                                i + 1,
                                fact.key,
                                local_marker,
                                fact.category,
                                fact.value,
                                fact.confidence * 100.0
                            ));
                        }
                        if facts.len() > 30 {
                            output.push_str(&format!("...and {} more", facts.len() - 30));
                        }
                        output
                    }
                }
                Err(e) => format!("Failed to load profile: {}", e),
            },
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /profile:search command
    async fn handle_profile_search(&mut self, query: &str) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::PersonalKnowledgeCache;

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => match PersonalKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let matches = cache.search_facts(query);

                    if matches.is_empty() {
                        format!("No facts found matching \"{}\"", query)
                    } else {
                        let mut output = format!("🔍 Search Results for \"{}\"\n\n", query);
                        for (i, fact) in matches.iter().enumerate() {
                            let local_marker = if fact.local_only { " 🔒" } else { "" };
                            output.push_str(&format!(
                                "{}. **{}**{} ({:?})\n   {}\n   Confidence: {:.0}%\n\n",
                                i + 1,
                                fact.key,
                                local_marker,
                                fact.category,
                                fact.value,
                                fact.confidence * 100.0
                            ));
                        }
                        output
                    }
                }
                Err(e) => format!("Failed to load profile: {}", e),
            },
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /profile:delete command
    async fn handle_profile_delete(&mut self, id_or_key: &str) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::PersonalKnowledgeCache;

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => {
                match PersonalKnowledgeCache::new(&db_path, 100) {
                    Ok(mut cache) => {
                        // Try to delete by ID first, then by key
                        match cache.remove_fact(id_or_key) {
                            Ok(true) => format!("✅ Deleted fact: {}", id_or_key),
                            Ok(false) => {
                                // Try by key
                                match cache.remove_fact_by_key(id_or_key) {
                                    Ok(true) => format!("✅ Deleted fact with key: {}", id_or_key),
                                    _ => format!("❌ Fact not found: {}", id_or_key),
                                }
                            }
                            Err(e) => format!("❌ Failed to delete fact: {}", e),
                        }
                    }
                    Err(e) => format!("Failed to load profile: {}", e),
                }
            }
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.add_console_message(result);
        self.clear_input();
    }

    /// Handle /profile:sync command
    async fn handle_profile_sync(&mut self) {
        // For now, just show a message - actual sync would require
        // the HTTP client and backend URL from config
        self.add_console_message("🔄 Syncing personal profile with server...".to_string());

        // TODO: Implement actual sync with PersonalKnowledgeApiClient
        self.add_console_message(
            "ℹ️  Server sync not yet implemented - facts stored locally".to_string(),
        );

        self.clear_input();
    }

    /// Handle /profile:export command
    async fn handle_profile_export(&mut self, path: Option<&str>) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::PersonalKnowledgeCache;

        let export_path = path.map(std::path::PathBuf::from).unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("brainwires-profile.json")
        });

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => {
                match PersonalKnowledgeCache::new(&db_path, 100) {
                    Ok(cache) => {
                        match cache.export_json() {
                            Ok(json) => {
                                // Write to file
                                match std::fs::write(&export_path, &json) {
                                    Ok(_) => {
                                        let count = json.matches("\"id\"").count();
                                        format!(
                                            "✅ Exported {} facts to:\n{}",
                                            count,
                                            export_path.display()
                                        )
                                    }
                                    Err(e) => format!("❌ Failed to write file: {}", e),
                                }
                            }
                            Err(e) => format!("❌ Export failed: {}", e),
                        }
                    }
                    Err(e) => format!("Failed to load profile: {}", e),
                }
            }
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.add_console_message(result);
        self.clear_input();
    }

    /// Handle /profile:import command
    async fn handle_profile_import(&mut self, path: &str) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::PersonalKnowledgeCache;

        let import_path = std::path::PathBuf::from(path);

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => {
                match PersonalKnowledgeCache::new(&db_path, 100) {
                    Ok(mut cache) => {
                        // Read file first
                        match std::fs::read_to_string(&import_path) {
                            Ok(json) => match cache.import_json(&json) {
                                Ok(result) => format!(
                                    "✅ Imported {} new facts, updated {} existing facts from:\n{}",
                                    result.imported,
                                    result.updated,
                                    import_path.display()
                                ),
                                Err(e) => format!("❌ Import failed: {}", e),
                            },
                            Err(e) => format!("❌ Failed to read file: {}", e),
                        }
                    }
                    Err(e) => format!("Failed to load profile: {}", e),
                }
            }
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.add_console_message(result);
        self.clear_input();
    }

    /// Handle /profile:stats command
    async fn handle_profile_stats(&mut self) {
        use crate::utils::paths::PlatformPaths;
        use brainwires::brain::bks_pks::personal::{PersonalFactCategory, PersonalKnowledgeCache};

        let result = match PlatformPaths::personal_knowledge_db() {
            Ok(db_path) => match PersonalKnowledgeCache::new(&db_path, 100) {
                Ok(cache) => {
                    let stats = cache.stats();
                    format!(
                        "📊 Personal Knowledge Statistics\n\n\
                            Total facts: {}\n\
                            Local-only facts: {}\n\
                            Average confidence: {:.0}%\n\
                            Pending submissions: {}\n\
                            Pending feedback: {}\n\
                            Last sync: {}\n\n\
                            By Category:\n\
                            • Identity: {}\n\
                            • Preference: {}\n\
                            • Capability: {}\n\
                            • Context: {}\n\
                            • Constraint: {}\n\
                            • Relationship: {}",
                        stats.total_facts,
                        stats.local_only_facts,
                        stats.avg_confidence * 100.0,
                        stats.pending_submissions,
                        stats.pending_feedback,
                        if stats.last_sync > 0 {
                            chrono::DateTime::from_timestamp(stats.last_sync, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Never".to_string()
                        },
                        stats
                            .by_category
                            .get(&PersonalFactCategory::Identity)
                            .unwrap_or(&0),
                        stats
                            .by_category
                            .get(&PersonalFactCategory::Preference)
                            .unwrap_or(&0),
                        stats
                            .by_category
                            .get(&PersonalFactCategory::Capability)
                            .unwrap_or(&0),
                        stats
                            .by_category
                            .get(&PersonalFactCategory::Context)
                            .unwrap_or(&0),
                        stats
                            .by_category
                            .get(&PersonalFactCategory::Constraint)
                            .unwrap_or(&0),
                        stats
                            .by_category
                            .get(&PersonalFactCategory::Relationship)
                            .unwrap_or(&0),
                    )
                }
                Err(e) => format!("Failed to load profile: {}", e),
            },
            Err(e) => format!("Failed to get profile database path: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    // ==================== Multi-Agent Commands ====================

    /// Handle /agents command - list all active agents
    async fn handle_list_agents(&mut self) {
        use crate::ipc::list_agent_sessions_with_metadata;

        let result = match list_agent_sessions_with_metadata() {
            Ok(agents) => {
                if agents.is_empty() {
                    "No active agents found.\n\n\
                    Commands:\n\
                    - /spawn        - Spawn a new child agent\n\
                    - /switch <id>  - Switch to a specific agent\n\
                    - /agent:tree   - Show agent hierarchy"
                        .to_string()
                } else {
                    let mut output = format!("Active Agents ({} total)\n\n", agents.len());
                    for agent in &agents {
                        let marker = if agent.session_id == self.session_id {
                            " <-- current"
                        } else {
                            ""
                        };
                        let busy_str = if agent.is_busy { " (busy)" } else { "" };
                        let parent_str = agent
                            .parent_agent_id
                            .as_ref()
                            .map(|p| format!(" [parent: {}]", &p[..8.min(p.len())]))
                            .unwrap_or_default();

                        output.push_str(&format!(
                            "- {} [{}]{}{}{}\n",
                            agent.session_id, agent.model, busy_str, parent_str, marker
                        ));
                    }
                    output.push_str("\nUse /switch <session_id> to switch agents");
                    output
                }
            }
            Err(e) => format!("Failed to list agents: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /switch command - switch to a different agent
    async fn handle_switch_agent(&mut self, session_id: &str) {
        use crate::ipc::is_agent_alive;

        // Check if target agent exists
        if !is_agent_alive(session_id).await {
            self.add_console_message(format!("Agent '{}' is not running", session_id));
            return;
        }

        // Store switch request - actual switch happens in the main TUI loop
        self.pending_agent_switch = Some(session_id.to_string());
        self.add_console_message(format!("Switching to agent: {}", session_id));
        self.clear_input();
    }

    /// Handle /spawn command - spawn a new child agent
    async fn handle_spawn_child_agent(&mut self, model: Option<String>, reason: Option<String>) {
        // The actual spawning will be handled via IPC message to current agent
        // This command just shows feedback
        let reason_str = reason.as_deref().unwrap_or("child agent");
        let model_str = model.as_deref().unwrap_or("default");

        self.add_console_message(format!(
            "Spawning new agent...\nModel: {}\nReason: {}\n\n\
            The new agent will appear in /agents list once ready.",
            model_str, reason_str
        ));

        // Store the spawn request to be sent via IPC
        self.pending_agent_spawn = Some((model, reason));
        self.clear_input();
    }

    /// Handle /agent:tree command - show agent hierarchy
    async fn handle_agent_tree(&mut self) {
        use crate::ipc::format_agent_tree;

        let result = match format_agent_tree(Some(&self.session_id)) {
            Ok(tree) => tree,
            Err(e) => format!("Failed to build agent tree: {}", e),
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /hibernate command - hibernate all agents
    async fn handle_hibernate_agents(&mut self) {
        use crate::agent::hibernate_agents;

        self.add_console_message("Hibernating all agents...".to_string());

        match hibernate_agents().await {
            Ok(hibernated) => {
                if hibernated.is_empty() {
                    self.add_console_message("No agents were hibernated.".to_string());
                } else {
                    self.add_console_message(format!(
                        "Successfully hibernated {} agent(s):\n{}",
                        hibernated.len(),
                        hibernated
                            .iter()
                            .map(|s| format!("  - {}", s))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                    self.add_console_message(
                        "Use /resume:agents to restore them later.".to_string(),
                    );
                }
            }
            Err(e) => {
                self.add_console_message(format!("Failed to hibernate agents: {}", e));
            }
        }

        self.clear_input();
    }

    /// Handle /resume:agents command - resume hibernated agents
    async fn handle_resume_agents(&mut self) {
        use crate::agent::{has_hibernated_agents, resume_agents};

        // Check if there are any hibernated agents
        match has_hibernated_agents() {
            Ok(true) => {
                self.add_console_message("Resuming hibernated agents...".to_string());

                match resume_agents().await {
                    Ok(resumed) => {
                        if resumed.is_empty() {
                            self.add_console_message(
                                "No agents were resumed (all may be already running).".to_string(),
                            );
                        } else {
                            self.add_console_message(format!(
                                "Successfully resumed {} agent(s):\n{}",
                                resumed.len(),
                                resumed
                                    .iter()
                                    .map(|s| format!("  - {}", s))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            ));
                            self.add_console_message(
                                "Use /agents to see all running agents.".to_string(),
                            );
                        }
                    }
                    Err(e) => {
                        self.add_console_message(format!("Failed to resume agents: {}", e));
                    }
                }
            }
            Ok(false) => {
                self.add_console_message(
                    "No hibernated agents found.\n\n\
                    Use /hibernate to save current agents for later."
                        .to_string(),
                );
            }
            Err(e) => {
                self.add_console_message(format!("Failed to check for hibernated agents: {}", e));
            }
        }

        self.clear_input();
    }
}

/// Extract server name from MCP tool name (mcp_{server}_{tool})
fn extract_mcp_server_name(tool_name: &str) -> Option<String> {
    if tool_name.starts_with("mcp_") {
        let rest = tool_name.strip_prefix("mcp_")?;
        // Find the first underscore to get the server name
        if let Some(idx) = rest.find('_') {
            return Some(rest[..idx].to_string());
        }
    }
    None
}

// ==================== Skill Command Handlers ====================

impl App {
    /// Handle /skill <name> [args...] - invoke a skill
    async fn handle_invoke_skill(&mut self, name: &str, args: Vec<String>) {
        use brainwires_skills::SkillSource;

        // Try to get the skill from registry
        if let Some(ref mut registry) = self.skill_registry {
            match registry.get_skill(name) {
                Ok(skill) => {
                    // Build the skill invocation message
                    let source_str = match skill.metadata.source {
                        SkillSource::Personal => "personal",
                        SkillSource::Project => "project",
                        SkillSource::Builtin => "builtin",
                    };

                    let args_str = if args.is_empty() {
                        String::new()
                    } else {
                        format!("\n\n**Arguments:** {}", args.join(" "))
                    };

                    // Add the skill instructions to the conversation
                    let skill_msg = format!(
                        "**Invoking skill: {}** ({}){}\n\n---\n\n{}",
                        name, source_str, args_str, skill.instructions
                    );

                    // Add as user message so the AI can see and act on it
                    self.messages.push(TuiMessage {
                        role: "user".to_string(),
                        content: skill_msg.clone(),
                        created_at: chrono::Utc::now().timestamp(),
                    });

                    use crate::types::message::{Message, MessageContent, Role};
                    self.conversation_history.push(Message {
                        role: Role::User,
                        content: MessageContent::Text(skill_msg),
                        name: None,
                        metadata: None,
                    });

                    self.status = format!("Skill '{}' invoked", name);
                }
                Err(e) => {
                    self.add_console_message(format!("Failed to load skill '{}': {}", name, e));
                }
            }
        } else {
            self.add_console_message("Skill registry not initialized".to_string());
        }

        self.clear_input();
    }

    /// Handle /skills - list all available skills
    async fn handle_list_skills(&mut self) {
        use brainwires_skills::SkillSource;

        let result = if let Some(ref registry) = self.skill_registry {
            let skills = registry.list_skills();

            if skills.is_empty() {
                "No skills found.\n\n\
                Skills can be placed in:\n\
                - Personal: ~/.brainwires/skills/\n\
                - Project: .brainwires/skills/\n\n\
                Each skill is a SKILL.md file with YAML frontmatter."
                    .to_string()
            } else {
                let mut output = format!("Available Skills ({} total)\n\n", skills.len());

                // Group by source
                let mut personal: Vec<_> = Vec::new();
                let mut project: Vec<_> = Vec::new();
                let mut builtin: Vec<_> = Vec::new();

                for name in skills {
                    if let Some(meta) = registry.get_metadata(name) {
                        match meta.source {
                            SkillSource::Personal => personal.push(meta),
                            SkillSource::Project => project.push(meta),
                            SkillSource::Builtin => builtin.push(meta),
                        }
                    }
                }

                if !project.is_empty() {
                    output.push_str("**Project Skills:**\n");
                    for meta in &project {
                        output.push_str(&format!(
                            "  - /{} - {}\n",
                            meta.name,
                            truncate_description(&meta.description, 60)
                        ));
                    }
                    output.push('\n');
                }

                if !personal.is_empty() {
                    output.push_str("**Personal Skills:**\n");
                    for meta in &personal {
                        output.push_str(&format!(
                            "  - /{} - {}\n",
                            meta.name,
                            truncate_description(&meta.description, 60)
                        ));
                    }
                    output.push('\n');
                }

                if !builtin.is_empty() {
                    output.push_str("**Builtin Skills:**\n");
                    for meta in &builtin {
                        output.push_str(&format!(
                            "  - /{} - {}\n",
                            meta.name,
                            truncate_description(&meta.description, 60)
                        ));
                    }
                    output.push('\n');
                }

                output
                    .push_str("\nUse /skill:show <name> for details, or /<skill-name> to invoke.");
                output
            }
        } else {
            "Skill registry not initialized".to_string()
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /skill:show <name> - show skill details
    async fn handle_show_skill(&mut self, name: &str) {
        use brainwires_skills::SkillSource;

        let result = if let Some(ref mut registry) = self.skill_registry {
            match registry.get_skill(name) {
                Ok(skill) => {
                    let source_str = match skill.metadata.source {
                        SkillSource::Personal => "Personal (~/.brainwires/skills/)",
                        SkillSource::Project => "Project (.brainwires/skills/)",
                        SkillSource::Builtin => "Builtin",
                    };

                    let allowed_tools = skill
                        .metadata
                        .allowed_tools
                        .as_ref()
                        .map(|tools| tools.join(", "))
                        .unwrap_or_else(|| "all".to_string());

                    let model = skill.metadata.model.as_deref().unwrap_or("default");
                    let license = skill.metadata.license.as_deref().unwrap_or("unspecified");

                    // Truncate instructions for display
                    let instructions_preview = if skill.instructions.len() > 500 {
                        format!(
                            "{}...\n\n(truncated, {} chars total)",
                            &skill.instructions[..500],
                            skill.instructions.len()
                        )
                    } else {
                        skill.instructions.clone()
                    };

                    format!(
                        "**Skill: {}**\n\n\
                        **Description:**\n{}\n\n\
                        **Source:** {}\n\
                        **Model:** {}\n\
                        **Allowed Tools:** {}\n\
                        **License:** {}\n\n\
                        **Instructions:**\n{}",
                        name,
                        skill.metadata.description,
                        source_str,
                        model,
                        allowed_tools,
                        license,
                        instructions_preview
                    )
                }
                Err(e) => format!("Failed to load skill '{}': {}", name, e),
            }
        } else {
            "Skill registry not initialized".to_string()
        };

        self.messages.push(TuiMessage {
            role: "system".to_string(),
            content: result,
            created_at: chrono::Utc::now().timestamp(),
        });
        self.clear_input();
    }

    /// Handle /skill:reload - reload skills from disk
    async fn handle_reload_skills(&mut self) {
        if let Some(ref mut registry) = self.skill_registry {
            match registry.reload() {
                Ok(()) => {
                    let count = registry.list_skills().len();
                    self.add_console_message(format!("Reloaded {} skill(s)", count));
                }
                Err(e) => {
                    self.add_console_message(format!("Failed to reload skills: {}", e));
                }
            }
        } else {
            self.add_console_message("Skill registry not initialized".to_string());
        }

        self.clear_input();
    }

    /// Handle /skill:create <name> [location] - create a new skill
    async fn handle_create_skill(&mut self, name: &str, location: Option<&str>) {
        use crate::utils::paths::PlatformPaths;

        // Validate name
        if name.len() > 64 || !name.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
            self.add_console_message(
                "Invalid skill name. Use lowercase letters and hyphens only, max 64 chars."
                    .to_string(),
            );
            return;
        }

        // Determine target directory
        let skills_dir = match location {
            Some("project") | None => {
                // Default to project
                std::env::current_dir()
                    .map(|cwd| cwd.join(".brainwires/skills"))
                    .unwrap_or_else(|_| std::path::PathBuf::from(".brainwires/skills"))
            }
            Some("personal") => match PlatformPaths::personal_skills_dir() {
                Ok(dir) => dir,
                Err(e) => {
                    self.add_console_message(format!("Failed to get personal skills dir: {}", e));
                    return;
                }
            },
            Some(other) => {
                self.add_console_message(format!(
                    "Invalid location: {}. Use 'personal' or 'project'.",
                    other
                ));
                return;
            }
        };

        // Ensure directory exists
        if let Err(e) = std::fs::create_dir_all(&skills_dir) {
            self.add_console_message(format!("Failed to create skills directory: {}", e));
            return;
        }

        // Create the skill file
        let skill_path = skills_dir.join(format!("{}.md", name));

        if skill_path.exists() {
            self.add_console_message(format!(
                "Skill '{}' already exists at: {}",
                name,
                skill_path.display()
            ));
            return;
        }

        let template = format!(
            r#"---
name: {name}
description: |
  Brief description of what this skill does.
  This is used for semantic matching when suggesting skills.
allowed-tools:
  - Read
  - Grep
  - Glob
# model: claude-sonnet  # Optional: override model
# license: MIT
metadata:
  category: utility
  execution: inline  # inline | subagent | script
---

# {name} Skill Instructions

Write your skill instructions here. These will be injected into the
conversation when the skill is invoked.

## Example Usage

Describe how to use this skill and what it does.
"#,
            name = name
        );

        match std::fs::write(&skill_path, template) {
            Ok(()) => {
                self.add_console_message(format!(
                    "Created new skill at:\n{}\n\nEdit the file to customize your skill, then use /skill:reload to load it.",
                    skill_path.display()
                ));

                // Reload to pick up the new skill
                if let Some(ref mut registry) = self.skill_registry {
                    let _ = registry.reload();
                }
            }
            Err(e) => {
                self.add_console_message(format!("Failed to create skill file: {}", e));
            }
        }

        self.clear_input();
    }
}

/// Truncate a description to max length, adding ellipsis if needed
fn truncate_description(s: &str, max_len: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len.saturating_sub(3)])
    } else {
        first_line.to_string()
    }
}
