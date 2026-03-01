# brainwires-knowledge

[![Crates.io](https://img.shields.io/crates/v/brainwires-knowledge.svg)](https://crates.io/crates/brainwires-knowledge)
[![Documentation](https://img.shields.io/docsrs/brainwires-knowledge)](https://docs.rs/brainwires-knowledge)
[![License](https://img.shields.io/crates/l/brainwires-knowledge.svg)](LICENSE)

Behavioral and personal knowledge systems for the Brainwires Agent Framework.

## Overview

`brainwires-knowledge` provides two complementary knowledge systems that let agents learn and remember across sessions:

- **BKS (Behavioral Knowledge System)** — collective intelligence shared across all clients. Learns universal truths about better ways to accomplish tasks (e.g., "pm2 logs requires --nostream to avoid blocking").
- **PKS (Personal Knowledge System)** — user-scoped facts, preferences, and profile information. Strictly private and synced per-user (e.g., "User prefers Rust", "Current project is brainwires-cli").

**Design principles:**

- **Dual knowledge** — BKS stores objective behavioral truths; PKS stores subjective personal facts
- **EMA confidence** — exponential moving average tracks belief strength with reinforcement and contradiction
- **Time-based decay** — unused knowledge fades naturally, with category-specific decay rates
- **Offline-first** — SQLite-backed local caches with offline queues for when the server is unavailable
- **Sync-ready** — bidirectional sync with the Brainwires server via REST API
- **Implicit detection** — regex-based collectors extract knowledge from natural conversation

```text
  ┌─────────────────────────────────────────────────────────────────────┐
  │                      brainwires-knowledge                           │
  │                                                                     │
  │  ┌─── Behavioral Knowledge System (BKS) ────────────────────────┐  │
  │  │                                                               │  │
  │  │  LearningCollector ──► TruthInferenceEngine                   │  │
  │  │       │                       │                               │  │
  │  │       ▼                       ▼                               │  │
  │  │  LearningSignal ──► BehavioralTruth ──► BehavioralKnowledge  │  │
  │  │  (explicit,          (EMA confidence,     Cache (SQLite)      │  │
  │  │   correction,         decay, version)         │               │  │
  │  │   tool outcome,                               ▼               │  │
  │  │   strategy)           ContextMatcher ◄── KnowledgeApiClient  │  │
  │  │                       (match + conflict)  (server sync)       │  │
  │  └───────────────────────────────────────────────────────────────┘  │
  │                                                                     │
  │  ┌─── Personal Knowledge System (PKS) ──────────────────────────┐  │
  │  │                                                               │  │
  │  │  PersonalFactCollector ──► PersonalFact ──► PersonalKnowledge │  │
  │  │  (regex patterns:          (category-       Cache (SQLite)    │  │
  │  │   identity, pref,           specific decay,      │            │  │
  │  │   capability, ctx,          local_only flag)     ▼            │  │
  │  │   constraint)                                PersonalKnowledge│  │
  │  │                       PersonalFactMatcher ◄── ApiClient       │  │
  │  │                       (relevance scoring,  (user-scoped sync) │  │
  │  │                        profile formatting)                    │  │
  │  │                                                               │  │
  │  │  PksIntegration (coordinator: messages, tools, background)    │  │
  │  └───────────────────────────────────────────────────────────────┘  │
  └─────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-knowledge = "0.1"
```

Record a behavioral truth and match it later:

```rust
use brainwires_knowledge::{
    BehavioralTruth, TruthCategory, TruthSource,
    ContextMatcher, matcher::format_truths_for_prompt,
};

// Create a truth from explicit teaching
let truth = BehavioralTruth::new(
    TruthCategory::CommandUsage,
    "pm2 logs".into(),
    "Use --nostream flag to avoid blocking".into(),
    "pm2 logs streams indefinitely by default".into(),
    TruthSource::ExplicitCommand,
    Some("client-123".into()),
);

assert_eq!(truth.confidence, 0.8); // ExplicitCommand initial confidence

// Match truths against current context
let matcher = ContextMatcher::new(0.5, 30, 10);
let truths = vec![truth];
let matches = matcher.find_matches("run pm2 logs for myapp", truths.iter());

// Format for prompt injection
let prompt_section = format_truths_for_prompt(&matches);
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Enables SQLite persistence (`rusqlite`), HTTP sync (`reqwest`), and Tokio runtime |
| `wasm` | No | WASM-compatible build (no SQLite/HTTP, forwards `brainwires-core/wasm`) |

```toml
# Default (native with SQLite + HTTP sync)
brainwires-knowledge = "0.1"

# WASM target (no persistence or sync)
brainwires-knowledge = { version = "0.1", default-features = false, features = ["wasm"] }
```

## Architecture

### Behavioral Knowledge System (BKS)

#### BehavioralTruth

The core unit of learned knowledge — an objective rule about how to accomplish tasks better.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | UUID |
| `category` | `TruthCategory` | What kind of knowledge (see below) |
| `context_pattern` | `String` | When this truth applies (e.g., `"pm2 logs"`) |
| `rule` | `String` | Human-readable rule |
| `rationale` | `String` | Why this is better |
| `confidence` | `f32` | Current confidence score (0.0–1.0), EMA-weighted |
| `reinforcements` | `u32` | Times confirmed by successful use |
| `contradictions` | `u32` | Times the user chose differently |
| `last_used` | `i64` | Unix timestamp of last use |
| `source` | `TruthSource` | How this truth was learned |
| `version` | `u64` | Server-side version for sync conflict resolution |
| `deleted` | `bool` | Soft-delete flag |

#### TruthCategory

| Variant | Code | Description |
|---------|------|-------------|
| `CommandUsage` | `cmd` | CLI flags and arguments |
| `TaskStrategy` | `task` | Task execution strategies |
| `ToolBehavior` | `tool` | Tool-specific knowledge |
| `ErrorRecovery` | `error` | Error recovery patterns |
| `ResourceManagement` | `resource` | Context window, parallelism decisions |
| `PatternAvoidance` | `avoid` | Anti-patterns to avoid |
| `PromptingTechnique` | `prompt` | Prompting technique effectiveness |
| `ClarifyingQuestions` | `clarify` | Clarifying question effectiveness (AT-CoT) |

#### TruthSource

| Variant | Initial Confidence | Description |
|---------|-------------------|-------------|
| `ExplicitCommand` | 0.8 | User taught via `/learn` command |
| `ConversationCorrection` | 0.6 | User corrected agent mid-conversation |
| `SuccessPattern` | 0.4 | Detected from successful outcomes |
| `FailurePattern` | 0.5 | Detected from failures |

### Learning & Inference

#### LearningCollector

Collects learning signals from all sources and converts them into truths.

**LearningSignal variants:**

| Variant | Fields | Description |
|---------|--------|-------------|
| `ExplicitTeaching` | `rule`, `rationale`, `category`, `context` | User taught via `/learn` |
| `Correction` | `context`, `wrong_behavior`, `right_behavior` | User corrected agent |
| `ToolOutcome` | `tool_name`, `command`, `success`, `error_message`, `execution_time_ms` | Tool result |
| `StrategyOutcome` | `strategy`, `context`, `success`, `details` | Strategy result |

**FailurePattern** tracks repeated failures. When occurrences exceed the configurable `failure_threshold` (default: 3), the collector generates a `PatternAvoidance` truth.

#### TruthInferenceEngine

Converts patterns into truths with built-in knowledge of common fixes:

- `pm2 logs` → `--nostream`
- `docker logs` → `--follow=false`
- `tail -f` → `tail -n`
- `watch` → `-n 1 -e`

Also provides `should_merge()` and `merge_truths()` for deduplication via Jaccard similarity.

### Context Matching

#### ContextMatcher

Finds relevant truths for the current context using tokenized word overlap with stop-word filtering.

```rust
let matcher = ContextMatcher::new(
    0.5,  // min_confidence
    30,   // decay_days
    10,   // max_results
);

let matches = matcher.find_matches("run pm2 logs", truths.iter());
let by_category = matcher.find_by_category(TruthCategory::CommandUsage, truths.iter());
let searched = matcher.search("nostream", truths.iter());
```

**MatchedTruth** contains: `truth` reference, `match_score` (pattern coverage), `effective_confidence` (after decay). The `combined_score()` method multiplies them for ranking.

**ConflictInfo / ConflictType** — `detect_conflict()` checks if user instructions contradict a learned truth:

| ConflictType | Description |
|--------------|-------------|
| `MissingSuggested` | Instruction omits something the truth recommends |
| `UsingAvoided` | Instruction uses something the truth says to avoid |
| `General` | Generic conflict |

`format_truths_for_prompt()` and `format_conflict_prompt()` produce ready-to-inject prompt sections.

### BKS Cache & Sync

#### BehavioralKnowledgeCache (native only)

SQLite-backed local cache with in-memory HashMap for fast access.

```rust
use brainwires_knowledge::BehavioralKnowledgeCache;

let mut cache = BehavioralKnowledgeCache::new("~/.brainwires/knowledge.db", 100)?;

cache.add_truth(truth)?;
let matching = cache.get_matching_truths("pm2 logs myapp");
let reliable = cache.get_reliable_truths(0.5, 30);

// Offline queue
cache.queue_submission(new_truth)?;
cache.queue_feedback(TruthFeedback::reinforcement(id, None))?;

// Server merge (version-based conflict resolution)
let result = cache.merge_from_server(server_truths)?;
// result.added, result.updated, result.conflicts
```

#### KnowledgeApiClient (native only)

REST client for syncing with the Brainwires server.

| Method | Endpoint | Description |
|--------|----------|-------------|
| `sync()` | `POST /api/knowledge/sync` | Bidirectional sync with `SyncRequest`/`SyncResponse` |
| `get_truths()` | `GET /api/knowledge/truths` | Query truths with filters |
| `submit_truth()` | `POST /api/knowledge/truths` | Submit a new truth |
| `reinforce()` | `POST /api/knowledge/truths/{id}/reinforce` | Report successful use |
| `contradict()` | `POST /api/knowledge/truths/{id}/contradict` | Report contradiction |
| `health_check()` | `GET /api/health` | Check server availability |

### Personal Knowledge System (PKS)

#### PersonalFact

A learned fact about the user with category-specific decay rates.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | UUID |
| `category` | `PersonalFactCategory` | Fact category (see below) |
| `key` | `String` | Fact key (e.g., `"preferred_language"`) |
| `value` | `String` | Fact value (e.g., `"Rust"`) |
| `context` | `Option<String>` | When this applies (e.g., `"frontend projects"`) |
| `confidence` | `f32` | EMA-weighted confidence (0.0–1.0) |
| `source` | `PersonalFactSource` | How the fact was learned |
| `local_only` | `bool` | If true, never syncs to server |

#### PersonalFactCategory

| Variant | Code | Decay Days | Description |
|---------|------|-----------|-------------|
| `Identity` | `id` | 180 | Name, role, team, organization |
| `Preference` | `pref` | 60 | Coding style, communication tone, tools |
| `Capability` | `cap` | 90 | Skills, languages, frameworks |
| `Context` | `ctx` | 14 | Current project, recent work, active files |
| `Constraint` | `limit` | 90 | Limitations, access restrictions, time zones |
| `Relationship` | `rel` | 60 | Connections between facts (Zettelkasten-style) |
| `AmbiguityTypePreference` | `amb` | 60 | Disambiguation preferences (AT-CoT) |

#### PersonalFactSource

| Variant | Initial Confidence | Description |
|---------|-------------------|-------------|
| `ExplicitStatement` | 0.9 | User stated via `/profile` command |
| `InferredFromBehavior` | 0.7 | Detected from conversation patterns |
| `ProfileSetup` | 0.85 | From initial profile setup |
| `SystemObserved` | 0.6 | Observed from tool usage patterns |

### Personal Fact Detection

#### PersonalFactCollector

Regex-based implicit fact detection from user messages. Organized by category:

| Category | Example Patterns |
|----------|-----------------|
| **Identity** | `"My name is {Name}"`, `"Call me {Name}"`, `"I work at {Org}"`, `"I'm on the {team} team"` |
| **Preference** | `"I prefer {X} over {Y}"`, `"I like using {tool}"`, `"I'd rather {X} than {Y}"`, `"My favorite {thing} is {X}"` |
| **Capability** | `"I'm proficient in {lang}"`, `"I know {tech}"`, `"I've been using {X} for {N} years"`, `"I'm an expert in {X}"` |
| **Context** | `"I'm working on {project}"`, `"My current project is {X}"`, `"Today I'm {task}"` |
| **Constraint** | `"I can't access {X}"`, `"I'm in the {tz} timezone"`, `"I'm limited to {X}"`, `"I'm not allowed to {X}"` |

```rust
use brainwires_knowledge::PersonalFactCollector;

let collector = PersonalFactCollector::default(); // min_confidence: 0.7, enabled: true
let facts = collector.process_message("I prefer Rust over Python and I work at Anthropic");

// Returns PersonalFact { key: "preference", value: "Rust", category: Preference }
// and    PersonalFact { key: "organization", value: "Anthropic", category: Identity }
```

#### PersonalFactMatcher

Selects relevant facts for context injection using category-priority scoring:

| Category | Priority Multiplier |
|----------|-------------------|
| Identity | 1.2 |
| Constraint | 1.2 |
| Preference | 1.15 |
| Capability | 1.1 |
| AmbiguityTypePreference | 1.1 |
| Context | 1.0 |
| Relationship | 0.9 |

```rust
use brainwires_knowledge::PersonalFactMatcher;

let matcher = PersonalFactMatcher::new(0.5, 15, true);
let relevant = matcher.get_relevant_facts(facts.iter(), Some("working with Rust"));
let formatted = matcher.format_for_context(&relevant);
// Output:
// [User Profile]
// - name: John
// - preference: Rust
// - current_project: brainwires-cli
```

### PKS Cache & Sync

**PersonalKnowledgeCache** (native) — SQLite-backed cache identical in structure to `BehavioralKnowledgeCache` but for personal facts.

**PersonalKnowledgeApiClient** (native) — REST client for user-scoped sync with the Brainwires server (RLS-protected).

**PksIntegration** (native) — coordinator that ties together the collector, cache, tool usage tracking, and background sync. Provides `DetectedFact` with `DetectionSource` (`ImplicitDetection`, `BehavioralInference`, `ServerSync`).

### Confidence & Decay

Both systems use the same EMA and decay formulas:

**EMA confidence update:**

```
reinforce:  new_confidence = α × 1.0 + (1 - α) × old_confidence
contradict: new_confidence = α × 0.0 + (1 - α) × old_confidence
```

Default `α = 0.1` — slow adaptation, stable beliefs.

**Time-based decay:**

```
if days_since_use > decay_threshold:
    confidence × 0.99^(days_since_use - decay_threshold)
```

BKS uses a single `decay_days` setting (default: 30). PKS uses category-specific thresholds (14–180 days).

**Reliability check:** `is_reliable(min_confidence, decay_days)` returns `true` if decayed confidence exceeds the threshold.

## Usage Examples

### Create and reinforce a BehavioralTruth

```rust
use brainwires_knowledge::{BehavioralTruth, TruthCategory, TruthSource};

let mut truth = BehavioralTruth::new(
    TruthCategory::CommandUsage,
    "pm2 logs".into(),
    "Use --nostream flag to avoid blocking".into(),
    "pm2 logs streams indefinitely by default".into(),
    TruthSource::ExplicitCommand,
    None,
);

assert_eq!(truth.confidence, 0.8);

// Reinforce (EMA: 0.1 * 1.0 + 0.9 * 0.8 = 0.82)
truth.reinforce(0.1);
assert!((truth.confidence - 0.82).abs() < 0.001);

// Contradict (EMA: 0.1 * 0.0 + 0.9 * 0.82 = 0.738)
truth.contradict(0.1);
assert!((truth.confidence - 0.738).abs() < 0.001);
```

### LearningCollector processing signals into truths

```rust
use brainwires_knowledge::{LearningCollector, TruthCategory};

let mut collector = LearningCollector::new(3, Some("client-1".into()));

// Explicit teaching
collector.record_explicit_teaching(
    "Use --nostream with pm2 logs",
    Some("Avoids blocking"),
    TruthCategory::CommandUsage,
    Some("pm2 logs"),
);

// Record repeated failures (triggers pattern detection at threshold)
for i in 0..3 {
    collector.record_tool_outcome("bash", &format!("pm2 logs app{}", i), false, Some("timeout"), 30000);
}

let truths = collector.process_signals();
// Returns explicit teaching truth + failure pattern truth
```

### ContextMatcher with conflict detection

```rust
use brainwires_knowledge::{ContextMatcher, matcher::format_truths_for_prompt};

let matcher = ContextMatcher::new(0.5, 30, 10);

// Find matching truths
let matches = matcher.find_matches("run pm2 logs for myapp", truths.iter());

// Detect conflicts with user instructions
if let Some(conflict) = matcher.detect_conflict("show me pm2 logs", &truth) {
    println!("Conflict: {:?} — {}", conflict.conflict_type, conflict.suggested_action);
}

// Format for prompt injection
let section = format_truths_for_prompt(&matches);
```

### PersonalFact creation and category-specific decay

```rust
use brainwires_knowledge::{PersonalFact, PersonalFactCategory, PersonalFactSource};

let mut fact = PersonalFact::new(
    PersonalFactCategory::Preference,
    "preferred_language".into(),
    "Rust".into(),
    None,
    PersonalFactSource::ExplicitStatement,
    false, // sync to server
);

assert_eq!(fact.confidence, 0.9);
assert_eq!(fact.category.decay_days(), 60); // Preferences decay after 60 days

// Context facts decay much faster
let ctx_fact = PersonalFact::new(
    PersonalFactCategory::Context,
    "current_project".into(),
    "brainwires-cli".into(),
    None,
    PersonalFactSource::SystemObserved,
    false,
);
assert_eq!(ctx_fact.category.decay_days(), 14);
```

### PersonalFactCollector implicit detection

```rust
use brainwires_knowledge::PersonalFactCollector;

let collector = PersonalFactCollector::new(0.7, true);

let facts = collector.process_message("My name is John Smith and I'm working on brainwires-cli");
// Detects: Identity/name = "John Smith", Context/current_project = "brainwires-cli"

let facts = collector.process_message("I'm proficient in Rust and I prefer VSCode");
// Detects: Capability/proficient_in = "Rust", Preference/preference = "VSCode"
```

### BehavioralKnowledgeCache with SQLite persistence

```rust
use brainwires_knowledge::BehavioralKnowledgeCache;

// Create file-backed cache (or in-memory for tests)
let mut cache = BehavioralKnowledgeCache::in_memory(100)?;

cache.add_truth(truth)?;

// Query by context
let matches = cache.get_matching_truths("pm2 logs myapp");

// Query with relevance scores
let scored = cache.get_matching_truths_with_scores("pm2 logs", 0.5, 5)?;

// Apply time-based decay to all truths
let decayed_count = cache.apply_decay(30)?;

// Check cache statistics
let stats = cache.stats();
println!("Total: {}, Avg confidence: {:.2}", stats.total_truths, stats.avg_confidence);
```

### PersonalFactMatcher formatting for context injection

```rust
use brainwires_knowledge::PersonalFactMatcher;

let matcher = PersonalFactMatcher::default(); // min: 0.5, max: 15 facts

let relevant = matcher.get_relevant_facts(all_facts.iter(), Some("Rust project"));
let context_block = matcher.format_for_context(&relevant);
let summary = matcher.format_profile_summary(&relevant);
```

## Configuration

### KnowledgeSettings (BKS)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Master toggle |
| `enable_explicit_learning` | `bool` | `true` | Learn from `/learn` command |
| `enable_implicit_learning` | `bool` | `true` | Learn from conversation corrections |
| `enable_aggressive_learning` | `bool` | `true` | Learn from success/failure patterns |
| `min_confidence_to_apply` | `f32` | `0.5` | Minimum confidence to inject into prompt |
| `min_confidence_to_prompt` | `f32` | `0.7` | Minimum confidence to prompt about conflicts |
| `failure_threshold` | `u32` | `3` | Failures before detecting a pattern |
| `ema_alpha` | `f32` | `0.1` | EMA decay factor |
| `decay_days` | `u32` | `30` | Days of non-use before decay starts |
| `sync_interval_secs` | `u64` | `300` | Server sync interval (seconds) |
| `offline_queue_size` | `usize` | `100` | Maximum queued offline submissions |
| `show_applied_truths` | `bool` | `true` | Show when truths are applied |
| `show_conflict_prompts` | `bool` | `true` | Ask user about conflicts |

**Presets:** `KnowledgeSettings::full()`, `KnowledgeSettings::explicit_only()`, `KnowledgeSettings::disabled()`

### PersonalKnowledgeSettings (PKS)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Master toggle |
| `enable_explicit_learning` | `bool` | `true` | Learn from `/profile` command |
| `enable_implicit_learning` | `bool` | `true` | Detect facts from conversation |
| `enable_observed_learning` | `bool` | `true` | Infer from tool usage |
| `min_confidence_to_apply` | `f32` | `0.5` | Minimum confidence to include in context |
| `implicit_detection_confidence` | `f32` | `0.6` | Confidence threshold for implicit detection |
| `ema_alpha` | `f32` | `0.1` | EMA decay factor |
| `sync_interval_secs` | `u64` | `300` | Server sync interval (seconds) |
| `offline_queue_size` | `usize` | `50` | Maximum queued offline submissions |
| `default_local_only` | `bool` | `false` | Never sync new facts to server |
| `show_applied_facts` | `bool` | `false` | Show when facts are applied |

**Presets:** `PersonalKnowledgeSettings::full()`, `PersonalKnowledgeSettings::explicit_only()`, `PersonalKnowledgeSettings::local_only()`, `PersonalKnowledgeSettings::disabled()`

## Integration

Use via the `brainwires` facade crate with the `knowledge` feature, or depend on `brainwires-knowledge` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.1", features = ["knowledge"] }

# Direct
[dependencies]
brainwires-knowledge = "0.1"
```

The `prelude` module re-exports the most commonly used types:

```rust
use brainwires_knowledge::prelude::*;
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
