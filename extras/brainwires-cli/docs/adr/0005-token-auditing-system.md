# ADR 0005: Token Usage Auditing System

## Status

Accepted

## Context

Brainwires CLI uses multiple token-intensive AI techniques including:
- Multi-agent orchestration (multiple concurrent agents)
- MDAP voting (k× multiplier for consensus)
- Infinite context memory (embedding/retrieval overhead)
- Adaptive prompting (technique-specific overhead)

Without visibility into token consumption:
- Costs can spiral unexpectedly
- Inefficient patterns go undetected
- Budget management is impossible
- Optimization opportunities are missed

The QUICK-NOTES.md explicitly calls out: "We really need a system to audit token usage and make sure we are not overusing tokens or using them inefficiently."

## Options Considered

### 1. External Monitoring Only

**Pros:**
- No code changes needed
- Use provider dashboards (Anthropic Console, OpenAI Usage)

**Cons:**
- No session-level granularity
- Can't correlate with agent/task activity
- No real-time visibility during execution
- Can't implement budget enforcement

### 2. Per-Request Logging Only

**Pros:**
- Simple implementation
- Low overhead

**Cons:**
- No aggregation or reporting
- Manual analysis required
- No optimization insights
- No budget enforcement

### 3. Comprehensive Token Auditing (Chosen)

**Pros:**
- Per-session and per-agent tracking
- Real-time budget enforcement
- Efficiency metrics and recommendations
- Integration with cost tracking

**Cons:**
- Implementation complexity
- Minor performance overhead
- Storage for usage history

## Decision

Implement a **comprehensive token auditing system** with:

1. **Token Estimation**: Character-based heuristics calibrated per model family
2. **Cost Tracking**: Per-event cost calculation with provider-specific pricing
3. **Budget Enforcement**: Daily/monthly limits with warning thresholds
4. **Session Reports**: Detailed breakdown on agent completion
5. **Optimization Recommendations**: Automated suggestions for token savings

## Architecture

```
┌────────────────┐    ┌────────────────┐    ┌────────────────┐
│   Tokenizer    │───>│  Cost Tracker  │───>│  Audit Logger  │
│   (Estimator)  │    │   (Pricing)    │    │   (Events)     │
└────────────────┘    └────────────────┘    └────────────────┘
        │                     │                     │
        v                     v                     v
┌─────────────────────────────────────────────────────────────┐
│                    Session Report Generator                  │
└─────────────────────────────────────────────────────────────┘
```

## Token Estimation Strategy

Use character-based heuristics rather than actual tokenizers:

| Content Type | Chars/Token | Confidence |
|-------------|-------------|------------|
| English Prose | 4.0 | 95% |
| Code | 3.5 | 90% |
| CJK Text | 1.5-2.0 | 85% |

**Rationale:**
- Avoids dependency on tiktoken-rs or provider-specific tokenizers
- Fast (O(n) string scan vs O(n) tokenization)
- Accuracy within 5-10% is sufficient for budgeting
- Can be calibrated per model family

## Budget Enforcement

Three-tier approach:

1. **Warning Threshold** (default 80%): Log warning, continue operation
2. **Soft Limit**: Require confirmation to continue
3. **Hard Limit**: Block further API calls

```rust
pub struct BudgetConfig {
    pub daily_limit_usd: Option<f64>,
    pub monthly_limit_usd: Option<f64>,
    pub warning_threshold: f64, // 0.0 - 1.0
}
```

## Session Reports

Generate on agent completion:

```
TOKEN USAGE REPORT
══════════════════════════════════════════════
Session: agent-abc123 | Duration: 5m 32s

Tokens:  35,801 (Input: 23,456 | Output: 12,345)
Cost:    $0.47

Breakdown:
  Planning:       3,200  ( 8.9%)
  Tool Execution: 8,456  (23.6%)
  Validation:     2,100  ( 5.9%)
  Response Gen:  22,045  (61.6%)

Efficiency: ⭐⭐⭐⭐ (2,983 tokens/iteration)

Recommendations:
• Use claude-3-haiku for simple file reads
• 3 iterations had redundant context
══════════════════════════════════════════════
```

## Optimization Strategies

### 1. Sub-Context Windows

Use smaller context for exploration before committing to main context:

```
Main Context (128K)
├── Sub-Context (8K) - Exploration/search
└── Main conversation history
```

**Estimated savings:** 40-60% on exploration tokens

### 2. Model Tiering

| Operation | Model | Relative Cost |
|-----------|-------|--------------|
| Complex reasoning | claude-opus-4 | 1× |
| Code generation | claude-3.5-sonnet | 0.2× |
| File reading | claude-3-haiku | 0.02× |
| Search | Local inference | 0× |

### 3. Context Pruning

- Age-based relevance decay
- Tool result summarization
- Duplicate detection and removal

## Implementation Details

### Core Types

```rust
pub struct TokenEstimate {
    pub tokens: usize,
    pub confidence: f32,
    pub model_family: ModelFamily,
}

pub struct UsageEvent {
    pub timestamp: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
    pub session_id: Option<String>,
}

pub struct UsageStats {
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_calls: u64,
    pub by_model: HashMap<String, f64>,
    pub by_provider: HashMap<String, f64>,
}
```

### Integration Points

1. **Provider Layer**: Track tokens after each API call
2. **Agent Layer**: Aggregate per-agent usage
3. **CLI Layer**: Display reports, enforce budgets
4. **TUI Layer**: Real-time usage display

## Consequences

### Positive
- Visibility into token consumption patterns
- Budget enforcement prevents cost overruns
- Optimization recommendations improve efficiency
- Per-agent tracking enables performance comparison
- Data for future ML-based optimization

### Negative
- Minor performance overhead (token estimation)
- Storage for usage history (~1KB per session)
- Complexity in report generation

### Mitigations
- Token estimation is O(n) and fast
- Prune history older than 30 days
- Reports generated only on completion

## Future Enhancements

1. **Predictive Budgeting**: Estimate tokens needed for task completion
2. **Auto-Scaling**: Switch to cheaper models near budget limits
3. **Anomaly Detection**: Alert on unusual consumption patterns
4. **Learning**: Use historical data to predict task token needs

## References

- `src/utils/tokenizer.rs` - Token estimation
- `src/utils/cost_tracker.rs` - Cost tracking implementation
- `docs/TOKEN_USAGE_AUDITING.md` - User documentation
- QUICK-NOTES.md - Original requirement
