//! Context/Working Set Commands
//!
//! Commands for managing the working set (files currently in context)

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute context-related built-in commands
    pub(super) fn execute_context_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "context" => Some(self.cmd_context_show()),
            "context:add" => Some(self.cmd_context_add(args)),
            "context:remove" => Some(self.cmd_context_remove(args)),
            "context:pin" => Some(self.cmd_context_pin(args)),
            "context:unpin" => Some(self.cmd_context_unpin(args)),
            "context:clear" => Some(self.cmd_context_clear(args)),
            _ => None,
        }
    }

    /// /context - Show working set
    fn cmd_context_show(&self) -> Result<CommandResult> {
        Ok(CommandResult::Action(CommandAction::ContextShow))
    }

    /// /context:add <path> [pinned]
    fn cmd_context_add(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /context:add <path> [pinned]\n  path: Path to file\n  pinned: true/false (optional, default: false)"
            );
        }

        let path = args[0].clone();
        let pinned = args
            .get(1)
            .map(|s| s.to_lowercase() == "true" || s == "1")
            .unwrap_or(false);

        Ok(CommandResult::Action(CommandAction::ContextAdd(
            path, pinned,
        )))
    }

    /// /context:remove <path>
    fn cmd_context_remove(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /context:remove <path>");
        }

        let path = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ContextRemove(path)))
    }

    /// /context:pin <path>
    fn cmd_context_pin(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /context:pin <path>");
        }

        let path = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ContextPin(path)))
    }

    /// /context:unpin <path>
    fn cmd_context_unpin(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /context:unpin <path>");
        }

        let path = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ContextUnpin(path)))
    }

    /// /context:clear [keep_pinned]
    fn cmd_context_clear(&self, args: &[String]) -> Result<CommandResult> {
        let keep_pinned = args
            .first()
            .map(|s| s.to_lowercase() != "false" && s != "0")
            .unwrap_or(true); // Default to keeping pinned files

        Ok(CommandResult::Action(CommandAction::ContextClear(
            keep_pinned,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_executor() -> CommandExecutor {
        CommandExecutor::new().expect("Failed to create executor")
    }

    #[test]
    fn test_context_show() {
        let executor = make_executor();
        let result = executor.execute_context_command("context", &[]);
        assert!(result.is_some());

        if let Some(Ok(CommandResult::Action(CommandAction::ContextShow))) = result {
            // Success
        } else {
            panic!("Expected ContextShow action");
        }
    }

    #[test]
    fn test_context_add() {
        let executor = make_executor();

        // With path only
        let result = executor.execute_context_command("context:add", &["src/main.rs".to_string()]);
        if let Some(Ok(CommandResult::Action(CommandAction::ContextAdd(path, pinned)))) = result {
            assert_eq!(path, "src/main.rs");
            assert!(!pinned);
        } else {
            panic!("Expected ContextAdd action");
        }

        // With pinned flag
        let result = executor.execute_context_command(
            "context:add",
            &["src/main.rs".to_string(), "true".to_string()],
        );
        if let Some(Ok(CommandResult::Action(CommandAction::ContextAdd(path, pinned)))) = result {
            assert_eq!(path, "src/main.rs");
            assert!(pinned);
        } else {
            panic!("Expected ContextAdd action with pinned=true");
        }
    }

    #[test]
    fn test_context_add_missing_path() {
        let executor = make_executor();
        let result = executor.execute_context_command("context:add", &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_context_remove() {
        let executor = make_executor();
        let result =
            executor.execute_context_command("context:remove", &["src/main.rs".to_string()]);
        if let Some(Ok(CommandResult::Action(CommandAction::ContextRemove(path)))) = result {
            assert_eq!(path, "src/main.rs");
        } else {
            panic!("Expected ContextRemove action");
        }
    }

    #[test]
    fn test_context_clear() {
        let executor = make_executor();

        // Default (keep pinned)
        let result = executor.execute_context_command("context:clear", &[]);
        if let Some(Ok(CommandResult::Action(CommandAction::ContextClear(keep_pinned)))) = result {
            assert!(keep_pinned);
        } else {
            panic!("Expected ContextClear action");
        }

        // Explicit false
        let result = executor.execute_context_command("context:clear", &["false".to_string()]);
        if let Some(Ok(CommandResult::Action(CommandAction::ContextClear(keep_pinned)))) = result {
            assert!(!keep_pinned);
        } else {
            panic!("Expected ContextClear action with keep_pinned=false");
        }
    }
}
