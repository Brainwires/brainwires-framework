# brainwires-session

Pluggable session-persistence backend for the Brainwires Agent Framework.
A single `SessionStore` trait replaces what had been four different
session-storage patterns across `extras/` apps (in-memory `Vec`, LanceDB,
ad-hoc `Option<SessionStore>`, volatile per-user `DashMap`).

## Features

| Flag     | Default | Enables                                          |
|----------|---------|--------------------------------------------------|
| default  | —       | `InMemorySessionStore` (always available).       |
| `sqlite` | off     | `SqliteSessionStore` — disk-backed, single-node. |

## The trait

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load(&self, id: &SessionId)
        -> Result<Option<Vec<Message>>>;
    async fn save(&self, id: &SessionId, messages: &[Message])
        -> Result<()>;
    async fn list(&self)
        -> Result<Vec<SessionRecord>>;
    async fn list_paginated(&self, opts: ListOptions)
        -> Result<Vec<SessionRecord>>;
    async fn delete(&self, id: &SessionId)
        -> Result<()>;
}
```

All implementations are cheap to `Arc::clone` and safe to call from any
async context. Saves must be atomic — a crash mid-write must leave the
store with either the old or new transcript, never a partial one.

### Pagination

`list_paginated` takes `ListOptions { offset, limit }`. The default
implementation defers to `list()` and slices in memory, so any existing
store works out of the box. `SqliteSessionStore` overrides it to push
`LIMIT / OFFSET` into the query so listing the first page of a 100k-row
table doesn't load every row.

```rust
use brainwires_session::{ListOptions, SessionStore};
let page = store
    .list_paginated(ListOptions { offset: 0, limit: Some(50) })
    .await?;
```

## Usage

```rust
use std::sync::Arc;
use brainwires_session::{InMemorySessionStore, SessionId, SessionStore};

let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
let id = SessionId::new("user-42");

store.save(&id, &agent.messages()).await?;
// …later, possibly a different process with SqliteSessionStore
if let Some(msgs) = store.load(&id).await? {
    agent.restore_messages(msgs);
}
```

### SQLite backend

```rust
use brainwires_session::SqliteSessionStore;
let store = SqliteSessionStore::open("/var/lib/brainwires/sessions.db")?;
```

Schema is auto-migrated on first `open`. Concurrent access is serialised
through a single connection — adequate for a single-node agent process.
For multi-writer workloads, pair with `brainwires-storage` locking or
front with your own pool.

## Types

```rust
pub struct SessionId(String);
pub struct SessionRecord {
    pub id: SessionId,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
pub type ArcSessionStore = Arc<dyn SessionStore>;
```

`SessionRecord` is metadata-only — use `load()` to fetch message content.

## Status

Trait is stable (the `list_paginated` addition has a default impl so
existing backends don't break). `InMemorySessionStore` and
`SqliteSessionStore` are production-ready for single-node use. A
LanceDB-backed vector session store that deduplicates near-identical
conversation turns is tracked in the roadmap.
