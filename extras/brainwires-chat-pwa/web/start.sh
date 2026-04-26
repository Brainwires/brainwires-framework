#!/usr/bin/env bash
# brainwires-chat-pwa launcher.
#
# Usage:
#   ./web/start.sh             # production (default)
#   ./web/start.sh prod
#   ./web/start.sh dev         # live-edit with docker compose watch
#
# Always brings down any existing chat-pwa instance before bringing up
# the requested mode, so switching between prod and dev is seamless.
#
# prod  → docker compose up --build (foreground, single command).
# dev   → three loops:
#           1. esbuild  --watch       (web/src       → web/app.js, web/sw.js)
#           2. cargo-watch + wasm-pack (wasm/        → web/pkg/)
#           3. docker compose watch    (host web/    → nginx docroot)
#         Edits to JS/CSS/HTML hit the browser on next reload without an
#         image rebuild. With DEV_MODE=true (which this script exports in
#         dev mode), boot.js unregisters any existing service worker and
#         clears bw-chat-cache-v1; bw-models-v1 is preserved.
#
# Ctrl-C cleans up all three loops in dev mode.

set -euo pipefail

cd "$(dirname "$0")"
WEB_DIR="$(pwd)"
cd ..
PWA_DIR="$(pwd)"
WASM_CRATE_DIR="$PWA_DIR/wasm"

# ── Mode dispatch ─────────────────────────────────────────────────────
MODE="${1:-prod}"
case "$MODE" in
    prod|production) MODE=prod ;;
    dev|development) MODE=dev ;;
    -h|--help|help)
        cat <<USAGE
Usage: $(basename "$0") [prod|dev]

  prod   docker compose up --build (default)
  dev    docker compose watch + esbuild --watch + cargo-watch wasm-pack

Always shuts down any existing chat-pwa instance first, so switching
between modes is seamless.
USAGE
        exit 0
        ;;
    *)
        echo "Unknown mode: $MODE (use prod|dev)" >&2
        exit 1
        ;;
esac

# ── Pre-flight ─────────────────────────────────────────────────────────
command -v docker >/dev/null || { echo "docker missing" >&2; exit 1; }

if [ "$MODE" = "dev" ]; then
    command -v wasm-pack >/dev/null \
        || { echo "wasm-pack missing — cargo install wasm-pack" >&2; exit 1; }
    command -v cargo-watch >/dev/null \
        || { echo "cargo-watch missing — cargo install cargo-watch --locked" >&2; exit 1; }
    if [ ! -d "$WEB_DIR/node_modules" ]; then
        echo "==> npm install"
        ( cd "$WEB_DIR" && npm install )
    fi
fi

# ── Always bring down any existing instance first ──────────────────────
echo "==> stopping any running chat-pwa instance"
( cd "$PWA_DIR" && docker compose down --remove-orphans ) >/dev/null 2>&1 || true

# DEV_MODE flows through docker-compose.yml's ${DEV_MODE:-false} into
# both the build arg and the runtime BRAINWIRES_DEV_MODE env var.
if [ "$MODE" = "dev" ]; then
    export DEV_MODE=true
else
    export DEV_MODE=false
fi

# ── prod: simple foreground compose up ─────────────────────────────────
if [ "$MODE" = "prod" ]; then
    echo "==> starting chat-pwa (production)"
    cd "$PWA_DIR"
    exec docker compose up --build
fi

# ── dev: three watchers + cleanup trap ─────────────────────────────────
PIDS=()
cleanup() {
    echo
    echo "==> stopping dev loops"
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    ( cd "$PWA_DIR" && docker compose down ) >/dev/null 2>&1 || true
}
trap cleanup INT TERM EXIT

# Watcher 1: esbuild
( cd "$WEB_DIR" && node build.mjs --watch ) &
PIDS+=("$!")

# Watcher 2: wasm-pack via cargo-watch.
# Mirrors the wasm-pack invocation in web/build.sh exactly.
( cargo watch \
    --workdir "$WASM_CRATE_DIR" \
    -w "$WASM_CRATE_DIR/src" \
    -w "$WASM_CRATE_DIR/Cargo.toml" \
    -s "wasm-pack build --target web --release --out-dir \"$WEB_DIR/pkg\" --out-name brainwires_chat_pwa \"$WASM_CRATE_DIR\"" \
) &
PIDS+=("$!")

# Watcher 3: docker compose watch (foreground).
echo "==> starting chat-pwa (dev — docker compose watch)"
cd "$PWA_DIR"
docker compose watch
