//! Project Commands
//!
//! Commands for project RAG operations: project:index, project:query, etc.

use anyhow::Result;

use super::{CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute project-related built-in commands
    pub(super) fn execute_project_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "project:index" => Some(self.cmd_project_index(args)),
            "project:query" => Some(self.cmd_project_query(args)),
            "project:stats" => Some(self.cmd_project_stats()),
            "project:search" => Some(self.cmd_project_search(args)),
            "project:clear" => Some(self.cmd_project_clear()),
            "project:git-search" => Some(self.cmd_project_git_search(args)),
            "project:definition" => Some(self.cmd_project_definition(args)),
            "project:references" => Some(self.cmd_project_references(args)),
            "project:callgraph" => Some(self.cmd_project_callgraph(args)),
            _ => None,
        }
    }

    fn cmd_project_index(&self, args: &[String]) -> Result<CommandResult> {
        let path = args.first()
            .map(|s| s.clone())
            .unwrap_or_else(|| std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(String::from))
                .unwrap_or_else(|| ".".to_string()));

        let message = format!(
            "Please use the mcp__project__index_codebase tool to index the codebase at path: {}\n\n\
            This will create embeddings for semantic search across the codebase.",
            path
        );
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_query(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!("Usage: /project:query <search_query>\n\nSearch the indexed codebase using semantic search.");
        }
        let query = args.join(" ");
        let message = format!(
            "Please use the mcp__project__query_codebase tool with the following query: {}\n\n\
            This will search the indexed codebase using semantic embeddings.",
            query
        );
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_stats(&self) -> Result<CommandResult> {
        let message = "Please use the mcp__project__get_statistics tool to show index statistics.\n\n\
            This will display information about indexed files, chunks, and languages.".to_string();
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_search(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /project:search <query> [extensions] [languages]\n\n\
                Example: /project:search authentication rs,toml Rust\n\n\
                Search with optional file type and language filters."
            );
        }

        let query = args[0].clone();
        let extensions = args.get(1).map(|s| s.clone());
        let languages = args.get(2).map(|s| s.clone());

        let mut message = format!(
            "Please use the mcp__project__search_by_filters tool with:\n\
            - query: {}\n",
            query
        );

        if let Some(ext) = extensions {
            message.push_str(&format!("- file_extensions: [{}]\n", ext));
        }
        if let Some(lang) = languages {
            message.push_str(&format!("- languages: [{}]\n", lang));
        }

        message.push_str("\nThis will perform advanced semantic search with filters.");
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_clear(&self) -> Result<CommandResult> {
        let message = "Please use the mcp__project__clear_index tool to clear all indexed data.\n\n\
            WARNING: This will delete the entire index. You'll need to reindex to search again.".to_string();
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_git_search(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /project:git-search <query> [max_commits]\n\n\
                Example: /project:git-search authentication 20\n\n\
                Search git commit history using semantic search."
            );
        }

        let query = args[0].clone();
        let max_commits = args.get(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(10);

        let message = format!(
            "Please use the mcp__project__search_git_history tool with:\n\
            - query: {}\n\
            - max_commits: {}\n\n\
            This will search git commit history semantically.",
            query, max_commits
        );
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_definition(&self, args: &[String]) -> Result<CommandResult> {
        if args.len() < 3 {
            anyhow::bail!(
                "Usage: /project:definition <file> <line> <column>\n\n\
                Example: /project:definition src/main.rs 42 10\n\n\
                Find where a symbol is defined (LSP-like go-to-definition).\n\
                - file: Path to the file containing the symbol\n\
                - line: Line number (1-based)\n\
                - column: Column number (0-based)"
            );
        }

        let file_path = args[0].clone();
        let line = args[1].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid line number: {}", args[1]))?;
        let column = args[2].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid column number: {}", args[2]))?;

        let message = format!(
            "Please use the mcp__project__find_definition tool with:\n\
            - file_path: {}\n\
            - line: {}\n\
            - column: {}\n\n\
            This will find where the symbol at the given location is defined.",
            file_path, line, column
        );
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_references(&self, args: &[String]) -> Result<CommandResult> {
        if args.len() < 3 {
            anyhow::bail!(
                "Usage: /project:references <file> <line> <column> [limit]\n\n\
                Example: /project:references src/main.rs 42 10 50\n\n\
                Find all references to a symbol at the given location.\n\
                - file: Path to the file containing the symbol\n\
                - line: Line number (1-based)\n\
                - column: Column number (0-based)\n\
                - limit: Maximum references to return (default: 100)"
            );
        }

        let file_path = args[0].clone();
        let line = args[1].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid line number: {}", args[1]))?;
        let column = args[2].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid column number: {}", args[2]))?;
        let limit = args.get(3)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(100);

        let message = format!(
            "Please use the mcp__project__find_references tool with:\n\
            - file_path: {}\n\
            - line: {}\n\
            - column: {}\n\
            - limit: {}\n\n\
            This will find all references to the symbol at the given location.",
            file_path, line, column, limit
        );
        Ok(CommandResult::Message(message))
    }

    fn cmd_project_callgraph(&self, args: &[String]) -> Result<CommandResult> {
        if args.len() < 3 {
            anyhow::bail!(
                "Usage: /project:callgraph <file> <line> <column> [depth]\n\n\
                Example: /project:callgraph src/main.rs 42 10 3\n\n\
                Get the call graph for a function (callers and callees).\n\
                - file: Path to the file containing the function\n\
                - line: Line number (1-based)\n\
                - column: Column number (0-based)\n\
                - depth: Maximum traversal depth (default: 2)"
            );
        }

        let file_path = args[0].clone();
        let line = args[1].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid line number: {}", args[1]))?;
        let column = args[2].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid column number: {}", args[2]))?;
        let depth = args.get(3)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(2);

        let message = format!(
            "Please use the mcp__project__get_call_graph tool with:\n\
            - file_path: {}\n\
            - line: {}\n\
            - column: {}\n\
            - depth: {}\n\n\
            This will return the call graph showing callers and callees of the function.",
            file_path, line, column, depth
        );
        Ok(CommandResult::Message(message))
    }
}
