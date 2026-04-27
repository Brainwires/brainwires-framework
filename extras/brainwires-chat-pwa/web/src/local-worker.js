// brainwires-chat-pwa — local-model Web Worker
//
// Phase 2 of "make-a-plan-to-bright-scroll": move the WASM module
// entirely off the main thread. Workers have full access to
// `caches` (Cache Storage), so we read the multi-GB model bytes here
// instead of postMessage'ing them across the thread boundary.
//
// Wire protocol (main → worker):
//
//   { requestId, type: 'load',   modelId }
//   { requestId, type: 'chat',   conversationId, messageId, messages, params }
//   { requestId, type: 'cancel', conversationId }
//   { requestId, type: 'unload' }
//
// Wire protocol (worker → main):
//
//   { type: 'load_progress', phase: 'loading'|'ready', modelId }
//   { requestId, type: 'load_done',     modelId }
//   { requestId, type: 'load_error',    error }
//   { type: 'chat_chunk',  conversationId, messageId, delta }
//   { requestId, type: 'chat_done',     conversationId, messageId, usage, tokensReceived }
//   { requestId, type: 'chat_error',    conversationId, messageId, error }
//   { requestId, type: 'cancel_ack',    conversationId }
//   { requestId, type: 'unload_ack' }
//
// The worker's state:
//   - `wasm`     : the wasm-pack module (lazy-init'd on first 'load').
//   - `handle`   : the active LocalModelHandle, or null.
//   - `modelId`  : id of the loaded model.
//   - `inflight` : Map<conversationId, { aborted: boolean, reader }>.

// Resolved against the worker's actual runtime location. The worker
// is always served from `web/local-worker.js` (esbuild bundles to the
// web root), so `./pkg/...` lands on the wasm-pack output directory.
const PKG_URL = new URL('./pkg/brainwires_chat_pwa.js', import.meta.url).href;
const CACHE_NAME = 'bw-models-v1';

// Mirror of the registry in src/model-store.js. Kept in lock-step; the
// only fields we actually need here are the HF (repo, revision) and the
// list of files (kind, filename) so we can build the cache key.
const KNOWN_MODELS = {
    'gemma-4-e2b': {
        id: 'gemma-4-e2b',
        hf: { repo: 'google/gemma-4-e2b', revision: 'main' },
        files: [
            { kind: 'weights', filename: 'model.safetensors' },
            { kind: 'tokenizer', filename: 'tokenizer.json' },
        ],
    },
};

function cacheKey(modelId, filename) {
    const m = KNOWN_MODELS[modelId];
    if (!m) throw new Error(`unknown model: ${modelId}`);
    const rev = encodeURIComponent(m.hf.revision || 'main');
    return `https://huggingface.co/${m.hf.repo}/resolve/${rev}/${filename}`;
}

let wasm = null;
let wasmPromise = null;
let handle = null;
let loadedModelId = null;

const inflight = new Map();

async function getWasm() {
    if (wasm) return wasm;
    if (wasmPromise) return wasmPromise;
    wasmPromise = (async () => {
        const mod = await import(PKG_URL);
        if (typeof mod.default === 'function') await mod.default();
        if (typeof mod.init === 'function') {
            try { mod.init(); } catch (_) { /* idempotent */ }
        }
        wasm = mod;
        return mod;
    })();
    return wasmPromise;
}

async function isDownloaded(modelId) {
    const m = KNOWN_MODELS[modelId];
    if (!m) return false;
    if (typeof caches === 'undefined') return false;
    const cache = await caches.open(CACHE_NAME);
    for (const f of m.files) {
        const hit = await cache.match(cacheKey(modelId, f.filename));
        if (!hit) return false;
    }
    return true;
}

async function getModelBytes(modelId) {
    const m = KNOWN_MODELS[modelId];
    if (!m) throw new Error(`unknown model: ${modelId}`);
    if (typeof caches === 'undefined') throw new Error('Cache Storage unavailable');
    const cache = await caches.open(CACHE_NAME);
    const out = {};
    for (const f of m.files) {
        const hit = await cache.match(cacheKey(modelId, f.filename));
        if (!hit) throw new Error(`model not downloaded: ${modelId} (${f.filename})`);
        const buf = await hit.arrayBuffer();
        out[f.kind] = new Uint8Array(buf);
    }
    if (!out.weights) throw new Error(`model ${modelId} missing weights file`);
    if (!out.tokenizer) out.tokenizer = new Uint8Array(0);
    return out;
}

// ── Message dispatch ───────────────────────────────────────────

self.addEventListener('message', (ev) => {
    const msg = ev.data;
    if (!msg || typeof msg !== 'object') return;
    switch (msg.type) {
        case 'load':   handleLoad(msg);   break;
        case 'chat':   handleChat(msg);   break;
        case 'cancel': handleCancel(msg); break;
        case 'unload': handleUnload(msg); break;
        default: console.error('local-worker: unknown message type', msg.type);
    }
});

async function handleLoad(msg) {
    const { requestId, modelId } = msg;
    try {
        if (handle && loadedModelId === modelId) {
            self.postMessage({ requestId, type: 'load_done', modelId });
            return;
        }
        if (!(await isDownloaded(modelId))) {
            self.postMessage({ requestId, type: 'load_error', error: 'not_downloaded' });
            return;
        }

        // Tell the UI we've left the "verifying" phase and entered the
        // synchronous wasm-init phase. Mirrors the event the main thread
        // used to dispatch in Phase 1.
        self.postMessage({ type: 'load_progress', phase: 'loading', modelId });

        const mod = await getWasm();
        if (typeof mod.init_local_model !== 'function') {
            self.postMessage({
                requestId,
                type: 'load_error',
                error: 'wasm.init_local_model() not available — rebuild the WASM crate',
            });
            return;
        }

        // Drop any previously-loaded handle before replacing it; halves
        // the peak heap footprint when switching models.
        if (handle && typeof handle.free === 'function') {
            try { handle.free(); } catch (_) { /* idempotent */ }
        }
        handle = null;
        loadedModelId = null;

        let { weights, tokenizer } = await getModelBytes(modelId);
        try {
            handle = await mod.init_local_model(weights, tokenizer, modelId);
        } finally {
            // Drop our local refs; wasm has copied the bytes into linear
            // memory by now, so the worker-side ArrayBuffers (~2.5 GB)
            // can be GC'd immediately.
            weights = null;
            tokenizer = null;
        }
        loadedModelId = modelId;

        self.postMessage({ type: 'load_progress', phase: 'ready', modelId });
        self.postMessage({ requestId, type: 'load_done', modelId });
    } catch (err) {
        const error = err && err.message ? err.message : String(err);
        console.error('local-worker: load failed', err);
        self.postMessage({ requestId, type: 'load_error', error });
    }
}

async function handleChat(msg) {
    const { requestId, conversationId, messageId, messages, params } = msg;
    if (handle === null) {
        self.postMessage({
            requestId,
            type: 'chat_error',
            conversationId,
            messageId,
            error: 'no_model_loaded',
        });
        return;
    }
    const mod = await getWasm();
    if (typeof mod.local_chat_stream !== 'function') {
        self.postMessage({
            requestId,
            type: 'chat_error',
            conversationId,
            messageId,
            error: 'wasm.local_chat_stream() not available — rebuild the WASM crate',
        });
        return;
    }

    // Track in-flight stream so 'cancel' can short-circuit it.
    const ctl = { aborted: false, reader: null };
    inflight.set(conversationId, ctl);

    let usage = null;
    let tokensReceived = 0;
    try {
        const stream = await mod.local_chat_stream(
            handle,
            JSON.stringify(messages || []),
            JSON.stringify(params || {}),
        );
        if (!stream || typeof stream.getReader !== 'function') {
            throw new Error('local_chat_stream did not return a ReadableStream');
        }
        const reader = stream.getReader();
        ctl.reader = reader;
        const decoder = new TextDecoder('utf-8');
        let buffer = '';

        while (true) {
            if (ctl.aborted) break;
            const { value, done } = await reader.read();
            if (done) break;
            buffer += decoder.decode(value, { stream: true });
            let nl;
            while ((nl = buffer.indexOf('\n')) !== -1) {
                const line = buffer.slice(0, nl).replace(/\r$/, '');
                buffer = buffer.slice(nl + 1);
                if (line.trim() === '') continue;
                let obj;
                try { obj = JSON.parse(line); } catch (_) { continue; }
                if (!obj || typeof obj !== 'object') continue;
                if (typeof obj.error === 'string' && obj.error !== '') {
                    throw new Error(obj.error);
                }
                if (typeof obj.delta === 'string' && obj.delta !== '') {
                    tokensReceived += 1;
                    self.postMessage({
                        type: 'chat_chunk',
                        conversationId,
                        messageId,
                        delta: obj.delta,
                    });
                }
                if (obj.usage && typeof obj.usage === 'object') usage = obj.usage;
                // obj.finished is informational; the reader's `done` is authoritative.
            }
        }
        // Flush trailing partial line, if any.
        if (!ctl.aborted && buffer.trim() !== '') {
            try {
                const obj = JSON.parse(buffer.trim());
                if (obj && typeof obj.delta === 'string' && obj.delta !== '') {
                    tokensReceived += 1;
                    self.postMessage({
                        type: 'chat_chunk',
                        conversationId,
                        messageId,
                        delta: obj.delta,
                    });
                }
                if (obj && obj.usage) usage = obj.usage;
            } catch (_) { /* ignore */ }
            buffer = '';
        }
        try { reader.releaseLock(); } catch (_) { /* already released */ }

        if (ctl.aborted) {
            self.postMessage({
                requestId,
                type: 'chat_error',
                conversationId,
                messageId,
                error: 'aborted',
            });
        } else {
            self.postMessage({
                requestId,
                type: 'chat_done',
                conversationId,
                messageId,
                usage,
                tokensReceived,
            });
        }
    } catch (err) {
        const error = err && err.message ? err.message : String(err);
        console.error('local-worker: chat failed', err);
        self.postMessage({
            requestId,
            type: 'chat_error',
            conversationId,
            messageId,
            error,
        });
    } finally {
        inflight.delete(conversationId);
    }
}

function handleCancel(msg) {
    const { requestId, conversationId } = msg;
    const ctl = inflight.get(conversationId);
    if (ctl) {
        ctl.aborted = true;
        // The reader will exit on the next iteration; we can also try to
        // cancel it eagerly so wasm stops generating ASAP.
        if (ctl.reader && typeof ctl.reader.cancel === 'function') {
            try { ctl.reader.cancel(); } catch (_) { /* ignore */ }
        }
    }
    self.postMessage({ requestId, type: 'cancel_ack', conversationId });
}

function handleUnload(msg) {
    const { requestId } = msg;
    if (handle && typeof handle.free === 'function') {
        try { handle.free(); } catch (_) { /* idempotent */ }
    }
    handle = null;
    loadedModelId = null;
    self.postMessage({ requestId, type: 'unload_ack' });
}
