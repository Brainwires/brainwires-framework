# Contributing to Brainwires Framework

Thank you for your interest in contributing! This guide will help you get started.

## Getting Started

**Prerequisites:**
- Rust 1.91+ (edition 2024)
- `cargo` (comes with Rust)

```bash
git clone https://github.com/Brainwires/brainwires-framework.git
cd brainwires-framework
cargo build
cargo test
```

## Project Structure

The framework is a Cargo workspace organized around a facade pattern. For the full list of crates and architecture details, see the [README](README.md) and [crates overview](crates/README.md). Standalone apps built on the framework live in [`extras/`](extras/README.md).

## Development Workflow

### Building

```bash
# Full workspace
cargo build

# Single crate
cargo build -p brainwires-agents

# With specific features
cargo build --features "providers,storage,rag"

# All features
cargo build --all-features
```

Feature flag bundles: `researcher`, `agent-full`, `learning`, `full`. See the root `Cargo.toml` for the complete list.

### Testing

```bash
# All tests
cargo test

# Single crate
cargo test -p brainwires-core

# Specific test
cargo test -p brainwires-agents test_task_agent

# With output
cargo test -- --nocapture
```

See [TESTING.md](TESTING.md) for the evaluation framework (`brainwires-eval`).

### Linting

```bash
cargo clippy --all-features
cargo fmt --check
```

## Code Style

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(agents): add retry logic to task orchestrator
fix(rag): correct chunk overlap calculation
docs(changelog): update for 0.1.0 release
refactor(providers): split into protocol-based modules
chore: update dependencies
```

### Documentation

All crates enforce `#![deny(missing_docs)]`. Every public item needs a `///` doc comment.

### Changelog

We follow [Keep a Changelog](https://keepachangelog.com/). If your change is user-facing, add an entry under `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md), grouped by crate:

```markdown
### Added
#### Agents (`brainwires-agents`)
- New retry strategy for task execution
```

## Pull Requests

1. Branch from `main`
2. Make your changes with tests
3. Ensure `cargo test` and `cargo clippy --all-features` pass
4. Update CHANGELOG.md for user-facing changes
5. Open a PR with a clear description of what and why

## Extending the Framework

The framework is designed for extension via traits. See [docs/EXTENSIBILITY.md](docs/EXTENSIBILITY.md) for:

- Custom AI providers (`Provider` trait)
- Custom embeddings (`EmbeddingProvider` trait)
- Custom vector stores (`VectorStore` trait)
- Custom tools (`ToolExecutor` trait)
- Custom agent runtimes (`AgentRuntime` trait)
- Working examples in `crates/brainwires/examples/`

## License

Brainwires Framework is dual-licensed under [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE). By contributing, you agree that your contributions will be licensed under the same terms.
