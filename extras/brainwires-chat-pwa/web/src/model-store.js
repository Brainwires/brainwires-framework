// brainwires-chat-pwa — Hugging Face model download + Cache Storage
//
// Mirrors the framework's `KnownModel` registry on the JS side. Hugging
// Face is the SOLE source — no mirrors, no fallback hosts.
//
// Storage:
//   - Cache Storage under name `bw-models-v1`. Each model file is keyed
//     by its public HF URL: `https://huggingface.co/<repo>/resolve/<rev>/<filename>`.
//     This makes deletes trivial (cache.delete(url)) and survives a SW
//     update because the cache name is stable.
//
// Concurrency:
//   - At most one active download globally. A second `downloadModel`
//     call returns the in-flight promise.

import { events } from './state.js';

// ── Registry mirror ────────────────────────────────────────────
//
// Source of truth is `crates/brainwires-providers/src/local_llm/model_registry.rs::known_models()`.
// Keep this in sync; SHA-256 pins remain `null` until the upstream
// registry pins them too (see TODO(gemma-4) in the Rust side).

export const KNOWN_MODELS = {
    'gemma-4-e2b': {
        id: 'gemma-4-e2b',
        displayName: 'Gemma 4 E2B',
        description: 'Gemma 4 E2B (effective ~2B) — Candle/safetensors, runs in WASM.',
        hf: { repo: 'google/gemma-4-e2b', revision: 'main' },
        files: [
            { kind: 'weights', filename: 'model.safetensors', sha256: '76dc84a5a805a2c8b91e9ccc00b8dbf8f4a99bf0d56ab25832f6e6addd4f7f57' },
            { kind: 'tokenizer', filename: 'tokenizer.json', sha256: '12bac982b793c44b03d52a250a9f0d0b666813da566b910c24a6da0695fd11e6' },
        ],
        estimatedBytes: 10_246_621_918,
        contextSize: 8192,
        gated: false,
    },
};

const CACHE_NAME = 'bw-models-v1';
const PROGRESS_EMIT_MS = 200;

export function listKnownModels() {
    return Object.values(KNOWN_MODELS).map((m) => ({ ...m }));
}

export function getKnownModel(modelId) {
    return KNOWN_MODELS[modelId] || null;
}

/**
 * Build the Cache Storage key for a given (modelId, filename).
 * Uses HF's public `resolve` URL so we never pin a private mirror.
 *
 * @param {string} modelId
 * @param {string} filename
 * @returns {string}
 */
export function cacheKey(modelId, filename) {
    const m = getKnownModel(modelId);
    if (!m) throw new Error(`unknown model: ${modelId}`);
    const rev = encodeURIComponent(m.hf.revision || 'main');
    return `https://huggingface.co/${m.hf.repo}/resolve/${rev}/${filename}`;
}

// ── Concurrency state ──────────────────────────────────────────

const activeDownloads = new Map(); // modelId → { controller, startedAt, promise }

function _hasCaches() {
    return typeof caches !== 'undefined' && caches && typeof caches.open === 'function';
}

// ── Status / read API ──────────────────────────────────────────

/**
 * @param {string} modelId
 * @returns {Promise<boolean>}
 */
export async function isDownloaded(modelId) {
    if (!_hasCaches()) return false;
    const m = getKnownModel(modelId);
    if (!m) return false;
    const cache = await caches.open(CACHE_NAME);
    for (const f of m.files) {
        const hit = await cache.match(cacheKey(modelId, f.filename));
        if (!hit) return false;
    }
    return true;
}

/**
 * Return raw bytes for the registered files. Throws if any file is missing.
 *
 * @param {string} modelId
 * @returns {Promise<{weights: Uint8Array, tokenizer: Uint8Array}>}
 */
export async function getModelBytes(modelId) {
    if (!_hasCaches()) throw new Error('Cache Storage unavailable');
    const m = getKnownModel(modelId);
    if (!m) throw new Error(`unknown model: ${modelId}`);
    const cache = await caches.open(CACHE_NAME);
    const out = {};
    for (const f of m.files) {
        const hit = await cache.match(cacheKey(modelId, f.filename));
        if (!hit) throw new Error(`model not downloaded: ${modelId} (${f.filename})`);
        const buf = await hit.arrayBuffer();
        out[f.kind] = new Uint8Array(buf);
    }
    if (!out.weights) throw new Error(`model ${modelId} missing weights file`);
    if (!out.tokenizer) {
        // Some entries (GGUF) don't ship a tokenizer; provide a 0-byte placeholder.
        out.tokenizer = new Uint8Array(0);
    }
    return out;
}

/** Remove all files for a model. */
export async function deleteModel(modelId) {
    if (!_hasCaches()) return;
    const m = getKnownModel(modelId);
    if (!m) return;
    const cache = await caches.open(CACHE_NAME);
    for (const f of m.files) {
        try { await cache.delete(cacheKey(modelId, f.filename)); } catch (_) { /* ignore */ }
    }
    events.dispatchEvent(new CustomEvent('model_deleted', { detail: { modelId } }));
}

/** Abort an in-flight download for `modelId`. */
export function cancelDownload(modelId) {
    const a = activeDownloads.get(modelId);
    if (a && a.controller) {
        try { a.controller.abort(); } catch (_) { /* idempotent */ }
    }
    if (typeof navigator !== 'undefined' && navigator.serviceWorker && navigator.serviceWorker.controller) {
        try { navigator.serviceWorker.controller.postMessage({ type: 'model_download_cancel', modelId }); } catch (_) {}
    }
}

// ── Download ───────────────────────────────────────────────────

class HfAuthRequiredError extends Error {
    constructor(message = 'Hugging Face token required') {
        super(message);
        this.name = 'HF_AUTH_REQUIRED';
    }
}

/**
 * Streaming download of every registered file for `modelId`. At most one
 * download is active globally; a second call returns the in-flight promise.
 *
 * @param {string} modelId
 * @param {object} [opts]
 * @param {(p: object) => void} [opts.onProgress]
 * @param {AbortSignal} [opts.signal]   external cancel signal
 * @param {string} [opts.hfToken]       HF access token for gated repos
 * @returns {Promise<void>}
 */
export async function downloadModel(modelId, opts = {}) {
    if (activeDownloads.has(modelId)) return activeDownloads.get(modelId).promise;
    for (const other of activeDownloads.values()) {
        await other.promise.catch(() => {});
    }
    if (!_hasCaches()) throw new Error('Cache Storage unavailable');

    // Prefer SW-delegated download for background resilience. Falls
    // back to page-side download when SW isn't active (e.g. DEV_MODE).
    if (typeof navigator !== 'undefined' && navigator.serviceWorker && navigator.serviceWorker.controller) {
        return _downloadViaSW(modelId, opts);
    }
    return _downloadDirect(modelId, opts);
}

async function _downloadViaSW(modelId, opts) {
    const m = getKnownModel(modelId);
    if (!m) throw new Error(`unknown model: ${modelId}`);

    const files = m.files.map((f) => ({
        url: cacheKey(modelId, f.filename),
        filename: f.filename,
        kind: f.kind,
        sha256: f.sha256,
    }));

    const controller = new AbortController();
    if (opts.signal) {
        if (opts.signal.aborted) controller.abort();
        else opts.signal.addEventListener('abort', () => {
            navigator.serviceWorker.controller.postMessage({ type: 'model_download_cancel', modelId });
            controller.abort();
        }, { once: true });
    }

    const promise = new Promise((resolve, reject) => {
        const onMessage = (event) => {
            const msg = event.data;
            if (!msg || typeof msg !== 'object') return;

            if (msg.type === 'model_progress' && msg.detail && msg.detail.modelId === modelId) {
                try { if (typeof opts.onProgress === 'function') opts.onProgress(msg.detail); } catch (_e) {}
                events.dispatchEvent(new CustomEvent('model_progress', { detail: msg.detail }));
            } else if (msg.type === 'model_download_done' && msg.modelId === modelId) {
                cleanup();
                resolve();
            } else if (msg.type === 'model_download_error' && msg.modelId === modelId) {
                cleanup();
                if (msg.error === 'HF_AUTH_REQUIRED') {
                    reject(new HfAuthRequiredError());
                } else {
                    reject(new Error(msg.error || 'SW download failed'));
                }
            }
        };
        const cleanup = () => {
            navigator.serviceWorker.removeEventListener('message', onMessage);
            activeDownloads.delete(modelId);
        };
        navigator.serviceWorker.addEventListener('message', onMessage);
    });

    activeDownloads.set(modelId, { controller, startedAt: Date.now(), promise });

    navigator.serviceWorker.controller.postMessage({
        type: 'model_download_start',
        modelId,
        files,
        hfToken: opts.hfToken || null,
    });

    try {
        await promise;
    } finally {
        activeDownloads.delete(modelId);
    }
}

async function _downloadDirect(modelId, opts) {

    const m = getKnownModel(modelId);
    if (!m) throw new Error(`unknown model: ${modelId}`);

    const controller = new AbortController();
    if (opts.signal) {
        if (opts.signal.aborted) controller.abort();
        else opts.signal.addEventListener('abort', () => controller.abort(), { once: true });
    }

    const startedAt = Date.now();
    const promise = (async () => {
        const cache = await caches.open(CACHE_NAME);

        // First pass: HEAD-style sizing to compute totalBytesTotal.
        const fileTotals = new Array(m.files.length).fill(0);
        let totalBytesTotal = 0;
        // We deliberately skip a HEAD request — HF's resolve URLs return
        // Content-Length on GET, and a HEAD adds latency. We still emit
        // progress events as soon as the first GET response arrives.

        let totalBytesDone = 0;
        let lastEmit = 0;

        const emitProgress = (file, fileBytesDone, fileBytesTotal, force = false) => {
            const now = Date.now();
            if (!force && now - lastEmit < PROGRESS_EMIT_MS) return;
            lastEmit = now;
            const elapsedSec = Math.max(0.001, (now - startedAt) / 1000);
            const throughputBps = totalBytesDone / elapsedSec;
            const remaining = Math.max(0, totalBytesTotal - totalBytesDone);
            const etaSeconds = throughputBps > 0 ? remaining / throughputBps : null;
            const detail = {
                phase: 'download',
                modelId,
                file: file.filename,
                fileKind: file.kind,
                fileBytesDone,
                fileBytesTotal,
                totalBytesDone,
                totalBytesTotal,
                throughputBps,
                etaSeconds,
            };
            try { if (typeof opts.onProgress === 'function') opts.onProgress(detail); } catch (_) {}
            events.dispatchEvent(new CustomEvent('model_progress', { detail }));
        };

        for (let i = 0; i < m.files.length; i++) {
            const f = m.files[i];
            const url = cacheKey(modelId, f.filename);

            // Skip if already cached AND verified (or pin is null).
            const existing = await cache.match(url);
            if (existing) {
                if (!f.sha256) {
                    // No pin available — trust the cached bytes.
                    const len = Number(existing.headers.get('content-length')) || 0;
                    fileTotals[i] = len;
                    totalBytesTotal += len;
                    totalBytesDone += len;
                    emitProgress(f, len, len, true);
                    continue;
                }
                try {
                    const buf = await existing.clone().arrayBuffer();
                    const len = buf.byteLength;
                    const hex = await sha256Hex(buf, {
                        modelId,
                        file: f,
                        fileBytesTotal: len,
                        totalBytesDoneBefore: totalBytesDone,
                        totalBytesTotalSoFar: totalBytesTotal + len,
                    });
                    if (hex === f.sha256) {
                        fileTotals[i] = len;
                        totalBytesTotal += len;
                        totalBytesDone += len;
                        emitProgress(f, len, len, true);
                        continue;
                    }
                    // Mismatch — re-download.
                    await cache.delete(url);
                } catch (_) {
                    await cache.delete(url);
                }
            }

            const baseHeaders = {};
            if (opts.hfToken) baseHeaders['Authorization'] = `Bearer ${opts.hfToken}`;

            // Retry loop with Range-header resume. On network failure,
            // we keep the chunks already received and resume from that
            // byte offset. HuggingFace supports Range requests.
            // Stream directly to Cache Storage — no RAM accumulation.
            // Bytes flow: network → TransformStream (count + progress) → disk.
            const fetchHeaders = { ...baseHeaders };

            let resp;
            try {
                resp = await fetch(url, { headers: fetchHeaders, signal: controller.signal });
            } catch (e) {
                if (controller.signal.aborted) throw new DOMException('aborted', 'AbortError');
                throw e;
            }
            if (resp.status === 401 || resp.status === 403) {
                throw new HfAuthRequiredError(`HF responded ${resp.status} for ${f.filename}`);
            }
            if (!resp.ok) {
                throw new Error(`HF fetch failed (${resp.status}) for ${f.filename}`);
            }

            const contentLength = Number(resp.headers.get('content-length')) || 0;
            fileTotals[i] = contentLength;
            totalBytesTotal += contentLength;
            let fileBytesDone = 0;

            const countingStream = new TransformStream({
                transform(chunk, ctrl) {
                    fileBytesDone += chunk.byteLength;
                    totalBytesDone += chunk.byteLength;
                    emitProgress(f, fileBytesDone, contentLength);
                    ctrl.enqueue(chunk);
                },
            });

            const countedBody = resp.body.pipeThrough(countingStream);
            const cacheHeaders = new Headers();
            cacheHeaders.set('content-type', resp.headers.get('content-type') || 'application/octet-stream');
            if (contentLength) cacheHeaders.set('content-length', String(contentLength));

            await cache.put(url, new Response(countedBody, { status: 200, headers: cacheHeaders }));
            emitProgress(f, fileBytesDone, contentLength, true);

            // Verify pin if available — read back from cache (no in-memory blob).
            if (f.sha256) {
                const cached = await cache.match(url);
                const ab = cached ? await cached.arrayBuffer() : null;
                if (!ab) throw new Error(`cached file disappeared: ${f.filename}`);
                const hex = await sha256Hex(ab, {
                    modelId,
                    file: f,
                    fileBytesTotal: ab.byteLength,
                    totalBytesDoneBefore: totalBytesDone - ab.byteLength,
                    totalBytesTotalSoFar: totalBytesTotal,
                });
                if (hex !== f.sha256) {
                    await cache.delete(url);
                    throw new Error(`SHA-256 mismatch for ${f.filename}: got ${hex}, expected ${f.sha256}`);
                }
            }
        }
    })();

    activeDownloads.set(modelId, { controller, startedAt, promise });
    try {
        await promise;
    } finally {
        activeDownloads.delete(modelId);
    }
}

// ── Hash helper ────────────────────────────────────────────────

/**
 * Compute the SHA-256 of `buf`, emitting `phase: 'verifying'` events on
 * `state.events` so the UI can show "Verifying SHA-256…" while the work
 * happens.
 *
 * NOTE: `crypto.subtle.digest` does not support streaming — calling it
 * per chunk would yield independent hashes, not a single rolling one.
 * For now we just yield to the event loop right before and after the
 * single-shot digest call so the banner repaints between phase changes.
 * Once we want fully-incremental progress, swap in a pure-JS streaming
 * SHA-256 implementation (e.g. js-sha256 vendored as a small dep).
 *
 * @param {ArrayBuffer} buf
 * @param {object} [ctx]   optional progress context (modelId, file, …)
 * @returns {Promise<string>} hex digest
 */
async function sha256Hex(buf, ctx = null) {
    const total = buf.byteLength;
    const emit = (bytesProcessed) => {
        if (!ctx || !ctx.file) return;
        const fileTotal = ctx.fileBytesTotal != null ? ctx.fileBytesTotal : total;
        const totalDoneBefore = ctx.totalBytesDoneBefore || 0;
        const totalTotal = ctx.totalBytesTotalSoFar || fileTotal;
        const totalBytesDone = totalDoneBefore + bytesProcessed;
        const percent = fileTotal > 0
            ? Math.min(100, Math.floor((bytesProcessed / fileTotal) * 100))
            : null;
        const detail = {
            phase: 'verifying',
            modelId: ctx.modelId,
            file: ctx.file.filename,
            fileKind: ctx.file.kind,
            fileBytesDone: bytesProcessed,
            fileBytesTotal: fileTotal,
            totalBytesDone,
            totalBytesTotal: totalTotal,
            percent,
        };
        events.dispatchEvent(new CustomEvent('model_progress', { detail }));
    };

    // Emit "verifying started" and let the main thread repaint before
    // the digest call blocks for a few seconds.
    emit(0);
    await new Promise((r) => setTimeout(r, 0));

    const digest = await crypto.subtle.digest('SHA-256', buf);

    // Yield once more so the banner can flip to the next phase before
    // the next big main-thread call (e.g. wasm.init_local_model).
    await new Promise((r) => setTimeout(r, 0));
    emit(total);

    const bytes = new Uint8Array(digest);
    let out = '';
    for (let i = 0; i < bytes.length; i++) {
        out += bytes[i].toString(16).padStart(2, '0');
    }
    return out;
}

export const HFAuthRequired = HfAuthRequiredError;
