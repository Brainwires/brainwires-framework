# CLI Chat Quick Reference

Quick reference for `brainwires chat` modes and options.

## Command Syntax

```bash
brainwires chat [OPTIONS]
```

## Common Options

| Option | Short | Description |
|--------|-------|-------------|
| `--model <MODEL>` | `-m` | Model to use (overrides config) |
| `--provider <PROVIDER>` | `-p` | Provider to use (overrides config) |
| `--system <SYSTEM>` | | Custom system prompt |
| `--prompt <PROMPT>` | | Single-shot mode: one prompt and exit |
| `--batch` | | Batch mode: process stdin line by line |
| `--quiet` | `-q` | Suppress decorative output |
| `--format <FORMAT>` | | Output format: full, plain, or json |
| `--tui` | | Launch full-screen TUI |
| `--mcp-server` | | Run as MCP server (stdio) |
| `--json` | | Output conversation as JSON on exit |

## Quick Examples

### Interactive Chat
```bash
brainwires chat
```

### Single-Shot Queries
```bash
# Basic
brainwires chat --prompt "What is Rust ownership?"

# Scripting-friendly
brainwires chat --prompt "Calculate 7*8" --quiet --format=plain

# JSON output
brainwires chat --prompt "Explain async" --format=json
```

### Batch Processing
```bash
# From file
cat questions.txt | brainwires chat --batch

# Piped input
printf "Q1\nQ2\nQ3\n" | brainwires chat --batch

# JSON output
cat prompts.txt | brainwires chat --batch --format=json > results.json
```

### Piped Input
```bash
# Interactive mode with piped input
echo "What is 2+2?" | brainwires chat

# Quiet piped input
echo "Hello" | brainwires chat --quiet
```

### Shell Integration
```bash
# Capture in variable
RESULT=$(brainwires chat --prompt "What is 5*5?" --quiet --format=plain)

# Git integration
git diff | brainwires chat --prompt "Summarize changes" --quiet

# Code review
cat file.rs | brainwires chat --prompt "Review this code" --format=plain
```

## Output Formats

| Format | Output | Use Case |
|--------|--------|----------|
| `full` | Rich formatted with labels | Default, best for humans |
| `plain` | Just the text | Scripting, parsing |
| `json` | Structured JSON | Automation, data processing |

### Format Examples

**Full (default):**
```
Assistant: Hello! How can I help you today?
```

**Plain:**
```
Hello! How can I help you today?
```

**JSON:**
```json
{
  "model": "gpt-4o",
  "response": "Hello! How can I help you today?"
}
```

## Mode Selection Logic

The CLI automatically selects the mode based on flags:

1. `--mcp-server` → MCP Server Mode
2. `--tui` → TUI Mode
3. `--prompt` → Single-Shot Mode
4. `--batch` → Batch Mode
5. (default) → Interactive Mode

## Stdin Detection

Interactive mode auto-detects input source:
- **Terminal**: Uses interactive prompts
- **Pipe**: Reads lines from stdin

```bash
# Same command works both ways
brainwires chat              # Interactive
echo "Hello" | brainwires chat   # Piped
```

## Combining Flags

Common useful combinations:

```bash
# Ultimate scripting mode
--prompt "..." --quiet --format=plain

# Batch with structured output
--batch --format=json

# Quiet batch processing
--batch --quiet --format=plain

# Single-shot with JSON
--prompt "..." --format=json
```

## Exit Codes

- `0`: Success
- Non-zero: Error (check stderr for details)

## Environment Variables

- `ANTHROPIC_API_KEY`: Anthropic API key
- `OPENAI_API_KEY`: OpenAI API key
- `GOOGLE_API_KEY`: Google API key

## Configuration

Default config location: `~/.brainwires/config.json`

Override with command-line options:
```bash
brainwires chat --model claude-3-5-sonnet-20241022 --provider anthropic
```

## Performance Tips

- Use `--quiet` to disable spinners and decorations
- Use `--format=plain` for fastest parsing
- Use `--batch` for multiple independent queries
- Use `--prompt` for one-off questions

## Full Documentation

See [CLI_CHAT_MODES.md](CLI_CHAT_MODES.md) for comprehensive documentation.
