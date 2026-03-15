# Publishing Checklist

Reusable checklist for releasing new versions of the Brainwires Framework to crates.io.

## 1. Pre-release Checks

- [ ] All changes committed, clean working tree (`git status`)
- [ ] `CHANGELOG.md` has release notes under `## [Unreleased]` (version stamp is automatic — see step 2)
- [ ] **README.md files updated** — verify all crate READMEs reflect the release changes:
  - Root `README.md` — crate descriptions, feature tables, architecture diagrams
  - Each changed crate's `README.md` — API tables, code examples, feature flags
  - `crates/README.md` — dependency tree and crate descriptions
  - `crates/brainwires/README.md` (facade) — feature table, crate count, prelude types
  - `extras/` server READMEs — cross-references to library crates
- [ ] `cargo xtask` passes (fmt, check, clippy, test, doc)
- [ ] **No unfinished code** — run `cargo xtask check-stubs` to scan for runtime-panic stubs and unfinished markers:
  ```bash
  cargo xtask check-stubs            # Should use "--strict"! Only fails on todo!(), unimplemented!(); warn on FIXME, HACK, etc.
  cargo xtask check-stubs --strict   # also fail on comment markers (FIXME, HACK, XXX, STUB, STOPSHIP)
  cargo xtask check-stubs --verbose  # list every file scanned
  ```
  Hard blockers (`todo!()`, `unimplemented!()`) in trait impls or public API must be replaced with `Err(...)` or the module removed before release. Comment markers (FIXME, HACK, etc.) are warnings by default — use `--strict` to enforce zero markers.
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes

## 2. Version Bump

The bump tool has two modes, selected automatically based on whether the version change is a **patch** (same major.minor) or **minor/major** bump.

### Minor / Major bump (all crates)

```bash
cargo xtask bump-version 0.5.0
```

Bumps **every** crate to the new version. This:
- Resets any per-crate version overrides back to `version.workspace = true` (cleans up after previous patch releases)
- Updates `[workspace.package].version` in root `Cargo.toml`
- Updates all `version = "..."` on internal crate deps in `[workspace.dependencies]`
- Updates member `Cargo.toml` files with direct path deps (e.g. brainwires-wasm)
- Updates hardcoded version strings in `*.rs` source files
- Updates `*.md` files (skips CHANGELOG)
- Stamps `CHANGELOG.md`: `## [Unreleased]` → `## [X.Y.Z] - YYYY-MM-DD`

### Patch bump (selective)

```bash
# Auto-detect changed crates from git (uses git diff against last version tag)
cargo xtask bump-version 0.4.1

# Or specify crates manually
cargo xtask bump-version 0.4.1 --crates brainwires-core,brainwires-storage
```

Only bumps **affected crates** + their transitive dependents. This:
- Detects which crates changed since the last version tag (`v0.4.0`), or uses `--crates` if specified
- **Cascades** to all crates that depend (directly or transitively) on any affected crate
- Prints the full list before making changes
- Sets affected crates to explicit `version = "0.4.1"` (overriding `version.workspace = true`)
- Updates `[workspace.dependencies]` version fields for affected crates only
- Updates `.rs` and `.md` files only within affected crate directories
- Stamps `CHANGELOG.md`
- Leaves the workspace root version unchanged (e.g., stays at `0.4.0`)

**Cascade example:**

```
$ cargo xtask bump-version 0.4.1 --crates brainwires-core

Patch bump to 0.4.1:
  Direct:  brainwires-core
  Cascade: brainwires-agents, brainwires-autonomy, brainwires-mcp, ...
  Total:   14 crate(s)
```

On the next minor release (`0.5.0`), all crates reset to `version.workspace = true` automatically.

### After bumping

```bash
git diff                    # Review changes
cargo check --workspace     # Verify it compiles
git add -A && git commit -m "chore: bump version to X.Y.Z"
```

**Note:** The bumper handles version numbers automatically, but you must still manually verify README *content* (descriptions, API tables, architecture diagrams) matches the release changes.

## 3. Publish to crates.io

### Dry run (default)

```bash
./scripts/publish.sh
```

Only leaf crates fully verify in dry-run mode (later layers can't resolve deps not yet on crates.io). This is expected.

### Live publish

```bash
./scripts/publish.sh --live
```

The script handles:
- **Dependency ordering** — 7 layers, 18 crates, leaves first, facade last
- **Rate limiting** — burst 30 at once, then 1/min (18 crates fits in burst)
- **Idempotency** — already-published versions are skipped automatically
- **Tagging** — creates and pushes `vX.Y.Z` git tag on success

### Publish order

| Layer | Crates |
|-------|--------|
| 1 | brainwires-core, brainwires-a2a, brainwires-code-interpreters, brainwires-skills |
| 2 | brainwires-mcp, brainwires-permissions, brainwires-datasets, brainwires-providers, brainwires-storage |
| 3 | brainwires-cognition |
| 4 | brainwires-tool-system, brainwires-agent-network, brainwires-audio, brainwires-training |
| 5 | brainwires-agents, brainwires-wasm |
| 6 | brainwires-autonomy, brainwires-proxy |
| 7 | brainwires (facade) |

## 4. Post-publish

- [ ] Verify on crates.io: `cargo search brainwires`
- [ ] Tag pushed automatically by publish script (`vX.Y.Z`)
- [ ] Update CLI workspace version refs if needed (`/home/nightness/dev/brainwires-cli/Cargo.toml`)

## 5. Troubleshooting

**Publish fails mid-way?** Re-run `./scripts/publish.sh --live` — already-published crates are skipped.

**Rate limited?** Wait a few minutes and re-run. The script handles burst vs sustained rate limits.

**Version conflict?** A crate version already exists on crates.io. Bump to a new patch version and re-publish.
