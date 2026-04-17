//! Built-in Slash Commands
//!
//! Defines default commands like /clear, /status, /model, /help

use super::registry::{Command, CommandRegistry};

/// Register all built-in commands
pub fn register_builtin_commands(registry: &mut CommandRegistry) {
    // /help - Show available commands
    registry.register(Command::builtin(
        "help".to_string(),
        "Show available slash commands".to_string(),
        "".to_string(), // Empty content, handled specially in executor
    ));

    // /clear - Clear conversation history
    registry.register(Command::builtin(
        "clear".to_string(),
        "Clear conversation history".to_string(),
        "".to_string(), // Empty content, handled specially in executor
    ));

    // /status - Show session status
    registry.register(Command::builtin(
        "status".to_string(),
        "Show current session status".to_string(),
        "".to_string(), // Empty content, handled specially in executor
    ));

    // /model - List or switch AI models
    registry.register(
        Command::builtin(
            "model".to_string(),
            "List available models or switch to a different model".to_string(),
            "".to_string(), // Empty content, handled specially in executor
        )
        .with_arg(
            "name".to_string(),
            Some("Model name to switch to (optional - omit to list models)".to_string()),
            false,
        ),
    );

    // /provider - List or switch AI providers
    registry.register(
        Command::builtin(
            "provider".to_string(),
            "List available providers or switch to a different provider".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some(
                "Provider name (e.g. anthropic, openai, ollama) — omit to list providers"
                    .to_string(),
            ),
            false,
        ),
    );

    // /rewind - Rewind to previous checkpoint
    registry.register(
        Command::builtin(
            "rewind".to_string(),
            "Rewind conversation to a previous checkpoint".to_string(),
            "".to_string(), // Empty content, handled specially in executor
        )
        .with_arg(
            "steps".to_string(),
            Some("Number of steps to rewind".to_string()),
            false,
        ),
    );

    // /review - Code review mode
    registry.register(Command::builtin(
        "review".to_string(),
        "Enter code review mode for current changes".to_string(),
        "Please review the following code changes and provide feedback on:\n\
             - Code quality and best practices\n\
             - Potential bugs or issues\n\
             - Performance considerations\n\
             - Security concerns\n\
             - Suggestions for improvement"
            .to_string(),
    ));

    // /commands - List all available commands (alias for /help)
    registry.register(Command::builtin(
        "commands".to_string(),
        "List all available commands".to_string(),
        "".to_string(),
    ));

    // /checkpoint - Create a named checkpoint
    registry.register(
        Command::builtin(
            "checkpoint".to_string(),
            "Create a named checkpoint of the current conversation".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Optional checkpoint name".to_string()),
            false,
        ),
    );

    // /restore - Restore from a checkpoint
    registry.register(
        Command::builtin(
            "restore".to_string(),
            "Restore conversation from a checkpoint".to_string(),
            "".to_string(),
        )
        .with_arg(
            "id".to_string(),
            Some("Checkpoint ID or index (1-based)".to_string()),
            false,
        ),
    );

    // /checkpoints - List all checkpoints
    registry.register(Command::builtin(
        "checkpoints".to_string(),
        "List all checkpoints for this conversation".to_string(),
        "".to_string(),
    ));

    // /resume - Load a conversation from history
    registry.register(
        Command::builtin(
            "resume".to_string(),
            "Show conversation picker to resume a previous conversation".to_string(),
            "".to_string(),
        )
        .with_arg(
            "conversation_id".to_string(),
            Some("Optional: specific conversation ID to load".to_string()),
            false,
        ),
    );

    // /exit - Exit the application
    registry.register(Command::builtin(
        "exit".to_string(),
        "Exit the application".to_string(),
        "".to_string(),
    ));

    // /plan - Create and persist a plan using the planning agent
    registry.register(
        Command::builtin(
            "plan".to_string(),
            "Create a plan using the planning agent (saved for later reference)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task".to_string(),
            Some("Description of the task to plan".to_string()),
            true,
        ),
    );

    // /plans - List saved plans
    registry.register(
        Command::builtin(
            "plans".to_string(),
            "List saved execution plans".to_string(),
            "".to_string(),
        )
        .with_arg(
            "conversation_id".to_string(),
            Some("Optional: filter by conversation ID".to_string()),
            false,
        ),
    );

    // /plan:show - Show a specific plan
    registry.register(
        Command::builtin(
            "plan:show".to_string(),
            "Display details of a saved plan".to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Plan ID to display".to_string()),
            true,
        ),
    );

    // /plan:delete - Delete a plan
    registry.register(
        Command::builtin(
            "plan:delete".to_string(),
            "Delete a saved plan".to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Plan ID to delete".to_string()),
            true,
        ),
    );

    // /plan:activate - Activate a plan
    registry.register(
        Command::builtin(
            "plan:activate".to_string(),
            "Set a plan as the active working plan (agent will follow it)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Plan ID to activate".to_string()),
            true,
        ),
    );

    // /plan:deactivate - Deactivate current plan
    registry.register(Command::builtin(
        "plan:deactivate".to_string(),
        "Clear the active plan".to_string(),
        "".to_string(),
    ));

    // /plan:current - Show current active plan
    registry.register(Command::builtin(
        "plan:current".to_string(),
        "Show the currently active plan and progress".to_string(),
        "".to_string(),
    ));

    // /plan:pause - Pause current plan
    registry.register(Command::builtin(
        "plan:pause".to_string(),
        "Pause the current plan (task state is preserved)".to_string(),
        "".to_string(),
    ));

    // /plan:resume - Resume a paused plan
    registry.register(
        Command::builtin(
            "plan:resume".to_string(),
            "Resume a paused plan with its saved task state".to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Plan ID to resume".to_string()),
            true,
        ),
    );

    // /plan:execute - Execute a plan automatically
    registry.register(
        Command::builtin(
            "plan:execute".to_string(),
            "Execute a plan with AI-driven task completion (modes: suggest, auto-edit, full-auto)"
                .to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Plan ID to execute".to_string()),
            true,
        )
        .with_arg(
            "mode".to_string(),
            Some(
                "Approval mode: suggest, auto-edit, or full-auto (default: full-auto)".to_string(),
            ),
            false,
        ),
    );

    // /plan:search - Search plans by text
    registry.register(
        Command::builtin(
            "plan:search".to_string(),
            "Search plans by title, description, or content".to_string(),
            "".to_string(),
        )
        .with_arg("query".to_string(), Some("Search query".to_string()), true),
    );

    // /plan:branch - Create a sub-plan branch from the active plan
    registry.register(
        Command::builtin(
            "plan:branch".to_string(),
            "Create a sub-plan branch from the current active plan".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Branch name (e.g., 'auth-feature')".to_string()),
            true,
        )
        .with_arg(
            "task".to_string(),
            Some("Task description for the branch".to_string()),
            true,
        ),
    );

    // /plan:merge - Mark a branch as merged back to parent
    registry.register(
        Command::builtin(
            "plan:merge".to_string(),
            "Mark a branch plan as merged (completes the branch)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Branch plan ID to merge (optional, defaults to active plan)".to_string()),
            false,
        ),
    );

    // /plan:tree - Show plan hierarchy
    registry.register(
        Command::builtin(
            "plan:tree".to_string(),
            "Show the plan hierarchy (parent and child plans)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "plan_id".to_string(),
            Some("Plan ID to show hierarchy for (optional, defaults to active plan)".to_string()),
            false,
        ),
    );

    // /tasks - Show tasks for current plan
    registry.register(Command::builtin(
        "tasks".to_string(),
        "Show task list for the current active plan".to_string(),
        "".to_string(),
    ));

    // /task:complete - Mark a task as complete
    registry.register(
        Command::builtin(
            "task:complete".to_string(),
            "Mark a task as complete (defaults to current in-progress task)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task_id".to_string(),
            Some("Task ID to complete (optional)".to_string()),
            false,
        ),
    );

    // /task:skip - Skip a task
    registry.register(
        Command::builtin(
            "task:skip".to_string(),
            "Skip a task (mark as skipped)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task_id".to_string(),
            Some("Task ID to skip (optional - defaults to current)".to_string()),
            false,
        )
        .with_arg(
            "reason".to_string(),
            Some("Reason for skipping".to_string()),
            false,
        ),
    );

    // /task:add - Add a new task
    registry.register(
        Command::builtin(
            "task:add".to_string(),
            "Add a new task to the current plan".to_string(),
            "".to_string(),
        )
        .with_arg(
            "description".to_string(),
            Some("Task description".to_string()),
            true,
        ),
    );

    // /task:start - Start a specific task
    registry.register(
        Command::builtin(
            "task:start".to_string(),
            "Start working on a specific task".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task_id".to_string(),
            Some("Task ID to start".to_string()),
            true,
        ),
    );

    // /task:block - Block a task
    registry.register(
        Command::builtin(
            "task:block".to_string(),
            "Mark a task as blocked".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task_id".to_string(),
            Some("Task ID to block".to_string()),
            true,
        )
        .with_arg(
            "reason".to_string(),
            Some("Reason for blocking".to_string()),
            false,
        ),
    );

    // /task:depends - Add a dependency between tasks
    registry.register(
        Command::builtin(
            "task:depends".to_string(),
            "Make a task depend on another task".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task_id".to_string(),
            Some("Task that will depend on another".to_string()),
            true,
        )
        .with_arg(
            "depends_on".to_string(),
            Some("Task to depend on".to_string()),
            true,
        ),
    );

    // /task:ready - Show tasks ready to execute
    registry.register(Command::builtin(
        "task:ready".to_string(),
        "Show tasks that are ready to execute (all dependencies complete)".to_string(),
        "".to_string(),
    ));

    // /task:time - Show time info for tasks
    registry.register(
        Command::builtin(
            "task:time".to_string(),
            "Show time tracking info for a task or all tasks".to_string(),
            "".to_string(),
        )
        .with_arg(
            "task_id".to_string(),
            Some("Task ID (optional - shows all if omitted)".to_string()),
            false,
        ),
    );

    // /task:list - Show enhanced task list with IDs
    registry.register(Command::builtin(
        "task:list".to_string(),
        "Show enhanced task list with IDs and dependencies".to_string(),
        "".to_string(),
    ));

    // /approvals - Set approval mode for AI actions
    registry.register(
        Command::builtin(
            "approvals".to_string(),
            "Set approval mode: suggest (review all), auto-edit (auto file edits), or full-auto (auto everything)".to_string(),
            "".to_string(),
        )
        .with_arg("mode".to_string(), Some("Approval mode: suggest, auto-edit, or full-auto".to_string()), false)
    );

    // /brainwires - Load project instructions from BRAINWIRES.md
    registry.register(Command::builtin(
        "brainwires".to_string(),
        "Load project-specific instructions from BRAINWIRES.md (supports @file.md imports)"
            .to_string(),
        "".to_string(),
    ));

    // /exec - Execute a shell command with confirmation
    registry.register(
        Command::builtin(
            "exec".to_string(),
            "Execute a shell command (requires user confirmation for safety)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "command".to_string(),
            Some("Shell command to execute".to_string()),
            true,
        ),
    );

    // /shells - View shell command history
    registry.register(Command::builtin(
        "shells".to_string(),
        "View history of executed shell commands".to_string(),
        "".to_string(),
    ));

    // /shell - Drop into an interactive shell with the terminal handed over
    registry.register(Command::builtin(
        "shell".to_string(),
        "Drop into an interactive shell (exit or Ctrl+D to return)".to_string(),
        "".to_string(),
    ));

    // /hotkeys - Open hotkey configuration dialog
    registry.register(Command::builtin(
        "hotkeys".to_string(),
        "View and configure keyboard shortcuts".to_string(),
        "".to_string(),
    ));

    // /keys - Alias for /hotkeys
    registry.register(Command::builtin(
        "keys".to_string(),
        "View and configure keyboard shortcuts (alias for /hotkeys)".to_string(),
        "".to_string(),
    ));

    // Project RAG MCP Tools
    // /project:index - Index codebase for semantic search
    registry.register(
        Command::builtin(
            "project:index".to_string(),
            "Index a codebase directory for semantic search using project RAG".to_string(),
            "".to_string(),
        )
        .with_arg(
            "path".to_string(),
            Some("Path to codebase directory".to_string()),
            false,
        ),
    );

    // /project:query - Query indexed codebase
    registry.register(
        Command::builtin(
            "project:query".to_string(),
            "Search indexed codebase using semantic search".to_string(),
            "".to_string(),
        )
        .with_arg("query".to_string(), Some("Search query".to_string()), true),
    );

    // /project:stats - Get index statistics
    registry.register(Command::builtin(
        "project:stats".to_string(),
        "Show statistics about the indexed codebase".to_string(),
        "".to_string(),
    ));

    // /project:search - Advanced search with filters
    registry.register(
        Command::builtin(
            "project:search".to_string(),
            "Advanced semantic search with file type and language filters".to_string(),
            "".to_string(),
        )
        .with_arg("query".to_string(), Some("Search query".to_string()), true)
        .with_arg(
            "extensions".to_string(),
            Some("Comma-separated file extensions (e.g., rs,toml)".to_string()),
            false,
        )
        .with_arg(
            "languages".to_string(),
            Some("Comma-separated languages (e.g., Rust,Python)".to_string()),
            false,
        ),
    );

    // /project:clear - Clear index
    registry.register(Command::builtin(
        "project:clear".to_string(),
        "Clear all indexed data from the vector database".to_string(),
        "".to_string(),
    ));

    // /project:git-search - Search git history
    registry.register(
        Command::builtin(
            "project:git-search".to_string(),
            "Search git commit history using semantic search".to_string(),
            "".to_string(),
        )
        .with_arg(
            "query".to_string(),
            Some("Search query for commit messages".to_string()),
            true,
        )
        .with_arg(
            "max_commits".to_string(),
            Some("Maximum commits to search (default: 10)".to_string()),
            false,
        ),
    );

    // /project:definition - Find symbol definition (LSP-like)
    registry.register(
        Command::builtin(
            "project:definition".to_string(),
            "Find where a symbol is defined (LSP-like go-to-definition)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "file".to_string(),
            Some("File path containing the symbol".to_string()),
            true,
        )
        .with_arg(
            "line".to_string(),
            Some("Line number (1-based)".to_string()),
            true,
        )
        .with_arg(
            "column".to_string(),
            Some("Column number (0-based)".to_string()),
            true,
        ),
    );

    // /project:references - Find all references to a symbol
    registry.register(
        Command::builtin(
            "project:references".to_string(),
            "Find all references to a symbol at a given location".to_string(),
            "".to_string(),
        )
        .with_arg(
            "file".to_string(),
            Some("File path containing the symbol".to_string()),
            true,
        )
        .with_arg(
            "line".to_string(),
            Some("Line number (1-based)".to_string()),
            true,
        )
        .with_arg(
            "column".to_string(),
            Some("Column number (0-based)".to_string()),
            true,
        )
        .with_arg(
            "limit".to_string(),
            Some("Maximum references to return (default: 100)".to_string()),
            false,
        ),
    );

    // /project:callgraph - Get call graph for a function
    registry.register(
        Command::builtin(
            "project:callgraph".to_string(),
            "Get call graph for a function (callers and callees)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "file".to_string(),
            Some("File path containing the function".to_string()),
            true,
        )
        .with_arg(
            "line".to_string(),
            Some("Line number (1-based)".to_string()),
            true,
        )
        .with_arg(
            "column".to_string(),
            Some("Column number (0-based)".to_string()),
            true,
        )
        .with_arg(
            "depth".to_string(),
            Some("Maximum traversal depth (default: 2)".to_string()),
            false,
        ),
    );

    // Template Commands

    // /templates - List all templates
    registry.register(Command::builtin(
        "templates".to_string(),
        "List all saved plan templates".to_string(),
        "".to_string(),
    ));

    // /template:save - Save current plan as template
    registry.register(
        Command::builtin(
            "template:save".to_string(),
            "Save the current active plan as a reusable template".to_string(),
            "".to_string(),
        )
        .with_arg("name".to_string(), Some("Template name".to_string()), true)
        .with_arg(
            "description".to_string(),
            Some("Template description".to_string()),
            false,
        ),
    );

    // /template:show - Show a template
    registry.register(
        Command::builtin(
            "template:show".to_string(),
            "Display a template's content and variables".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Template name or ID".to_string()),
            true,
        ),
    );

    // /template:use - Instantiate a template
    registry.register(
        Command::builtin(
            "template:use".to_string(),
            "Create a new plan from a template with variable substitutions".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Template name or ID".to_string()),
            true,
        )
        .with_arg(
            "vars".to_string(),
            Some("Variable substitutions as key=value pairs".to_string()),
            false,
        ),
    );

    // /template:delete - Delete a template
    registry.register(
        Command::builtin(
            "template:delete".to_string(),
            "Delete a saved template".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Template name or ID".to_string()),
            true,
        ),
    );

    // Context/Working Set Commands

    // /context - Show or manage working set (files in context)
    registry.register(Command::builtin(
        "context".to_string(),
        "Show files currently in the working set (agent's file context)".to_string(),
        "".to_string(),
    ));

    // /context:add - Add file to working set
    registry.register(
        Command::builtin(
            "context:add".to_string(),
            "Add a file to the working set (loads into context)".to_string(),
            "".to_string(),
        )
        .with_arg("path".to_string(), Some("Path to file".to_string()), true)
        .with_arg(
            "pinned".to_string(),
            Some("Pin file to prevent eviction (true/false)".to_string()),
            false,
        ),
    );

    // /context:remove - Remove file from working set
    registry.register(
        Command::builtin(
            "context:remove".to_string(),
            "Remove a file from the working set".to_string(),
            "".to_string(),
        )
        .with_arg(
            "path".to_string(),
            Some("Path to file to remove".to_string()),
            true,
        ),
    );

    // /context:pin - Pin a file (prevent eviction)
    registry.register(
        Command::builtin(
            "context:pin".to_string(),
            "Pin a file in the working set (prevents automatic eviction)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "path".to_string(),
            Some("Path to file to pin".to_string()),
            true,
        ),
    );

    // /context:unpin - Unpin a file
    registry.register(
        Command::builtin(
            "context:unpin".to_string(),
            "Unpin a file (allows automatic eviction when stale)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "path".to_string(),
            Some("Path to file to unpin".to_string()),
            true,
        ),
    );

    // /context:clear - Clear working set
    registry.register(
        Command::builtin(
            "context:clear".to_string(),
            "Clear all files from the working set".to_string(),
            "".to_string(),
        )
        .with_arg(
            "keep_pinned".to_string(),
            Some("Keep pinned files (true/false, default: true)".to_string()),
            false,
        ),
    );

    // /ask - Switch to Ask mode (read-only) or ask a question in Ask mode
    registry.register(
        Command::builtin(
            "ask".to_string(),
            "Switch to Ask mode (read-only). Optionally provide a question to ask immediately."
                .to_string(),
            "".to_string(),
        )
        .with_arg(
            "query".to_string(),
            Some("Optional question to ask in read-only mode".to_string()),
            false,
        ),
    );

    // /edit - Switch to Edit mode (full tools) or send a message in Edit mode
    registry.register(
        Command::builtin(
            "edit".to_string(),
            "Switch to Edit mode (full tool access). Optionally provide an instruction to send immediately.".to_string(),
            "".to_string(),
        )
        .with_arg("query".to_string(), Some("Optional instruction to send in edit mode".to_string()), false)
    );

    // /tools - Configure tool selection mode
    registry.register(
        Command::builtin(
            "tools".to_string(),
            "Configure which tools are available: full, explicit, smart (default), core, or none"
                .to_string(),
            "".to_string(),
        )
        .with_arg(
            "mode".to_string(),
            Some("Tool mode: full, explicit, smart, core, or none".to_string()),
            false,
        ),
    );

    // MDAP Commands

    // /mdap - Show MDAP status or toggle MDAP mode
    registry.register(
        Command::builtin(
            "mdap".to_string(),
            "Show MDAP status or toggle high-reliability mode (on/off)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "state".to_string(),
            Some("on/off to enable/disable MDAP mode".to_string()),
            false,
        ),
    );

    // /mdap:on - Enable MDAP mode
    registry.register(Command::builtin(
        "mdap:on".to_string(),
        "Enable MDAP high-reliability mode".to_string(),
        "".to_string(),
    ));

    // /mdap:off - Disable MDAP mode
    registry.register(Command::builtin(
        "mdap:off".to_string(),
        "Disable MDAP mode".to_string(),
        "".to_string(),
    ));

    // /mdap:k - Set vote margin
    registry.register(
        Command::builtin(
            "mdap:k".to_string(),
            "Set MDAP vote margin (k) - higher values increase reliability and cost".to_string(),
            "".to_string(),
        )
        .with_arg(
            "value".to_string(),
            Some("Vote margin (1-10, default: 3)".to_string()),
            true,
        ),
    );

    // /mdap:target - Set target success rate
    registry.register(
        Command::builtin(
            "mdap:target".to_string(),
            "Set MDAP target success rate (0.5-0.999)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "rate".to_string(),
            Some("Target rate as decimal (0.95) or percentage (95%)".to_string()),
            true,
        ),
    );

    // Knowledge Commands (Behavioral Knowledge System)

    // /learn - Explicitly teach a behavioral truth
    registry.register(
        Command::builtin(
            "learn".to_string(),
            "Teach the agent a behavioral truth (shared across all users)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "rule".to_string(),
            Some(
                "The behavioral rule to learn (e.g., \"pm2 logs requires --nostream\")".to_string(),
            ),
            true,
        )
        .with_arg(
            "rationale".to_string(),
            Some("Why this is better (optional)".to_string()),
            false,
        ),
    );

    // /knowledge - Show knowledge status
    registry.register(Command::builtin(
        "knowledge".to_string(),
        "Show behavioral knowledge system status and statistics".to_string(),
        "".to_string(),
    ));

    // /knowledge:list - List learned truths
    registry.register(
        Command::builtin(
            "knowledge:list".to_string(),
            "List all learned behavioral truths (optionally filtered by category)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "category".to_string(),
            Some(
                "Filter by category: command, strategy, tool, error, resource, pattern".to_string(),
            ),
            false,
        ),
    );

    // /knowledge:search - Search truths
    registry.register(
        Command::builtin(
            "knowledge:search".to_string(),
            "Search learned truths by keyword".to_string(),
            "".to_string(),
        )
        .with_arg("query".to_string(), Some("Search query".to_string()), true),
    );

    // /knowledge:sync - Force sync with server
    registry.register(Command::builtin(
        "knowledge:sync".to_string(),
        "Force synchronization with the Brainwires server".to_string(),
        "".to_string(),
    ));

    // /knowledge:contradict - Report a truth as incorrect
    registry.register(
        Command::builtin(
            "knowledge:contradict".to_string(),
            "Report a learned truth as incorrect (reduces its confidence)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "id".to_string(),
            Some("Truth ID to contradict".to_string()),
            true,
        )
        .with_arg(
            "reason".to_string(),
            Some("Reason for contradiction (optional)".to_string()),
            false,
        ),
    );

    // /knowledge:delete - Delete a truth (local only)
    registry.register(
        Command::builtin(
            "knowledge:delete".to_string(),
            "Delete a truth from local cache".to_string(),
            "".to_string(),
        )
        .with_arg(
            "id".to_string(),
            Some("Truth ID to delete".to_string()),
            true,
        ),
    );

    // Personal Knowledge Commands (Personal Knowledge System)

    // /profile - Show current profile summary
    registry.register(Command::builtin(
        "profile".to_string(),
        "Show your personal profile summary (facts learned about you)".to_string(),
        "".to_string(),
    ));

    // /profile:set - Set a personal fact explicitly
    registry.register(
        Command::builtin(
            "profile:set".to_string(),
            "Set a personal fact (e.g., preferred_language, name, current_project)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "key".to_string(),
            Some("Fact key (e.g., name, preferred_language, current_project)".to_string()),
            true,
        )
        .with_arg("value".to_string(), Some("Fact value".to_string()), true)
        .with_arg(
            "local".to_string(),
            Some("Set to 'true' to keep this fact local-only (never synced)".to_string()),
            false,
        ),
    );

    // /profile:name - Shorthand for setting name
    registry.register(
        Command::builtin(
            "profile:name".to_string(),
            "Set your name (shorthand for /profile:set name <name>)".to_string(),
            "".to_string(),
        )
        .with_arg("name".to_string(), Some("Your name".to_string()), true),
    );

    // /profile:list - List personal facts by category
    registry.register(
        Command::builtin(
            "profile:list".to_string(),
            "List personal facts (optionally filtered by category)".to_string(),
            "".to_string(),
        )
        .with_arg(
            "category".to_string(),
            Some(
                "Filter by: identity, preference, capability, context, constraint, relationship"
                    .to_string(),
            ),
            false,
        ),
    );

    // /profile:search - Search personal facts
    registry.register(
        Command::builtin(
            "profile:search".to_string(),
            "Search personal facts by keyword".to_string(),
            "".to_string(),
        )
        .with_arg("query".to_string(), Some("Search query".to_string()), true),
    );

    // /profile:delete - Delete a personal fact
    registry.register(
        Command::builtin(
            "profile:delete".to_string(),
            "Delete a personal fact".to_string(),
            "".to_string(),
        )
        .with_arg(
            "id".to_string(),
            Some("Fact ID or key to delete".to_string()),
            true,
        ),
    );

    // /profile:sync - Force sync with server
    registry.register(Command::builtin(
        "profile:sync".to_string(),
        "Force synchronization of personal facts with server".to_string(),
        "".to_string(),
    ));

    // /profile:export - Export profile as JSON
    registry.register(
        Command::builtin(
            "profile:export".to_string(),
            "Export your profile as JSON file".to_string(),
            "".to_string(),
        )
        .with_arg(
            "file".to_string(),
            Some("Output file path (default: ~/brainwires-profile.json)".to_string()),
            false,
        ),
    );

    // /profile:import - Import profile from JSON
    registry.register(
        Command::builtin(
            "profile:import".to_string(),
            "Import profile from a JSON file".to_string(),
            "".to_string(),
        )
        .with_arg(
            "file".to_string(),
            Some("JSON file to import".to_string()),
            true,
        ),
    );

    // /profile:stats - Show profile statistics
    registry.register(Command::builtin(
        "profile:stats".to_string(),
        "Show statistics about your personal knowledge base".to_string(),
        "".to_string(),
    ));

    // /remember - Quick command to remember facts (shortcut for /profile:set)
    registry.register(
        Command::builtin(
            "remember".to_string(),
            "Quick command to remember contextual facts (auto-generates key from content)"
                .to_string(),
            "".to_string(),
        )
        .with_arg(
            "fact".to_string(),
            Some("The fact to remember (entire remaining text)".to_string()),
            true,
        ),
    );

    // ==========================================================================
    // Skill Commands
    // ==========================================================================

    // /skill - Invoke a skill
    registry.register(
        Command::builtin(
            "skill".to_string(),
            "Invoke a skill by name with optional arguments".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Skill name to invoke".to_string()),
            true,
        )
        .with_arg(
            "args".to_string(),
            Some("Optional arguments as key=value pairs".to_string()),
            false,
        ),
    );

    // /skills - List all available skills
    registry.register(Command::builtin(
        "skills".to_string(),
        "List all available skills from personal and project directories".to_string(),
        "".to_string(),
    ));

    // /skill:show - Show skill details
    registry.register(
        Command::builtin(
            "skill:show".to_string(),
            "Display detailed information about a skill".to_string(),
            "".to_string(),
        )
        .with_arg("name".to_string(), Some("Skill name".to_string()), true),
    );

    // /skill:reload - Reload skills from disk
    registry.register(Command::builtin(
        "skill:reload".to_string(),
        "Reload skills from disk (useful after editing SKILL.md files)".to_string(),
        "".to_string(),
    ));

    // /skill:create - Create a new skill
    registry.register(
        Command::builtin(
            "skill:create".to_string(),
            "Create a new skill from a template".to_string(),
            "".to_string(),
        )
        .with_arg(
            "name".to_string(),
            Some("Skill name (lowercase, hyphens only)".to_string()),
            true,
        )
        .with_arg(
            "location".to_string(),
            Some("personal or project (default: personal)".to_string()),
            false,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_builtin() {
        let mut registry = CommandRegistry::new();
        register_builtin_commands(&mut registry);

        assert!(registry.get("help").is_some());
        assert!(registry.get("clear").is_some());
        assert!(registry.get("status").is_some());
        assert!(registry.get("model").is_some());
        assert!(registry.get("rewind").is_some());
        assert!(registry.get("review").is_some());
        assert!(registry.get("commands").is_some());
    }

    #[test]
    fn test_builtin_flags() {
        let mut registry = CommandRegistry::new();
        register_builtin_commands(&mut registry);

        let clear_cmd = registry.get("clear").unwrap();
        assert!(clear_cmd.builtin);
    }

    #[test]
    fn test_skill_commands_registered() {
        let mut registry = CommandRegistry::new();
        register_builtin_commands(&mut registry);

        assert!(registry.get("skill").is_some());
        assert!(registry.get("skills").is_some());
        assert!(registry.get("skill:show").is_some());
        assert!(registry.get("skill:reload").is_some());
        assert!(registry.get("skill:create").is_some());
    }

    #[test]
    fn test_skill_command_args() {
        let mut registry = CommandRegistry::new();
        register_builtin_commands(&mut registry);

        let skill_cmd = registry.get("skill").unwrap();
        assert_eq!(skill_cmd.args.len(), 2);
        assert_eq!(skill_cmd.args[0].name, "name");
        assert!(skill_cmd.args[0].required);

        let show_cmd = registry.get("skill:show").unwrap();
        assert_eq!(show_cmd.args.len(), 1);
        assert!(show_cmd.args[0].required);
    }
}
