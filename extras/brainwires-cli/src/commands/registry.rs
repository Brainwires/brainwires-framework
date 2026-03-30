//! Command Registry
//!
//! Central registry for managing slash commands

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// Command argument definition
#[derive(Debug, Clone)]
pub struct CommandArg {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

/// Slash command definition
#[derive(Debug, Clone)]
pub struct Command {
    /// Command name (without the leading slash)
    pub name: String,
    /// Command description
    pub description: String,
    /// Command arguments
    pub args: Vec<CommandArg>,
    /// Command template/content
    pub content: String,
    /// Whether this is a built-in command
    pub builtin: bool,
    /// Source file path (for custom commands)
    pub source_path: Option<PathBuf>,
}

impl Command {
    /// Create a new command
    pub fn new(name: String, description: String, content: String) -> Self {
        Self {
            name,
            description,
            args: Vec::new(),
            content,
            builtin: false,
            source_path: None,
        }
    }

    /// Create a built-in command
    pub fn builtin(name: String, description: String, content: String) -> Self {
        Self {
            name,
            description,
            args: Vec::new(),
            content,
            builtin: true,
            source_path: None,
        }
    }

    /// Add an argument to the command
    pub fn with_arg(mut self, name: String, description: Option<String>, required: bool) -> Self {
        self.args.push(CommandArg {
            name,
            description,
            required,
        });
        self
    }

    /// Set the source path
    pub fn with_source_path(mut self, path: PathBuf) -> Self {
        self.source_path = Some(path);
        self
    }
}

/// Command registry managing all available commands
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a command
    pub fn register(&mut self, command: Command) {
        self.commands.insert(command.name.clone(), command);
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    /// List all command names
    pub fn list_commands(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get all commands
    pub fn commands(&self) -> &HashMap<String, Command> {
        &self.commands
    }

    /// Load built-in commands
    pub fn load_builtin(&mut self) -> Result<()> {
        super::builtin::register_builtin_commands(self);
        Ok(())
    }

    /// Load custom commands from .brainwires/commands/ directory
    pub fn load_custom(&mut self, commands_dir: &std::path::Path) -> Result<()> {
        if !commands_dir.exists() {
            return Ok(()); // No custom commands directory
        }

        for entry in std::fs::read_dir(commands_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                match super::parser::parse_command_file(&path) {
                    Ok(command) => {
                        tracing::info!("Loaded custom command: /{}", command.name);
                        self.register(command);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load command from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = Command::new(
            "test".to_string(),
            "Test command".to_string(),
            "Do something".to_string(),
        );

        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, "Test command");
        assert!(!cmd.builtin);
    }

    #[test]
    fn test_command_with_args() {
        let cmd = Command::new(
            "review".to_string(),
            "Review code".to_string(),
            "Review {{file}}".to_string(),
        )
        .with_arg("file".to_string(), Some("File to review".to_string()), false);

        assert_eq!(cmd.args.len(), 1);
        assert_eq!(cmd.args[0].name, "file");
    }

    #[test]
    fn test_registry() {
        let mut registry = CommandRegistry::new();

        let cmd = Command::builtin(
            "clear".to_string(),
            "Clear conversation".to_string(),
            "".to_string(),
        );

        registry.register(cmd);

        assert!(registry.get("clear").is_some());
        assert!(registry.get("nonexistent").is_none());
        assert_eq!(registry.list_commands(), vec!["clear"]);
    }
}
