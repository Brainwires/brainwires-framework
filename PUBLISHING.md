# Publishing Checklist

Reusable checklist for releasing new versions of the Brainwires Framework to crates.io.

## 1. Pre-release Checks

- [ ] All changes committed, clean working tree (`git status`)
- [ ] `CHANGELOG.md` updated with new version section
- [ ] **README.md files updated** — verify all crate READMEs reflect the release changes:
  - Root `README.md` — crate descriptions, feature tables, architecture diagrams
  - Each changed crate's `README.md` — API tables, code examples, feature flags
  - `crates/README.md` — dependency tree and crate descriptions
  - `crates/brainwires/README.md` (facade) — feature table, crate count, prelude types
  - `extras/` server READMEs — cross-references to library crates
- [ ] `cargo xtask ci` passes (fmt, check, clippy, test, doc)
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes

## 2. Version Bump

```bash
cargo xtask bump-version X.Y.Z
```

This updates:
- `[workspace.package].version` in root `Cargo.toml`
- All `version = "..."` on internal crate deps in `[workspace.dependencies]`
- Member `Cargo.toml` files with direct path deps (e.g. brainwires-wasm)
- Hardcoded version strings in `*.rs` source files (`"version": "X.Y.Z"`, `version: "X.Y.Z".into()`)
- `*.md` files: both `brainwires-* = { version = "X.Y" }` (inline table) and `brainwires-* = "X.Y"` (simple form). Skips CHANGELOG.

**Note:** The bumper handles version numbers automatically, but you must still manually verify README *content* (descriptions, API tables, architecture diagrams) matches the release changes.

After bumping:

```bash
git diff                    # Review changes
cargo check --workspace     # Verify it compiles
git add -A && git commit -m "chore: bump version to X.Y.Z"
```

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
- **Dependency ordering** — 7 layers, 20 crates, leaves first, facade last
- **Rate limiting** — burst 30 at once, then 1/min (20 crates fits in burst)
- **Idempotency** — already-published versions are skipped automatically
- **Tagging** — creates and pushes `vX.Y.Z` git tag on success

### Publish order

| Layer | Crates |
|-------|--------|
| 1 | brainwires-core, brainwires-a2a, brainwires-code-interpreters, brainwires-skills |
| 2 | brainwires-mcp, brainwires-mdap, brainwires-permissions, brainwires-datasets, brainwires-providers, brainwires-storage |
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
