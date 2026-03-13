# brainwires-seal (DEPRECATED)

This crate has been absorbed into [`brainwires-agents`](https://crates.io/crates/brainwires-agents) as a feature-gated module.

Replace in your `Cargo.toml`:

```toml
# Before
brainwires-seal = "0.2"

# After
brainwires-agents = { version = "0.3", features = ["seal"] }
```
