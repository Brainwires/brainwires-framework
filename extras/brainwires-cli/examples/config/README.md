# Configuration Examples

This directory contains example configuration files for different use cases.

## Available Configurations

### [default.json](default.json)
Balanced settings for general use. Good starting point for most users.
- Standard model selection (Claude Sonnet)
- Basic validation enabled
- Ask-mode permissions for sensitive operations
- Moderate logging

### [high-reliability.json](high-reliability.json)
Maximum correctness for critical tasks.
- MDAP voting enabled (k=5 agents)
- Comprehensive validation including build checks
- File backups before modifications
- Extended logging for audit trails
- All write operations require approval

### [cost-optimized.json](cost-optimized.json)
Minimize API costs while maintaining functionality.
- Smaller/faster models (Haiku)
- Prefers local Ollama when available
- Lower iteration limits
- Auto-approves safe operations
- MDAP disabled
- Minimal logging

## Usage

Copy the desired configuration to `~/.brainwires/config.json`:

```bash
# Use default configuration
cp examples/config/default.json ~/.brainwires/config.json

# Use high-reliability configuration
cp examples/config/high-reliability.json ~/.brainwires/config.json

# Use cost-optimized configuration
cp examples/config/cost-optimized.json ~/.brainwires/config.json
```

Or specify a configuration file at runtime:

```bash
brainwires chat --config path/to/config.json
```

## Configuration Sections

### Provider
Controls AI model selection and fallback behavior:
```json
{
  "provider": {
    "default": "anthropic",
    "model": "claude-sonnet-4-20250514",
    "fallback_providers": ["openai", "ollama"]
  }
}
```

### Agent
Controls agent execution behavior:
```json
{
  "agent": {
    "max_iterations": 100,
    "enable_validation": true,
    "mdap": {
      "enabled": true,
      "preset": "high_reliability"
    }
  }
}
```

### Permissions
Controls what operations require user approval:
```json
{
  "permissions": {
    "mode": "ask",
    "auto_approve": ["read_file"],
    "always_ask": ["bash", "write_file"]
  }
}
```

Permission modes:
- `"auto"` - Automatically approve all operations
- `"ask"` - Ask for approval based on `auto_approve`/`always_ask` lists
- `"reject"` - Reject all operations (read-only mode)

### Storage
Controls conversation persistence and embeddings:
```json
{
  "storage": {
    "database_path": "~/.brainwires/data/lance.db",
    "memory_tiers": {
      "hot_max_messages": 100,
      "warm_max_age_hours": 24
    }
  }
}
```

### Logging
Controls log output:
```json
{
  "logging": {
    "level": "info",
    "file": "~/.brainwires/logs/cli.log"
  }
}
```

Log levels: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`

## Customization

You can merge configurations by starting with a base and overriding specific values. The CLI loads configuration in this order:

1. Built-in defaults
2. `~/.brainwires/config.json` (if exists)
3. `--config` flag (if provided)
4. Command-line flags (highest priority)

## Environment Variables

Some settings can be overridden via environment variables:

| Variable | Description |
|----------|-------------|
| `BRAINWIRES_PROVIDER` | Default provider |
| `BRAINWIRES_MODEL` | Default model |
| `BRAINWIRES_LOG_LEVEL` | Log level |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |

Environment variables take precedence over config file settings.
