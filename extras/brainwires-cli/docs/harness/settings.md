# Harness Settings

This page documents the harness-shaped features that sit alongside the provider/model
config: layered `settings.json`, hooks, auto-memory, and the `ask_user_question` tool.

`settings.json` is **not** the same file as `config.json`. `config.json` holds
provider, model, temperature, and other LLM-shaped state. `settings.json` holds
harness behavior — permissions, hooks, environment. Keeping them separate means
you can edit one without risking the other.

---

## `settings.json` layering

The CLI merges `settings.json` from four locations, in this order (later wins
for scalars, arrays concatenate):

1. `~/.brainwires/settings.json` — user-wide defaults, shared across projects.
2. `~/.claude/settings.json` — **read-only migrator compatibility**. If you
   already had a Claude Code config, it's picked up as-is. Nothing is ever
   written here.
3. `<project-root>/.brainwires/settings.json` — shared project rules. Commit
   this to the repo.
4. `<project-root>/.brainwires/settings.local.json` — local overrides. Add to
   `.gitignore`.

"Project root" is the first ancestor of your current directory containing one
of `.git`, `.brainwires/`, `BRAINWIRES.md`, or `CLAUDE.md`.

Malformed JSON in any single file is logged via `tracing` and skipped — one bad
file never disables every other rule.

### Schema

```json
{
  "permissions": {
    "allow": ["Read", "Bash(ls:*)"],
    "deny":  ["Bash(rm:*)"],
    "ask":   ["WebFetch"]
  },
  "hooks": {
    "PreToolUse":  [ /* see Hooks below */ ],
    "PostToolUse": [],
    "UserPromptSubmit": [],
    "Stop": []
  },
  "env": {
    "FOO": "bar"
  }
}
```

All fields are optional. `permissions` arrays concatenate across files so
project settings can add rules without clobbering user-wide rules. `env`
entries later-wins on key collision. `hooks` matchers concatenate per event.

---

## Tool-specific permissions

Rules under `permissions.allow` / `deny` / `ask` control tool execution.
Syntax matches Claude Code's conventions so you can migrate rules directly.

### Patterns

| Pattern                  | Meaning                                                                |
|--------------------------|------------------------------------------------------------------------|
| `"Read"`                 | The brainwires `read_file` tool with any args. Short-name aliases:     |
|                          | `Bash` → `execute_command`, `Read` → `read_file`, `Write` → `write_file`, |
|                          | `Edit` → `edit_file`, `Grep` → `search`, `Glob` → `glob`, `WebFetch`,  |
|                          | `WebSearch`.                                                           |
| `"Bash(ls:*)"`           | `execute_command` where the `command` field is exactly `ls` or starts  |
|                          | with `ls `. `:*` is a prefix wildcard; bare `Bash(ls)` would be an     |
|                          | exact match only.                                                      |
| `"Edit(src/**/*.rs)"`    | `edit_file` where `file_path` matches the glob. `**` = any depth,      |
|                          | `*` = single path component, `?` = any one non-slash char.             |
| `"mcp__github__create_pr"` | Exact match of an MCP tool by its full prefixed name.                |

### Decision order

`deny` beats `ask` beats `allow`. `deny` overrides **everything**, including
`PermissionMode::Full` — this is your safety net. `ask` forces a user prompt
even on tools that normally wouldn't require one. `allow` bypasses the approval
prompt but still runs through the audit logger.

### Example — lock down destructive bash

```json
{
  "permissions": {
    "allow": ["Read", "Bash(ls:*)", "Bash(git:*)", "Bash(cargo:*)"],
    "deny":  ["Bash(rm:*)", "Bash(sudo:*)", "Bash(mkfs:*)"]
  }
}
```

---

## Hooks

Hooks are shell commands the CLI runs at four lifecycle points. They're
configured under `settings.hooks`.

### Events

| Event              | Fires                                                        |
|--------------------|--------------------------------------------------------------|
| `PreToolUse`       | After approval, **before** the tool implementation runs.     |
| `PostToolUse`      | After the tool returns, before the audit record is logged.   |
| `UserPromptSubmit` | Right after your prompt is added to the conversation.        |
| `Stop`             | After the assistant's final message of the turn is stored.   |

### Matchers

Each event can hold multiple matchers. A matcher optionally filters by tool
name (using the same short-name aliases as permissions). A matcher without a
`matcher` field fires on every event of its type.

### Exit-code semantics

Matches Claude Code:

| Exit code    | Effect                                                                 |
|--------------|------------------------------------------------------------------------|
| `0`          | **Continue.** Tool runs / turn proceeds as normal.                     |
| `2`          | **Block.** `stderr` is captured and surfaced as feedback. For          |
|              | `PreToolUse`, the tool never runs. For `PostToolUse`/`Stop`, the side  |
|              | effect has already happened; the block is advisory but the model sees  |
|              | the reason.                                                            |
| any other    | **Soft error.** Logged via `tracing::warn!`, execution continues.      |

Default timeout is **5 seconds per hook command**. Override with `timeout_ms`.

### Event payload

The hook command receives a JSON blob on stdin:

```json
{
  "event": "PreToolUse",
  "tool_name": "execute_command",
  "tool_args": {"command": "rm -rf /tmp/scratch"},
  "cwd": "/home/user/proj"
}
```

`PostToolUse` also includes `tool_result` and `is_error`. `UserPromptSubmit`
includes `prompt`. `Stop` includes `final_message`.

### Example — log every tool call to a file

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": null,
        "hooks": [
          {
            "type": "command",
            "command": "cat >> /tmp/bw-hooks.log; echo '' >> /tmp/bw-hooks.log"
          }
        ]
      }
    ]
  }
}
```

### Example — block dangerous bash with a feedback message

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.tool_args.command' | grep -qE '^(rm|mkfs|dd) ' && { echo 'destructive command blocked' >&2; exit 2; } || exit 0",
            "timeout_ms": 500
          }
        ]
      }
    ]
  }
}
```

---

## Auto-memory

Per-project memory notes live at
`~/.brainwires/projects/<encoded-cwd>/memory/`, where `<encoded-cwd>` replaces
path separators with `-`. The layout matches Claude Code 1:1 so you can
symlink or copy existing memory dirs.

```
~/.brainwires/projects/-home-me-proj/memory/
    MEMORY.md           # index, always loaded into the system prompt
    user_role.md        # typed memory files with YAML frontmatter
    feedback_terse.md
    project_deadline.md
    reference_dashboard.md
```

### Memory types

| Type        | When to use                                                                 |
|-------------|-----------------------------------------------------------------------------|
| `user`      | Facts about the user's role, skills, preferences — shapes how to collaborate. |
| `feedback`  | Corrections / confirmations. Include a `**Why:**` line.                     |
| `project`   | Stakeholder context, deadlines, decisions, constraints.                     |
| `reference` | Pointers to external systems (dashboards, issue trackers, Slack channels).  |

### Agent-facing tools

- `memory_save(name, type, description, content)` — creates or updates
  `<name>.md` with frontmatter and appends an entry to `MEMORY.md`. Idempotent.
- `memory_delete(name)` — removes the file and prunes the index line.
- `memory_list()` — returns the current `MEMORY.md` index.

Every memory mutation rewrites `MEMORY.md` from the files on disk, so stray
entries (e.g., from a manual `rm`) prune automatically.

### Opt-out

Set `BRAINWIRES_DISABLE_AUTO_MEMORY=1` to skip memory injection for one run.
Mirrors `BRAINWIRES_DISABLE_AUTO_INSTRUCTIONS` for `BRAINWIRES.md`/`CLAUDE.md`.

---

## `ask_user_question` tool

The agent can pause and ask you directly. Useful when a decision hinges on
information only the user has (a preference, a file path, a yes/no).

### Input

```json
{
  "question": "Which database backend should we target?",
  "options": ["SQLite", "Postgres", "DuckDB"],
  "multi_select": false
}
```

`options` is optional — omit for a free-text prompt. `multi_select` only takes
effect when `options` is non-empty.

### Result

The tool result is a JSON string containing one of:

```json
{"answer":   "Postgres"}    // free-text or single-choice
{"selected": ["a", "c"]}    // multi-select
{"cancelled": true}         // user pressed Esc, or non-TTY env
```

### UI routing

- **TUI mode** (`--tui`): the existing question panel renders the prompt as a
  modal overlay. Navigate with ↑/↓ and Space, submit with Enter, cancel with
  Esc.
- **Plain CLI mode**: falls back to `dialoguer::Select`/`Input`. If stdin
  isn't a TTY (CI, piped input), the tool returns `{"cancelled": true}`
  rather than hanging.

---

## Keybindings

Top-level global TUI shortcuts are remappable under `settings.keybindings`.
Per-mode keys (arrow keys inside dialogs, text editing inside nano, etc.)
stay hardcoded for now — remapping those is a larger refactor.

### Remappable actions

| Action              | Default   | Effect                                           |
|---------------------|-----------|--------------------------------------------------|
| `console_view`      | `Ctrl+D`  | Toggle the console / journal view                |
| `plan_mode_toggle`  | `Ctrl+P`  | Enter/exit plan mode                             |

More actions can be added in later passes — the default fallback restores
the current behavior exactly for anything unset.

### Key-spec grammar

Keys are written as one or more modifiers separated by `+` followed by a
key name. Modifiers (case-insensitive): `Ctrl`, `Alt` (alias `Meta`),
`Shift`. Key names (case-insensitive): a single character, `Esc`,
`Enter`, `Tab`, `Space`, `Backspace`, `Delete`, `Up`, `Down`, `Left`,
`Right`, `Home`, `End`, `PageUp`, `PageDown`, `F1`–`F24`.

Examples: `"Ctrl+D"`, `"Alt+Shift+F4"`, `"Esc"`, `"F1"`, `"PageUp"`.

### Example

Rebind the console view to `Ctrl+K` and leave plan mode on the default:

```json
{
  "keybindings": {
    "global": {
      "console_view": "Ctrl+K"
    }
  }
}
```

Typos and unknown actions are logged via `tracing::warn!` and ignored;
the rest of the config still applies.

## Custom status line

Set `Config.status_line_command` (in `config.json`) to any shell command —
its trimmed stdout is appended to the TUI status bar. Cached for one second
to keep rendering cheap; commands that run longer than 200 ms are killed and
the previous value is retained.

```json
{
  "status_line_command": "gitprompt 2>/dev/null"
}
```
