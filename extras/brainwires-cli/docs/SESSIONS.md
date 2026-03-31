# Sessions

Sessions are the core unit of persistence and multiplexing in brainwires-cli. Each session is a named, resumable conversation with an Agent — it survives terminal disconnections and can be reattached from any terminal.

---

## What is a Session

A **session** is identified by a UUID, e.g. `3f2a8b1c-d4e9-4f01-b712-0a1c3e7d92ab`. It encapsulates:

- The full conversation history (messages, tool executions)
- The Agent's current state and any active tasks
- Two Unix domain sockets for client connectivity (see below)
- Optional LanceDB-backed persistence for conversation + task history

Sessions are stored in:

| Platform | Path |
|----------|------|
| Linux/Unix (XDG) | `$XDG_DATA_HOME/brainwires/sessions/` or `~/.local/share/brainwires/sessions/` |
| macOS | `~/Library/Application Support/brainwires/sessions/` |
| Fallback | `~/.brainwires/sessions/` |

---

## Session Lifecycle

```
Created → Active → [Suspended] → Dead
              ↑          |
              └──────────┘   (reattach)
```

| State | Description |
|-------|-------------|
| **Created** | Session ID generated; daemon process and PTY socket bound |
| **Active** | TUI or client is connected; Agent can process messages |
| **Suspended** | No client connected; Agent continues running in background, responds when reattached |
| **Dead** | Agent exited or crashed; IPC socket is gone; `list_sessions` filters these out |

`list_sessions()` checks for a live `.sock` IPC socket to determine if a session is still alive. Stale `.pty.sock` files with no corresponding live `.sock` are treated as dead.

---

## Dual-Socket Architecture

Each session has **two** sockets:

### PTY Socket (`<session_id>.pty.sock`)

- Carries raw terminal I/O (bytes)
- Used by the terminal client to attach/detach from the TUI
- Communicates window size changes (resize events)
- Managed by the `SessionServer` daemon process

### IPC Socket (`<session_id>.sock`)

- Carries typed `ViewerMessage` / `AgentMessage` JSON over newlines
- Used for programmatic communication: sending messages, checking status
- Encrypted variant (`EncryptedIpcConnection`) uses ChaCha20-Poly1305
- The presence of this socket is the liveness signal for `list_sessions`

Socket paths:

```
{sessions_dir}/{session_id}.pty.sock   ← raw terminal
{sessions_dir}/{session_id}.sock       ← IPC (typed messages)
```

### Message Types

**Viewer → Agent** (`ViewerMessage`):

| Message | Purpose |
|---------|---------|
| `UserInput { content, context_files }` | Submit a user message |
| `Cancel` | Cancel the current streaming response or tool execution |
| `SyncRequest` | Request a full state resync |
| `Detach { exit_when_done }` | Detach viewer (optionally exit when idle) |
| `Exit` | Immediately exit the agent |
| `SlashCommand { command, args }` | Run a slash command remotely |

**Agent → Viewer** (`AgentMessage`):

| Message | Purpose |
|---------|---------|
| `StatusUpdate { agent_id, status, details }` | Agent status changed |
| `TaskResult { agent_id, success, output }` | Task completed |
| `ToolRequest { tool_name, args }` | Agent wants to call a tool |
| `HelpRequest { issue, blocking }` | Agent needs assistance |

---

## CLI Commands

```bash
# Start a new session (creates session, opens TUI)
brainwires chat

# Resume an existing session by ID
brainwires chat --session <session_id>

# Attach to a running session's PTY
brainwires attach <session_id>

# List all live sessions
brainwires sessions
# or
brainwires ls
```

### TUI Background/Suspend

| Key | Effect |
|-----|--------|
| **Ctrl+Z** | Opens dialog: Background or Suspend |
| Background | Detaches TUI; Agent keeps running; reconnect with `brainwires attach <id>` |
| Suspend | Suspends the TUI process (SIGTSTP); resume with `fg` in the terminal |

---

## Sub-Agent Sessions

TaskAgents and MDAP microagents can each have their own IPC socket, enabling direct interaction from the **Sub-Agent Viewer** (`Ctrl+B`).

A sub-agent has an IPC socket when it was spawned with a session ID and its socket file is present at `{sessions_dir}/{agent_id}.sock`. The Sub-Agent Viewer indicates this with a `●` badge in the agent list.

When the IPC socket is available and the right panel is focused, you can compose and send messages directly to a sub-agent — useful for providing guidance mid-task or checking intermediate state.

See [TUI Keyboard Shortcuts — Sub-Agent Viewer](./interface/TUI_KEYBOARD_SHORTCUTS.md#sub-agent-viewer-ctrlb) for keybindings.

---

## Persistence

Conversation history and task state are stored in **LanceDB** (a columnar vector database):

| Data | Storage |
|------|---------|
| Messages | LanceDB `messages` table, per session |
| Tool executions | LanceDB `tool_executions` table, per session |
| Tasks | LanceDB `tasks` table |
| Vector embeddings | LanceDB (for semantic search / infinite context) |

On reattach, the session reloads its message and tool history, restoring the full conversation view. The Journal tree is rebuilt from this flat history on first render.

---

## See Also

- [TUI Keyboard Shortcuts](./interface/TUI_KEYBOARD_SHORTCUTS.md)
- [IPC & Remote Control](./distributed-swarms/IPC_AND_REMOTE_CONTROL.md)
- [CLI Chat Modes](./interface/CLI_CHAT_MODES.md)
