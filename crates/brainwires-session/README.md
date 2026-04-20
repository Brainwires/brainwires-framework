# brainwires-session

Pluggable session-persistence backend for the Brainwires Agent Framework.

Unifies what had been four different session-storage patterns across the
`extras/` apps (in-memory `Vec`, LanceDB, optional SessionStore, volatile
per-user DashMap) behind a single trait:

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load(&self, id: &SessionId) -> Result<Option<Vec<Message>>>;
    async fn save(&self, id: &SessionId, messages: &[Message]) -> Result<()>;
    async fn list(&self) -> Result<Vec<SessionRecord>>;
    async fn delete(&self, id: &SessionId) -> Result<()>;
}
```

## Impls

| Feature | Impl | Purpose |
|---------|------|---------|
| default | `InMemorySessionStore` | Tests, ephemeral sessions |
| `sqlite` | `SqliteSessionStore` | Disk-backed, single-node |

## Usage

```rust
use std::sync::Arc;
use brainwires_session::{InMemorySessionStore, SessionId, SessionStore};

let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
let id = SessionId::new("user-42");

store.save(&id, agent.messages()).await?;
// … later
if let Some(msgs) = store.load(&id).await? {
    agent.restore_messages(msgs);
}
```

## Status

Experimental. The trait is stable; additional impls (LanceDB-backed vector
session store) tracked in the roadmap.
