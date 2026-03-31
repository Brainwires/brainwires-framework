//! Template Commands
//!
//! Commands for template management: templates, template:save, template:show, etc.

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute template-related built-in commands
    pub(super) fn execute_template_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "templates" => Some(Ok(CommandResult::Action(CommandAction::ListTemplates))),
            "template:save" => Some(self.cmd_template_save(args)),
            "template:show" => Some(self.cmd_template_show(args)),
            "template:use" => Some(self.cmd_template_use(args)),
            "template:delete" => Some(self.cmd_template_delete(args)),
            _ => None,
        }
    }

    fn cmd_template_save(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /template:save <name> [description]\n\n\
                Save the current active plan as a reusable template.\n\
                Use {{variable}} syntax in plans for placeholders."
            );
        }
        let name = args[0].clone();
        let description = if args.len() > 1 {
            Some(args[1..].join(" "))
        } else {
            None
        };
        Ok(CommandResult::Action(CommandAction::SaveTemplate(
            name,
            description,
        )))
    }

    fn cmd_template_show(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /template:show <name_or_id>\n\nDisplay a template's content and variables."
            );
        }
        let name = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ShowTemplate(name)))
    }

    fn cmd_template_use(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /template:use <name> [var1=value1 var2=value2 ...]\n\n\
                Create a new plan from a template with variable substitutions.\n\n\
                Example: /template:use feature-impl component=Auth feature=login"
            );
        }
        let name = args[0].clone();
        let vars = args[1..].to_vec();
        Ok(CommandResult::Action(CommandAction::UseTemplate(
            name, vars,
        )))
    }

    fn cmd_template_delete(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /template:delete <name_or_id>\n\nDelete a saved template.");
        }
        let name = args[0].clone();
        Ok(CommandResult::Action(CommandAction::DeleteTemplate(name)))
    }
}
