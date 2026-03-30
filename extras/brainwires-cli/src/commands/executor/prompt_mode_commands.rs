//! Prompt Mode Commands
//!
//! Handles /ask and /edit commands for switching prompt modes.

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute prompt mode commands (/ask, /edit)
    pub(super) fn execute_prompt_mode_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "ask" => Some(self.cmd_prompt_mode_ask(args)),
            "edit" => Some(self.cmd_prompt_mode_edit(args)),
            _ => None,
        }
    }

    fn cmd_prompt_mode_ask(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            // Just switch mode, no message
            Ok(CommandResult::Action(CommandAction::SetPromptModeAsk))
        } else {
            // Switch mode AND send the query
            let query = args.join(" ");
            Ok(CommandResult::ActionWithMessage(
                CommandAction::SetPromptModeAsk,
                query,
            ))
        }
    }

    fn cmd_prompt_mode_edit(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            // Just switch mode, no message
            Ok(CommandResult::Action(CommandAction::SetPromptModeEdit))
        } else {
            // Switch mode AND send the instruction
            let query = args.join(" ");
            Ok(CommandResult::ActionWithMessage(
                CommandAction::SetPromptModeEdit,
                query,
            ))
        }
    }
}
