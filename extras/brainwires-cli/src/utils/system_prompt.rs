//! System Prompt Builder
//!
//! Shared utility for building system prompts with project context awareness.
//! Uses Universal Programmatic Tool Calling (Rhai scripts) as the primary interface.
//! Supports injection of learned behavioral knowledge from the BKS.

use crate::types::WorkingSet;
use anyhow::Result;
use brainwires::brain::bks_pks::matcher::{MatchedTruth, format_truths_for_prompt};

/// Build the default system prompt with current working directory context.
///
/// This prompt instructs the AI to use local tools for understanding the current project
/// rather than searching the web or fetching remote URLs.
pub fn build_system_prompt(custom: Option<String>) -> Result<String> {
    build_system_prompt_with_context(custom, None)
}

/// Build system prompt with optional working set context.
///
/// If a WorkingSet is provided and non-empty, file contents will be injected
/// into the system prompt so the AI has immediate access to those files.
pub fn build_system_prompt_with_context(
    custom: Option<String>,
    working_set: Option<&WorkingSet>,
) -> Result<String> {
    if let Some(custom_msg) = custom {
        return Ok(custom_msg);
    }

    let cwd = std::env::current_dir()?.display().to_string();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let base_prompt = format!(
        r#"You are a coding agent with access to powerful tools for exploring and understanding code projects.
Current date: {}
Current working directory: {}

## MANDATORY RULE - FILE OPERATIONS
When the user asks you to CREATE, WRITE, MAKE, or GENERATE a file:
1. You MUST call the `write_file` tool with the file path and content
2. You must NOT output the file content as text in your response
3. After calling write_file, confirm the file was created

Example - if user says "create index.html":
WRONG: Outputting the HTML code in your response
CORRECT: Calling write_file("index.html", "<html>...</html>")

## Tool Usage - Programmatic Tool Calling

Your PRIMARY tool is `execute_script` - write Rhai scripts to orchestrate multiple tool calls efficiently.
Benefits: 37% token reduction, loops/conditionals, batch operations, only final result enters context.

Use `search_tools` to discover available tools, then call them from your Rhai scripts.

### Example - Project Overview:
```rhai
let files = list_directory(".");
let readme = read_file("README.md");
let has_cargo = files.contains("Cargo.toml");
let config = if has_cargo {{ read_file("Cargo.toml") }} else {{ "No config" }};
`Files: ${{files}}\nREADME: ${{readme}}\nConfig: ${{config}}`
```

### Available Tools (via search_tools or in scripts):
- File ops: read_file, write_file, edit_file, list_directory, create_directory, delete_file
- Search: search_files, search_code, query_codebase, index_codebase
- Git: git_status, git_diff, git_log, git_show
- Shell: execute_command (safe commands only in scripts)

### Guidelines:
- For 'this project' questions: use LOCAL tools only, never web/fetch_url
- For multi-step operations: prefer execute_script over sequential individual calls
- For simple single operations: individual tool calls are fine
- Be proactive - use tools without asking permission first
- IMPORTANT: When asked to CREATE or WRITE files, you MUST use write_file tool - NEVER just output the content as text
- When asked to EDIT files, use edit_file tool - don't just show the changes
- Always execute the actual file operations, don't just describe what you would do"#,
        today, cwd
    );

    // Inject working set file contents if available
    if let Some(ws) = working_set
        && let Some(context_injection) = ws.build_context_injection()
    {
        return Ok(format!("{}\n\n{}", base_prompt, context_injection));
    }

    Ok(base_prompt)
}

/// Build a read-only system prompt for Ask mode.
///
/// This prompt restricts the AI to read-only operations: explaining, analyzing,
/// and answering questions without modifying any files.
pub fn build_ask_mode_system_prompt(working_set: Option<&WorkingSet>) -> Result<String> {
    let cwd = std::env::current_dir()?.display().to_string();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let base_prompt = format!(
        r#"You are a coding assistant in READ-ONLY mode. You can explore and explain code but MUST NOT modify any files.
Current date: {}
Current working directory: {}

## READ-ONLY MODE
You are in Ask mode. Your role is to:
- Explain code, architecture, and design decisions
- Answer questions about the codebase
- Analyze code for bugs, performance issues, or improvements
- Describe how features work

You MUST NOT:
- Create, write, edit, or delete any files
- Execute shell commands that modify state
- Make git commits, pushes, or other write operations

## Available Tools (read-only)
- read_file: Read file contents
- list_directory: List directory contents
- search_files: Search for files by name/pattern
- search_code: Search code content
- query_codebase: Semantic code search
- git_status: Show git status
- git_diff: Show git diffs
- git_log: Show git history
- git_show: Show git commit details
- execute_script: Rhai scripts using ONLY read-only tools above

## Guidelines
- For 'this project' questions: use LOCAL tools only, never web/fetch_url
- Be thorough in your explanations
- Reference specific files and line numbers when relevant
- Use execute_script for multi-step read operations"#,
        today, cwd
    );

    // Inject working set file contents if available
    if let Some(ws) = working_set
        && let Some(context_injection) = ws.build_context_injection()
    {
        return Ok(format!("{}\n\n{}", base_prompt, context_injection));
    }

    Ok(base_prompt)
}

/// Build system prompt with behavioral knowledge injection
///
/// This function extends the base system prompt with learned behavioral truths
/// from the collective knowledge system.
pub fn build_system_prompt_with_knowledge(
    custom: Option<String>,
    working_set: Option<&WorkingSet>,
    matched_truths: &[MatchedTruth],
) -> Result<String> {
    let base_prompt = build_system_prompt_with_context(custom, working_set)?;

    // If we have matched truths, inject them
    if !matched_truths.is_empty() {
        let knowledge_section = format_truths_for_prompt(matched_truths);
        Ok(format!("{}\n{}", base_prompt, knowledge_section))
    } else {
        Ok(base_prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_prompt_returned() {
        let custom = Some("Custom system prompt".to_string());
        let result = build_system_prompt(custom).unwrap();
        assert_eq!(result, "Custom system prompt");
    }

    #[test]
    fn test_default_prompt_includes_cwd() {
        let result = build_system_prompt(None).unwrap();
        assert!(result.contains("Current working directory:"));
        assert!(result.contains("execute_script"));
    }
}
