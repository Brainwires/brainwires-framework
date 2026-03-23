# brainwires-skills

[![Crates.io](https://img.shields.io/crates/v/brainwires-skills.svg)](https://crates.io/crates/brainwires-skills)
[![Documentation](https://img.shields.io/docsrs/brainwires-skills)](https://docs.rs/brainwires-skills)
[![License](https://img.shields.io/crates/l/brainwires-skills.svg)](LICENSE)

Agent skills system for the Brainwires Agent Framework.

## Attribution

This crate implements the [Agent Skills](https://agentskills.io/) open standard, originally developed by [Anthropic](https://www.anthropic.com/) and published as an open specification on December 18, 2025. The Agent Skills format was designed by Barry Zhang, Keith Lazuka, and Mahesh Murag at Anthropic.

**Official resources:**

- [Agent Skills Specification](https://agentskills.io/specification) — the complete format specification
- [anthropics/skills](https://github.com/anthropics/skills) — Anthropic's official example skills repository
- [agentskills/agentskills](https://github.com/agentskills/agentskills) — the open standard repository and reference library
- [Blog: Equipping agents for the real world with Agent Skills](https://claude.com/blog/equipping-agents-for-the-real-world-with-agent-skills)

`brainwires-skills` aims to faithfully implement the Agent Skills specification while adding Brainwires-specific extensions. All spec-compliant fields and validation rules are supported. Extensions beyond the spec are clearly documented in the [Brainwires Extensions](#brainwires-extensions) section below.

## Overview

`brainwires-skills` implements the Agent Skills markdown-based format that extends agent capabilities through composable, reusable skill packages. Skills are defined in `SKILL.md` files with YAML frontmatter for metadata and markdown body for instructions. The crate provides parsing, registry management, keyword-based routing, and multi-mode execution — enabling agents to discover, match, and run skills on demand.

The system uses a **progressive disclosure** pattern (as defined by the spec): at startup, only lightweight metadata (name, description) is loaded for fast matching. Full skill content is loaded on-demand when a skill is activated, and cached for subsequent use. This enables fast startup and efficient memory usage even with hundreds of registered skills.

**Design principles:**

- **Progressive disclosure** — metadata loaded at startup (~100 bytes per skill), full instructions loaded lazily on activation and cached in memory
- **Three execution modes** — inline (instructions injected into conversation), subagent (background task via AgentPool), and script (Rhai script via OrchestratorTool)
- **Skill hierarchy** — personal (`~/.brainwires/skills/`), project (`.brainwires/skills/`), and built-in skills; project skills override personal skills with the same name
- **Tool restrictions** — optional `allowed-tools` list restricts which tools a skill can use during execution (accepts both YAML lists and spec-compliant space-delimited strings)
- **Template rendering** — simple `{{arg}}` substitution and `{{#if var}}...{{/if}}` conditionals for parameterized skills

```text
  ┌───────────────────────────────────────────────────────────────────────┐
  │                         brainwires-skills                            │
  │                                                                      │
  │  SKILL.md Files                                                      │
  │      │                                                               │
  │      ▼                                                               │
  │  ┌─── Parser ──────────────────────────────────────────────────────┐ │
  │  │  parse_skill_metadata() → SkillMetadata (lightweight)           │ │
  │  │  parse_skill_file()     → Skill (full content)                  │ │
  │  │  render_template()      → String (variable substitution)        │ │
  │  │  Validation: name (lowercase, hyphens, ≤64), desc (≤1024)      │ │
  │  └──────────────────────────────────────┬───────────────────────────┘ │
  │                                         │                            │
  │                                         ▼                            │
  │  ┌─── Registry ────────────────────────────────────────────────────┐ │
  │  │  discover_from(paths) → load metadata from directories          │ │
  │  │  get_metadata(name)   → &SkillMetadata (fast lookup)            │ │
  │  │  get_skill(name)      → &Skill (lazy load + cache)              │ │
  │  │  Hierarchy: Personal → Project → Builtin (later overrides)      │ │
  │  └──────────────────────────────────────┬───────────────────────────┘ │
  │                                         │                            │
  │                                         ▼                            │
  │  ┌─── Router ──────────────────────────────────────────────────────┐ │
  │  │  match_skills(query)      → Vec<SkillMatch> (sorted by conf.)   │ │
  │  │  format_suggestions()     → "The skill `/name` may help."       │ │
  │  │  explicit_match(name)     → SkillMatch (confidence 1.0)         │ │
  │  │  Keyword matching: name(3x), name-words(2x), desc-words(1x)    │ │
  │  └──────────────────────────────────────┬───────────────────────────┘ │
  │                                         │                            │
  │                                         ▼                            │
  │  ┌─── Executor ────────────────────────────────────────────────────┐ │
  │  │  execute(skill, args)       → SkillResult                       │ │
  │  │  prepare_subagent(skill)    → SubagentPrepared (task + prompt)   │ │
  │  │  prepare_script(skill)      → ScriptPrepared (rendered script)   │ │
  │  │  filter_allowed_tools()     → enforces tool restrictions         │ │
  │  └─────────────────────────────────────────────────────────────────┘ │
  └───────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-skills = "0.6"
```

Register and execute a skill:

```rust
use brainwires_skills::{SkillRegistry, SkillExecutor, SkillRouter};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Discover skills from directories
    let mut registry = SkillRegistry::new();
    registry.discover_from(&[
        ("~/.brainwires/skills/".into(), brainwires_skills::SkillSource::Personal),
        (".brainwires/skills/".into(), brainwires_skills::SkillSource::Project),
    ])?;

    let registry = Arc::new(RwLock::new(registry));

    // Route a user query to matching skills
    let router = SkillRouter::new(Arc::clone(&registry));
    let matches = router.match_skills("review my pull request").await;

    if let Some(suggestion) = router.format_suggestions(&matches) {
        println!("{}", suggestion);
        // → "The skill `/review-pr` may help. Use the command to activate."
    }

    // Execute a skill by name
    let executor = SkillExecutor::new(Arc::clone(&registry));
    let args = HashMap::new();
    let result = executor.execute_by_name("review-pr", args).await?;

    Ok(())
}
```

## Architecture

### SKILL.md Format

Skills are defined as markdown files with YAML frontmatter per the [Agent Skills specification](https://agentskills.io/specification):

```markdown
---
name: review-pr
description: Reviews pull requests for code quality and security issues.
allowed-tools: Read Grep                     # spec format: space-delimited
license: MIT
compatibility: Requires git CLI
model: claude-sonnet-4                       # Brainwires extension
metadata:
  category: code-review
  execution: subagent                        # Brainwires extension
hooks:                                       # Brainwires extension
  - agent_started
---

# PR Review Instructions

When reviewing a pull request:
1. Check for code quality issues
2. Look for security vulnerabilities
3. Verify test coverage
```

`allowed-tools` also accepts YAML list format:
```yaml
allowed-tools:
  - Read
  - Grep
```

**Supported file layouts:**

| Layout | Path | Description |
|--------|------|-------------|
| Direct file | `skills/review-pr.md` | Single `.md` file named after the skill |
| Subdirectory | `skills/review-pr/SKILL.md` | Skill in its own directory |

### SkillMetadata

Lightweight metadata loaded at startup for fast matching and display.

| Field | Type | Spec | Description |
|-------|------|------|-------------|
| `name` | `String` | Required | Skill identifier (lowercase, hyphens only, max 64 chars, no consecutive hyphens) |
| `description` | `String` | Required | Used for semantic matching (max 1024 chars) |
| `allowed_tools` | `Option<Vec<String>>` | Optional | Tool restrictions (`None` = all tools allowed). Accepts YAML list or space-delimited string |
| `license` | `Option<String>` | Optional | Software license |
| `compatibility` | `Option<String>` | Optional | Environment requirements (max 500 chars) |
| `metadata` | `Option<HashMap<String, String>>` | Optional | Custom key-value pairs (category, execution, author, version) |
| `model` | `Option<String>` | Extension | Model override for execution (Brainwires extension) |
| `hooks` | `Option<Vec<String>>` | Extension | Lifecycle hook event subscriptions (Brainwires extension) |
| `source` | `SkillSource` | Internal | Where the skill was loaded from |
| `source_path` | `PathBuf` | Internal | File path for lazy loading |

| Method | Description |
|--------|-------------|
| `new(name, description)` | Create metadata with defaults |
| `with_source(source)` | Set the source location (builder pattern) |
| `with_source_path(path)` | Set the file path (builder pattern) |
| `execution_mode()` | Get execution mode from metadata map |
| `get_metadata(key)` | Get a custom metadata value |
| `has_tool_restrictions()` | Check if skill has an `allowed-tools` list |
| `is_tool_allowed(tool_name)` | Check if a specific tool is permitted |

### SkillSource

| Variant | Path | Description |
|---------|------|-------------|
| `Personal` | `~/.brainwires/skills/` | User-level skills (default) |
| `Project` | `.brainwires/skills/` | Project-level skills (override personal) |
| `Builtin` | (bundled) | Skills shipped with the application |

### SkillExecutionMode

| Variant | Description |
|---------|-------------|
| `Inline` | Instructions injected into current conversation (default) |
| `Subagent` | Dedicated background agent spawned via AgentPool |
| `Script` | Rhai script executed via OrchestratorTool |

### Skill

Full skill content loaded on-demand when activated.

| Field | Type | Description |
|-------|------|-------------|
| `metadata` | `SkillMetadata` | The lightweight metadata |
| `instructions` | `String` | Full markdown body after frontmatter |
| `execution_mode` | `SkillExecutionMode` | Derived from metadata |

| Method | Description |
|--------|-------------|
| `new(metadata, instructions)` | Create from metadata and instruction content |
| `name()` | Get the skill name |
| `description()` | Get the skill description |
| `allowed_tools()` | Get the allowed tools list (if any) |
| `model()` | Get the model override (if any) |
| `runs_as_subagent()` | Check if execution mode is `Subagent` |
| `is_script()` | Check if execution mode is `Script` |

### SkillResult

| Variant | Fields | Description |
|---------|--------|-------------|
| `Inline` | `instructions`, `model_override` | Instructions for conversation injection |
| `Subagent` | `agent_id` | Spawned agent identifier |
| `Script` | `output`, `is_error` | Script execution output |

### SkillRegistry

Central registry managing all available skills with progressive disclosure.

| Method | Description |
|--------|-------------|
| `new()` | Create empty registry |
| `discover_from(paths)` | Clear and load metadata from `(path, source)` pairs |
| `reload()` | Reload using same paths as last `discover_from` |
| `register(metadata)` | Register a skill directly (for built-in skills) |
| `get_metadata(name)` | Get metadata by name → `Option<&SkillMetadata>` |
| `get_skill(name)` | Lazy load full skill (cached) → `Result<&Skill>` |
| `get_skill_mut(name)` | Mutable access with lazy load → `Result<&mut Skill>` |
| `contains(name)` | Check if a skill exists |
| `list_skills()` | All skill names, sorted alphabetically |
| `all_metadata()` | All metadata for matching |
| `skills_by_source(source)` | Filter skills by source |
| `skills_by_category(category)` | Filter by metadata category value |
| `len()` / `is_empty()` | Registry size |
| `clear_cache()` | Clear loaded cache, force disk reload on next access |
| `remove(name)` | Remove a skill from registry and cache |
| `format_skill_list()` | Display all skills grouped by source |
| `format_skill_detail(name)` | Display detailed info for a single skill |

### SkillRouter

Handles skill activation through keyword matching against user queries. Skills are **suggested** to users, not auto-activated.

| Method | Description |
|--------|-------------|
| `new(registry)` | Create router with shared registry reference |
| `with_min_confidence(confidence)` | Set minimum confidence threshold (default: 0.5) |
| `match_skills(query)` | Match query against descriptions → `Vec<SkillMatch>` (sorted by confidence) |
| `format_suggestions(matches)` | Format top 3 matches as suggestion text → `Option<String>` |
| `skill_exists(name)` | Check if a skill exists by name |
| `explicit_match(skill_name)` | Create explicit match (confidence 1.0, used for `/skill-name`) |

**Keyword matching weights:**

| Match type | Weight | Description |
|------------|--------|-------------|
| Full name match | +3 | Query contains skill name or vice versa |
| Name word match | +2 | Individual skill name words found in query |
| Description word match | +1 | Query words found in skill description |

**Confidence formula:** `0.6 + (match_count * 0.05)`, capped at 0.9.

**`MatchSource` enum:**

| Variant | Description |
|---------|-------------|
| `Semantic` | Matched via semantic similarity (future enhancement) |
| `Keyword` | Matched via keyword patterns |
| `Explicit` | User explicitly invoked (`/skill-name`) |

### SkillExecutor

Executes skills in one of three modes. Tool restrictions from `allowed-tools` are enforced during preparation.

| Method | Description |
|--------|-------------|
| `new(registry)` | Create executor with shared registry reference |
| `execute_by_name(name, args)` | Load skill from registry and execute → `Result<SkillResult>` |
| `execute(skill, args)` | Execute a skill with template-rendered arguments → `Result<SkillResult>` |
| `prepare_subagent(skill, tools, args)` | Prepare subagent context → `Result<SubagentPrepared>` |
| `prepare_script(skill, tools, args)` | Prepare script execution → `Result<ScriptPrepared>` |
| `get_execution_mode(name)` | Look up execution mode by skill name |

**`SubagentPrepared`:**

| Field | Type | Description |
|-------|------|-------------|
| `task_description` | `String` | Rendered instructions |
| `allowed_tool_names` | `Vec<String>` | Filtered tool list |
| `system_prompt` | `String` | System prompt for the subagent |
| `model_override` | `Option<String>` | Model override |

**`ScriptPrepared`:**

| Field | Type | Description |
|-------|------|-------------|
| `script_content` | `String` | Rendered Rhai script |
| `allowed_tool_names` | `Vec<String>` | Filtered tool list |
| `model_override` | `Option<String>` | Model override |
| `skill_name` | `String` | Skill name for logging |

### Parser

Functions for parsing SKILL.md files and rendering templates.

| Function | Description |
|----------|-------------|
| `parse_skill_metadata(path)` | Parse only YAML frontmatter → `Result<SkillMetadata>` |
| `parse_skill_file(path)` | Parse complete file (metadata + instructions) → `Result<Skill>` |
| `render_template(template, args)` | Substitute `{{arg}}` placeholders and `{{#if var}}...{{/if}}` conditionals |

**Skill name validation rules** (per [Agent Skills specification](https://agentskills.io/specification)):

| Rule | Constraint |
|------|------------|
| Characters | Lowercase letters, digits, and hyphens only |
| Length | 1-64 characters |
| Boundaries | Cannot start or end with a hyphen |
| Consecutive hyphens | Cannot contain `--` |
| Directory match | Name should match parent directory name (warning if mismatched) |
| Examples | `review-pr`, `commit`, `explain-code-123` |

## Usage Examples

### Discover and list skills

```rust
use brainwires_skills::{SkillRegistry, SkillSource};

let mut registry = SkillRegistry::new();
registry.discover_from(&[
    ("/home/user/.brainwires/skills/".into(), SkillSource::Personal),
    ("/project/.brainwires/skills/".into(), SkillSource::Project),
])?;

// List all skills
let names = registry.list_skills();
println!("Available skills: {:?}", names);

// Display formatted listing (grouped by source)
println!("{}", registry.format_skill_list());

// Filter by category
let dev_skills = registry.skills_by_category("development");
for skill in dev_skills {
    println!("  {} — {}", skill.name, skill.description);
}
```

### Route queries to skills

```rust
use brainwires_skills::{SkillRegistry, SkillRouter};
use std::sync::Arc;
use tokio::sync::RwLock;

let registry = Arc::new(RwLock::new(SkillRegistry::new()));
let router = SkillRouter::new(Arc::clone(&registry))
    .with_min_confidence(0.6);

// Match against user query
let matches = router.match_skills("help me review this code").await;
for m in &matches {
    println!("  {} (confidence: {:.2}, source: {})", m.skill_name, m.confidence, m.source);
}

// Format as user-facing suggestion
if let Some(msg) = router.format_suggestions(&matches) {
    println!("{}", msg);
    // → "The skill `/review-pr` may help. Use the command to activate."
}

// Explicit invocation (user typed /review-pr)
let explicit = router.explicit_match("review-pr");
assert_eq!(explicit.confidence, 1.0);
```

### Execute skills with template arguments

```rust
use brainwires_skills::{SkillExecutor, SkillResult, SkillRegistry};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

let registry = Arc::new(RwLock::new(SkillRegistry::new()));
let executor = SkillExecutor::new(Arc::clone(&registry));

// Execute with template arguments
let mut args = HashMap::new();
args.insert("pr_number".to_string(), "42".to_string());
args.insert("branch".to_string(), "feature/auth".to_string());

let result = executor.execute_by_name("review-pr", args).await?;

match result {
    SkillResult::Inline { instructions, model_override } => {
        println!("Inject into conversation:\n{}", instructions);
        if let Some(model) = model_override {
            println!("Use model: {}", model);
        }
    }
    SkillResult::Subagent { agent_id } => {
        println!("Spawned agent: {}", agent_id);
        // Caller monitors via AgentPool
    }
    SkillResult::Script { output, is_error } => {
        if is_error {
            eprintln!("Script error: {}", output);
        } else {
            println!("Script output: {}", output);
        }
    }
}
```

### Prepare subagent execution with tool restrictions

```rust
use brainwires_skills::{SkillExecutor, SkillRegistry, SkillSource, SkillMetadata, Skill, SkillExecutionMode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

let registry = Arc::new(RwLock::new(SkillRegistry::new()));
let executor = SkillExecutor::new(Arc::clone(&registry));

let mut metadata = SkillMetadata::new(
    "review-pr".to_string(),
    "Reviews pull requests".to_string(),
);
metadata.allowed_tools = Some(vec!["Read".to_string(), "Grep".to_string()]);

let skill = Skill::new(metadata, "Review the PR for {{focus}}".to_string());

let available_tools = vec![
    "Read".to_string(), "Write".to_string(),
    "Grep".to_string(), "Bash".to_string(),
];

let mut args = HashMap::new();
args.insert("focus".to_string(), "security issues".to_string());

let prepared = executor.prepare_subagent(&skill, &available_tools, args).await?;

// Only Read and Grep are allowed
assert_eq!(prepared.allowed_tool_names, vec!["Read", "Grep"]);
println!("System prompt:\n{}", prepared.system_prompt);
// Caller spawns via AgentPool with these constraints
```

### Use template rendering directly

```rust
use brainwires_skills::render_template;
use std::collections::HashMap;

let template = "Review {{file_path}}{{#if focus}} with focus on {{focus}}{{/if}}.";

// With all args
let mut args = HashMap::new();
args.insert("file_path".to_string(), "src/main.rs".to_string());
args.insert("focus".to_string(), "error handling".to_string());

let result = render_template(template, &args);
assert_eq!(result, "Review src/main.rs with focus on error handling.");

// Without optional arg
let mut args2 = HashMap::new();
args2.insert("file_path".to_string(), "src/main.rs".to_string());
args2.insert("focus".to_string(), "".to_string());

let result2 = render_template(template, &args2);
assert_eq!(result2, "Review src/main.rs.");
```

### Parse skill files directly

```rust
use brainwires_skills::{parse_skill_metadata, parse_skill_file};
use std::path::Path;

// Parse only metadata (fast, for startup)
let metadata = parse_skill_metadata(Path::new("skills/review-pr/SKILL.md"))?;
println!("Skill: {} — {}", metadata.name, metadata.description);
println!("Execution mode: {}", metadata.execution_mode());
println!("Has tool restrictions: {}", metadata.has_tool_restrictions());

// Parse full skill (on activation)
let skill = parse_skill_file(Path::new("skills/review-pr/SKILL.md"))?;
println!("Instructions ({} chars)", skill.instructions.len());
println!("Runs as subagent: {}", skill.runs_as_subagent());
```

## Integration

Use via the `brainwires` facade crate with the `skills` feature, or depend on `brainwires-skills` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.6", features = ["skills"] }

# Direct
[dependencies]
brainwires-skills = "0.6"
```

The crate re-exports all components at the top level:

```rust
use brainwires_skills::{
    // Parser
    parse_skill_file, parse_skill_metadata, render_template,

    // Registry
    SkillRegistry,

    // Router
    SkillRouter,

    // Executor
    SkillExecutor, SubagentPrepared, ScriptPrepared,

    // Core types
    Skill, SkillMetadata, SkillSource,
    SkillExecutionMode, SkillResult,
    SkillMatch, MatchSource,
};
```

## Brainwires Extensions

This crate extends the [Agent Skills specification](https://agentskills.io/specification) with the following Brainwires-specific features. These are clearly marked as extensions and do not conflict with spec-compliant skills.

| Extension | Description |
|-----------|-------------|
| `model` field | Optional model override per skill (e.g., `model: claude-sonnet-4`). Overrides the default model when executing the skill. |
| `hooks` field | Optional lifecycle hook event subscriptions (e.g., `hooks: [agent_started, tool_after_execute]`). Registers the skill to fire on matching lifecycle events. |
| Execution modes | The `metadata.execution` key controls how a skill runs: `inline` (default, injected into conversation), `subagent` (spawned via AgentPool), or `script` (Rhai script via OrchestratorTool). |
| Flat file layout | Skills can be defined as a single file (`skills/review-pr.md`) in addition to the spec-defined subdirectory layout (`skills/review-pr/SKILL.md`). |
| YAML list `allowed-tools` | In addition to the spec's space-delimited string format, `allowed-tools` also accepts YAML lists for convenience. |

Skills that use only spec-defined fields (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`) are fully portable across any Agent Skills-compatible agent.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
