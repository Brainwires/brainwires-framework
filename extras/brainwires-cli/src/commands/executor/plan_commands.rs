//! Plan Commands
//!
//! Commands for plan management: plan, plans, plan:show, plan:activate, etc.

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute plan-related built-in commands
    pub(super) fn execute_plan_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "plan" => Some(self.cmd_plan(args)),
            "plans" => Some(self.cmd_plans(args)),
            "plan:show" => Some(self.cmd_plan_show(args)),
            "plan:delete" => Some(self.cmd_plan_delete(args)),
            "plan:activate" => Some(self.cmd_plan_activate(args)),
            "plan:deactivate" => Some(Ok(CommandResult::Action(CommandAction::DeactivatePlan))),
            "plan:current" => Some(Ok(CommandResult::Action(CommandAction::PlanStatus))),
            "plan:pause" => Some(Ok(CommandResult::Action(CommandAction::PausePlan))),
            "plan:resume" => Some(self.cmd_plan_resume(args)),
            "plan:execute" => Some(self.cmd_plan_execute(args)),
            "plan:search" => Some(self.cmd_plan_search(args)),
            "plan:branch" => Some(self.cmd_plan_branch(args)),
            "plan:merge" => Some(self.cmd_plan_merge(args)),
            "plan:tree" => Some(self.cmd_plan_tree(args)),
            // Plan mode commands
            "plan:mode" => Some(self.cmd_plan_mode(args)),
            "plan:mode:exit" => Some(Ok(CommandResult::Action(CommandAction::ExitPlanMode))),
            "plan:mode:status" => Some(Ok(CommandResult::Action(CommandAction::PlanModeStatus))),
            "plan:mode:clear" => Some(Ok(CommandResult::Action(CommandAction::ClearPlanMode))),
            "plan:mode:export" => Some(self.cmd_plan_mode_export(args)),
            _ => None,
        }
    }

    fn cmd_plan(&self, args: &[String]) -> Result<CommandResult> {
        let task_description = if args.is_empty() {
            anyhow::bail!(
                "Usage: /plan <task description>\n\n\
                Creates a detailed plan using the planning agent.\n\
                The plan will be saved and can be viewed with /plans or /plan:show.\n\n\
                Example: /plan implement user authentication"
            );
        } else {
            args.join(" ")
        };

        let planning_prompt = format!(
            "Please use the plan_task tool to create a detailed execution plan for: {}\n\n\
            The planning agent will research the codebase and create a comprehensive plan.\n\
            The plan will be automatically saved and can be retrieved later.",
            task_description
        );

        Ok(CommandResult::Message(planning_prompt))
    }

    fn cmd_plans(&self, args: &[String]) -> Result<CommandResult> {
        let conversation_id = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::ListPlans(conversation_id)))
    }

    fn cmd_plan_show(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /plan:show <plan_id>\n\nDisplay details of a saved plan.");
        }
        let plan_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ShowPlan(plan_id)))
    }

    fn cmd_plan_delete(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /plan:delete <plan_id>\n\nDelete a saved plan.");
        }
        let plan_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::DeletePlan(plan_id)))
    }

    fn cmd_plan_activate(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /plan:activate <plan_id>\n\n\
                Set a plan as the active working plan.\n\
                The agent will use this plan to guide its work."
            );
        }
        let plan_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ActivatePlan(plan_id)))
    }

    fn cmd_plan_resume(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /plan:resume <plan_id>\n\n\
                Resume a paused plan with its saved task state."
            );
        }
        let plan_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ResumePlan(plan_id)))
    }

    fn cmd_plan_execute(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /plan:execute <plan_id> [mode]\n\n\
                Execute a plan with AI-driven task completion.\n\n\
                Modes:\n  \
                  suggest    - Ask before each task (safest)\n  \
                  auto-edit  - Auto-approve file edits, ask for shell commands\n  \
                  full-auto  - Auto-approve everything (default)\n\n\
                Example: /plan:execute abc123 full-auto"
            );
        }
        let plan_id = args[0].clone();
        let mode = args.get(1).map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::ExecutePlan(plan_id, mode)))
    }

    fn cmd_plan_search(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /plan:search <query>\n\n\
                Search plans by title, description, or content.\n\n\
                Example: /plan:search authentication"
            );
        }
        let query = args.join(" ");
        Ok(CommandResult::Action(CommandAction::SearchPlans(query)))
    }

    fn cmd_plan_branch(&self, args: &[String]) -> Result<CommandResult> {
        if args.len() < 2 {
            anyhow::bail!(
                "Usage: /plan:branch <name> <task_description>\n\n\
                Create a sub-plan branch from the current active plan.\n\n\
                Example: /plan:branch auth-feature Implement user authentication"
            );
        }
        let branch_name = args[0].clone();
        let task_description = args[1..].join(" ");
        Ok(CommandResult::Action(CommandAction::BranchPlan(branch_name, task_description)))
    }

    fn cmd_plan_merge(&self, args: &[String]) -> Result<CommandResult> {
        let plan_id = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::MergePlan(plan_id)))
    }

    fn cmd_plan_tree(&self, args: &[String]) -> Result<CommandResult> {
        let plan_id = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::PlanTree(plan_id)))
    }

    // Plan mode commands

    fn cmd_plan_mode(&self, args: &[String]) -> Result<CommandResult> {
        // If called with no args, toggle plan mode
        // If called with a focus, enter plan mode with that focus
        let focus = if args.is_empty() {
            None
        } else {
            Some(args.join(" "))
        };
        Ok(CommandResult::Action(CommandAction::EnterPlanMode(focus)))
    }

    fn cmd_plan_mode_export(&self, args: &[String]) -> Result<CommandResult> {
        let path = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::ExportPlanMode(path)))
    }
}
