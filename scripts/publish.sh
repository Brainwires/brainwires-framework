#!/usr/bin/env bash
set -euo pipefail

# Brainwires Framework — crates.io publish script
#
# Rate limits for BRAND NEW crates (as of 2026):
#   - Burst: 5 new crates at once
#   - After burst: 1 new crate every 10 minutes
#   - 24 crates total = 5 burst + 19 × 10min = ~3.2 hours
#
# Strategy: publish first 5 quickly (burst), then 10-minute gaps.
# Crates are ordered by dependency DAG (leaves first, facade last).
#
# Usage:
#   ./scripts/publish.sh          # Dry run (default)
#   ./scripts/publish.sh --live   # Actually publish

DRY_RUN=true
if [[ "${1:-}" == "--live" ]]; then
    DRY_RUN=false
    echo "=== LIVE PUBLISH MODE ==="
    echo "This will publish all 24 crates to crates.io."
    echo "Estimated time: ~3.5 hours (new-crate rate limit: burst 5, then 1/10min)"
    echo "Press Ctrl+C within 5 seconds to abort..."
    sleep 5
fi

# All 24 crates in strict dependency order (leaves → facade).
# Within each layer, crates have no mutual dependencies.
CRATES=(
    # Layer 1: Leaf crates (no internal deps)
    brainwires-core
    brainwires-a2a
    brainwires-code-interpreters
    brainwires-skills

    # Layer 2: Depend only on leaves
    brainwires-providers
    brainwires-mcp
    brainwires-mdap
    brainwires-permissions
    brainwires-datasets
    brainwires-rag
    brainwires-mesh

    # Layer 3: Tool & agent layer
    brainwires-tool-system
    brainwires-agents
    brainwires-storage

    # Layer 4: Integration layer
    brainwires-relay
    brainwires-audio
    brainwires-training
    brainwires-brain

    # Layer 5: Higher-level
    brainwires-prompting
    brainwires-seal

    # Layer 6: Top-level
    brainwires-autonomy
    brainwires-wasm
    brainwires-proxy

    # Layer 7: Facade (must be last)
    brainwires
)

BURST_LIMIT=5
BURST_DELAY=15          # seconds between crates in the burst (index propagation)
POST_BURST_DELAY=610    # 10 min 10 sec between crates after burst exhausted

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

    # Rate limiting: burst the first 5, then wait 10 min between each
    if [ "$n" -lt "$TOTAL" ]; then
        if [ "$n" -lt "$BURST_LIMIT" ]; then
            echo "  [burst $n/$BURST_LIMIT] Waiting ${BURST_DELAY}s..."
            sleep "$BURST_DELAY"
        elif [ "$n" -eq "$BURST_LIMIT" ]; then
            remaining=$((TOTAL - n))
            eta_min=$((remaining * 10))
            echo "  [burst exhausted] Switching to 10-minute intervals."
            echo "  $remaining crates remaining (~${eta_min} minutes)."
            echo "  Waiting ${POST_BURST_DELAY}s..."
            sleep "$POST_BURST_DELAY"
        else
            remaining=$((TOTAL - n))
            eta_min=$((remaining * 10))
            echo "  Waiting 10 minutes... ($remaining crates left, ~${eta_min} min remaining)"
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
