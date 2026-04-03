//! Interactive UI mode system prompts for the brainwires CLI.
//!
//! Each prompt function corresponds to one of the CLI's interactive modes
//! (Edit, Ask, Plan, Batch) or a specialised sub-agent spawned by a tool.
//! This is the single location to find and edit every mode-specific prompt.

use crate::types::WorkingSet;
use anyhow::Result;
use brainwires::brain::bks_pks::matcher::{MatchedTruth, format_truths_for_prompt};

// ── Edit mode (full read/write access) ─────────────────────────────────────

/// Build the default Edit-mode system prompt.
///
/// Instructs the AI to use local tools for understanding the current project
/// and enforces mandatory file-operation rules (write_file, edit_file).
pub fn build_system_prompt(custom: Option<String>) -> Result<String> {
    build_system_prompt_with_context(custom, None)
}

/// Build the Edit-mode system prompt with optional working-set injection.
///
/// When a non-empty [`WorkingSet`] is provided the open files are injected
/// into the prompt so the AI has immediate context without an extra round-trip.
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

    if let Some(ws) = working_set
        && let Some(context_injection) = ws.build_context_injection()
    {
        return Ok(format!("{}\n\n{}", base_prompt, context_injection));
    }

    Ok(base_prompt)
}

/// Build the Edit-mode system prompt extended with learned behavioral knowledge.
pub fn build_system_prompt_with_knowledge(
    custom: Option<String>,
    working_set: Option<&WorkingSet>,
    matched_truths: &[MatchedTruth],
) -> Result<String> {
    let base_prompt = build_system_prompt_with_context(custom, working_set)?;
    if !matched_truths.is_empty() {
        let knowledge_section = format_truths_for_prompt(matched_truths);
        Ok(format!("{}\n{}", base_prompt, knowledge_section))
    } else {
        Ok(base_prompt)
    }
}

// ── Ask mode (read-only) ────────────────────────────────────────────────────

/// Build the Ask-mode system prompt.
///
/// Restricts the AI to read-only operations: explaining, analysing, and
/// answering questions without modifying any files.
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

    if let Some(ws) = working_set
        && let Some(context_injection) = ws.build_context_injection()
    {
        return Ok(format!("{}\n\n{}", base_prompt, context_injection));
    }

    Ok(base_prompt)
}

/// Build the Ask-mode system prompt extended with learned behavioral knowledge.
pub fn build_ask_mode_system_prompt_with_knowledge(
    working_set: Option<&WorkingSet>,
    matched_truths: &[MatchedTruth],
) -> Result<String> {
    let base_prompt = build_ask_mode_system_prompt(working_set)?;
    if !matched_truths.is_empty() {
        let knowledge_section = format_truths_for_prompt(matched_truths);
        Ok(format!("{}\n{}", base_prompt, knowledge_section))
    } else {
        Ok(base_prompt)
    }
}

// ── Batch mode ──────────────────────────────────────────────────────────────

/// Build the Batch-mode system prompt.
///
/// Batch mode processes multiple independent inputs in sequence and is optimised
/// for throughput over interactivity. The prompt emphasises concise, consistent
/// output rather than exploratory dialogue.
pub fn build_batch_mode_system_prompt(
    custom: Option<String>,
    working_set: Option<&WorkingSet>,
) -> Result<String> {
    if let Some(custom_msg) = custom {
        return Ok(custom_msg);
    }

    let cwd = std::env::current_dir()?.display().to_string();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let base_prompt = format!(
        r#"You are a coding agent processing a batch of inputs.
Current date: {}
Current working directory: {}

## BATCH MODE
You are processing one item in a sequence. Optimise for:
- Concise, structured output that is easy to parse
- Consistent format across all batch items
- Self-contained responses — no references to previous items
- Efficient tool use (prefer multi-step scripts over sequential calls)

## MANDATORY RULE - FILE OPERATIONS
When asked to CREATE, WRITE, or GENERATE a file you MUST call `write_file` — never
output file content as text. When asked to EDIT, use `edit_file`.

## Available Tools (via search_tools or in scripts):
- File ops: read_file, write_file, edit_file, list_directory, create_directory, delete_file
- Search: search_files, search_code, query_codebase, index_codebase
- Git: git_status, git_diff, git_log, git_show
- Shell: execute_command (safe commands only)

Complete each batch item fully before reporting done."#,
        today, cwd
    );

    if let Some(ws) = working_set
        && let Some(context_injection) = ws.build_context_injection()
    {
        return Ok(format!("{}\n\n{}", base_prompt, context_injection));
    }

    Ok(base_prompt)
}

// ── Plan mode ───────────────────────────────────────────────────────────────

/// Build the Plan-mode system prompt.
///
/// `focus` is the user-specified planning topic; defaults to `"the task at hand"`.
/// Plan mode is read-only and isolated from the main conversation context.
pub fn build_plan_mode_system_prompt(focus: &str) -> String {
    format!(
        r#"You are in PLAN MODE - an isolated planning context.

## Your Role
You are a planning assistant focused on: {}

## Guidelines
1. **Research & Explore**: Use read-only tools to understand the codebase and gather information.
2. **No Modifications**: Do NOT create, edit, or delete files. Only read and search.
3. **Think Through**: Consider multiple approaches and their trade-offs.
4. **Document Your Plan**: Create a clear, actionable plan with:
   - Summary of what needs to be done
   - Key files that will be affected
   - Step-by-step implementation approach
   - Potential risks or considerations

## Available Actions
- Read files to understand existing code
- Search for patterns and implementations
- Ask clarifying questions
- Propose implementation approaches

## Output Format
When you have a plan ready, format it clearly with headers and bullet points.
The plan should be concrete enough that it can be directly executed.

Remember: This is a PLANNING context. Your research and exploration here is isolated
from the main conversation. Only the final plan will be shared with the main context."#,
        focus
    )
}

// ── Planning sub-agent (plan_task tool) ────────────────────────────────────

/// System prompt for the planning sub-agent spawned by the `plan_task` tool.
///
/// Unlike the interactive Plan mode this agent runs autonomously in a
/// `TaskAgent` context. It is strictly read-only and outputs a structured plan.
pub fn planning_agent_system_prompt(working_dir: &str) -> String {
    format!(
        r#"You are a planning agent. Your task is to create a detailed execution plan.

Working Directory: {}

Your role:
1. Research the codebase using available read-only tools (read_file, list_directory, search_code, query_codebase)
2. Understand the existing architecture and patterns
3. Create a comprehensive, step-by-step execution plan

Your plan should include:
- Clear, numbered steps
- Dependencies between steps
- Files that need to be modified or created
- Potential risks or challenges
- Testing considerations

Use your tools to explore the codebase before creating the plan. Be thorough in your research.

When you have gathered enough information, provide your final plan in a clear, structured format.
Do NOT execute any changes - only create the plan."#,
        working_dir
    )
}
