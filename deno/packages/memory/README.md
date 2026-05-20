# @brainwires/memory

Tiered memory orchestration for the Brainwires Agent Framework.

In v0.11.0 this split out of `@brainwires/storage` to mirror the Rust
`brainwires-memory` crate. Tier substrate (StorageBackend trait, embeddings,
domain-store schemas) stays in `@brainwires/storage` and `@brainwires/stores`;
this package layers retention, multi-factor scoring, and hot/warm/cold tier flow
on top.
