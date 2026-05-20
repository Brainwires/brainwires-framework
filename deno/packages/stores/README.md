# @brainwires/stores

Domain stores built on top of `@brainwires/storage`'s `StorageBackend` trait.

In v0.11.0 these were extracted out of `@brainwires/storage` to mirror the Rust
restructure (`brainwires-stores`). The schemas live here; the underlying backend
traits (Postgres / Qdrant / SurrealDB / Pinecone / etc.) remain in
`@brainwires/storage`.

## Stores

- **Message store** — chat history with metadata
- **Conversation store** — multi-turn conversation aggregates
- **Task store** — task graph + agent state
- **Plan store** — saved Plan-Work-Judge plan instances
- **Template store** — reusable plan templates with variable substitution

Tiered memory orchestration lives in `@brainwires/memory`.
