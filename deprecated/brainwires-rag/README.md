# brainwires-rag (DEPRECATED)

This crate has been merged into [`brainwires-cognition`](https://crates.io/crates/brainwires-cognition) and [`brainwires-storage`](https://crates.io/crates/brainwires-storage).

Replace in your `Cargo.toml`:

```toml
# Before
brainwires-rag = "0.4"

# After
brainwires-cognition = { version = "0.4", features = ["rag", "spectral", "code-analysis"] }
# Vector DB layer moved to:
brainwires-storage = { version = "0.4", features = ["vector-db"] }
```
