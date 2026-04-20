# brainwires-sandbox

Container-based sandboxing for tool execution in the Brainwires framework. Provides a
`Sandbox` trait with Docker and Podman implementations (via [bollard](https://crates.io/crates/bollard))
for isolating tool invocations from the host: resource limits, read-only root
filesystems, network policies, and whitelisted bind mounts. The `HostSandbox`
pass-through runtime is behind the `unsafe-host` feature and is intended strictly
for local development — it performs no isolation and must never be used in
production. `DockerSandbox` is the supported production runtime.

See the [framework README](../../README.md) for how this crate integrates with
`brainwires-tools` executors.
