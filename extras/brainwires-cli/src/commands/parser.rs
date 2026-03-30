//! Markdown Command Parser
//!
//! Parses .md files from .brainwires/commands/ directory
//!
//! Format:
//! ```markdown
//! ---
//! name: review
//! description: Review code changes
//! args:
//!   - name: file
//!     description: File to review
//!     required: false
//! ---
//!
//! Please review the following:
//! {{file}}
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::registry::Command;

/// YAML frontmatter for command definition
#[derive(Debug, Deserialize, Serialize)]
struct CommandFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    args: Vec<ArgDefinition>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ArgDefinition {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    required: bool,
}

/// Parse a command file from a markdown file
pub fn parse_command_file(path: &Path) -> Result<Command> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read command file: {}", path.display()))?;

    parse_command(&content, path)
}

/// Parse command from markdown content
fn parse_command(content: &str, path: &Path) -> Result<Command> {
    // Split frontmatter and body
    let parts: Vec<&str> = content.splitn(3, "---").collect();

    if parts.len() < 3 {
        anyhow::bail!("Invalid command file format: missing frontmatter");
    }

    // Parse YAML frontmatter
    let frontmatter: CommandFrontmatter = serde_yml::from_str(parts[1].trim())
        .context("Failed to parse command frontmatter")?;

    // Extract body (after second ---)
    let body = parts[2].trim();

    // Create command
    let mut command = Command::new(
        frontmatter.name,
        frontmatter.description,
        body.to_string(),
    )
    .with_source_path(path.to_path_buf());

    // Add arguments
    for arg in frontmatter.args {
        command = command.with_arg(arg.name, arg.description, arg.required);
    }

    Ok(command)
}

/// Render command template with arguments
pub fn render_template(template: &str, args: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();

    // Simple template substitution: {{arg_name}}
    for (key, value) in args {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_command() {
        let content = r#"---
name: test
description: Test command
args:
  - name: input
    description: Input value
    required: true
---

Test content with {{input}}"#;

        let path = Path::new("test.md");
        let command = parse_command(content, path).unwrap();

        assert_eq!(command.name, "test");
        assert_eq!(command.description, "Test command");
        assert_eq!(command.args.len(), 1);
        assert_eq!(command.args[0].name, "input");
        assert!(command.args[0].required);
        assert!(command.content.contains("{{input}}"));
    }

    #[test]
    fn test_render_template() {
        let template = "Hello {{name}}, you are {{age}} years old";
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("age".to_string(), "30".to_string());

        let result = render_template(template, &args);
        assert_eq!(result, "Hello Alice, you are 30 years old");
    }

    #[test]
    fn test_render_template_missing_arg() {
        let template = "Hello {{name}}";
        let args = HashMap::new();

        let result = render_template(template, &args);
        // Missing args are left as-is
        assert_eq!(result, "Hello {{name}}");
    }
}
