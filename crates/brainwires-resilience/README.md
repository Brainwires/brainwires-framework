# brainwires-resilience

Provider-layer resilience middleware for the Brainwires Agent Framework.

Wraps any `brainwires_core::Provider` with composable decorators:

- **`RetryProvider`** — exponential backoff with jitter on transient failures (429, 5xx, network). Honors `Retry-After`.
- **`BudgetProvider`** — atomic token / USD / round counters. Pre-flight rejection when caps would be exceeded; post-flight accumulation from `Usage`.
- **`CircuitBreakerProvider`** — half-open state machine keyed by `(provider, model)`. Trips on N consecutive failures; optional fallback.

## Quick start

```rust
use std::sync::Arc;
use brainwires_resilience::{BudgetConfig, BudgetProvider, RetryPolicy, RetryProvider};

let base: Arc<dyn brainwires_core::Provider> = /* your provider */;
let wrapped = Arc::new(BudgetProvider::new(
    RetryProvider::new(base, RetryPolicy::default()),
    BudgetConfig { max_usd_cents: Some(1000), max_tokens: None, max_rounds: Some(50) },
));
```

## Status

Experimental. Not yet wired into the default `ChatAgent` construction — opt-in via `ChatAgentBuilder::with_budget` (coming in a follow-up change).
