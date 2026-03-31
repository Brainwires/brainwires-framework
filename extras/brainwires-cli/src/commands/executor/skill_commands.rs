//! Skill Commands
//!
//! Handlers for skill-related slash commands:
//! - /skill <name> - Invoke a skill
//! - /skills - List all skills
//! - /skill:show <name> - Show skill details
//! - /skill:reload - Reload skills from disk
//! - /skill:create <name> - Create a new skill

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Try to execute a skill command
    pub(super) fn execute_skill_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "skill" => Some(self.cmd_skill(args)),
            "skills" => Some(self.cmd_skills(args)),
            "skill:show" => Some(self.cmd_skill_show(args)),
            "skill:reload" => Some(self.cmd_skill_reload(args)),
            "skill:create" => Some(self.cmd_skill_create(args)),
            _ => None,
        }
    }

    /// /skill <name> [args...] - Invoke a skill by name
    fn cmd_skill(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /skill <name> [key=value ...]");
        }

        let skill_name = &args[0];
        let skill_args = if args.len() > 1 {
            args[1..].to_vec()
        } else {
            Vec::new()
        };

        Ok(CommandResult::Action(CommandAction::InvokeSkill(
            skill_name.clone(),
            skill_args,
        )))
    }

    /// /skills - List all available skills
    fn cmd_skills(&self, _args: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::Action(CommandAction::ListSkills))
    }

    /// /skill:show <name> - Show detailed information about a skill
    fn cmd_skill_show(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /skill:show <name>");
        }

        let skill_name = &args[0];
        Ok(CommandResult::Action(CommandAction::ShowSkill(
            skill_name.clone(),
        )))
    }

    /// /skill:reload - Reload skills from disk
    fn cmd_skill_reload(&self, _args: &[String]) -> Result<CommandResult> {
        Ok(CommandResult::Action(CommandAction::ReloadSkills))
    }

    /// /skill:create <name> [location] - Create a new skill
    fn cmd_skill_create(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /skill:create <name> [personal|project]");
        }

        let skill_name = &args[0];
        let location = args.get(1).cloned();

        Ok(CommandResult::Action(CommandAction::CreateSkill(
            skill_name.clone(),
            location,
        )))
    }
}
