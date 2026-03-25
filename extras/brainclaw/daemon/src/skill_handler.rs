//! Skill handler — detects /commands and dispatches to skill system.

use std::path::PathBuf;

use anyhow::Result;
use brainwires_skills::{SkillRegistry, SkillSource};

/// Handles skill-based /commands from user messages.
pub struct SkillHandler {
    registry: SkillRegistry,
}

impl SkillHandler {
    /// Create a new skill handler that discovers skills from the given directories.
    pub fn new(skill_dirs: &[PathBuf]) -> Result<Self> {
        let mut registry = SkillRegistry::new();

        if !skill_dirs.is_empty() {
            let paths: Vec<(PathBuf, SkillSource)> = skill_dirs
                .iter()
                .map(|dir| (dir.clone(), SkillSource::Project))
                .collect();
            registry.discover_from(&paths)?;
        }

        Ok(Self { registry })
    }

    /// Create an empty skill handler with no skills loaded.
    pub fn empty() -> Self {
        Self {
            registry: SkillRegistry::new(),
        }
    }

    /// Parse a /command from the beginning of a text message.
    ///
    /// Returns `Some((command, args))` if the text starts with `/`,
    /// or `None` if it does not.
    pub fn parse_command(text: &str) -> Option<(&str, &str)> {
        let text = text.trim();
        if !text.starts_with('/') {
            return None;
        }

        // Split on first whitespace
        let without_slash = &text[1..];
        if without_slash.is_empty() {
            return None;
        }

        match without_slash.find(char::is_whitespace) {
            Some(pos) => {
                let command = &without_slash[..pos];
                let args = without_slash[pos..].trim_start();
                Some((command, args))
            }
            None => Some((without_slash, "")),
        }
    }

    /// Handle a /command by looking up the skill and returning its content.
    pub fn handle_command(&self, command: &str, _args: &str) -> Result<String> {
        match self.registry.get_metadata(command) {
            Some(metadata) => {
                // Return the skill description as a basic response
                Ok(format!(
                    "Skill '{}': {}",
                    metadata.name, metadata.description
                ))
            }
            None => Ok(format!("Unknown command: /{command}. No matching skill found.")),
        }
    }

    /// Return the number of loaded skills.
    pub fn skill_count(&self) -> usize {
        self.registry.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_simple() {
        let result = SkillHandler::parse_command("/help");
        assert_eq!(result, Some(("help", "")));
    }

    #[test]
    fn test_parse_command_with_args() {
        let result = SkillHandler::parse_command("/review-pr 123");
        assert_eq!(result, Some(("review-pr", "123")));
    }

    #[test]
    fn test_parse_command_with_multi_args() {
        let result = SkillHandler::parse_command("/search code patterns");
        assert_eq!(result, Some(("search", "code patterns")));
    }

    #[test]
    fn test_parse_command_with_leading_whitespace() {
        let result = SkillHandler::parse_command("  /help ");
        assert_eq!(result, Some(("help", "")));
    }

    #[test]
    fn test_parse_command_not_a_command() {
        assert!(SkillHandler::parse_command("hello world").is_none());
        assert!(SkillHandler::parse_command("").is_none());
        assert!(SkillHandler::parse_command("no slash").is_none());
    }

    #[test]
    fn test_parse_command_bare_slash() {
        assert!(SkillHandler::parse_command("/").is_none());
    }

    #[test]
    fn test_empty_handler() {
        let handler = SkillHandler::empty();
        assert_eq!(handler.skill_count(), 0);
    }

    #[test]
    fn test_handle_unknown_command() {
        let handler = SkillHandler::empty();
        let result = handler.handle_command("nonexistent", "").unwrap();
        assert!(result.contains("Unknown command"));
        assert!(result.contains("/nonexistent"));
    }

    #[test]
    fn test_new_with_nonexistent_dir() {
        // Should succeed even with non-existent directories (they are skipped)
        let handler = SkillHandler::new(&[PathBuf::from("/nonexistent/skills/dir")]);
        assert!(handler.is_ok());
        assert_eq!(handler.unwrap().skill_count(), 0);
    }
}
