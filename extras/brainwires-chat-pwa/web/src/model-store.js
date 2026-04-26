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
            // TODO(gemma-4): mirror sha256 pins from the Rust registry once
            // upstream publishes the model card.
            { kind: 'weights', filename: 'model.safetensors', sha256: null },
            { kind: 'tokenizer', filename: 'tokenizer.json', sha256: null },
        ],
        // ~2.4 GB; refine when registry pins land.
        estimatedBytes: 2_500_000_000,
        contextSize: 8192,
        gated: true,
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
    // Enforce single active download globally — if any other model is in
    // flight, wait for it. (We could parallelize per-model in the future.)
    for (const other of activeDownloads.values()) {
        await other.promise.catch(() => {});
    }
    if (!_hasCaches()) throw new Error('Cache Storage unavailable');

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
                    const hex = await sha256Hex(buf);
                    if (hex === f.sha256) {
                        const len = buf.byteLength;
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

            const headers = {};
            if (opts.hfToken) headers['Authorization'] = `Bearer ${opts.hfToken}`;

            let resp;
            try {
                resp = await fetch(url, { headers, signal: controller.signal });
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

            // Read the body as chunks; build an ArrayBuffer at the end.
            // Blob path: simpler than tee()'ing into the cache.
            const reader = resp.body.getReader();
            const chunks = [];
            let fileBytesDone = 0;
            while (true) {
                const { value, done } = await reader.read();
                if (done) break;
                if (controller.signal.aborted) throw new DOMException('aborted', 'AbortError');
                chunks.push(value);
                fileBytesDone += value.byteLength;
                totalBytesDone += value.byteLength;
                emitProgress(f, fileBytesDone, contentLength);
            }
            try { reader.releaseLock(); } catch (_) {}

            // Reassemble into a single ArrayBuffer for hashing + caching.
            const blob = new Blob(chunks);
            // Preserve content-type if the server set one.
            const cacheHeaders = new Headers();
            const ct = resp.headers.get('content-type');
            if (ct) cacheHeaders.set('content-type', ct);
            cacheHeaders.set('content-length', String(blob.size));
            const cachedResp = new Response(blob, { status: 200, headers: cacheHeaders });
            await cache.put(url, cachedResp);

            emitProgress(f, fileBytesDone, contentLength, true);

            // Verify pin if available.
            if (f.sha256) {
                const ab = await blob.arrayBuffer();
                const hex = await sha256Hex(ab);
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

async function sha256Hex(buf) {
    const digest = await crypto.subtle.digest('SHA-256', buf);
    const bytes = new Uint8Array(digest);
    let out = '';
    for (let i = 0; i < bytes.length; i++) {
        out += bytes[i].toString(16).padStart(2, '0');
    }
    return out;
}

export const HFAuthRequired = HfAuthRequiredError;
