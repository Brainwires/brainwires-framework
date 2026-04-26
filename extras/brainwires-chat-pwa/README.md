# brainwires-chat-pwa

## Overview

Installable PWA UI over the brainwires framework. Supports cloud providers
plus Hugging-Face-downloaded local models via [candle](https://github.com/huggingface/candle).
Streaming survives screen-off / tab-backgrounded states by routing through a
service worker. Mobile-first.

This extra is intentionally not published to crates.io — it is a deliverable
artifact (a static PWA bundle), not a library.

## Build

```sh
./web/build.sh
```

The script invokes `wasm-pack` against `wasm/`, then bundles JS via esbuild
and patches `sw.js` with SRI hashes. Output lands under `web/`:

- `web/pkg/brainwires_chat_pwa_bg.wasm` — wasm-pack output
- `web/pkg/brainwires_chat_pwa.js` — wasm-pack JS shim
- `web/app.js` (+ sourcemap) — bundled boot module
- `web/sw.js` — service worker with SRI table substituted
- `web/build-info.js` — build timestamp + git SHA

## Dev

```sh
cd web
npm install
npm run serve
```

Serves the bundle on http://127.0.0.1:3000 with esbuild's built-in dev
server. Use `npm run watch` if you only want incremental rebuilds without
the server.

### Live editing against the Docker image

```sh
./web/dev.sh
```

Orchestrates three loops: esbuild `--watch` (JS/SW), cargo-watch +
wasm-pack (Rust → wasm), and `docker compose` with `docker-compose.dev.yml`
overlaying `web/` into the nginx docroot read-only. Combined with
`DEV_MODE=true` (which the overlay forces via `BRAINWIRES_DEV_MODE`),
`boot.js` unregisters any existing service worker and clears
`bw-chat-cache-v1`, so HTML/CSS/JS edits hit the browser on next reload
without an image rebuild. `bw-models-v1` is preserved.

## Layout

```
extras/brainwires-chat-pwa/
├── README.md
├── wasm/                      # Rust → wasm32 crate (cdylib + rlib)
│   ├── Cargo.toml
│   └── src/lib.rs
└── web/                       # Static PWA assets + build glue
    ├── .gitignore
    ├── build.mjs              # esbuild + SRI patcher
    ├── build.sh               # one-shot pipeline (wasm-pack → bundle)
    ├── icons/                 # icon-192.png, icon-512.png
    ├── index.html
    ├── manifest.json
    ├── package.json           # devDependency: esbuild
    ├── styles.css
    ├── sw.source.js           # checked-in SW template
    ├── crypto-store.js        # passphrase-derived key + AES-GCM helpers
    ├── src/                   # boot, views, streaming, providers, voice, …
    │   ├── boot.js            # entry, bootstraps the WASM module
    │   ├── providers/         # anthropic / openai / google / ollama adapters
    │   └── …                  # db, model-store, ui-*, streaming, i18n, utils
    └── tests/
        ├── unit.test.mjs      # node --test, runs in CI
        └── e2e/e2e.test.mjs   # scaffold (currently all test.skip)
```

`web/pkg/`, `web/app.js`, `web/sw.js`, and `web/build-info.js` are all
build artifacts — they are regenerated on every `./build.sh` and stay
ignored from git.

## Tests

```sh
cd web
node --test tests/unit.test.mjs        # streaming, crypto-store, providers, db, utils
node --test tests/e2e/e2e.test.mjs     # scaffold; scenarios are skipped pending a
                                       # browser harness (Thalora / Playwright).
```

## Docker

A multi-stage Dockerfile bundles the wasm + JS pipeline and serves the
result behind nginx.

```sh
# From the workspace root:
docker build -f extras/brainwires-chat-pwa/Dockerfile -t brainwires-chat-pwa .
docker run --rm -p 8080:80 brainwires-chat-pwa
# → http://localhost:8080
```

Or via compose (run from `extras/brainwires-chat-pwa/`):

```sh
docker compose up --build
# → http://localhost:8080  (compose default)

# Or with the example overrides:
cp .env.example .env
docker compose up --build
# → http://localhost:8888
```

`.env` is git-ignored. Compose loads it automatically; anything not set
falls back to the defaults baked into `docker-compose.yml`. Useful keys:

| Var             | Compose default | `.env.example` value | Effect                                    |
|-----------------|-----------------|----------------------|-------------------------------------------|
| `HOST_PORT`     | `8080`          | `8888`               | Host-side port mapped to container `:80`  |
| `DEV_MODE`      | `false`         | `false`              | Enables debug surfaces in the PWA         |
| `BUILD_VERSION` | `0.1.0`         | (commented)          | Stamped into `build-info.js`              |
| `BUILD_COMMIT`  | (auto)          | (commented)          | Stamped into `build-info.js`              |
| `BUILD_DATE`    | (auto)          | (commented)          | Stamped into `build-info.js`              |

The image is ~30 MB at runtime: nginx:alpine plus the static bundle. The
builder stage uses `rust:1-bookworm` + Node 20 + `wasm-pack`; first-time
builds compile the workspace crates the wasm crate depends on, so expect
a few minutes. Subsequent builds reuse Docker layers.

`entrypoint.sh` rewrites `build-info.js` at container start so build
metadata and `DEV_MODE` can be flipped via env vars without rebuilding:

| Env var                       | Effect                              |
|-------------------------------|-------------------------------------|
| `BRAINWIRES_DEV_MODE`         | Sets `DEV_MODE` exported by build-info.js |
| `BRAINWIRES_BUILD_VERSION`    | Overrides `BUILD_VERSION`           |
| `BRAINWIRES_BUILD_COMMIT`     | Overrides `BUILD_GIT`               |
| `BRAINWIRES_BUILD_DATE`       | Overrides `BUILD_TIME`              |

`nginx.conf` ships with a CSP that allows `wasm-unsafe-eval` (with a
fallback for Safari ≤ 16.0), Cross-Origin-Isolation headers for
`SharedArrayBuffer`, long-cache for `*.wasm`, and a no-cache rule for
`/sw.js`. There is no backend in this image — no relay, no TURN, no
proxy. The PWA talks to LLM providers (Anthropic, OpenAI, Gemini, Ollama)
or runs Candle locally in-browser.

## Constraints

No model weights are bundled. Every model is fetched from huggingface.co
at runtime. The crate ships only the runtime shell — no `*.gguf`,
`*.safetensors`, or `*.bin` ever ride along in the artifact.
