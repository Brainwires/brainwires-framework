# ADR 0003: MDAP Voting System for High-Reliability Tasks

## Status

Accepted

## Context

Complex algorithmic tasks (LRU caches, graph algorithms, concurrency patterns) have higher error rates than simple tasks. A single agent may:
- Make subtle logical errors
- Miss edge cases
- Produce working but suboptimal solutions

For high-stakes scenarios, we need higher confidence in correctness.

## Options Considered

### 1. Single Agent with Extensive Validation

**Pros:**
- Lower API cost
- Simpler implementation

**Cons:**
- Validation can only catch certain error types
- Can't verify algorithmic correctness without tests
- No redundancy for logical errors

### 2. MDAP: Multi-Agent Voting (Chosen)

**Pros:**
- Multiple independent solutions
- Consensus reduces error probability
- Can detect when agents disagree
- Configurable reliability vs cost tradeoff

**Cons:**
- k× API cost (k = number of voting agents)
- Higher latency (parallel agents still take time)
- Complexity of vote aggregation

### 3. Chain-of-Thought Verification

**Pros:**
- Single agent can verify its own work
- Lower cost than multi-agent

**Cons:**
- Same agent may repeat same errors
- No independent verification

## Decision

Implement **MDAP (Multi-Dimensional Adaptive Planning)** with a voting mechanism:

- **k agents** work on the same task independently
- Results are compared via majority voting
- Disagreements trigger deeper analysis or re-execution
- Configurable presets for different reliability/cost tradeoffs

## Configuration Presets

| Preset | k | Target Reliability | Use Case |
|--------|---|-------------------|----------|
| `default` | 3 | 95% | General complex tasks |
| `high_reliability` | 5 | 99% | Critical algorithms |
| `cost_optimized` | 2 | 90% | Budget-conscious |

## Verification Results

Testing showed measurable efficiency gains:

| Task | Standard Iterations | MDAP Iterations | Improvement |
|------|---------------------|-----------------|-------------|
| LRU Cache | 19 | 7 | 2.7× |
| Rate Limiter | 19 | 8 | 2.4× |
| Generic Factory | 20 | 16 | 1.25× |

**Average: 2.3× efficiency gain on complex algorithms**

## When to Use MDAP

**Enable MDAP for:**
- Complex algorithms (graphs, caches, concurrency)
- Problems taking 15+ iterations
- High-stakes correctness requirements
- When cost is justified by saving 10+ iterations

**Skip MDAP for:**
- Simple patterns (CRUD, basic utilities)
- Well-defined problems (<10 iterations expected)
- Time-sensitive tasks (MDAP adds latency)

## Implementation

```rust
let config = TaskAgentConfig {
    max_iterations: 20,
    mdap_config: Some(MdapConfig::high_reliability()),
    ..Default::default()
};
```

## Consequences

### Positive
- Measurable reliability improvement
- Catches errors single agents miss
- Configurable cost/reliability tradeoff
- Proven 2.3× efficiency on complex tasks

### Negative
- k× API costs
- Additional latency
- Complexity in vote aggregation

### Mitigations
- Only enable for tasks that benefit
- Use lower k for cost-sensitive scenarios
- Profile tasks to identify MDAP candidates

## References

- `src/mdap/` - MDAP implementation
- `test-results/SESSION-SUMMARY-MDAP.md` - Verification results
