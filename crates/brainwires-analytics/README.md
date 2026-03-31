# brainwires-analytics

Unified analytics collection, persistence, and querying for the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework).

## Overview

`brainwires-analytics` provides a multi-sink analytics dispatcher with 10 typed event variants, a drop-in `tracing-subscriber` layer, and optional SQLite persistence with aggregated reporting.

## Features

- **`AnalyticsCollector`** — multi-sink dispatcher with typed event variants: `ProviderCall`, `AgentRun`, `ToolCall`, `McpRequest`, `ChannelMessage`, `StorageOp`, `NetworkMessage`, `DreamCycle`, `AutonomySession`, `Custom`
- **`AnalyticsLayer`** — drop-in `tracing-subscriber` layer; intercepts known span names automatically without modifying instrumented code
- **`MemoryAnalyticsSink`** — in-process ring buffer
- **`SqliteAnalyticsSink`** + **`AnalyticsQuery`** (feature `sqlite`) — local SQLite persistence with `cost_by_model()`, `tool_frequency()`, `daily_summary()`, `rebuild_summaries()`

## Usage

```toml
[dependencies]
brainwires-analytics = { version = "0.7", features = ["sqlite"] }
```

```rust
use brainwires_analytics::{AnalyticsCollector, MemoryAnalyticsSink, AnalyticsEvent};

let sink = MemoryAnalyticsSink::new(1000);
let collector = AnalyticsCollector::new(vec![Box::new(sink)]);
collector.record(AnalyticsEvent::custom("my_event", serde_json::json!({"key": "value"}))).await;
```

## License

MIT OR Apache-2.0
