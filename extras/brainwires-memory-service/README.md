# brainwires-memory-service

A Mem0-compatible memory REST API server for Brainwires agents.

Gives every agent a persistent, per-user memory store accessible over HTTP — point any Mem0 SDK client (or plain `curl`) at it and your agents remember things between sessions.

As Nate B Jones put it: *"whoever solves orchestration at infrastructure grade is going to own the most valuable position in the agent stack."* Memory is how agents build context across sessions; this service is the persistence layer for that.

## Quick start

```sh
# Run with default settings (localhost:8765, ~/.local/share/brainwires/memories.db)
cargo run --bin brainwires-memory

# Override via environment variables
MEMORY_HOST=0.0.0.0 MEMORY_PORT=8765 MEMORY_DB=/data/memories.db \
  cargo run --bin brainwires-memory
```

## API

### Add memory

```http
POST /v1/memories
Content-Type: application/json

{
  "memory": "The user prefers concise answers.",
  "user_id": "user-42"
}
```

Or pass raw message history (role + content pairs) and the service extracts each turn as a separate memory:

```http
POST /v1/memories
{
  "messages": [
    { "role": "user", "content": "I prefer Python over Ruby." },
    { "role": "assistant", "content": "Got it, I'll use Python for examples." }
  ],
  "user_id": "user-42"
}
```

Response:
```json
{
  "results": [
    { "id": "uuid", "memory": "I prefer Python over Ruby.", "event": "add" }
  ]
}
```

### List memories

```http
GET /v1/memories?user_id=user-42&page=1&page_size=20
```

### Get a memory

```http
GET /v1/memories/{id}
```

### Search memories

```http
POST /v1/memories/search
{
  "query": "preferred programming language",
  "user_id": "user-42",
  "limit": 5
}
```

### Update a memory

```http
PATCH /v1/memories/{id}
{ "memory": "Updated content." }
```

### Delete a memory

```http
DELETE /v1/memories/{id}
```

### Delete all memories for a user

```http
DELETE /v1/memories?user_id=user-42
```

### Health check

```http
GET /health
→ { "status": "ok" }
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `MEMORY_HOST` | `127.0.0.1` | Bind address |
| `MEMORY_PORT` | `8765` | Listen port |
| `MEMORY_DB` | `~/.local/share/brainwires/memories.db` | SQLite database path |
| `RUST_LOG` | `brainwires_memory_service=info` | Log filter |

## Using with Mem0 SDK

```python
from mem0 import MemoryClient

client = MemoryClient(host="http://localhost:8765", api_key="unused")
client.add("I prefer Rust over Go", user_id="user-42")
results = client.search("programming language preference", user_id="user-42")
```

## Architecture

```
┌──────────────────────────────────┐
│        brainwires-memory         │
│                                  │
│  POST /v1/memories               │
│  GET  /v1/memories               │  Axum HTTP server
│  POST /v1/memories/search   ─────┼──► MemoryStore (SQLite WAL)
│  PATCH/DELETE /v1/memories/{id}  │
│  GET  /health                    │
└──────────────────────────────────┘
```

The server is stateless apart from the SQLite file — scale horizontally by pointing multiple instances at a shared network filesystem or swap the store for a Postgres backend.

## License

MIT OR Apache-2.0
