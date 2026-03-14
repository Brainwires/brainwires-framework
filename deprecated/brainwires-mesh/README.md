# brainwires-mesh (DEPRECATED)

This crate has been merged into [`brainwires-agent-network`](https://crates.io/crates/brainwires-agent-network) under the `mesh` feature flag.

Replace in your `Cargo.toml`:

```toml
# Before
brainwires-mesh = "0.4"

# After
brainwires-agent-network = { version = "0.4", features = ["mesh"] }
```
