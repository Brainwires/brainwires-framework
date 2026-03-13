# brainwires-rag (DEPRECATED)

This crate has been merged into [`brainwires-cognition`](https://crates.io/crates/brainwires-cognition) and [`brainwires-storage`](https://crates.io/crates/brainwires-storage).

Replace in your `Cargo.toml`:

```toml
# Before
brainwires-rag = "0.2"

# After
brainwires-cognition = { version = "0.3", features = ["rag", "spectral", "code-analysis"] }
# Vector DB layer moved to:
brainwires-storage = { version = "0.3", features = ["vector-db"] }
```
