//! Personal Profile Commands
//!
//! Commands for the Personal Knowledge System (PKS):
//! /profile, /profile:set, /profile:name, /profile:list, /profile:search,
//! /profile:delete, /profile:sync, /profile:export, /profile:import, /profile:stats

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute personal profile commands
    pub(super) fn execute_personal_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "profile" => Some(Ok(CommandResult::Action(CommandAction::ProfileShow))),
            "profile:set" => Some(self.cmd_profile_set(args)),
            "profile:name" => Some(self.cmd_profile_name(args)),
            "profile:list" => Some(self.cmd_profile_list(args)),
            "profile:search" => Some(self.cmd_profile_search(args)),
            "profile:delete" => Some(self.cmd_profile_delete(args)),
            "profile:sync" => Some(Ok(CommandResult::Action(CommandAction::ProfileSync))),
            "profile:export" => Some(self.cmd_profile_export(args)),
            "profile:import" => Some(self.cmd_profile_import(args)),
            "profile:stats" => Some(Ok(CommandResult::Action(CommandAction::ProfileStats))),
            "remember" => Some(self.cmd_remember(args)),
            _ => None,
        }
    }

    fn cmd_profile_set(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /profile:set [--local] <key> <value>\n\n\
                Set a personal fact about yourself.\n\n\
                Options:\n\
                  --local    Store locally only (never sync to server)\n\n\
                Examples:\n\
                  /profile:set preferred_language Rust\n\
                  /profile:set coding_style \"minimal comments, clear names\"\n\
                  /profile:set --local api_key secret123\n\n\
                Facts are synced to the server by default for cross-session persistence."
            );
        }

        // Check for --local flag
        let (local_only, remaining_args): (bool, Vec<String>) = if args.first() == Some(&"--local".to_string()) {
            (true, args[1..].to_vec())
        } else {
            (false, args.to_vec())
        };

        if remaining_args.len() < 2 {
            anyhow::bail!(
                "Usage: /profile:set [--local] <key> <value>\n\n\
                Both key and value are required."
            );
        }

        let key = remaining_args[0].clone();
        let value = remaining_args[1..].join(" ");

        Ok(CommandResult::Action(CommandAction::ProfileSet(key, value, local_only)))
    }

    fn cmd_profile_name(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /profile:name <your_name>\n\n\
                Set your display name.\n\n\
                Examples:\n\
                  /profile:name John\n\
                  /profile:name \"John Smith\""
            );
        }

        let name = args.join(" ");
        Ok(CommandResult::Action(CommandAction::ProfileSet(
            "name".to_string(),
            name,
            false, // Name should sync to server
        )))
    }

    fn cmd_profile_list(&self, args: &[String]) -> Result<CommandResult> {
        let category = args.first().cloned();

        // Validate category if provided
        if let Some(ref cat) = category {
            let valid_categories = ["identity", "preference", "capability", "context", "constraint", "relationship"];
            if !valid_categories.contains(&cat.to_lowercase().as_str()) {
                anyhow::bail!(
                    "Invalid category: {}\n\nValid categories:\n\
                    - identity: Name, role, organization, team\n\
                    - preference: Coding style, tools, communication\n\
                    - capability: Skills, languages, expertise\n\
                    - context: Current project, active work\n\
                    - constraint: Limitations, restrictions, timezone\n\
                    - relationship: Connections between facts",
                    cat
                );
            }
        }

        Ok(CommandResult::Action(CommandAction::ProfileList(category)))
    }

    fn cmd_profile_search(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /profile:search <query>\n\n\
                Search your personal facts.\n\n\
                Examples:\n\
                  /profile:search rust\n\
                  /profile:search project\n\
                  /profile:search coding"
            );
        }

        let query = args.join(" ");
        Ok(CommandResult::Action(CommandAction::ProfileSearch(query)))
    }

    fn cmd_profile_delete(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /profile:delete <id_or_key>\n\n\
                Delete a personal fact by ID or key.\n\n\
                Examples:\n\
                  /profile:delete abc123\n\
                  /profile:delete preferred_language"
            );
        }

        let id = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ProfileDelete(id)))
    }

    fn cmd_profile_export(&self, args: &[String]) -> Result<CommandResult> {
        let path = args.first().cloned();
        Ok(CommandResult::Action(CommandAction::ProfileExport(path)))
    }

    fn cmd_profile_import(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /profile:import <file_path>\n\n\
                Import personal facts from a JSON file.\n\n\
                Example:\n\
                  /profile:import ~/brainwires-profile.json\n\n\
                The file should be a JSON export from /profile:export."
            );
        }

        let path = args[0].clone();
        Ok(CommandResult::Action(CommandAction::ProfileImport(path)))
    }

    fn cmd_remember(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /remember <fact>\n\n\
                Quick command to remember contextual facts.\n\
                This is a shortcut for /profile:set with automatic key generation.\n\n\
                Examples:\n\
                  /remember Rust 2024 edition is stable as of early 2024\n\
                  /remember Using Next.js 15 for this project\n\
                  /remember Team prefers functional programming style\n\n\
                Facts are synced to the server and will appear in future conversations."
            );
        }

        let fact_text = args.join(" ");

        // Generate a key from the fact text (use first few words, sanitized)
        let key_words: Vec<&str> = fact_text
            .split_whitespace()
            .take(3)
            .collect();
        let key = format!("context_{}",
            key_words.join("_")
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        );

        // Return ProfileSet action with generated key
        Ok(CommandResult::Action(CommandAction::ProfileSet(
            key,
            fact_text,
            false, // Sync to server
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_executor() -> CommandExecutor {
        CommandExecutor::default()
    }

    #[test]
    fn test_profile_show() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileShow) => {}
            _ => panic!("Expected ProfileShow action"),
        }
    }

    #[test]
    fn test_profile_set() {
        let executor = create_executor();
        let result = executor.execute_personal_command(
            "profile:set",
            &["preferred_language".to_string(), "Rust".to_string()]
        );
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileSet(key, value, local_only)) => {
                assert_eq!(key, "preferred_language");
                assert_eq!(value, "Rust");
                assert!(!local_only);
            }
            _ => panic!("Expected ProfileSet action"),
        }
    }

    #[test]
    fn test_profile_set_local() {
        let executor = create_executor();
        let result = executor.execute_personal_command(
            "profile:set",
            &["--local".to_string(), "secret".to_string(), "value".to_string()]
        );
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileSet(key, value, local_only)) => {
                assert_eq!(key, "secret");
                assert_eq!(value, "value");
                assert!(local_only);
            }
            _ => panic!("Expected ProfileSet action with local_only=true"),
        }
    }

    #[test]
    fn test_profile_set_no_args() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:set", &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_profile_name() {
        let executor = create_executor();
        let result = executor.execute_personal_command(
            "profile:name",
            &["John".to_string(), "Smith".to_string()]
        );
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileSet(key, value, local_only)) => {
                assert_eq!(key, "name");
                assert_eq!(value, "John Smith");
                assert!(!local_only);
            }
            _ => panic!("Expected ProfileSet action for name"),
        }
    }

    #[test]
    fn test_profile_list() {
        let executor = create_executor();

        // Without filter
        let result = executor.execute_personal_command("profile:list", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileList(None)) => {}
            _ => panic!("Expected ProfileList action with None"),
        }

        // With filter
        let result = executor.execute_personal_command("profile:list", &["preference".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileList(Some(cat))) => {
                assert_eq!(cat, "preference");
            }
            _ => panic!("Expected ProfileList action with category"),
        }
    }

    #[test]
    fn test_profile_list_invalid_category() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:list", &["invalid".to_string()]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_profile_search() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:search", &["rust".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileSearch(query)) => {
                assert_eq!(query, "rust");
            }
            _ => panic!("Expected ProfileSearch action"),
        }
    }

    #[test]
    fn test_profile_delete() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:delete", &["abc123".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileDelete(id)) => {
                assert_eq!(id, "abc123");
            }
            _ => panic!("Expected ProfileDelete action"),
        }
    }

    #[test]
    fn test_profile_sync() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:sync", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileSync) => {}
            _ => panic!("Expected ProfileSync action"),
        }
    }

    #[test]
    fn test_profile_export() {
        let executor = create_executor();

        // Without path
        let result = executor.execute_personal_command("profile:export", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileExport(None)) => {}
            _ => panic!("Expected ProfileExport action with None"),
        }

        // With path
        let result = executor.execute_personal_command("profile:export", &["/tmp/profile.json".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileExport(Some(path))) => {
                assert_eq!(path, "/tmp/profile.json");
            }
            _ => panic!("Expected ProfileExport action with path"),
        }
    }

    #[test]
    fn test_profile_import() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:import", &["/tmp/profile.json".to_string()]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileImport(path)) => {
                assert_eq!(path, "/tmp/profile.json");
            }
            _ => panic!("Expected ProfileImport action"),
        }
    }

    #[test]
    fn test_profile_import_no_args() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:import", &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_profile_stats() {
        let executor = create_executor();
        let result = executor.execute_personal_command("profile:stats", &[]);
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileStats) => {}
            _ => panic!("Expected ProfileStats action"),
        }
    }

    #[test]
    fn test_remember() {
        let executor = create_executor();
        let result = executor.execute_personal_command(
            "remember",
            &[
                "Rust".to_string(),
                "2024".to_string(),
                "edition".to_string(),
                "is".to_string(),
                "stable".to_string(),
            ]
        );
        assert!(result.is_some());
        let result = result.unwrap().unwrap();
        match result {
            CommandResult::Action(CommandAction::ProfileSet(key, value, local_only)) => {
                assert!(key.starts_with("context_"));
                assert_eq!(value, "Rust 2024 edition is stable");
                assert!(!local_only);
            }
            _ => panic!("Expected ProfileSet action"),
        }
    }

    #[test]
    fn test_remember_no_args() {
        let executor = create_executor();
        let result = executor.execute_personal_command("remember", &[]);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }
}
