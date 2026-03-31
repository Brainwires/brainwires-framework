# Project RAG Slash Commands

This document describes the slash commands available for the Project RAG (Retrieval-Augmented Generation) system, which provides semantic search capabilities across your codebase.

## Overview

The Project RAG system uses vector embeddings to enable semantic search across large codebases. Instead of simple text matching, it understands the meaning of code and can find relevant sections even when exact keywords don't match.

## Available Commands

### 1. `/project:index [path]`

**Description:** Index a codebase directory for semantic search

**Usage:**
```
/project:index
/project:index /path/to/project
/project:index .
```

**Parameters:**
- `path` (optional): Path to the codebase directory to index. Defaults to current directory.

**What it does:**
- Creates semantic embeddings for all code files in the project
- Stores embeddings in a local vector database
- Automatically performs incremental updates if the path was previously indexed
- Respects `.gitignore` patterns

**Example:**
```
/project:index ~/projects/my-rust-app
```

**Note:** First-time indexing may take a few minutes for large codebases. Subsequent updates are much faster.

---

### 2. `/project:query <search_query>`

**Description:** Search indexed codebase using semantic search

**Usage:**
```
/project:query authentication logic
/project:query how database connections work
/project:query error handling patterns
```

**Parameters:**
- `search_query` (required): Natural language query describing what you're looking for

**What it does:**
- Performs semantic search across the indexed codebase
- Returns relevant code chunks with similarity scores
- Uses natural language understanding to find conceptually similar code

**Example:**
```
/project:query JWT token validation
```

**Returns:** Code chunks that implement JWT token validation, even if they don't use those exact terms.

---

### 3. `/project:stats`

**Description:** Show statistics about the indexed codebase

**Usage:**
```
/project:stats
```

**What it does:**
- Displays number of indexed files
- Shows number of code chunks
- Lists programming languages detected
- Shows index size and location

**Example output:**
```
Index Statistics:
- Total files: 245
- Total chunks: 3,892
- Languages: Rust (180), TOML (15), Markdown (50)
- Database: .brainwires/lancedb
```

---

### 4. `/project:search <query> [extensions] [languages]`

**Description:** Advanced semantic search with file type and language filters

**Usage:**
```
/project:search authentication
/project:search database rs,toml
/project:search API endpoints rs,md Rust,Markdown
```

**Parameters:**
- `query` (required): Search query
- `extensions` (optional): Comma-separated file extensions (e.g., `rs,toml,md`)
- `languages` (optional): Comma-separated languages (e.g., `Rust,Python,JavaScript`)

**What it does:**
- Performs semantic search with filtering
- Only searches files matching the specified extensions or languages
- More precise results when you know what file types to search

**Examples:**
```
# Search only in Rust files
/project:search error handling rs Rust

# Search in configuration files
/project:search server config toml,yaml

# Search documentation
/project:search API documentation md Markdown
```

---

### 5. `/project:clear`

**Description:** Clear all indexed data from the vector database

**Usage:**
```
/project:clear
```

**What it does:**
- Deletes the entire vector database
- Removes all indexed code embeddings
- Requires full reindexing to search again

**Warning:** This is destructive and cannot be undone. You'll need to reindex all projects to use search again.

**Use case:** Use this if you want to start fresh or if the index becomes corrupted.

---

### 6. `/project:git-search <query> [max_commits]`

**Description:** Search git commit history using semantic search

**Usage:**
```
/project:git-search authentication
/project:git-search bug fix 20
/project:git-search refactor database 50
```

**Parameters:**
- `query` (required): Search query for commit messages and changes
- `max_commits` (optional): Maximum number of commits to search (default: 10)

**What it does:**
- Performs semantic search across git commit history
- Searches commit messages and diffs
- Finds relevant commits even without exact keyword matches

**Examples:**
```
# Find authentication-related commits
/project:git-search authentication implementation

# Search last 50 commits for database changes
/project:git-search database migration 50

# Find bug fixes
/project:git-search null pointer fix 30
```

---

### 7. `/project:definition <file> <line> <column>`

**Description:** Find where a symbol is defined (LSP-like go-to-definition)

**Usage:**
```
/project:definition src/main.rs 42 10
/project:definition src/lib.rs 100 5
```

**Parameters:**
- `file` (required): Path to the file containing the symbol
- `line` (required): Line number (1-based)
- `column` (required): Column number (0-based)

**What it does:**
- Finds the definition location of a symbol at the specified position
- Works like go-to-definition in an IDE
- Supports functions, structs, enums, traits, and other symbols

**Examples:**
```
# Find where a function is defined
/project:definition src/commands/executor.rs 42 10

# Find struct definition
/project:definition src/types/mod.rs 15 8
```

---

### 8. `/project:references <file> <line> <column> [limit]`

**Description:** Find all references to a symbol at a given location

**Usage:**
```
/project:references src/main.rs 42 10
/project:references src/lib.rs 100 5 50
```

**Parameters:**
- `file` (required): Path to the file containing the symbol
- `line` (required): Line number (1-based)
- `column` (required): Column number (0-based)
- `limit` (optional): Maximum references to return (default: 100)

**What it does:**
- Finds all locations where a symbol is used
- Includes the definition itself by default
- Helps understand how widely a function or type is used

**Examples:**
```
# Find all usages of a function
/project:references src/commands/executor.rs 42 10

# Find references with a lower limit
/project:references src/types/mod.rs 15 8 25
```

---

### 9. `/project:callgraph <file> <line> <column> [depth]`

**Description:** Get call graph for a function (callers and callees)

**Usage:**
```
/project:callgraph src/main.rs 42 10
/project:callgraph src/lib.rs 100 5 3
```

**Parameters:**
- `file` (required): Path to the file containing the function
- `line` (required): Line number (1-based)
- `column` (required): Column number (0-based)
- `depth` (optional): Maximum traversal depth (default: 2)

**What it does:**
- Builds a call graph for the function at the given location
- Shows both callers (functions that call this one) and callees (functions this one calls)
- Traverses up to the specified depth level
- Helps understand the flow of execution through the codebase

**Examples:**
```
# Get call graph with default depth
/project:callgraph src/commands/executor.rs 42 10

# Get deeper call graph
/project:callgraph src/main.rs 20 5 4
```

---

## Workflow Examples

### Example 1: First-time project exploration

```
# Index the project
/project:index ~/projects/webapp

# Get statistics
/project:stats

# Search for specific functionality
/project:query user authentication flow

# Search in specific file types
/project:search API endpoints rs Rust
```

### Example 2: Daily development workflow

```
# Update index with recent changes
/project:index

# Search for implementation examples
/project:query error handling patterns

# Find relevant git history
/project:git-search error handling refactor
```

### Example 3: Code review

```
# Search for security-related code
/project:search authentication validation

# Find related commits
/project:git-search security patch 20

# Search test files
/project:search test coverage rs
```

---

## Technical Details

### Embedding Model
- **Model**: all-MiniLM-L6-v2
- **Dimensions**: 384
- **Type**: Sentence transformers

### Vector Database
- **Backend**: LanceDB
- **Location**: `.brainwires/lancedb` (project-specific, in project root)
- **Features**: Local, fast, persistent, per-project isolation

### Cache & Models
- **Embedding Models**: `~/.local/share/brainwires/fastembed/` (shared globally)
- **Hash Cache**: `.brainwires/hash_cache.json` (per-project)
- **Git Cache**: `.brainwires/git_cache.json` (per-project)

### Performance
- **Indexing Speed**: ~1000 files/minute
- **Search Latency**: 20-30ms
- **Chunk Size**: 50 lines of code per chunk
- **Overlap**: 5 lines between chunks

### Privacy
- **100% Local**: All processing happens on your machine
- **No API Calls**: No code leaves your computer
- **No Telemetry**: No usage tracking

---

## Troubleshooting

### Index not found error

**Problem:** Query returns "no index found"

**Solution:**
```
/project:index /path/to/project
```

You need to index the project first before searching.

### Search returns no results

**Possible causes:**
1. Project not indexed: Run `/project:index`
2. Query too specific: Try broader terms
3. File types filtered out: Check your filter parameters

**Solution:**
```
# Try broader query
/project:query authentication  # instead of "JWT RS256 token validation"

# Check what's indexed
/project:stats
```

### Slow indexing

**Problem:** Indexing takes a long time

**Normal behavior:**
- Large codebases (>10,000 files) can take 10+ minutes
- First-time indexing is slower than updates

**Tips:**
- Use incremental updates (`/project:index`) after initial index
- Exclude unnecessary directories (handled by `.gitignore`)

### Index corruption

**Problem:** Errors when searching or inconsistent results

**Solution:**
```
# Clear and reindex
/project:clear
/project:index /path/to/project
```

---

## Best Practices

1. **Index once, search often**: Initial indexing is expensive, but searches are fast

2. **Use natural language**: Don't just search for exact terms
   - Good: `/project:query user authentication flow`
   - Okay: `/project:query authenticate user`
   - Less effective: `/project:query auth`

3. **Update regularly**: Run `/project:index` after significant code changes

4. **Use filters**: Narrow down results with `/project:search` when you know file types

5. **Explore git history**: Use `/project:git-search` to understand how code evolved

6. **Check stats**: Use `/project:stats` to verify what's indexed

---

## Integration with Chat

All slash commands work seamlessly in chat mode:

```
User: How does authentication work in this project?
Assistant: Let me search for that.

/project:query authentication implementation

[Search results appear]

Based on the search results, the authentication system uses JWT tokens...
```

The AI assistant can use these commands to explore your codebase and provide informed answers.

---

## See Also

- [MCP Server Documentation](MCP_SERVER.md)
- [Configuration Guide](../README.md#configuration)
- [Slash Commands Reference](../BRAINWIRES.md#slash-commands)
