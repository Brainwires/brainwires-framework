#!/usr/bin/env bash
# brainwires-chat-pwa dev orchestrator. Runs three loops:
#
#   1. esbuild --watch (web/src → web/app.js + web/sw.js)
#   2. cargo watch on wasm/ → wasm-pack build → web/pkg/
#   3. docker compose watch (Compose 2.22+ file-sync)
#
# Edits to JS/CSS/HTML on the host are picked up in the browser
# automatically (esbuild rebuilds + compose-watch syncs the file into
# the nginx docroot + DEV_MODE bypasses the SW). Edits to the Rust wasm
# crate trigger wasm-pack via cargo-watch, and the regenerated
# web/pkg/ files are then synced by compose-watch.
#
# Ctrl-C cleans up all three loops.

set -euo pipefail

cd "$(dirname "$0")"
WEB_DIR="$(pwd)"

# ── Pre-flight ─────────────────────────────────────────────────────────
command -v wasm-pack >/dev/null || { echo "wasm-pack missing — cargo install wasm-pack" >&2; exit 1; }
command -v docker    >/dev/null || { echo "docker missing"                              >&2; exit 1; }

if ! command -v cargo-watch >/dev/null; then
    echo "cargo-watch not found. Install with:"
    echo "    cargo install cargo-watch --locked"
    exit 1
fi

if [ ! -d node_modules ]; then
    echo "==> npm install"
    npm install
fi

# Ensure DEV_MODE is set so docker-compose substitutes it correctly
# (BRAINWIRES_DEV_MODE -> ${DEV_MODE:-false} in docker-compose.yml).
export DEV_MODE=true

# ── Cleanup on exit ────────────────────────────────────────────────────
PIDS=()
cleanup() {
    echo
    echo "==> stopping dev loops"
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    ( cd "$PWA_DIR" && docker compose down ) 2>/dev/null || true
}
trap cleanup INT TERM EXIT

cd ..   # to extras/brainwires-chat-pwa/
PWA_DIR="$(pwd)"
WASM_CRATE_DIR="$PWA_DIR/wasm"

# ── Watcher 1: esbuild ─────────────────────────────────────────────────
( cd "$WEB_DIR" && node build.mjs --watch ) &
PIDS+=("$!")

# ── Watcher 2: wasm-pack via cargo-watch ───────────────────────────────
# Mirrors the wasm-pack invocation in web/build.sh exactly.
( cargo watch \
    --workdir "$WASM_CRATE_DIR" \
    -w "$WASM_CRATE_DIR/src" \
    -w "$WASM_CRATE_DIR/Cargo.toml" \
    -s "wasm-pack build --target web --release --out-dir \"$WEB_DIR/pkg\" --out-name brainwires_chat_pwa \"$WASM_CRATE_DIR\"" \
) &
PIDS+=("$!")

# ── Watcher 3: docker compose watch (foreground) ───────────────────────
docker compose watch
