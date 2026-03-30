//! Knowledge Commands
//!
//! Commands for the Behavioral Knowledge System (BKS):
//! /learn, /knowledge, /knowledge:list, /knowledge:search, /knowledge:sync, /knowledge:contradict, /knowledge:delete

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute knowledge-related built-in commands
    pub(super) fn execute_knowledge_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "learn" => Some(self.cmd_learn(args)),
            "knowledge" => Some(Ok(CommandResult::Action(CommandAction::KnowledgeStatus))),
            "knowledge:list" => Some(self.cmd_knowledge_list(args)),
            "knowledge:search" => Some(self.cmd_knowledge_search(args)),
            "knowledge:sync" => Some(Ok(CommandResult::Action(CommandAction::KnowledgeSync))),
            "knowledge:contradict" => Some(self.cmd_knowledge_contradict(args)),
            "knowledge:delete" => Some(self.cmd_knowledge_delete(args)),
            _ => None,
        }
    }

    fn cmd_learn(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /learn <rule> [rationale]\n\n\
                Teach the agent a behavioral truth that will be shared with all users.\n\n\
                Examples:\n\
                  /learn \"pm2 logs requires --nostream to avoid blocking\"\n\
                  /learn \"Use cargo-watch instead of loops\" \"More efficient for watch mode\"\n\n\
                The rule will be submitted to the Brainwires server and shared globally."
            );
        }

        // First argument is the rule, second (optional) is rationale
        let rule = args[0].clone();
        let rationale = args.get(1).cloned();

        Ok(CommandResult::Action(CommandAction::LearnTruth(rule, rationale)))
    }

    fn cmd_knowledge_list(&self, args: &[String]) -> Result<CommandResult> {
        let category = args.first().cloned();

        // Validate category if provided
        if let Some(ref cat) = category {
            let valid_categories = ["command", "strategy", "tool", "error", "resource", "pattern"];
            if !valid_categories.contains(&cat.to_lowercase().as_str()) {
                anyhow::bail!(
                    "Invalid category: {}\n\nValid categories:\n\
                    - command: CLI flags and arguments\n\
                    - strategy: Task approach strategies\n\
                    - tool: Tool-specific behaviors\n\
                    - error: Error recovery patterns\n\
                    - resource: Resource management\n\
                    - pattern: Anti-patterns to avoid",
                    cat
                );
            }
        }

        Ok(CommandResult::Action(CommandAction::KnowledgeList(category)))
    }

    fn cmd_knowledge_search(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /knowledge:search <query>\n\n\
                Search learned truths by keyword.\n\n\
                Examples:\n\
                  /knowledge:search pm2\n\
                  /knowledge:search nostream\n\
                  /knowledge:search cargo build"
            );
        }

        let query = args.join(" ");
        Ok(CommandResult::Action(CommandAction::KnowledgeSearch(query)))
    }

    fn cmd_knowledge_contradict(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /knowledge:contradict <truth_id> [reason]\n\n\
                Report a learned truth as incorrect. This will reduce its confidence\n\
                score across all clients.\n\n\
                Example:\n\
                  /knowledge:contradict abc123 \"This doesn't apply to pm2 v5\""
            );
        }

        let id = args[0].clone();
        let reason = if args.len() > 1 {
            Some(args[1..].join(" "))
        } else {
            None
        };

        Ok(CommandResult::Action(CommandAction::KnowledgeContradict(id, reason)))
    }

    fn cmd_knowledge_delete(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /knowledge:delete <truth_id>\n\n\
                Delete a truth from your local cache.\n\
                Note: This only affects your local cache, not the server."
            );
        }

        let id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::KnowledgeDelete(id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_executor() -> CommandExecutor {
        CommandExecutor::default()
    }

    #[test]
    fn test_learn_command() {
        let executor = create_executor();

        // Test with rule only
        let result = executor.execute_knowledge_command("learn", &["test rule".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::LearnTruth(rule, rationale)) => {
                assert_eq!(rule, "test rule");
                assert!(rationale.is_none());
            }
            _ => panic!("Expected LearnTruth action"),
        }

        // Test with rule and rationale
        let result = executor.execute_knowledge_command(
            "learn",
            &["test rule".to_string(), "because it's better".to_string()]
        );
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::LearnTruth(rule, rationale)) => {
                assert_eq!(rule, "test rule");
                assert_eq!(rationale, Some("because it's better".to_string()));
            }
            _ => panic!("Expected LearnTruth action"),
        }
    }

    #[test]
    fn test_learn_command_no_args() {
        let executor = create_executor();
        let result = executor.execute_knowledge_command("learn", &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_knowledge_status() {
        let executor = create_executor();
        let result = executor.execute_knowledge_command("knowledge", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::KnowledgeStatus) => {}
            _ => panic!("Expected KnowledgeStatus action"),
        }
    }

    #[test]
    fn test_knowledge_list() {
        let executor = create_executor();

        // Without filter
        let result = executor.execute_knowledge_command("knowledge:list", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::KnowledgeList(None)) => {}
            _ => panic!("Expected KnowledgeList action with None"),
        }

        // With filter
        let result = executor.execute_knowledge_command("knowledge:list", &["command".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::KnowledgeList(Some(cat))) => {
                assert_eq!(cat, "command");
            }
            _ => panic!("Expected KnowledgeList action with category"),
        }
    }

    #[test]
    fn test_knowledge_list_invalid_category() {
        let executor = create_executor();
        let result = executor.execute_knowledge_command("knowledge:list", &["invalid".to_string()]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_knowledge_search() {
        let executor = create_executor();
        let result = executor.execute_knowledge_command("knowledge:search", &["pm2".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::KnowledgeSearch(query)) => {
                assert_eq!(query, "pm2");
            }
            _ => panic!("Expected KnowledgeSearch action"),
        }
    }

    #[test]
    fn test_knowledge_contradict() {
        let executor = create_executor();
        let result = executor.execute_knowledge_command(
            "knowledge:contradict",
            &["abc123".to_string(), "not".to_string(), "valid".to_string()]
        );
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::KnowledgeContradict(id, reason)) => {
                assert_eq!(id, "abc123");
                assert_eq!(reason, Some("not valid".to_string()));
            }
            _ => panic!("Expected KnowledgeContradict action"),
        }
    }
}
