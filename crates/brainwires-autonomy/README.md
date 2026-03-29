# brainwires-autonomy

Autonomous agent operations for the Brainwires Framework — self-improvement, Git workflows, environment interaction, and human-out-of-loop execution.

## Features

- **Agent Operations** — attention management, health monitoring, hibernation, parallel execution, supervision
- **Self-Improvement** — feedback-driven strategy selection, code quality scanning, test coverage analysis, crash recovery with AI-powered diagnostics
- **Git Workflow Automation** — branch lifecycle, PR management, merge policies, webhook handling
- **CI/CD Orchestrator** — community-driven automation: GitHub Issues → investigate → fix → PR → merge
- **Cron Scheduler** — recurring autonomous tasks with failure policies and rate limiting
- **File System Reactor** — watch directories for changes, debounce events, trigger autonomous actions
- **System Service Management** — controlled systemd, Docker, and process management with safety guardrails
- **GPIO Hardware Access** — re-exported from [`brainwires-hardware`](../brainwires-hardware) — safe GPIO pin management with allow-lists and auto-release for embedded/IoT

## Feature Flags

| Feature | Description |
|---------|-------------|
| `self-improve` | Self-improvement controller, strategies, and crash recovery |
| `eval-driven` | Eval-driven feedback loop (requires `brainwires-eval`) |
| `supervisor` | Agent supervisor with health monitoring and restart |
| `attention` | Attention mechanism with RAG integration |
| `parallel` | Parallel coordinator with optional MDAP |
| `training` | Autonomous training loop |
| `git-workflow` | Automated Git workflow pipeline (issue → PR → merge) |
| `webhook` | Webhook server + CI/CD orchestrator for Git forge events |
| `scheduler` | Cron-based scheduled autonomous tasks |
| `reactor` | File system event reactor with debouncing |
| `services` | System service management (systemd, Docker, processes) |
| `gpio` | GPIO hardware access via `brainwires-hardware` (Linux) |
| `full` | All features enabled |

## Examples

```bash
# Core (no feature flags required)
cargo run -p brainwires-autonomy --example safety_guard
cargo run -p brainwires-autonomy --example health_monitor
cargo run -p brainwires-autonomy --example session_metrics

# Self-improvement
cargo run -p brainwires-autonomy --example self_improve_strategies --features self-improve
cargo run -p brainwires-autonomy --example crash_recovery --features self-improve

# Git workflow & CI/CD
cargo run -p brainwires-autonomy --example git_workflow_pipeline --features git-workflow
cargo run -p brainwires-autonomy --example cicd_orchestrator --features webhook

# Environment interaction
cargo run -p brainwires-autonomy --example cron_scheduler --features scheduler
cargo run -p brainwires-autonomy --example fs_reactor --features reactor
cargo run -p brainwires-autonomy --example service_manager --features services
```

## Safety

All environment-interaction features are designed with strict safety defaults:

- **Services**: read-only by default, hardcoded deny-list for critical system services (`sshd`, `dbus`, `systemd-*`, etc.)
- **GPIO**: empty allow-list by default (no pins accessible), auto-release on agent timeout — see [`brainwires-hardware`](../brainwires-hardware) for GPIO examples
- **Scheduler**: budget tracking, circuit breakers, per-task failure policies
- **Reactor**: rate limiting, debouncing, path allow/deny lists
- **Crash Recovery**: meta-crash detection (aborts if the crash handler itself keeps crashing), max fix attempts

## License

Licensed under either of [Apache License, Version 2.0](../../LICENSE-APACHE) or [MIT License](../../LICENSE-MIT) at your option.
