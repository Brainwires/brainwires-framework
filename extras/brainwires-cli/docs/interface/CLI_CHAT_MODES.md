# CLI Chat Modes

Brainwires CLI provides flexible chat modes designed for different workflows, from interactive conversations to automated scripting and batch processing.

## Overview

The `brainwires chat` command supports multiple modes:

- **Interactive Mode**: Traditional conversational interface
- **Single-Shot Mode** (`--prompt`): Execute one prompt and exit
- **Batch Mode** (`--batch`): Process multiple prompts from stdin
- **TUI Mode** (`--tui`): Full-screen terminal interface
- **MCP Server Mode** (`--mcp-server`): Expose as MCP server over stdio

## Command Line Options

```bash
brainwires chat [OPTIONS]

Options:
  -m, --model <MODEL>        Model to use (overrides config)
  -p, --provider <PROVIDER>  Provider to use (overrides config)
      --system <SYSTEM>      Custom system prompt
      --tui                  Use full-screen TUI mode
      --json                 Output full conversation as JSON on exit
      --mcp-server           Run as MCP server (stdio protocol)
      --prompt <PROMPT>      Single-shot mode: send one prompt and exit
  -q, --quiet                Quiet mode: suppress decorative output
      --batch                Batch mode: process multiple prompts from stdin
      --format <FORMAT>      Output format: full, plain, or json [default: full]
  -h, --help                 Print help
```

## Interactive Mode (Default)

The traditional conversational interface with rich formatting.

### Usage

```bash
brainwires chat
```

### Features

- Interactive prompts with `dialoguer`
- Auto-detection of piped input vs. terminal input
- Conversation history management
- Slash commands support (`/clear`, `/resume`, `/checkpoint`, etc.)
- Auto-save after each exchange
- Tool execution with visual feedback
- Typing effect for responses

### Example

```bash
$ brainwires chat
Brainwires Chat
───────────────
Model: gpt-4o (brainwires)
Conversation ID: abc123...
Type your message or 'exit' to quit

You: Explain Rust ownership
Assistant: Rust ownership is a memory management system...

You: exit
```

### Piped Input

Interactive mode automatically handles piped input:

```bash
echo "What is 2+2?" | brainwires chat
# Processes input and displays response

printf "Question 1\nQuestion 2\nexit\n" | brainwires chat
# Processes multiple prompts in sequence
```

## Single-Shot Mode

Execute a single prompt and exit immediately - perfect for scripting and command-line integration.

### Usage

```bash
brainwires chat --prompt "Your question here"
```

### Features

- No conversation loop - one prompt, one response, then exit
- Fast execution
- Clean output options
- Tool execution supported
- Perfect for scripts and automation

### Examples

**Basic usage:**
```bash
brainwires chat --prompt "What is Rust ownership?"
# Output: Full formatted response with "Assistant:" label
```

**Plain output for scripting:**
```bash
brainwires chat --prompt "What is 2+2?" --format=plain
# Output: Just the answer text
```

**Quiet mode for clean output:**
```bash
brainwires chat --prompt "Calculate 7*8" --quiet --format=plain
# Output: Minimal, just the result
```

**JSON output:**
```bash
brainwires chat --prompt "Explain async/await" --format=json
# Output: {"model": "gpt-4o", "response": "..."}
```

**Capture output in shell variable:**
```bash
ANSWER=$(brainwires chat --prompt "What is 15% of 200?" --quiet --format=plain)
echo "The answer is: $ANSWER"
```

**Code review:**
```bash
brainwires chat --prompt "Review this code: $(cat myfile.rs)" --format=plain
```

**Git integration:**
```bash
git diff | brainwires chat --prompt "Summarize these changes" --quiet
```

## Batch Mode

Process multiple prompts from stdin, one per line. Each prompt is processed independently (no conversation context between prompts).

### Usage

```bash
cat prompts.txt | brainwires chat --batch
```

### Features

- Read prompts from stdin, one per line
- Each prompt processed independently
- No conversation context between prompts
- Multiple output formats
- Error handling per prompt

### Examples

**From a file:**
```bash
cat questions.txt | brainwires chat --batch
```

**Piped input:**
```bash
printf "What is 2+2?\nWhat is 10-3?\nWhat is 5*2?\n" | brainwires chat --batch
```

**Full format output (default):**
```bash
$ printf "What is 2+2?\nWhat is 3+3?\n" | brainwires chat --batch
Q: What is 2+2?
A: 4

Q: What is 3+3?
A: 6
```

**Plain format (responses only):**
```bash
$ printf "What is 2+2?\nWhat is 3+3?\n" | brainwires chat --batch --format=plain
4
6
```

**JSON format (structured output):**
```bash
cat prompts.txt | brainwires chat --batch --format=json > results.json
```

Output:
```json
{
  "model": "gpt-4o",
  "results": [
    {
      "prompt": "What is 2+2?",
      "response": "4"
    },
    {
      "prompt": "What is 3+3?",
      "response": "6"
    }
  ]
}
```

**Quiet batch mode:**
```bash
cat prompts.txt | brainwires chat --batch --quiet
# Minimal output, no decorative text
```

## TUI Mode

Full-screen terminal user interface with rich formatting, visual controls, and background session support.

### Usage

```bash
brainwires chat
# or explicitly:
brainwires chat --tui
```

### Features

- Full-screen interface with split architecture (Agent + TUI Viewer)
- Message history scrolling
- Multiline input
- ANSI color and formatting support
- Tool execution visualization
- Background session support (detach/reattach without losing state)
- Event-driven IPC communication
- Keyboard shortcuts (see [TUI_KEYBOARD_SHORTCUTS.md](TUI_KEYBOARD_SHORTCUTS.md))

### Session Management

The TUI uses a split architecture where:
- **Agent Process**: Runs in the background, holds all session state (conversation, MCP connections, tokio runtime)
- **TUI Viewer**: Thin terminal client that can attach/detach without losing state

**Backgrounding a Session:**
1. Press `Ctrl+Z` to open the background/suspend dialog
2. Choose "Background" to detach the TUI while keeping the Agent running
3. Use `brainwires attach` later to reconnect

**Session Commands:**
```bash
# List backgrounded sessions
brainwires sessions

# Attach to most recent session
brainwires attach

# Attach to specific session
brainwires attach <session-id>

# Terminate a backgrounded session
brainwires exit <session-id>
```

**Exit Behavior:**
- `Ctrl+C` or `/exit`: Quits TUI AND shuts down the Agent
- `Ctrl+Z` → Background: Detaches TUI, keeps Agent running

### Example

```bash
brainwires chat
# Launches full-screen interface with background Agent

# Later, if backgrounded:
brainwires sessions
# Backgrounded sessions:
#   session-20251219-133511 (running)

brainwires attach session-20251219-133511
# Reconnects to the existing session
```

## MCP Server Mode

Expose Brainwires CLI as an MCP (Model Context Protocol) server over stdio.

### Usage

```bash
brainwires chat --mcp-server
```

### Features

- Stdio-based MCP protocol
- All CLI tools exposed as MCP tools
- Compatible with Claude Desktop and other MCP clients
- See [MCP_SERVER.md](MCP_SERVER.md) for details

## Output Formats

Control how responses are formatted using the `--format` option.

### Full Format (Default)

Rich formatting with labels, colors, and typing effects.

```bash
brainwires chat --prompt "Hello" --format=full
```

Output:
```
Assistant: Hello! How can I help you today?
```

Features:
- "Assistant:" label
- Colors and styling
- Typing effect (in interactive mode)
- Full conversation context

### Plain Format

Just the response text, no decoration.

```bash
brainwires chat --prompt "Hello" --format=plain
```

Output:
```
Hello! How can I help you today?
```

Features:
- No labels or formatting
- No colors
- Perfect for parsing in scripts
- Clean output for pipelines

### JSON Format

Structured JSON output with metadata.

```bash
brainwires chat --prompt "Hello" --format=json
```

Output:
```json
{
  "model": "gpt-4o",
  "response": "Hello! How can I help you today?"
}
```

For batch mode:
```json
{
  "model": "gpt-4o",
  "results": [
    {
      "prompt": "Question 1",
      "response": "Answer 1"
    },
    {
      "prompt": "Question 2",
      "response": "Answer 2"
    }
  ]
}
```

Features:
- Structured output
- Model information
- Easy to parse
- Perfect for automation

## Quiet Mode

Suppress decorative output for clean scripting.

### Usage

```bash
brainwires chat --quiet
```

### Features

- No welcome banner
- No spinners or progress indicators
- No "Saving conversation..." messages
- Clean output only
- Works with all modes

### Examples

```bash
# Quiet interactive mode
echo "What is 2+2?" | brainwires chat --quiet

# Quiet single-shot
brainwires chat --prompt "Calculate 3*7" --quiet

# Ultimate scripting mode
brainwires chat --prompt "What is Rust?" --quiet --format=plain

# Quiet batch mode
cat prompts.txt | brainwires chat --batch --quiet --format=json
```

## Combining Options

Mix and match options for powerful workflows:

```bash
# Single-shot + quiet + plain = perfect for scripts
RESULT=$(brainwires chat --prompt "What is 7*8?" --quiet --format=plain)

# Batch + JSON = structured data processing
cat prompts.txt | brainwires chat --batch --format=json | jq '.results'

# Single-shot + JSON = API-like behavior
brainwires chat --prompt "Summarize: $(cat doc.md)" --format=json

# Quiet + plain = clean pipeline
git log --oneline -5 | brainwires chat --prompt "Summarize commits" --quiet --format=plain
```

## Practical Use Cases

### 1. Quick Questions

```bash
brainwires chat --prompt "What is the capital of France?" --quiet --format=plain
```

### 2. Code Review

```bash
git diff HEAD~1 | brainwires chat --prompt "Review these changes" --format=plain
```

### 3. Batch Processing

```bash
# Create prompts file
cat > prompts.txt << EOF
What is 2+2?
What is the capital of France?
Explain Rust ownership in one sentence
EOF

# Process all prompts
cat prompts.txt | brainwires chat --batch --format=json > results.json

# Parse results
jq '.results[] | .response' results.json
```

### 4. Shell Scripts

```bash
#!/bin/bash

# Get AI analysis
ANALYSIS=$(brainwires chat --prompt "Analyze this log: $(cat error.log)" --quiet --format=plain)

# Use in script
if echo "$ANALYSIS" | grep -q "critical"; then
    echo "Critical issue detected!"
fi
```

### 5. Git Commit Messages

```bash
# Generate commit message from diff
MSG=$(git diff --staged | brainwires chat --prompt "Write a commit message for these changes" --quiet --format=plain)
git commit -m "$MSG"
```

### 6. Documentation Generation

```bash
# Generate docs for all functions
for file in src/*.rs; do
    brainwires chat --prompt "Document this code: $(cat $file)" --quiet --format=plain > "docs/$(basename $file .rs).md"
done
```

## Stdin Detection

The CLI automatically detects whether input is coming from a terminal or a pipe:

- **Terminal**: Uses interactive prompts with `dialoguer`
- **Pipe**: Reads lines directly from stdin

This means you can use the same command both interactively and in scripts:

```bash
# Interactive
brainwires chat

# Piped
echo "Hello" | brainwires chat
```

## Conversation Management

### Interactive Mode
- Conversations are auto-saved to the database
- Each session gets a unique conversation ID
- Use `--json` to export conversation on exit
- Resume conversations with `brainwires history open <id>`

### Single-Shot Mode
- No conversation history saved
- Each invocation is independent
- Use `--format=json` to get structured output

### Batch Mode
- Each prompt processed independently
- No conversation context between prompts
- Use `--format=json` for structured results

## Error Handling

### Single-Shot Mode
- Errors printed to stderr
- Exit code reflects success/failure
- JSON format includes error in response

### Batch Mode
- Each prompt's errors handled independently
- In JSON mode, errors included in results:
  ```json
  {
    "prompt": "...",
    "error": "Error message"
  }
  ```
- In plain/full mode, errors printed to stderr
- Processing continues on error

## Performance Tips

1. **Use `--quiet`** when you don't need visual feedback
2. **Use `--format=plain`** when parsing output
3. **Use batch mode** for multiple independent queries
4. **Avoid TUI mode** in scripts (use CLI modes instead)
5. **Use single-shot** for one-off queries

## Related Documentation

- [Slash Commands](SLASH_COMMANDS_RAG.md)
- [TUI Keyboard Shortcuts](TUI_KEYBOARD_SHORTCUTS.md)
- [MCP Server](MCP_SERVER.md)
