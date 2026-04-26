#!/usr/bin/env bash
# brainwires-chat-pwa launcher — daemon-mode for both prod and dev.
#
# Usage:
#   ./web/start.sh                  # production (default)
#   ./web/start.sh prod
#   ./web/start.sh dev              # live-edit
#   ./web/start.sh stop             # stop everything
#   ./web/start.sh status           # show running state
#   ./web/start.sh logs [WHICH]     # tail logs
#                                   #   WHICH = esbuild | cargo | compose | container | all
#
# Each start always shuts down any existing instance first (containers +
# host watchers), so switching between prod and dev is seamless and
# idempotent.
#
# prod  → docker compose up -d --build (containers detached).
# dev   → all three loops detached:
#           1. esbuild --watch            (web/src     → web/app.js, web/sw.js)
#           2. cargo-watch + wasm-pack    (wasm/       → web/pkg/)
#           3. docker compose up -d --watch  (host web/ → nginx docroot)
#         With DEV_MODE=true, boot.js unregisters the service worker
#         and clears bw-chat-cache-v1; bw-models-v1 is preserved.
#
# State (PIDs + log files) lives in web/.run/ (gitignored). The script
# returns control to the shell as soon as everything is launched —
# tail logs or stop with the corresponding subcommand.

set -euo pipefail

cd "$(dirname "$0")"
WEB_DIR="$(pwd)"
cd ..
PWA_DIR="$(pwd)"
WASM_CRATE_DIR="$PWA_DIR/wasm"
RUN_DIR="$WEB_DIR/.run"

CMD="${1:-prod}"

# ── Helpers ────────────────────────────────────────────────────────────

stop_pid_file() {
    local file=$1
    [ -f "$file" ] || return 0
    local pid
    pid=$(cat "$file" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        kill -TERM "$pid" 2>/dev/null || true
        for _ in 1 2 3 4 5; do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.2
        done
        kill -KILL "$pid" 2>/dev/null || true
    fi
    rm -f "$file"
}

stop_all() {
    if [ -d "$RUN_DIR" ]; then
        for f in "$RUN_DIR"/*.pid; do
            [ -f "$f" ] || continue
            stop_pid_file "$f"
        done
    fi
    ( cd "$PWA_DIR" && docker compose down --remove-orphans ) >/dev/null 2>&1 || true
}

write_pid() {
    local name=$1 pid=$2
    mkdir -p "$RUN_DIR"
    echo "$pid" > "$RUN_DIR/$name.pid"
}

# ── Subcommands ────────────────────────────────────────────────────────

start_prod() {
    echo "==> stopping any existing chat-pwa instance"
    stop_all

    mkdir -p "$RUN_DIR"
    echo "prod" > "$RUN_DIR/mode"

    echo "==> starting chat-pwa (production, detached)"
    cd "$PWA_DIR"
    DEV_MODE=false docker compose up -d --build

    echo
    echo "Containers detached. Open http://localhost:${HOST_PORT:-8080}"
    echo "  status:  ./web/start.sh status"
    echo "  logs:    ./web/start.sh logs"
    echo "  stop:    ./web/start.sh stop"
}

start_dev() {
    # Pre-flight (dev only)
    command -v wasm-pack >/dev/null \
        || { echo "wasm-pack missing — cargo install wasm-pack" >&2; exit 1; }
    command -v cargo-watch >/dev/null \
        || { echo "cargo-watch missing — cargo install cargo-watch --locked" >&2; exit 1; }
    if [ ! -d "$WEB_DIR/node_modules" ]; then
        echo "==> npm install"
        ( cd "$WEB_DIR" && npm install )
    fi

    echo "==> stopping any existing chat-pwa instance"
    stop_all

    mkdir -p "$RUN_DIR"
    echo "dev" > "$RUN_DIR/mode"

    echo "==> starting chat-pwa (dev, all loops detached)"

    # Watcher 1: esbuild
    ( cd "$WEB_DIR" && exec node build.mjs --watch ) \
        >"$RUN_DIR/esbuild.log" 2>&1 &
    write_pid esbuild "$!"

    # Watcher 2: cargo-watch + wasm-pack — mirrors web/build.sh
    ( exec cargo watch \
        --workdir "$WASM_CRATE_DIR" \
        -w "$WASM_CRATE_DIR/src" \
        -w "$WASM_CRATE_DIR/Cargo.toml" \
        -s "wasm-pack build --target web --release --out-dir \"$WEB_DIR/pkg\" --out-name brainwires_chat_pwa \"$WASM_CRATE_DIR\"" \
    ) >"$RUN_DIR/cargo-watch.log" 2>&1 &
    write_pid cargo-watch "$!"

    # Watcher 3: `docker compose up -d --watch` — containers detach,
    # compose itself stays foreground running the file watcher; we
    # background the whole compose process to free the shell.
    ( cd "$PWA_DIR" && exec env DEV_MODE=true docker compose up --build -d --watch ) \
        >"$RUN_DIR/compose-watch.log" 2>&1 &
    write_pid compose-watch "$!"

    sleep 1
    echo
    echo "All loops detached. State in: $RUN_DIR"
    echo "  open:    http://localhost:${HOST_PORT:-8080}"
    echo "  status:  ./web/start.sh status"
    echo "  logs:    ./web/start.sh logs        (combined)"
    echo "           ./web/start.sh logs esbuild | cargo | compose | container"
    echo "  stop:    ./web/start.sh stop"
}

cmd_stop() {
    echo "==> stopping chat-pwa"
    stop_all
    rm -rf "$RUN_DIR"
    echo "Stopped."
}

cmd_status() {
    if [ ! -d "$RUN_DIR" ] || [ ! -f "$RUN_DIR/mode" ]; then
        echo "chat-pwa: not running"
        return 0
    fi
    local mode
    mode=$(cat "$RUN_DIR/mode")
    echo "chat-pwa: $mode"
    if [ "$mode" = "dev" ]; then
        for name in esbuild cargo-watch compose-watch; do
            local f="$RUN_DIR/$name.pid"
            [ -f "$f" ] || continue
            local pid
            pid=$(cat "$f")
            if kill -0 "$pid" 2>/dev/null; then
                printf '  %-15s pid %-7s running\n' "$name" "$pid"
            else
                printf '  %-15s pid %-7s DEAD\n'    "$name" "$pid"
            fi
        done
    fi
    echo
    ( cd "$PWA_DIR" && docker compose ps )
}

cmd_logs() {
    local what="${1:-all}"
    case "$what" in
        esbuild)
            [ -f "$RUN_DIR/esbuild.log" ] || { echo "no esbuild log (is dev running?)" >&2; exit 1; }
            exec tail -F "$RUN_DIR/esbuild.log"
            ;;
        cargo|cargo-watch|wasm)
            [ -f "$RUN_DIR/cargo-watch.log" ] || { echo "no cargo-watch log (is dev running?)" >&2; exit 1; }
            exec tail -F "$RUN_DIR/cargo-watch.log"
            ;;
        compose|compose-watch)
            [ -f "$RUN_DIR/compose-watch.log" ] || { echo "no compose log (is dev running?)" >&2; exit 1; }
            exec tail -F "$RUN_DIR/compose-watch.log"
            ;;
        container|cnt|nginx)
            cd "$PWA_DIR"
            exec docker compose logs -f
            ;;
        all|*)
            local logs=()
            if [ -d "$RUN_DIR" ]; then
                for f in "$RUN_DIR"/*.log; do
                    [ -f "$f" ] && logs+=("$f")
                done
            fi
            if [ ${#logs[@]} -gt 0 ]; then
                exec tail -F "${logs[@]}"
            else
                cd "$PWA_DIR"
                exec docker compose logs -f
            fi
            ;;
    esac
}

print_help() {
    cat <<USAGE
Usage: $(basename "$0") <command> [args]

Commands:
  prod (default)      Start in production mode (containers detached).
  dev                 Start in dev mode with live-editing (all loops detached).
  stop                Stop everything (containers + host watchers).
  status              Show running state.
  logs [WHICH]        Tail logs.
                      WHICH = esbuild | cargo | compose | container | all (default).
  -h, --help, help    This message.

Each start command always shuts down any existing instance first, so
switching between prod and dev is seamless.
USAGE
}

# ── Dispatch ───────────────────────────────────────────────────────────

case "$CMD" in
    prod|production)    start_prod ;;
    dev|development)    start_dev ;;
    stop|down)          cmd_stop ;;
    status|ps)          cmd_status ;;
    logs|log)           cmd_logs "${2:-all}" ;;
    -h|--help|help)     print_help ;;
    *)
        echo "Unknown command: $CMD (try --help)" >&2
        exit 1
        ;;
esac
