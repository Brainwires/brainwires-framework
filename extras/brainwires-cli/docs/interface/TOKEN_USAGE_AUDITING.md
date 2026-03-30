# Token Usage Auditing System

This document describes the token usage auditing system in Brainwires CLI, designed to track, report, and optimize API token consumption across all AI operations.

## Overview

Brainwires CLI uses multiple token-intensive AI techniques including multi-agent orchestration, MDAP voting, infinite context memory, and adaptive prompting. The token auditing system provides visibility into token consumption and helps identify optimization opportunities.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                   Token Usage Auditing System                     │
│                                                                   │
│  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐  │
│  │   Tokenizer    │    │  Cost Tracker  │    │  Audit Logger  │  │
│  │   (Estimator)  │───>│   (Pricing)    │───>│   (Events)     │  │
│  └────────────────┘    └────────────────┘    └────────────────┘  │
│          │                     │                     │           │
│          v                     v                     v           │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                    Session Report Generator                  │ │
│  │  - Per-agent token breakdown                                │ │
│  │  - Efficiency metrics                                       │ │
│  │  - Optimization recommendations                             │ │
│  └─────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Token Estimation (`src/utils/tokenizer.rs`)

Character-based heuristics calibrated for different model families:

| Content Type | Characters per Token |
|-------------|---------------------|
| English Prose | 4.0 |
| Code | 3.5 |
| CJK Text | 1.5-2.0 |

**Model-Specific Calibration:**

```rust
// Anthropic models (slightly more efficient)
chars_per_token_prose: 3.8
chars_per_token_code: 3.3
image_base_tokens: 68

// OpenAI models
chars_per_token_prose: 4.0
chars_per_token_code: 3.5
image_base_tokens: 85

// Google models
chars_per_token_prose: 4.2
chars_per_token_code: 3.8
image_base_tokens: 258
```

### 2. Cost Tracking (`src/utils/cost_tracker.rs`)

Tracks API usage costs with provider-specific pricing:

| Model | Input (per 1K) | Output (per 1K) |
|-------|----------------|-----------------|
| claude-opus-4 | $0.015 | $0.075 |
| claude-3.5-sonnet | $0.003 | $0.015 |
| claude-3-haiku | $0.00025 | $0.00125 |
| gpt-4-turbo | $0.01 | $0.03 |
| gpt-3.5-turbo | $0.0005 | $0.0015 |

**Budget Enforcement:**
- Daily limits
- Monthly limits
- Warning thresholds (default: 80%)

### 3. Usage Events

Each API call generates a `UsageEvent`:

```rust
pub struct UsageEvent {
    timestamp: DateTime<Utc>,
    provider: String,
    model: String,
    input_tokens: u32,
    output_tokens: u32,
    cost_usd: f64,
    session_id: Option<String>,
}
```

## Token-Intensive Features

### Multi-Agent System

When agents are spawned, token usage is tracked per agent:

| Agent Type | Typical Token Range |
|-----------|---------------------|
| Simple Task | 5,000 - 15,000 |
| Complex Task | 15,000 - 50,000 |
| MDAP-Enabled | 45,000 - 150,000 (k × base) |

**MDAP Multiplier:**
- k=3 (default): 3× token usage
- k=5 (high_reliability): 5× token usage
- k=2 (cost_optimized): 2× token usage

### Infinite Context Memory

Token costs for context retrieval:

| Operation | Token Cost |
|-----------|-----------|
| Entity Extraction | 50-200 tokens |
| Context Injection | 500-2,000 tokens |
| Semantic Search Query | 100-500 tokens |

### Adaptive Prompting

Different techniques have varying token overhead:

| Technique | Additional Tokens |
|-----------|------------------|
| Chain of Thought | +200-500 |
| Tree of Thoughts | +500-1,500 |
| Self-Consistency | +100-300 |
| ReAct | +300-800 |

## Session Reports

When an agent loop completes, generate a report:

```
═══════════════════════════════════════════════════════════════
                    TOKEN USAGE REPORT
═══════════════════════════════════════════════════════════════

Session ID: agent-abc123
Duration: 5m 32s
Iterations: 12

┌─────────────────────────────────────────────────────────────┐
│ Token Summary                                               │
├─────────────────────────────────────────────────────────────┤
│ Input Tokens:     23,456                                    │
│ Output Tokens:    12,345                                    │
│ Total Tokens:     35,801                                    │
│ Estimated Cost:   $0.47                                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Breakdown by Phase                                          │
├─────────────────────────────────────────────────────────────┤
│ Planning:         3,200  (8.9%)                             │
│ Tool Execution:   8,456  (23.6%)                            │
│ Validation:       2,100  (5.9%)                             │
│ Response Gen:    22,045  (61.6%)                            │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Efficiency Metrics                                          │
├─────────────────────────────────────────────────────────────┤
│ Tokens per Iteration:  2,983                                │
│ Tokens per Tool Call:  1,128                                │
│ Context Utilization:   67%                                  │
│ Efficiency Rating:     ⭐⭐⭐⭐ (Good)                        │
└─────────────────────────────────────────────────────────────┘

Recommendations:
• Consider using claude-3-haiku for simple file reads
• 3 iterations had redundant context (estimated 2,100 token savings)
• Tool call batching could reduce tokens by ~15%
═══════════════════════════════════════════════════════════════
```

## Configuration

### Enable Token Auditing

In `~/.brainwires/config.json`:

```json
{
  "token_auditing": {
    "enabled": true,
    "report_on_completion": true,
    "warn_threshold_tokens": 100000,
    "budget": {
      "daily_limit_usd": 10.0,
      "monthly_limit_usd": 100.0,
      "warning_threshold": 0.80
    },
    "detailed_breakdown": true
  }
}
```

### CLI Flags

```bash
# Show token usage after each session
brainwires chat --show-tokens

# Set token budget for session
brainwires chat --token-budget 50000

# Export usage report
brainwires usage --export json --period today
```

## Efficiency Optimization Strategies

### 1. Sub-Context Windows

Use smaller, focused context windows for exploration before committing to main context:

```
┌─────────────────────────────────────────────────────────────┐
│ Main Context Window (128K tokens)                           │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ Sub-Context (8K tokens)                                 │ │
│ │ - Quick file exploration                                │ │
│ │ - Codebase search                                       │ │
│ │ - Pattern matching                                      │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│ [Main conversation history - 50K tokens used]               │
│ [Tool results - 30K tokens]                                 │
│ [System prompt - 5K tokens]                                 │
└─────────────────────────────────────────────────────────────┘
```

**Savings:** 40-60% reduction in exploration tokens

### 2. Model Tiering

Use appropriate models for different operations:

| Operation | Recommended Model | Token Efficiency |
|-----------|------------------|------------------|
| Code Generation | claude-opus-4 | 1x (baseline) |
| File Reading | claude-3-haiku | 10x cheaper |
| Search/Exploration | local inference | Free |
| Validation | claude-3-haiku | 10x cheaper |
| Complex Reasoning | claude-opus-4 | 1x |

### 3. Context Pruning

Automatically prune irrelevant context:

- Remove old tool results after use
- Compress file contents to summaries
- Deduplicate repeated information
- Age-based relevance decay

### 4. Prompt Caching

Cache and reuse common prompt patterns:

- System prompts
- Tool definitions
- Capability descriptions
- Code templates

## Monitoring Dashboard

View real-time usage:

```bash
brainwires usage --live

┌─────────────────────────────────────────────────────────────┐
│ Live Token Usage                              [Ctrl+C exit] │
├─────────────────────────────────────────────────────────────┤
│ Session Tokens:    12,456 / 100,000 (12.5%)                │
│ Daily Cost:        $2.34 / $10.00 (23.4%)                  │
│ Rate:              ~1,200 tokens/min                        │
│                                                             │
│ Recent Operations:                                          │
│ [12:34:05] read_file:    234 tokens                        │
│ [12:34:12] write_file:   567 tokens                        │
│ [12:34:18] bash:         189 tokens                        │
│ [12:34:25] query_code:   445 tokens                        │
└─────────────────────────────────────────────────────────────┘
```

## Integration with MDAP

For MDAP-enabled tasks, track voting token costs:

```
MDAP Token Breakdown (k=3):
├── Agent 1: 15,234 tokens
├── Agent 2: 14,987 tokens
├── Agent 3: 15,102 tokens
├── Voting Overhead: 1,200 tokens
└── Total: 46,523 tokens

Consensus Efficiency: 89% (high agreement, low redundancy)
```

## API for Custom Tracking

```rust
use brainwires_cli::utils::{CostTracker, estimate_tokens};

// Create tracker
let mut tracker = CostTracker::new();
tracker.set_session("my-session");

// Track usage
tracker.track_usage("anthropic", "claude-3-sonnet", 1000, 500);

// Get statistics
let stats = tracker.get_stats(TimePeriod::Today);
println!("Today's cost: ${:.4}", stats.total_cost_usd);

// Check budget
match tracker.check_budget() {
    BudgetStatus::Warning { used_pct, .. } => {
        println!("Warning: {}% of budget used", used_pct * 100.0);
    }
    _ => {}
}
```

## Future Enhancements

1. **Predictive Budgeting**: Estimate remaining tokens needed for task completion
2. **Auto-Scaling**: Automatically switch to cheaper models when approaching limits
3. **Cost Anomaly Detection**: Alert on unusual token consumption patterns
4. **Per-Tool Analytics**: Detailed breakdown by tool type
5. **Integration with Billing**: Link to Brainwires Studio billing dashboard

## Related Documentation

- [ARCHITECTURE.md](../ARCHITECTURE.md) - System architecture overview
- [PERMISSION_SYSTEM.md](../agents/PERMISSION_SYSTEM.md) - Resource quotas configuration
- [CLI_CHAT_MODES.md](./CLI_CHAT_MODES.md) - Chat mode token usage patterns
