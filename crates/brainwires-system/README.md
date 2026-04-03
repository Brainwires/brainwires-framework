# brainwires-system

OS-level primitives for the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework): filesystem event watching and system service management (systemd, Docker, processes).

## Features

| Feature    | Description                                                  |
|------------|--------------------------------------------------------------|
| `reactor`  | Filesystem event watcher — watch directories and trigger actions on changes |
| `services` | systemd unit management, Docker container control, process inspection |
| `full`     | All features enabled                                         |

## Usage

```toml
[dependencies]
brainwires-system = { version = "0.8", features = ["full"] }
```

### Filesystem Reactor

Watch directories and react to changes with debouncing and glob-pattern filtering:

```rust
use brainwires_system::reactor::{FsReactor, FsRule};
use brainwires_system::ReactorConfig;

let config = ReactorConfig::default();
let reactor = FsReactor::new(config);
reactor.watch("/path/to/dir", rules).await?;
```

### Service Management

Manage systemd units, Docker containers, and processes with a unified safety layer:

```rust
use brainwires_system::services::{ServiceManager, ServiceConfig};

let config = ServiceConfig { read_only: false, ..Default::default() };
let mgr = ServiceManager::new(config);
mgr.start("my-service").await?;
```

## Part of the Brainwires Framework

This crate is part of the [brainwires](https://crates.io/crates/brainwires) framework and is re-exported under the `system` feature flag:

```toml
brainwires = { version = "0.8", features = ["system"] }
```
