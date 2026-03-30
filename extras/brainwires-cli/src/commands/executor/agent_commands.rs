//! Agent Commands
//!
//! Multi-agent system commands: agents, switch, spawn, tree, hibernate, resume

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute agent-related built-in commands
    pub(super) fn execute_agent_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "agents" => Some(Ok(CommandResult::Action(CommandAction::ListAgents))),
            "agent:tree" | "tree" => Some(Ok(CommandResult::Action(CommandAction::AgentTree))),
            "switch" => Some(self.cmd_switch(args)),
            "spawn" => Some(self.cmd_spawn(args)),
            "hibernate" => Some(Ok(CommandResult::Action(CommandAction::HibernateAgents))),
            "resume:agents" => Some(Ok(CommandResult::Action(CommandAction::ResumeAgents))),
            _ => None,
        }
    }

    fn cmd_switch(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            // Show agent picker if no argument provided
            return Ok(CommandResult::Action(CommandAction::ListAgents));
        }

        let session_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::SwitchAgent(session_id)))
    }

    fn cmd_spawn(&self, args: &[String]) -> Result<CommandResult> {
        // /spawn [--model MODEL] [--reason REASON]
        let mut model = None;
        let mut reason = None;
        let mut i = 0;

        while i < args.len() {
            match args[i].as_str() {
                "--model" | "-m" => {
                    if i + 1 < args.len() {
                        model = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        anyhow::bail!("--model requires a value");
                    }
                }
                "--reason" | "-r" => {
                    if i + 1 < args.len() {
                        reason = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        anyhow::bail!("--reason requires a value");
                    }
                }
                _ => {
                    // Treat remaining args as reason if not specified
                    if reason.is_none() {
                        reason = Some(args[i..].join(" "));
                    }
                    break;
                }
            }
        }

        Ok(CommandResult::Action(CommandAction::SpawnChildAgent(model, reason)))
    }
}
