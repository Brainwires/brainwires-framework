//! Multi-agent slash commands — /agents, /agent:switch, /agent:spawn, hibernate/resume

use super::super::super::state::{App, TuiMessage};

impl App {
    /// Handle /agents command - list all active agents
    pub(super) async fn handle_list_agents(&mut self) {
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
    pub(super) async fn handle_switch_agent(&mut self, session_id: &str) {
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
    pub(super) async fn handle_spawn_child_agent(
        &mut self,
        model: Option<String>,
        reason: Option<String>,
    ) {
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
    pub(super) async fn handle_agent_tree(&mut self) {
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
    pub(super) async fn handle_hibernate_agents(&mut self) {
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
    pub(super) async fn handle_resume_agents(&mut self) {
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
