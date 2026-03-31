//! Tools Commands
//!
//! Commands for tool selection: /tools

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};
use crate::types::tool::ToolMode;

impl CommandExecutor {
    /// Execute tools-related built-in commands
    pub(super) fn execute_tools_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "tools" => Some(self.cmd_tools(args)),
            _ => None,
        }
    }

    fn cmd_tools(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            // No args: show current mode and usage
            return Ok(CommandResult::Action(CommandAction::ShowToolMode));
        }

        let mode = args[0].to_lowercase();
        match mode.as_str() {
            "full" => Ok(CommandResult::Action(CommandAction::SetToolMode(
                ToolMode::Full,
            ))),
            "explicit" => Ok(CommandResult::Action(CommandAction::OpenToolPicker)),
            "smart" => Ok(CommandResult::Action(CommandAction::SetToolMode(
                ToolMode::Smart,
            ))),
            "core" => Ok(CommandResult::Action(CommandAction::SetToolMode(
                ToolMode::Core,
            ))),
            "none" => Ok(CommandResult::Action(CommandAction::SetToolMode(
                ToolMode::None,
            ))),
            _ => anyhow::bail!(
                "Invalid tool mode: '{}'\n\n\
                Valid modes:\n\
                • full     - All available tools (~40 tools)\n\
                • explicit - Open picker to select specific tools\n\
                • smart    - Auto-select tools based on query (default)\n\
                • core     - Core tools only (13 essential tools)\n\
                • none     - Disable all tools",
                mode
            ),
        }
    }
}
