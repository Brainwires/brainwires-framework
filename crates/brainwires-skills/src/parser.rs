//! SKILL.md Parser
//!
//! Parses skill files from .brainwires/skills/ directories.
//! Follows the same pattern as commands/parser.rs.
//!
//! # Format
//!
//! ```markdown
//! ---
//! name: skill-name
//! description: What the skill does and when to use it
//! allowed-tools:
//!   - Read
//!   - Grep
//! license: Apache-2.0
//! model: claude-sonnet-4
//! metadata:
//!   category: development
//!   execution: inline
//! ---
//!
//! # Skill Instructions
//!
//! Step-by-step guidance for the agent...
//! ```

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use super::metadata::{Skill, SkillMetadata, SkillSource};
#[cfg(test)]
use super::metadata::SkillExecutionMode;

/// YAML frontmatter for skill definition
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    /// Skill name (lowercase, hyphens, max 64 chars)
    name: String,
    /// Description (max 1024 chars, used for semantic matching)
    description: String,
    /// Optional: Restrict available tools
    /// Accepts both a YAML list and a space-delimited string per the Agent Skills spec.
    #[serde(rename = "allowed-tools", default, deserialize_with = "deserialize_allowed_tools")]
    allowed_tools: Option<Vec<String>>,
    /// Optional: Software license
    license: Option<String>,
    /// Optional: Environment requirements (max 500 chars)
    compatibility: Option<String>,
    /// Optional: Specific model to use (Brainwires extension)
    model: Option<String>,
    /// Optional: Custom key-value pairs
    metadata: Option<HashMap<String, String>>,
    /// Optional: lifecycle hook event types (Brainwires extension)
    #[serde(default)]
    hooks: Option<Vec<String>>,
}

/// Deserialize `allowed-tools` from either a YAML list or a space-delimited string.
///
/// The Agent Skills specification defines allowed-tools as a space-delimited string
/// (e.g., `allowed-tools: Bash(git:*) Read`), but we also accept YAML lists for
/// convenience (e.g., `allowed-tools:\n  - Read\n  - Grep`).
fn deserialize_allowed_tools<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct AllowedToolsVisitor;

    impl<'de> de::Visitor<'de> for AllowedToolsVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a list of strings or a space-delimited string")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.split_whitespace().map(|s| s.to_string()).collect()))
            }
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut tools = Vec::new();
            while let Some(tool) = seq.next_element::<String>()? {
                tools.push(tool);
            }
            if tools.is_empty() {
                Ok(None)
            } else {
                Ok(Some(tools))
            }
        }
    }

    deserializer.deserialize_any(AllowedToolsVisitor)
}

/// Parse only the skill metadata (frontmatter) from a SKILL.md file
///
/// This is used for progressive disclosure - only loading metadata at startup.
/// The full content is loaded lazily when the skill is activated.
pub fn parse_skill_metadata(path: &Path) -> Result<SkillMetadata> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read skill file: {}", path.display()))?;

    parse_metadata_from_content(&content, path)
}

/// Parse metadata from content string
fn parse_metadata_from_content(content: &str, path: &Path) -> Result<SkillMetadata> {
    // Split frontmatter and body
    let parts: Vec<&str> = content.splitn(3, "---").collect();

    if parts.len() < 3 {
        anyhow::bail!(
            "Invalid SKILL.md format in {}: missing frontmatter (requires --- delimiters)",
            path.display()
        );
    }

    // Parse YAML frontmatter
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(parts[1].trim())
        .with_context(|| format!("Failed to parse skill frontmatter in {}", path.display()))?;

    // Validate constraints
    validate_skill_name(&frontmatter.name)
        .with_context(|| format!("Invalid skill name in {}", path.display()))?;
    validate_description(&frontmatter.description)
        .with_context(|| format!("Invalid skill description in {}", path.display()))?;
    if let Some(ref compat) = frontmatter.compatibility {
        validate_compatibility(compat)
            .with_context(|| format!("Invalid compatibility in {}", path.display()))?;
    }

    // Warn if skill name doesn't match parent directory name (spec recommendation)
    warn_name_directory_mismatch(&frontmatter.name, path);

    Ok(SkillMetadata {
        name: frontmatter.name,
        description: frontmatter.description,
        allowed_tools: frontmatter.allowed_tools,
        license: frontmatter.license,
        compatibility: frontmatter.compatibility,
        model: frontmatter.model,
        metadata: frontmatter.metadata,
        hooks: frontmatter.hooks,
        source: SkillSource::Personal, // Will be set by caller
        source_path: path.to_path_buf(),
    })
}

/// Parse a complete skill file (metadata + instructions)
///
/// Used when a skill is activated and full content is needed.
pub fn parse_skill_file(path: &Path) -> Result<Skill> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read skill file: {}", path.display()))?;

    parse_skill_from_content(&content, path)
}

/// Parse skill from content string
fn parse_skill_from_content(content: &str, path: &Path) -> Result<Skill> {
    // Split frontmatter and body
    let parts: Vec<&str> = content.splitn(3, "---").collect();

    if parts.len() < 3 {
        anyhow::bail!(
            "Invalid SKILL.md format in {}: missing frontmatter",
            path.display()
        );
    }

    // Parse metadata
    let metadata = parse_metadata_from_content(content, path)?;

    // Extract body (after second ---)
    let instructions = parts[2].trim().to_string();

    // Determine execution mode from metadata
    let execution_mode = metadata.execution_mode();

    Ok(Skill {
        metadata,
        instructions,
        execution_mode,
    })
}

/// Validate skill name constraints per the Agent Skills specification.
///
/// - Must be 1-64 characters
/// - Only lowercase letters, digits, and hyphens allowed
/// - Cannot start or end with hyphen
/// - Cannot contain consecutive hyphens (`--`)
fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Skill name cannot be empty");
    }

    if name.len() > 64 {
        anyhow::bail!(
            "Skill name exceeds 64 characters (got {}): '{}'",
            name.len(),
            name
        );
    }

    if name.starts_with('-') || name.ends_with('-') {
        anyhow::bail!("Skill name cannot start or end with a hyphen: '{}'", name);
    }

    if name.contains("--") {
        anyhow::bail!(
            "Skill name cannot contain consecutive hyphens: '{}'",
            name
        );
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            anyhow::bail!(
                "Skill name must be lowercase with hyphens only, found '{}' in '{}'",
                c,
                name
            );
        }
    }

    Ok(())
}

/// Validate description constraints
///
/// - Must not be empty
/// - Max 1024 characters
fn validate_description(desc: &str) -> Result<()> {
    if desc.trim().is_empty() {
        anyhow::bail!("Skill description cannot be empty");
    }

    if desc.len() > 1024 {
        anyhow::bail!(
            "Skill description exceeds 1024 characters (got {})",
            desc.len()
        );
    }

    Ok(())
}

/// Validate compatibility field constraints per the Agent Skills specification.
///
/// - Must be 1-500 characters if provided
fn validate_compatibility(compat: &str) -> Result<()> {
    if compat.trim().is_empty() {
        anyhow::bail!("Compatibility field cannot be empty when provided");
    }

    if compat.len() > 500 {
        anyhow::bail!(
            "Compatibility field exceeds 500 characters (got {})",
            compat.len()
        );
    }

    Ok(())
}

/// Warn if the skill name doesn't match the parent directory name.
///
/// The Agent Skills specification requires that the name field must match the parent
/// directory name. We emit a warning rather than an error since brainwires-skills also
/// supports flat file layout (`skills/review-pr.md`) which the spec doesn't define.
fn warn_name_directory_mismatch(name: &str, path: &Path) {
    // Only check for subdirectory layout (skill-name/SKILL.md)
    if path.file_name().map(|f| f == "SKILL.md").unwrap_or(false)
        && let Some(parent) = path.parent()
            && let Some(dir_name) = parent.file_name().and_then(|n| n.to_str())
                && dir_name != name {
                    tracing::warn!(
                        "Skill name '{}' does not match parent directory '{}' in {}. \
                         The Agent Skills spec requires these to match.",
                        name,
                        dir_name,
                        path.display()
                    );
                }
}

/// Render skill template with arguments
///
/// Replaces `{{arg_name}}` placeholders with provided values.
/// Supports Handlebars-style conditionals: `{{#if var}}...{{/if}}`
pub fn render_template(template: &str, args: &HashMap<String, String>) -> String {
    let mut result = template.to_string();

    // Simple template substitution: {{arg_name}}
    for (key, value) in args {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    // Handle simple conditionals: {{#if var}}content{{/if}}
    // This is a simplified version - full Handlebars would need a proper parser
    for (key, value) in args {
        let if_block = format!("{{{{#if {}}}}}", key);
        let endif = "{{/if}}";

        while let Some(start) = result.find(&if_block) {
            if let Some(end_offset) = result[start..].find(endif) {
                let end = start + end_offset + endif.len();
                let block_content = &result[start + if_block.len()..start + end_offset];

                // If value is non-empty/truthy, keep the content; otherwise remove block
                let replacement = if !value.is_empty() && value != "false" && value != "0" {
                    block_content.to_string()
                } else {
                    String::new()
                };

                result = format!("{}{}{}", &result[..start], replacement, &result[end..]);
            } else {
                break; // Malformed template, stop processing
            }
        }
    }

    // Remove any remaining if blocks for unset variables
    let if_pattern = regex::Regex::new(r"\{\{#if \w+\}\}.*?\{\{/if\}\}").ok();
    if let Some(re) = if_pattern {
        result = re.replace_all(&result, "").to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_skill_name_valid() {
        assert!(validate_skill_name("review-pr").is_ok());
        assert!(validate_skill_name("commit").is_ok());
        assert!(validate_skill_name("explain-code-123").is_ok());
        assert!(validate_skill_name("a").is_ok());
        assert!(validate_skill_name("a-b-c").is_ok());
    }

    #[test]
    fn test_validate_skill_name_invalid() {
        // Empty
        assert!(validate_skill_name("").is_err());

        // Too long
        let long_name = "a".repeat(65);
        assert!(validate_skill_name(&long_name).is_err());

        // Invalid characters
        assert!(validate_skill_name("Review-PR").is_err()); // uppercase
        assert!(validate_skill_name("review_pr").is_err()); // underscore
        assert!(validate_skill_name("review pr").is_err()); // space
        assert!(validate_skill_name("review.pr").is_err()); // dot

        // Hyphen at start/end
        assert!(validate_skill_name("-review").is_err());
        assert!(validate_skill_name("review-").is_err());

        // Consecutive hyphens (per Agent Skills spec)
        assert!(validate_skill_name("review--pr").is_err());
        assert!(validate_skill_name("a--b--c").is_err());
    }

    #[test]
    fn test_validate_description_valid() {
        assert!(validate_description("A short description").is_ok());
        assert!(validate_description(&"a".repeat(1024)).is_ok());
    }

    #[test]
    fn test_validate_description_invalid() {
        // Empty
        assert!(validate_description("").is_err());
        assert!(validate_description("   ").is_err());

        // Too long
        assert!(validate_description(&"a".repeat(1025)).is_err());
    }

    #[test]
    fn test_parse_skill_metadata() {
        let content = r#"---
name: test-skill
description: A test skill for testing
allowed-tools:
  - Read
  - Grep
license: MIT
model: claude-sonnet-4
metadata:
  category: testing
  execution: inline
---

# Test Skill Instructions

Do the test thing."#;

        let path = Path::new("test.md");
        let metadata = parse_metadata_from_content(content, path).unwrap();

        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.description, "A test skill for testing");
        assert_eq!(
            metadata.allowed_tools,
            Some(vec!["Read".to_string(), "Grep".to_string()])
        );
        assert_eq!(metadata.license, Some("MIT".to_string()));
        assert_eq!(metadata.model, Some("claude-sonnet-4".to_string()));
        assert_eq!(
            metadata.metadata.as_ref().unwrap().get("category"),
            Some(&"testing".to_string())
        );
        assert_eq!(metadata.execution_mode(), SkillExecutionMode::Inline);
    }

    #[test]
    fn test_parse_skill_full() {
        let content = r#"---
name: review-pr
description: Reviews pull requests for quality
metadata:
  execution: subagent
---

# PR Review

When reviewing:
1. Check code quality
2. Look for bugs"#;

        let path = Path::new("review-pr.md");
        let skill = parse_skill_from_content(content, path).unwrap();

        assert_eq!(skill.metadata.name, "review-pr");
        assert_eq!(skill.execution_mode, SkillExecutionMode::Subagent);
        assert!(skill.instructions.contains("PR Review"));
        assert!(skill.instructions.contains("Check code quality"));
    }

    #[test]
    fn test_parse_skill_minimal() {
        let content = r#"---
name: simple
description: A simple skill
---

Just do the thing."#;

        let path = Path::new("simple.md");
        let skill = parse_skill_from_content(content, path).unwrap();

        assert_eq!(skill.metadata.name, "simple");
        assert!(skill.metadata.allowed_tools.is_none());
        assert!(skill.metadata.license.is_none());
        assert!(skill.metadata.model.is_none());
        assert_eq!(skill.execution_mode, SkillExecutionMode::Inline);
        assert_eq!(skill.instructions, "Just do the thing.");
    }

    #[test]
    fn test_parse_skill_invalid_format() {
        let content = "No frontmatter here";
        let path = Path::new("invalid.md");
        assert!(parse_skill_from_content(content, path).is_err());
    }

    #[test]
    fn test_parse_skill_invalid_name() {
        let content = r#"---
name: Invalid_Name
description: A skill with invalid name
---

Instructions"#;

        let path = Path::new("invalid.md");
        assert!(parse_skill_from_content(content, path).is_err());
    }

    #[test]
    fn test_render_template_simple() {
        let template = "Hello {{name}}, you are working on {{task}}";
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Claude".to_string());
        args.insert("task".to_string(), "code review".to_string());

        let result = render_template(template, &args);
        assert_eq!(result, "Hello Claude, you are working on code review");
    }

    #[test]
    fn test_render_template_missing_arg() {
        let template = "Hello {{name}}";
        let args = HashMap::new();

        let result = render_template(template, &args);
        // Missing args are left as-is
        assert_eq!(result, "Hello {{name}}");
    }

    #[test]
    fn test_render_template_conditional() {
        let template = "Review{{#if pr_number}} PR #{{pr_number}}{{/if}} now";

        // With value
        let mut args = HashMap::new();
        args.insert("pr_number".to_string(), "123".to_string());
        let result = render_template(template, &args);
        assert_eq!(result, "Review PR #123 now");

        // Without value (empty)
        let mut args2 = HashMap::new();
        args2.insert("pr_number".to_string(), "".to_string());
        let result2 = render_template(template, &args2);
        assert_eq!(result2, "Review now");
    }

    #[test]
    fn test_render_template_multiline_description() {
        let content = r#"---
name: test
description: |
  A multiline description
  that spans multiple lines
  for better readability.
---

Instructions"#;

        let path = Path::new("test.md");
        let metadata = parse_metadata_from_content(content, path).unwrap();

        assert!(metadata.description.contains("multiline description"));
        assert!(metadata.description.contains("spans multiple lines"));
    }

    #[test]
    fn test_validate_compatibility() {
        // Valid
        assert!(validate_compatibility("Requires git and docker").is_ok());
        assert!(validate_compatibility(&"a".repeat(500)).is_ok());

        // Invalid: empty
        assert!(validate_compatibility("").is_err());
        assert!(validate_compatibility("   ").is_err());

        // Invalid: too long
        assert!(validate_compatibility(&"a".repeat(501)).is_err());
    }

    #[test]
    fn test_parse_skill_with_compatibility() {
        let content = r#"---
name: deploy
description: Deploys the application to production
compatibility: Requires docker, kubectl, and access to the internet
license: MIT
---

# Deploy Instructions

Run the deploy script."#;

        let path = Path::new("deploy.md");
        let metadata = parse_metadata_from_content(content, path).unwrap();

        assert_eq!(metadata.name, "deploy");
        assert_eq!(
            metadata.compatibility,
            Some("Requires docker, kubectl, and access to the internet".to_string())
        );
    }

    #[test]
    fn test_parse_allowed_tools_space_delimited() {
        let content = r#"---
name: git-helper
description: Helps with git operations
allowed-tools: Bash(git:*) Bash(jq:*) Read
---

# Git Helper

Help with git."#;

        let path = Path::new("git-helper.md");
        let metadata = parse_metadata_from_content(content, path).unwrap();

        assert_eq!(
            metadata.allowed_tools,
            Some(vec![
                "Bash(git:*)".to_string(),
                "Bash(jq:*)".to_string(),
                "Read".to_string(),
            ])
        );
    }

    #[test]
    fn test_parse_allowed_tools_yaml_list() {
        let content = r#"---
name: reviewer
description: Reviews code
allowed-tools:
  - Read
  - Grep
---

# Reviewer

Review code."#;

        let path = Path::new("reviewer.md");
        let metadata = parse_metadata_from_content(content, path).unwrap();

        assert_eq!(
            metadata.allowed_tools,
            Some(vec!["Read".to_string(), "Grep".to_string()])
        );
    }

    #[test]
    fn test_consecutive_hyphens_rejected() {
        let content = r#"---
name: bad--name
description: A skill with consecutive hyphens
---

Instructions"#;

        let path = Path::new("bad.md");
        assert!(parse_metadata_from_content(content, path).is_err());
    }
}
