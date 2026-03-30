# Teaching AI Facts with Personal Knowledge System

**Last Updated:** 2026-01-27

## Problem Statement

Claude's training data has a cutoff date, causing issues like:
- **"Rust 2024 edition is experimental"** (it's actually stable since early 2024)
- **Outdated library versions** in recommendations
- **Missing knowledge about recent events**

## Solution: Personal Knowledge System (PKS)

The Personal Knowledge System lets you **teach the AI facts that persist across all conversations**. Once you teach something, the AI will always remember it!

## Quick Start: `/remember` Command

The easiest way to teach facts:

```bash
# In brainwires-cli interactive chat:
> /remember Rust 2024 edition is stable as of early 2024

✅ Set profile fact

**context_rust_2024_edition** = Rust 2024 edition is stable as of early 2024
Category: Preference

# That's it! The AI now knows this fact forever.
```

### How It Works

1. **You teach the fact:** `/remember <anything>`
2. **Stored locally + synced to server:** `~/.brainwires/personal_facts.db`
3. **Future conversations mention related topics:**
   - SEAL detects entities like "Rust", "2024", "edition"
   - PKS is queried for matching facts
   - Fact is injected into the AI's system prompt
4. **AI responds with correct information!** ✅

## Advanced: `/profile:set` Command

For more control over how facts are stored:

```bash
# Basic usage (syncs to server)
> /profile:set rust_2024_status "stable and production-ready"

# Privacy mode (local only, never syncs)
> /profile:set --local api_key "secret123"

# Multiple word values (automatic joining)
> /profile:set coding_style "prefer functional programming with minimal comments"
```

## Real-World Examples

### Example 1: Current Year and Tech Status

```bash
# Teach current year context
> /remember It's 2026 and Rust 2024 edition is stable
> /remember Next.js 15 introduced Server Components by default
> /remember TypeScript 5.x is the current major version

# Later conversation:
User: "How do I use Rust 2024 async features?"

AI: (sees in system prompt)
# PERSONAL CONTEXT
#
# **context_its_2026_and:**
#   - It's 2026 and Rust 2024 edition is stable (confidence: 0.90)

AI: "Here's how to use Rust 2024 async features. Note that the 2024
     edition is now stable, so you can use it in production..."
```

### Example 2: Project-Specific Knowledge

```bash
> /remember This project uses pnpm instead of npm
> /remember API base URL is https://api.myapp.com/v2
> /remember Database uses PostgreSQL 16 with pgvector extension

# Now when you ask about package management:
User: "How do I install dependencies?"

AI: (sees)
# PERSONAL CONTEXT
#
# **context_this_project_uses:**
#   - This project uses pnpm instead of npm (confidence: 0.90)

AI: "Use `pnpm install` to install dependencies for this project."
```

### Example 3: Team Preferences

```bash
> /remember Team prefers TypeScript over JavaScript
> /remember We follow Airbnb ESLint rules with strict mode
> /remember Pull requests require 2 approvals minimum

# When generating code:
AI: (sees team preferences)
AI: "I'll write this in TypeScript following your team's Airbnb
     ESLint rules with strict mode enabled..."
```

### Example 4: Personal Work Context

```bash
> /profile:set name "Sarah Chen"
> /profile:set role "Senior Backend Engineer"
> /profile:set team "Platform Infrastructure"
> /profile:set timezone "PST"
> /profile:set current_project "Kubernetes migration"

# AI now has context about you:
User: "What should I focus on today?"

AI: "Based on your role as Senior Backend Engineer on the Platform
     Infrastructure team working on the Kubernetes migration, I'd
     suggest prioritizing..."
```

## Privacy Controls

### Local-Only Facts (Never Sync)

```bash
# Sensitive information - stays on your machine ONLY
> /profile:set --local database_password "secret123"
> /profile:set --local ssh_key_path "~/.ssh/id_prod_rsa"
> /profile:set --local internal_api_token "bearer_abc123"

# These facts are stored in ~/.brainwires/personal_facts.db
# but NEVER synced to the server ✅
```

### Server-Synced Facts (Default)

```bash
# Non-sensitive facts sync across devices
> /remember Rust 2024 is stable
> /profile:set preferred_editor "VSCode"
> /profile:set coding_style "functional with minimal comments"

# Available on ALL your devices after sync ✅
```

## Managing Your Facts

### List All Facts

```bash
# Show all facts
> /profile:list

# Filter by category
> /profile:list preference
> /profile:list context
> /profile:list identity
```

**Available categories:**
- `identity`: Name, role, organization, team
- `preference`: Coding style, tools, communication preferences
- `capability`: Skills, languages, expertise levels
- `context`: Current project, active work, environment
- `constraint`: Limitations, restrictions, timezone
- `relationship`: Connections between facts

### Search Facts

```bash
# Find facts containing keyword
> /profile:search rust
> /profile:search project
> /profile:search coding
```

### Delete Facts

```bash
# Delete by ID (shown in /profile:list)
> /profile:delete abc123

# Delete by key (generated from /remember or set in /profile:set)
> /profile:delete context_rust_2024_edition
> /profile:delete preferred_editor
```

### Force Sync

```bash
# Manually trigger server sync (normally automatic)
> /profile:sync
```

### Export/Import Profile

```bash
# Export to JSON file
> /profile:export ~/my-profile.json
> /profile:export  # Defaults to ~/brainwires-profile.json

# Import from JSON file
> /profile:import ~/my-profile.json

# Great for:
# - Backing up your profile
# - Sharing common facts with team
# - Migrating between accounts
```

### View Statistics

```bash
> /profile:stats

# Shows:
# - Total facts count
# - Facts by category
# - Most used facts
# - Recently added facts
# - Sync status
```

## How Context Injection Works

### Behind the Scenes

When you send a message:

1. **SEAL Entity Detection:**
   ```
   User: "How do I use Rust 2024?"
   ↓
   SEAL detects: ["Rust", "2024"]
   ```

2. **PKS Query:**
   ```rust
   // Queries PKS for facts matching detected entities
   pks.get_all_facts()
       .filter(|f| f.key.contains("rust") || f.value.contains("rust"))
       .filter(|f| f.key.contains("2024") || f.value.contains("2024"))
   ```

3. **Context Injection:**
   ```
   System Prompt:

   # PERSONAL CONTEXT

   **context_rust_2024_edition:**
     - Rust 2024 edition is stable as of early 2024 (confidence: 0.90)

   **context_using_rust_2024:**
     - This project uses Rust 2024 edition features (confidence: 0.85)
   ```

4. **AI Response:**
   - AI sees your facts in the prompt
   - Responds with accurate, personalized information
   - Persists across ALL future conversations ✅

### Confidence Scores

Each fact has a confidence score (0.0-1.0):

| Source | Confidence | Description |
|--------|------------|-------------|
| `ExplicitStatement` | 0.90 | You directly told the AI (e.g., `/profile:set`) |
| `ProfileSetup` | 0.85 | Set during initial profile setup |
| `InferredFromBehavior` | 0.70 | AI inferred from your usage patterns |
| `SystemObserved` | 0.60 | System observed your actions |

**Higher confidence = more likely to be injected into context**

## Best Practices

### ✅ Good Uses

```bash
# Correct outdated training data
> /remember Rust 2024 edition is stable

# Project-specific conventions
> /remember This codebase uses Conventional Commits

# Team standards
> /remember Code reviews require security and performance checks

# Personal preferences
> /profile:set preferred_language Rust
> /profile:set code_style "functional with types"

# Current context
> /remember Working on the authentication refactor sprint
```

### ❌ Avoid

```bash
# Don't store ephemeral data that changes frequently
> /remember I'm working on bug #4829  # This will be outdated tomorrow

# Don't store sensitive data without --local
> /profile:set password mySecretPass123  # Use --local flag!

# Don't store implementation details that belong in docs
> /remember Function parseUser takes (data, options) and returns User
# ^ This belongs in code comments, not PKS

# Don't store subjective opinions as universal facts
> /remember React is bad  # Be specific: "Team prefers Vue over React"
```

## Troubleshooting

### Fact Not Appearing in Context

**Check if fact exists:**
```bash
> /profile:search <keyword>
```

**Check confidence score:**
- Facts with confidence < 0.5 may not be injected
- Use `/profile:list` to see confidence scores

**Check SEAL quality:**
- Low SEAL quality score can prevent PKS context injection
- Occurs when entity detection is uncertain

**Verify entity matching:**
- PKS uses substring matching currently
- Fact must contain keywords from your message
- Example: Fact "rust_2024" matches query containing "rust" or "2024"

### Fact Syncing Issues

**Force manual sync:**
```bash
> /profile:sync
```

**Check local database:**
```bash
ls -lh ~/.brainwires/personal_facts.db
```

**Re-import from backup:**
```bash
> /profile:export ~/backup.json  # Create backup first
> /profile:import ~/backup.json  # Restore if needed
```

### Deleting Wrong Facts

**Undo recent addition:**
```bash
# List facts sorted by creation time
> /profile:list

# Delete by ID
> /profile:delete <id>
```

**Import previous export:**
```bash
> /profile:import ~/previous-backup.json
```

## Technical Details

### Storage Location

- **Linux/macOS:** `~/.brainwires/personal_facts.db`
- **Windows:** `%USERPROFILE%\.brainwires\personal_facts.db`

### Database Schema

```sql
CREATE TABLE personal_facts (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    key TEXT NOT NULL UNIQUE,
    value TEXT NOT NULL,
    context TEXT,
    confidence REAL NOT NULL,
    reinforcements INTEGER NOT NULL DEFAULT 0,
    contradictions INTEGER NOT NULL DEFAULT 0,
    last_used INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    source TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    deleted INTEGER NOT NULL DEFAULT 0,
    local_only INTEGER NOT NULL DEFAULT 0
);
```

### Sync Protocol

1. **Local changes queued:** Facts added/modified are queued for sync
2. **Periodic sync:** Every 5 minutes (configurable)
3. **Conflict resolution:** Server wins on conflicts (version-based)
4. **Privacy respected:** `local_only=true` facts NEVER leave your machine

### Performance

- **Lookup latency:** ~5-10ms (in-memory cache)
- **Context injection overhead:** ~15-30ms total
- **Storage:** ~100KB-500KB for 50-200 facts
- **Network:** Only non-local facts synced

## Future Enhancements (Roadmap)

Coming in future releases:

- **Implicit detection:** "Remember: X" automatically creates fact
- **Fuzzy matching:** Better entity recognition (not just substring)
- **Relationship graphs:** Link related facts together
- **Fact suggestions:** AI suggests facts to remember based on corrections
- **Batch import:** Import facts from text files or markdown
- **Fact decay:** Automatically reduce confidence of old, unused facts

## Related Documentation

- **SEAL Integration:** `docs/SEAL_KNOWLEDGE_INTEGRATION.md`
- **Knowledge System:** `crates/brainwires-framework/crates/brainwires-prompting/src/knowledge/mod.rs`
- **Personal Commands:** `src/commands/executor/personal_commands.rs`

## Support

- **GitHub Issues:** https://github.com/anthropics/brainwires-cli/issues
- **Documentation:** `docs/` directory
- **Source Code:** `crates/brainwires-framework/crates/brainwires-prompting/src/knowledge/personal/`

---

**Remember:** Facts persist forever across all conversations. Teach the AI once, benefit forever! 🎉
