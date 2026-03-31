# brainwires-audio (deprecated)

> **This crate is deprecated.** Use [`brainwires-hardware`](../../crates/brainwires-hardware) with the `audio` feature instead.

```toml
# Before
brainwires-audio = "0.5"

# After
brainwires-hardware = { version = "0.6", features = ["audio"] }
```

All public types, traits, and re-exports are identical. The audio module now lives at
`brainwires_hardware::audio` and all top-level re-exports remain available at the crate root.

This stub will receive one final `0.6.0` release to publish the deprecation notice, then
will not be updated further.
