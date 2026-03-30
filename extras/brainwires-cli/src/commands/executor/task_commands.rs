//! Task Commands
//!
//! Commands for task management: tasks, task:complete, task:skip, etc.

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute task-related built-in commands
    pub(super) fn execute_task_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "tasks" => Some(Ok(CommandResult::Action(CommandAction::ShowTasks))),
            "task:complete" => Some(self.cmd_task_complete(args)),
            "task:skip" => Some(self.cmd_task_skip(args)),
            "task:add" => Some(self.cmd_task_add(args)),
            "task:start" => Some(self.cmd_task_start(args)),
            "task:block" => Some(self.cmd_task_block(args)),
            "task:depends" => Some(self.cmd_task_depends(args)),
            "task:ready" => Some(Ok(CommandResult::Action(CommandAction::TaskReady))),
            "task:time" => Some(self.cmd_task_time(args)),
            "task:list" => Some(Ok(CommandResult::Action(CommandAction::TaskList))),
            _ => None,
        }
    }

    fn cmd_task_complete(&self, args: &[String]) -> Result<CommandResult> {
        let task_id = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::TaskComplete(task_id)))
    }

    fn cmd_task_skip(&self, args: &[String]) -> Result<CommandResult> {
        let task_id = args.first().map(|s| s.clone());
        let reason = if args.len() > 1 {
            Some(args[1..].join(" "))
        } else {
            None
        };
        Ok(CommandResult::Action(CommandAction::TaskSkip(task_id, reason)))
    }

    fn cmd_task_add(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /task:add <description>\n\nAdd a new task to the current plan.");
        }
        let description = args.join(" ");
        Ok(CommandResult::Action(CommandAction::TaskAdd(description)))
    }

    fn cmd_task_start(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /task:start <task_id>\n\nStart working on a specific task.");
        }
        let task_id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::TaskStart(task_id)))
    }

    fn cmd_task_block(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /task:block <task_id> [reason]\n\nMark a task as blocked.");
        }
        let task_id = args[0].clone();
        let reason = if args.len() > 1 {
            Some(args[1..].join(" "))
        } else {
            None
        };
        Ok(CommandResult::Action(CommandAction::TaskBlock(task_id, reason)))
    }

    fn cmd_task_depends(&self, args: &[String]) -> Result<CommandResult> {
        if args.len() < 2 {
            anyhow::bail!(
                "Usage: /task:depends <task_id> <depends_on_id>\n\n\
                Make a task depend on another task.\n\
                The first task will be blocked until the second is completed."
            );
        }
        let task_id = args[0].clone();
        let depends_on = args[1].clone();
        Ok(CommandResult::Action(CommandAction::TaskDepends(task_id, depends_on)))
    }

    fn cmd_task_time(&self, args: &[String]) -> Result<CommandResult> {
        let task_id = args.first().map(|s| s.clone());
        Ok(CommandResult::Action(CommandAction::TaskTime(task_id)))
    }
}
