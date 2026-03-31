#!/usr/bin/env bash
set -euo pipefail

# Brainwires Framework — crates.io publish script
#
# Rate limits for NEW VERSIONS of existing crates (as of 2026):
#   - Burst: 30 new versions at once
#   - After burst: 1 crate per minute
#   - 22 workspace crates total = all within burst → ~5 minutes
#
# Strategy: publish all 22 within the burst window with short index-propagation
# delays between each. If we ever exceed 30, fall back to 1/min after burst.
# Crates are ordered by dependency DAG (leaves first, facade last).
# Deprecated stubs are published separately after all workspace crates.
#
# Usage:
#   ./scripts/publish.sh          # Dry run (default)
#   ./scripts/publish.sh --live   # Actually publish

DRY_RUN=true
if [[ "${1:-}" == "--live" ]]; then
    DRY_RUN=false
    echo "=== LIVE PUBLISH MODE ==="
    echo "This will publish all 22 workspace crates + any unpublished deprecated crates to crates.io."
    echo "Estimated time: ~5 minutes (burst 30, then 1/min)"
    echo "Press Ctrl+C within 5 seconds to abort..."
    sleep 5
fi

# All 20 workspace crates in strict dependency order (leaves → facade).
# Within each layer, crates have no mutual dependencies.
CRATES=(
    # Layer 1: Leaf crates (no internal deps)
    brainwires-core
    brainwires-a2a
    brainwires-code-interpreters
    brainwires-skills
    brainwires-analytics

    # Layer 2: Depend only on core (or leaf crates)
    brainwires-mcp
    brainwires-mcp-server
    brainwires-permissions
    brainwires-datasets
    brainwires-providers
    brainwires-storage

    # Layer 3: Cognition (core + storage)
    brainwires-cognition

    # Layer 4: Tool & network layer
    brainwires-tool-system
    brainwires-agent-network
    brainwires-hardware
    brainwires-training

    # Layer 5: Agents (depends on tool-system, cognition, agent-network)
    brainwires-agents
    brainwires-channels
    brainwires-wasm

    # Layer 6: Top-level (depends on agents, cognition, training)
    brainwires-autonomy
    brainwires-proxy

    # Layer 7: Facade (must be last)
    brainwires
)

BURST_LIMIT=30
BURST_DELAY=15          # seconds between crates in the burst (index propagation)
POST_BURST_DELAY=70     # 1 min 10 sec between crates after burst exhausted

TOTAL=${#CRATES[@]}
PUBLISHED=0
FAILED=0

echo "============================================"
echo "Brainwires Framework — Publish to crates.io"
echo "Mode: $(if $DRY_RUN; then echo 'DRY RUN'; else echo 'LIVE'; fi)"
echo "Crates: $TOTAL"
echo "============================================"

for i in "${!CRATES[@]}"; do
    crate="${CRATES[$i]}"
    n=$((i + 1))

    echo ""
    echo "[$n/$TOTAL] Publishing $crate..."

    if $DRY_RUN; then
        # Dry run: only the leaf crates will fully verify (deps not on crates.io yet)
        if cargo publish --dry-run -p "$crate" 2>&1 | tail -3; then
            echo "OK: $crate (dry run)"
        else
            echo "SKIP: $crate (expected — deps not yet on crates.io)"
        fi
        PUBLISHED=$((PUBLISHED + 1))
        continue
    fi

    # Live publish
    publish_output=$(cargo publish -p "$crate" 2>&1) && publish_rc=0 || publish_rc=$?
    if [ "$publish_rc" -eq 0 ]; then
        echo "OK: $crate"
        PUBLISHED=$((PUBLISHED + 1))
    elif echo "$publish_output" | grep -q "already exists"; then
        echo "SKIP: $crate (already published)"
        PUBLISHED=$((PUBLISHED + 1))
        continue
    else
        echo "$publish_output"
        echo "FAILED: $crate"
        FAILED=$((FAILED + 1))
        echo ""
        echo "Publish failed. $PUBLISHED/$TOTAL published so far."
        echo "Fix the issue and re-run — already-published crates are skipped by crates.io."
        exit 1
    fi

    # Rate limiting: burst the first 30, then wait 1 min between each
    if [ "$n" -lt "$TOTAL" ]; then
        if [ "$n" -lt "$BURST_LIMIT" ]; then
            echo "  [burst $n/$BURST_LIMIT] Waiting ${BURST_DELAY}s..."
            sleep "$BURST_DELAY"
        elif [ "$n" -eq "$BURST_LIMIT" ]; then
            remaining=$((TOTAL - n))
            echo "  [burst exhausted] Switching to 1-minute intervals."
            echo "  $remaining crates remaining (~${remaining} minutes)."
            echo "  Waiting ${POST_BURST_DELAY}s..."
            sleep "$POST_BURST_DELAY"
        else
            remaining=$((TOTAL - n))
            echo "  Waiting 1 minute... ($remaining crates left, ~${remaining} min remaining)"
            sleep "$POST_BURST_DELAY"
        fi
    fi
done

echo ""
echo "============================================"
echo "Done! $PUBLISHED/$TOTAL crates published."
if [ "$FAILED" -gt 0 ]; then
    echo "$FAILED crate(s) failed."
fi
echo "============================================"

# Auto-detect and publish deprecated crates that haven't been published yet.
# Scans deprecated/ for Cargo.toml files, checks crates.io for the version,
# and publishes if needed. These go AFTER workspace crates.
SCRIPT_DIR_DEP="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPRECATED_DIR="$SCRIPT_DIR_DEP/../deprecated"

if [ -d "$DEPRECATED_DIR" ]; then
    for dep_toml in "$DEPRECATED_DIR"/*/Cargo.toml; do
        [ -f "$dep_toml" ] || continue
        dep_dir="$(dirname "$dep_toml")"
        dep_crate=$(grep -m1 '^name' "$dep_toml" | sed 's/.*"\(.*\)"/\1/')
        dep_version=$(grep -m1 '^version' "$dep_toml" | sed 's/.*"\(.*\)"/\1/')

        [ -z "$dep_crate" ] && continue
        [ -z "$dep_version" ] && continue

        # Check if this version is already on crates.io
        crate_info=$(curl -sf "https://crates.io/api/v1/crates/$dep_crate/$dep_version" 2>/dev/null || true)
        if echo "$crate_info" | grep -q '"version"'; then
            echo "[deprecated] SKIP: $dep_crate v$dep_version (already on crates.io)"
            continue
        fi

        echo ""
        echo "[deprecated] Publishing $dep_crate v$dep_version..."

        if $DRY_RUN; then
            if (cd "$dep_dir" && cargo publish --dry-run 2>&1 | tail -3); then
                echo "OK: $dep_crate (dry run)"
            else
                echo "SKIP: $dep_crate (dry run failed — may need workspace crates published first)"
            fi
            continue
        fi

        dep_output=$(cd "$dep_dir" && cargo publish 2>&1) && dep_rc=0 || dep_rc=$?
        if [ "$dep_rc" -eq 0 ]; then
            echo "OK: $dep_crate v$dep_version (deprecated crate published)"
        elif echo "$dep_output" | grep -q "already exists"; then
            echo "SKIP: $dep_crate (already published)"
        else
            echo "$dep_output"
            echo "WARNING: Failed to publish deprecated $dep_crate — non-fatal, continuing."
        fi
    done
fi

# Tag the release after successful publish
if ! $DRY_RUN && [ "$FAILED" -eq 0 ]; then
    # Determine the release version: use the highest version found across all
    # member crates (handles patch bumps where some crates have explicit versions
    # higher than the workspace base version).
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    WORKSPACE_ROOT="$SCRIPT_DIR/.."
    WORKSPACE_TOML="$WORKSPACE_ROOT/Cargo.toml"
    BASE_VERSION=$(grep -m1 '^version' "$WORKSPACE_TOML" | sed 's/.*"\(.*\)"/\1/')
    VERSION="$BASE_VERSION"
    for crate_dir in "$WORKSPACE_ROOT"/crates/*/; do
        crate_toml="$crate_dir/Cargo.toml"
        [ -f "$crate_toml" ] || continue
        v=$(grep -m1 '^version\s*=' "$crate_toml" 2>/dev/null | sed 's/.*"\(.*\)"/\1/' || true)
        if [ -n "$v" ] && [ "$v" != "$BASE_VERSION" ]; then
            # Simple semver comparison: pick the higher version
            if printf '%s\n%s\n' "$VERSION" "$v" | sort -V | tail -1 | grep -qx "$v"; then
                VERSION="$v"
            fi
        fi
    done

    TAG="v${VERSION}"
    echo ""
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        echo "Tag $TAG already exists — skipping."
    else
        echo "Tagging release as $TAG..."
        git tag -a "$TAG" -m "Release $TAG"
        echo "Created tag $TAG"
        echo "Pushing tag to remote..."
        git push origin "$TAG"
    fi
fi
