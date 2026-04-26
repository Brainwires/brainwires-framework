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
    └── src/boot.js            # entry, bootstraps the WASM module
```

`web/pkg/`, `web/app.js`, `web/sw.js`, and `web/build-info.js` are all
build artifacts — they are regenerated on every `./build.sh` and stay
ignored from git.

## Constraints

No model weights are bundled. Every model is fetched from huggingface.co
at runtime. The crate ships only the runtime shell — no `*.gguf`,
`*.safetensors`, or `*.bin` ever ride along in the artifact.
