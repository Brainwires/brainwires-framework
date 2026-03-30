//! Miscellaneous Commands
//!
//! Other commands: checkpoint, review, brainwires, exec, compact, approvals, shells

use anyhow::{Context, Result};

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute miscellaneous built-in commands
    pub(super) fn execute_misc_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "checkpoint" => Some(self.cmd_checkpoint(args)),
            "restore" => Some(self.cmd_restore(args)),
            "checkpoints" => Some(Ok(CommandResult::Action(CommandAction::ListCheckpoints))),
            "review" => Some(self.cmd_review()),
            "brainwires" => Some(self.cmd_brainwires()),
            "exec" => Some(self.cmd_exec(args)),
            "shells" => Some(Ok(CommandResult::Action(CommandAction::ShowShellHistory))),
            "hotkeys" | "keys" => Some(Ok(CommandResult::Action(CommandAction::OpenHotkeyDialog))),
            "approvals" => Some(self.cmd_approvals(args)),
            _ => None,
        }
    }

    fn cmd_checkpoint(&self, args: &[String]) -> Result<CommandResult> {
        let name = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::CreateCheckpoint(name)))
    }

    fn cmd_restore(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /restore <checkpoint_id_or_index>");
        }
        let checkpoint_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::RestoreCheckpoint(checkpoint_id)))
    }

    fn cmd_review(&self) -> Result<CommandResult> {
        use std::process::Command as ProcessCommand;

        // Try to get git diff
        let diff_result = ProcessCommand::new("git")
            .args(["diff", "--staged"])
            .output();

        let staged_diff = match diff_result {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).to_string()
            }
            _ => String::new()
        };

        // If no staged changes, try unstaged changes
        let diff = if staged_diff.trim().is_empty() {
            let unstaged_result = ProcessCommand::new("git")
                .args(["diff"])
                .output();

            match unstaged_result {
                Ok(output) if output.status.success() => {
                    String::from_utf8_lossy(&output.stdout).to_string()
                }
                _ => String::new()
            }
        } else {
            staged_diff
        };

        // Build review message
        let cmd = self.registry.get("review").unwrap();
        let message = if diff.trim().is_empty() {
            format!("{}\n\nNo git changes found. Please stage or modify some files first.", cmd.content)
        } else {
            format!("{}\n\nHere are the code changes:\n\n```diff\n{}\n```", cmd.content, diff.trim())
        };

        Ok(CommandResult::Message(message))
    }

    fn cmd_brainwires(&self) -> Result<CommandResult> {
        use crate::utils::brainwires_md;

        let cwd = std::env::current_dir()
            .context("Failed to get current working directory")?;

        match brainwires_md::load_brainwires_instructions(&cwd) {
            Ok(content) if content.is_empty() => {
                Ok(CommandResult::Message(
                    "No BRAINWIRES.md file found in current directory.\n\n\
                    Create a BRAINWIRES.md file to add project-specific instructions.\n\
                    You can use @file.md syntax to import other markdown files.".to_string()
                ))
            }
            Ok(content) => {
                let message = format!(
                    "Loaded project instructions from BRAINWIRES.md:\n\n{}\n\n\
                    These instructions will guide my responses for this project.",
                    content.trim()
                );
                Ok(CommandResult::Message(message))
            }
            Err(e) => {
                anyhow::bail!("Failed to load BRAINWIRES.md: {}", e)
            }
        }
    }

    fn cmd_exec(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /exec <command>\n\nExecute a shell command in a full-screen terminal overlay.");
        }

        let command = args.join(" ");

        // Security: Log the command for audit trail
        tracing::info!(target: "security", "User executing shell command: {}", command);

        Ok(CommandResult::Action(CommandAction::ExecCommand(command)))
    }

    fn cmd_approvals(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /approvals <mode>\n\
                Available modes:\n\
                - suggest: Review and approve all AI actions (safest)\n\
                - auto-edit: Auto-approve file edits, review other actions\n\
                - full-auto: Auto-approve all actions (least safe)"
            );
        }

        let mode = args[0].to_lowercase();
        if !["suggest", "auto-edit", "full-auto"].contains(&mode.as_str()) {
            anyhow::bail!(
                "Invalid approval mode: {}\n\
                Valid modes: suggest, auto-edit, full-auto",
                mode
            );
        }

        Ok(CommandResult::Action(CommandAction::SetApprovalMode(mode)))
    }

    /// Main dispatcher for built-in commands
    pub(super) fn execute_builtin(&self, name: &str, args: &[String]) -> Result<CommandResult> {
        // Try each category of commands
        if let Some(result) = self.execute_conversation_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_plan_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_task_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_template_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_project_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_context_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_tools_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_mdap_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_knowledge_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_personal_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_agent_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_skill_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_prompt_mode_command(name, args) {
            return result;
        }

        if let Some(result) = self.execute_misc_command(name, args) {
            return result;
        }

        anyhow::bail!("Unknown built-in command: {}", name)
    }
}
