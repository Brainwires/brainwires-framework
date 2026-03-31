//! Conversation Commands
//!
//! Commands for conversation management: help, clear, status, model, rewind, resume, exit

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute conversation-related built-in commands
    pub(super) fn execute_conversation_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "help" | "commands" => Some(self.cmd_help()),
            "clear" => Some(Ok(CommandResult::Action(CommandAction::ClearHistory))),
            "status" => Some(Ok(CommandResult::Action(CommandAction::ShowStatus))),
            "model" => Some(self.cmd_model(args)),
            "rewind" => Some(self.cmd_rewind(args)),
            "resume" => Some(self.cmd_resume(args)),
            "exit" => Some(Ok(CommandResult::Action(CommandAction::Exit))),
            _ => None,
        }
    }

    fn cmd_help(&self) -> Result<CommandResult> {
        let mut lines = vec![
            "Available Commands:".to_string(),
            "".to_string(),
            "Built-in Commands:".to_string(),
        ];

        for (cmd_name, cmd) in self.registry.commands() {
            if cmd.builtin {
                lines.push(format!("  /{:<15} - {}", cmd_name, cmd.description));
            }
        }

        // Add custom commands section if any exist
        let custom_commands: Vec<_> = self
            .registry
            .commands()
            .iter()
            .filter(|(_, cmd)| !cmd.builtin)
            .collect();

        if !custom_commands.is_empty() {
            lines.push("".to_string());
            lines.push("Custom Commands:".to_string());
            for (cmd_name, cmd) in custom_commands {
                lines.push(format!("  /{:<15} - {}", cmd_name, cmd.description));
            }
        }

        Ok(CommandResult::Help(lines))
    }

    fn cmd_model(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /model <model_name>");
        }
        let model_name = args[0].clone();
        Ok(CommandResult::Action(CommandAction::SwitchModel(
            model_name,
        )))
    }

    fn cmd_rewind(&self, args: &[String]) -> Result<CommandResult> {
        use anyhow::Context;

        let steps = if args.is_empty() {
            1
        } else {
            args[0]
                .parse::<usize>()
                .context("Invalid rewind steps, must be a number")?
        };
        Ok(CommandResult::Action(CommandAction::Rewind(steps)))
    }

    fn cmd_resume(&self, args: &[String]) -> Result<CommandResult> {
        let conversation_id = args.first().cloned();
        Ok(CommandResult::Action(CommandAction::ResumeHistory(
            conversation_id,
        )))
    }
}
