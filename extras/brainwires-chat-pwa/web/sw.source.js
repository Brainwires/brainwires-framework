// brainwires-chat-pwa — service worker source
//
// Headline responsibility: keep streaming chat responses alive when the
// page is backgrounded (mobile lock screen, tab in background, etc.).
// The SW owns the fetch + ReadableStream; it persists each chunk to
// IndexedDB and broadcasts deltas to any visible client. When the
// stream finishes with no foreground client, it raises a notification.
//
// Build pipeline:
//   sw.source.js  --(esbuild bundle, IIFE)-->  sw.bundle.js
//   sw.bundle.js  --(SRI substitution)----->  sw.js   (gitignored)
//
// The SRI table substituted into __SRI_HASHES__ pins the static assets
// the SW caches. Cloud provider URLs (huggingface.co, OpenAI, Anthropic,
// etc.) are NOT in the table and pass straight through to the network
// without ever being cached.
//
// Imports below are bundled by esbuild — the runtime sees a flat IIFE.

import {
    streamFromResponse,
} from './src/streaming.js';
import {
    decrypt as cryptoDecrypt,
    unpack as cryptoUnpack,
} from './crypto-store.js';
import {
    appendMessageChunk,
    putMessage,
} from './src/db.js';

// ── Cache versioning ───────────────────────────────────────────
const CACHE_NAME = 'bw-chat-cache-v1';

// ── Passthrough host allowlist ─────────────────────────────────
//
// The fetch handler already passes everything that's not pinned to
// the network unmodified. This list is informational: any host
// matching here is GUARANTEED never to be cached by the SW. We use
// it for an explicit early-return so a future maintainer adding new
// caching logic can't accidentally swallow these.
const PASSTHROUGH_HOST_PATTERNS = [
    /^huggingface\.co$/i,
    /\.huggingface\.co$/i,
    /^api\.anthropic\.com$/i,
    /^api\.openai\.com$/i,
    /^generativelanguage\.googleapis\.com$/i,
    /:11434$/,                        // any Ollama LAN host
];

function isPassthroughHost(url) {
    try {
        const u = new URL(url);
        const hostport = u.port ? `${u.hostname}:${u.port}` : u.hostname;
        return PASSTHROUGH_HOST_PATTERNS.some((re) => re.test(hostport) || re.test(u.hostname));
    } catch (_) { return false; }
}

// ── SRI hash table (build-time substituted) ────────────────────
//
// Keys are paths relative to the web root (e.g. 'app.js',
// 'pkg/brainwires_chat_pwa.js'). Values are 'sha256-<base64>' digests.
// `sw.js` itself is intentionally excluded (a worker can't verify itself).
const RESOURCE_HASHES = __SRI_HASHES__;

// ── Tiny log helper ────────────────────────────────────────────
// Production paths stay quiet; debug logs are silenced unless you
// flip DEBUG to true at build/test time.
const DEBUG = false;
let DEV_MODE = false;
function log(...args) { if (DEBUG) console.log('[bw-sw]', ...args); }
function warn(...args) { console.warn('[bw-sw]', ...args); }

// ── Hash helpers ───────────────────────────────────────────────

async function sha256Base64(buffer) {
    const hashBuf = await crypto.subtle.digest('SHA-256', buffer);
    let bin = '';
    const bytes = new Uint8Array(hashBuf);
    for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
    return btoa(bin);
}

function resourceKey(url) {
    const path = new URL(url).pathname.replace(/^\/+/, '');
    return path;
}

function isPinned(url) {
    return Object.prototype.hasOwnProperty.call(RESOURCE_HASHES, resourceKey(url));
}

// ── Lifecycle: install/activate ────────────────────────────────

self.addEventListener('install', (event) => {
    event.waitUntil((async () => {
        const cache = await caches.open(CACHE_NAME);
        const paths = Object.keys(RESOURCE_HASHES);
        // addAll is atomic; if any single asset 404s the cache install
        // fails. We tolerate that by trying assets individually so a
        // missing dev asset doesn't brick the SW.
        await Promise.all(paths.map(async (rel) => {
            try {
                const url = new URL('./' + rel, self.location).href;
                const resp = await fetch(url, { cache: 'no-cache' });
                if (resp && resp.ok) await cache.put(url, resp.clone());
            } catch (e) {
                warn('install: failed to cache', rel, e && e.message);
            }
        }));
        await self.skipWaiting();
    })());
});

self.addEventListener('activate', (event) => {
    event.waitUntil((async () => {
        const keys = await caches.keys();
        await Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k)));
        await self.clients.claim();
    })());
});

// ── Fetch: cache-first for pinned, network-only for everything else ──

self.addEventListener('fetch', (event) => {
    const req = event.request;
    if (req.method !== 'GET') return;

    const sameOrigin = req.url.startsWith(self.location.origin);
    if (isPassthroughHost(req.url)) {
        // Explicit allowlist: provider + HF URLs go straight to the
        // network and never land in any SW-managed cache.
        return;
    }
    if (!sameOrigin || !isPinned(req.url)) {
        return;
    }

    // Dev mode: network-first, no cache, no SRI — live-editing works
    // while the SW stays alive for model downloads.
    if (DEV_MODE) return;

    event.respondWith((async () => {
        const cache = await caches.open(CACHE_NAME);
        const cached = await cache.match(req);
        if (cached) {
            const expected = RESOURCE_HASHES[resourceKey(req.url)];
            try {
                const buf = await cached.clone().arrayBuffer();
                const actual = 'sha256-' + await sha256Base64(buf);
                if (actual === expected) return cached;
                warn('SRI mismatch for', req.url, '— evicting and refetching');
                await cache.delete(req);
            } catch (e) {
                warn('SRI verify failed for', req.url, e && e.message);
                // Fall through to network.
            }
        }
        // Cache miss or SRI eviction → fetch fresh, populate cache.
        try {
            const fresh = await fetch(req);
            if (fresh && fresh.ok) {
                cache.put(req, fresh.clone()).catch(() => {});
            }
            return fresh;
        } catch (e) {
            // Last resort: return the (mismatched) cached copy if we still have it.
            if (cached) return cached;
            throw e;
        }
    })());
});

// ── Active stream registry ─────────────────────────────────────
//
// Lost on SW eviction; durability is provided by IndexedDB writes.
// The map key is messageId so chat_status_query / chat_cancel can target
// in-flight streams without a conversationId lookup.
//
// Value shape: { conversationId, abortController, tokensReceived, startedAt }
const activeStreams = new Map();

// ── Message IPC ────────────────────────────────────────────────

self.addEventListener('message', (event) => {
    const msg = event.data;
    if (!msg || typeof msg !== 'object') return;

    switch (msg.type) {
        case 'chat_start':
            event.waitUntil(handleChatStart(msg, event));
            break;
        case 'chat_status_query': {
            const active = [];
            for (const [messageId, st] of activeStreams) {
                active.push({
                    conversationId: st.conversationId,
                    messageId,
                    tokensReceived: st.tokensReceived,
                    startedAt: st.startedAt,
                });
            }
            replyTo(event, { type: 'chat_status', active });
            break;
        }
        case 'chat_cancel': {
            const st = activeStreams.get(msg.messageId);
            if (st) {
                try { st.abortController.abort(); } catch (_) {}
            }
            break;
        }
        case 'model_download_start':
            event.waitUntil(handleModelDownload(msg, event));
            break;
        case 'model_download_cancel': {
            const dl = activeModelDownloads.get(msg.modelId);
            if (dl) { try { dl.controller.abort(); } catch (_) {} }
            break;
        }
        case 'set_dev_mode':
            DEV_MODE = !!msg.enabled;
            log('DEV_MODE set to', DEV_MODE);
            break;
        case 'sri_table':
            replyTo(event, { type: 'sri_table', hashes: RESOURCE_HASHES });
            break;
        default:
            log('unknown message type', msg.type);
    }
});

// ── Model download (background-resilient + resumable) ───────────
//
// Chunks are persisted to IndexedDB as they stream from the network.
// If the download is interrupted, the next attempt reads the partial
// state from IDB and resumes with a Range header. On completion, all
// chunks are assembled into a Blob and cache.put'd as a single
// Response, then the IDB partials are cleaned up.

const activeModelDownloads = new Map();
const MODEL_CACHE = 'bw-models-v1';
const DL_PARTIALS_DB = 'bw-download-partials';
const DL_PROGRESS_MS = 200;

function openPartialsDb() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open(DL_PARTIALS_DB, 1);
        req.onupgradeneeded = () => {
            const db = req.result;
            if (!db.objectStoreNames.contains('chunks')) db.createObjectStore('chunks');
            if (!db.objectStoreNames.contains('meta')) db.createObjectStore('meta');
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

function idbPut(db, store, key, value) {
    return new Promise((resolve, reject) => {
        const tx = db.transaction(store, 'readwrite');
        tx.objectStore(store).put(value, key);
        tx.oncomplete = () => resolve();
        tx.onerror = () => reject(tx.error);
    });
}

function idbGet(db, store, key) {
    return new Promise((resolve, reject) => {
        const tx = db.transaction(store, 'readonly');
        const req = tx.objectStore(store).get(key);
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

function idbGetAllKeys(db, store) {
    return new Promise((resolve, reject) => {
        const tx = db.transaction(store, 'readonly');
        const req = tx.objectStore(store).getAllKeys();
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

function idbDelete(db, store, key) {
    return new Promise((resolve, reject) => {
        const tx = db.transaction(store, 'readwrite');
        tx.objectStore(store).delete(key);
        tx.oncomplete = () => resolve();
        tx.onerror = () => reject(tx.error);
    });
}

async function clearPartials(db, prefix) {
    const keys = await idbGetAllKeys(db, 'chunks');
    for (const k of keys) {
        if (String(k).startsWith(prefix)) await idbDelete(db, 'chunks', k);
    }
    await idbDelete(db, 'meta', prefix).catch(() => {});
}

async function handleModelDownload(msg, _event) {
    const { modelId, files, hfToken } = msg;
    console.log('[bw-sw] handleModelDownload:', modelId, files ? files.length : 0, 'files');
    if (!files || !files.length) { console.warn('[bw-sw] no files to download'); return; }
    if (activeModelDownloads.has(modelId)) { console.log('[bw-sw] download already active for', modelId); return; }

    const controller = new AbortController();
    activeModelDownloads.set(modelId, { controller, startedAt: Date.now() });

    try {
        const cache = await caches.open(MODEL_CACHE);
        let totalBytesDone = 0;
        let totalBytesTotal = 0;
        const startedAt = Date.now();
        let lastEmit = 0;

        const broadcastDl = (detail) => {
            self.clients.matchAll({ type: 'window' }).then(cls => {
                for (const c of cls) c.postMessage(detail);
            });
        };

        const emitProgress = (file, fileBytesDone, fileBytesTotal, force) => {
            const now = Date.now();
            if (!force && now - lastEmit < DL_PROGRESS_MS) return;
            lastEmit = now;
            const elapsed = Math.max(0.001, (now - startedAt) / 1000);
            const bps = totalBytesDone / elapsed;
            const remaining = Math.max(0, totalBytesTotal - totalBytesDone);
            broadcastDl({
                type: 'model_progress',
                detail: {
                    phase: 'download', modelId,
                    file: file.filename, fileKind: file.kind,
                    fileBytesDone, fileBytesTotal,
                    totalBytesDone, totalBytesTotal,
                    throughputBps: bps,
                    etaSeconds: bps > 0 ? remaining / bps : null,
                },
            });
        };

        for (const f of files) {
            const url = f.url;
            const existing = await cache.match(url);
            if (existing) {
                const len = Number(existing.headers.get('content-length')) || 0;
                totalBytesTotal += len;
                totalBytesDone += len;
                emitProgress(f, len, len, true);
                continue;
            }

            // Resumable streaming download. Chunks persist to IDB so an
            // interrupted 10 GB download resumes from where it left off.
            const partialsDb = await openPartialsDb();
            const partialKey = `${modelId}:${f.filename}`;
            const MAX_RETRIES = 3;

            // Check for existing partial state from a prior interrupted download.
            let meta = (await idbGet(partialsDb, 'meta', partialKey)) || {
                receivedBytes: 0, chunkCount: 0, contentLength: 0,
            };
            let contentLength = meta.contentLength;
            let fileBytesDone = meta.receivedBytes;
            if (contentLength > 0) totalBytesTotal += contentLength;
            if (fileBytesDone > 0) {
                totalBytesDone += fileBytesDone;
                console.log(`[bw-sw] resuming ${f.filename} from ${fileBytesDone}/${contentLength}`);
            }

            // Skip the fetch entirely if all bytes are already in IDB.
            const alreadyComplete = contentLength > 0 && fileBytesDone >= contentLength;
            if (alreadyComplete) {
                console.log(`[bw-sw] ${f.filename}: all bytes in IDB, skipping fetch`);
            }

            for (let attempt = 0; !alreadyComplete && attempt <= MAX_RETRIES; attempt++) {
                const dlHeaders = {};
                if (hfToken) dlHeaders['Authorization'] = `Bearer ${hfToken}`;
                if (fileBytesDone > 0) dlHeaders['Range'] = `bytes=${fileBytesDone}-`;

                console.log(`[bw-sw] download ${f.filename} attempt ${attempt + 1} from byte ${fileBytesDone}`);

                let resp;
                try {
                    resp = await fetch(url, { headers: dlHeaders, signal: controller.signal });
                } catch (e) {
                    if (controller.signal.aborted) throw e;
                    console.warn(`[bw-sw] ${f.filename} fetch failed:`, e.message);
                    if (attempt === MAX_RETRIES) throw e;
                    await new Promise(r => setTimeout(r, 1000 * (attempt + 1)));
                    continue;
                }
                if (resp.status === 401 || resp.status === 403) {
                    broadcastDl({ type: 'model_download_error', modelId, error: 'HF_AUTH_REQUIRED' });
                    return;
                }
                if (resp.status === 416) break; // already complete
                if (!resp.ok && resp.status !== 206) {
                    console.warn(`[bw-sw] ${f.filename} HTTP ${resp.status}`);
                    if (attempt === MAX_RETRIES) throw new Error(`HF ${resp.status}`);
                    await new Promise(r => setTimeout(r, 1000 * (attempt + 1)));
                    continue;
                }

                // Server returned 200 (full file) instead of 206 — discard partials.
                if (resp.status === 200 && fileBytesDone > 0) {
                    console.log(`[bw-sw] ${f.filename}: server sent full file, discarding partial`);
                    totalBytesDone -= fileBytesDone;
                    fileBytesDone = 0;
                    meta = { receivedBytes: 0, chunkCount: 0, contentLength: 0 };
                    await clearPartials(partialsDb, partialKey);
                }

                if (contentLength === 0) {
                    contentLength = resp.status === 206
                        ? Number((resp.headers.get('content-range') || '').split('/')[1]) || 0
                        : Number(resp.headers.get('content-length')) || 0;
                    totalBytesTotal += contentLength;
                    meta.contentLength = contentLength;
                    console.log(`[bw-sw] ${f.filename}: ${contentLength} bytes total`);
                }

                // Stream chunks to IDB, batched to avoid per-chunk transaction
                // overhead. Accumulate ~4 MB in memory before flushing to IDB
                // as one entry. At most 4 MB lost on failure.
                const BATCH_SIZE = 4 * 1024 * 1024; // 4 MB
                const reader = resp.body.getReader();
                let streamFailed = false;
                let batchBuf = [];
                let batchBytes = 0;

                const flushBatch = async () => {
                    if (batchBuf.length === 0) return;
                    const merged = new Blob(batchBuf);
                    const ab = await merged.arrayBuffer();
                    await idbPut(partialsDb, 'chunks', `${partialKey}:${meta.chunkCount}`, ab);
                    meta.chunkCount++;
                    meta.receivedBytes = fileBytesDone;
                    await idbPut(partialsDb, 'meta', partialKey, { ...meta });
                    batchBuf = [];
                    batchBytes = 0;
                };

                try {
                    while (true) {
                        const { value, done } = await reader.read();
                        if (done) break;
                        if (controller.signal.aborted) throw new DOMException('aborted', 'AbortError');

                        batchBuf.push(value);
                        batchBytes += value.byteLength;
                        fileBytesDone += value.byteLength;
                        totalBytesDone += value.byteLength;
                        emitProgress(f, fileBytesDone, contentLength);

                        if (batchBytes >= BATCH_SIZE) await flushBatch();
                    }
                    await flushBatch(); // flush remaining
                } catch (e) {
                    if (controller.signal.aborted) throw e;
                    streamFailed = true;
                    // Flush whatever we have so partial progress is saved.
                    try { await flushBatch(); } catch (_) {}
                    console.warn(`[bw-sw] ${f.filename} stream broke at ${fileBytesDone}/${contentLength}:`, e.message);
                    if (attempt === MAX_RETRIES) throw e;
                }
                try { reader.releaseLock(); } catch (_) {}
                if (!streamFailed) break;
                await new Promise(r => setTimeout(r, 1000 * (attempt + 1)));
            }

            // Emit a progress event so the page-side timeout resets.
            emitProgress(f, fileBytesDone, contentLength, true);

            // Assemble all IDB chunks → Blob → Cache Storage.
            // Batch-read in parallel (100 at a time) to avoid 78,000+
            // sequential IDB transactions from pre-batching downloads.
            console.log(`[bw-sw] ${f.filename}: assembling ${meta.chunkCount} chunks`);
            const allChunks = [];
            const READ_BATCH = 100;
            for (let start = 0; start < meta.chunkCount; start += READ_BATCH) {
                const end = Math.min(start + READ_BATCH, meta.chunkCount);
                const batch = await Promise.all(
                    Array.from({ length: end - start }, (_, i) =>
                        idbGet(partialsDb, 'chunks', `${partialKey}:${start + i}`)
                    )
                );
                for (const buf of batch) { if (buf) allChunks.push(buf); }
                if (start % 10000 === 0 && start > 0) {
                    console.log(`[bw-sw] ${f.filename}: read ${start}/${meta.chunkCount} chunks`);
                }
            }
            const blob = new Blob(allChunks);
            const cacheHdrs = new Headers();
            cacheHdrs.set('content-type', 'application/octet-stream');
            cacheHdrs.set('content-length', String(blob.size));
            await cache.put(url, new Response(blob, { status: 200, headers: cacheHdrs }));

            // SHA-256 verification (if pin is set). Done here in the SW
            // so the page thread never has to hash 10+ GB.
            if (f.sha256) {
                console.log(`[bw-sw] ${f.filename}: verifying SHA-256...`);
                broadcastDl({ type: 'model_progress', detail: { phase: 'verifying', modelId, file: f.filename } });
                const cached = await cache.match(url);
                const ab = cached ? await cached.arrayBuffer() : null;
                if (ab) {
                    const hashBuf = await crypto.subtle.digest('SHA-256', ab);
                    const bytes = new Uint8Array(hashBuf);
                    let hex = '';
                    for (let bi = 0; bi < bytes.length; bi++) hex += bytes[bi].toString(16).padStart(2, '0');
                    if (hex !== f.sha256) {
                        console.error(`[bw-sw] ${f.filename}: SHA mismatch! got=${hex} expected=${f.sha256}`);
                        await cache.delete(url);
                        await clearPartials(partialsDb, partialKey);
                        throw new Error(`SHA-256 mismatch for ${f.filename}`);
                    }
                    console.log(`[bw-sw] ${f.filename}: SHA-256 verified ✓`);
                }
            }

            // Clean up IDB partials.
            await clearPartials(partialsDb, partialKey);
            console.log(`[bw-sw] ${f.filename}: cached (${blob.size} bytes), partials cleaned`);
            emitProgress(f, fileBytesDone, contentLength, true);
        }

        broadcastDl({ type: 'model_download_done', modelId });
    } catch (e) {
        const clients = await self.clients.matchAll({ type: 'window' });
        for (const c of clients) {
            c.postMessage({ type: 'model_download_error', modelId, error: e.message || String(e) });
        }
    } finally {
        activeModelDownloads.delete(modelId);
    }
}

function replyTo(event, payload) {
    if (event.source && typeof event.source.postMessage === 'function') {
        event.source.postMessage(payload);
    }
}

async function broadcast(payload) {
    const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
    for (const c of clients) {
        try { c.postMessage(payload); } catch (_) {}
    }
}

// ── Chat streaming ─────────────────────────────────────────────

/**
 * Re-import the session key the page handed us. Accepts either a
 * `CryptoKey` (preferred — `postMessage` clones it) or 32 raw bytes that
 * we re-import as AES-GCM.
 */
async function importSessionKey(sessionKey) {
    if (sessionKey && typeof sessionKey === 'object' && 'algorithm' in sessionKey && 'type' in sessionKey) {
        return sessionKey; // already a CryptoKey
    }
    const bytes = sessionKey instanceof Uint8Array
        ? sessionKey
        : (sessionKey && sessionKey.buffer ? new Uint8Array(sessionKey.buffer) : null);
    if (!bytes || bytes.length !== 32) {
        throw new Error('chat_start: sessionKey must be a CryptoKey or 32 raw bytes');
    }
    return crypto.subtle.importKey(
        'raw',
        bytes,
        { name: 'AES-GCM' },
        false,
        ['decrypt'],
    );
}

/**
 * Decrypt the API key blob the page handed in. The blob is a packed
 * base64url string from `crypto-store.pack()`.
 */
async function decryptApiKey(apiKeyEncrypted, sessionKey) {
    const key = await importSessionKey(sessionKey);
    const blob = cryptoUnpack(apiKeyEncrypted);
    return cryptoDecrypt(key, { iv: blob.iv, ciphertext: blob.ciphertext });
}

/**
 * Long-lived streaming task. Wrapped in event.waitUntil() by the caller.
 *
 * Persistence rule: flush to IndexedDB every 32 chunks OR every 250ms,
 * whichever comes first. Final flush on stream end / abort / error.
 */
async function handleChatStart(msg, event) {
    const { conversationId, messageId, requestPayload, apiKeyEncrypted, sessionKey } = msg;

    if (!conversationId || !messageId || !requestPayload) {
        replyTo(event, { type: 'chat_error', conversationId, messageId, error: 'missing required fields' });
        return;
    }
    if (activeStreams.has(messageId)) {
        replyTo(event, { type: 'chat_error', conversationId, messageId, error: 'already streaming' });
        return;
    }

    let apiKey = null;
    if (apiKeyEncrypted) {
        try {
            apiKey = await decryptApiKey(apiKeyEncrypted, sessionKey);
        } catch (e) {
            replyTo(event, {
                type: 'chat_error',
                conversationId,
                messageId,
                error: 'decrypt_failed: ' + (e && e.message ? e.message : String(e)),
            });
            return;
        }
    }

    const abortController = new AbortController();
    const state = {
        conversationId,
        abortController,
        tokensReceived: 0,
        startedAt: Date.now(),
    };
    activeStreams.set(messageId, state);

    // Buffered delta — flushed every 32 chunks or 250ms.
    let pending = '';
    let pendingCount = 0;
    let lastFlushAt = Date.now();
    const FLUSH_TOKENS = 32;
    const FLUSH_MS = 250;

    const flush = async (final) => {
        if (pending.length === 0 && !final) return;
        const delta = pending;
        pending = '';
        pendingCount = 0;
        lastFlushAt = Date.now();
        if (delta.length > 0) {
            try {
                await appendMessageChunk(conversationId, messageId, delta);
            } catch (e) {
                warn('appendMessageChunk failed', e && e.message);
            }
        }
    };

    const maybeFlush = async () => {
        if (pendingCount >= FLUSH_TOKENS || (Date.now() - lastFlushAt) >= FLUSH_MS) {
            await flush(false);
        }
    };

    const usage = null;

    try {
        const { url, method = 'POST', headers = {}, body, format } = requestPayload;
        if (!url) throw new Error('requestPayload.url required');
        if (format !== 'sse' && format !== 'ndjson') {
            throw new Error('requestPayload.format must be "sse" or "ndjson"');
        }

        // Caller embeds the sentinel '__API_KEY__' inside header values
        // and (for Gemini) the URL query string; the SW substitutes
        // the decrypted plaintext after the postMessage boundary so
        // the page never has to hold the plaintext key in memory
        // alongside the request envelope. See providers/index.js for
        // the full contract.
        const finalHeaders = { ...headers };
        let finalUrl = url;
        if (apiKey !== null) {
            for (const k of Object.keys(finalHeaders)) {
                if (typeof finalHeaders[k] === 'string' && finalHeaders[k].includes('__API_KEY__')) {
                    finalHeaders[k] = finalHeaders[k].split('__API_KEY__').join(apiKey);
                }
            }
            if (finalUrl.includes('__API_KEY__')) {
                finalUrl = finalUrl.split('__API_KEY__').join(encodeURIComponent(apiKey));
            }
        }

        const resp = await fetch(finalUrl, {
            method,
            headers: finalHeaders,
            body: body !== undefined && method !== 'GET' ? body : undefined,
            signal: abortController.signal,
        });

        if (!resp.ok) {
            const text = await resp.text().catch(() => '');
            throw new Error(`HTTP ${resp.status}: ${text.slice(0, 256)}`);
        }

        for await (const ev of streamFromResponse(resp, format)) {
            if (abortController.signal.aborted) break;

            let delta = '';
            if (format === 'sse') {
                if (ev && ev.done) break;
                // Caller's `body` shape is provider-specific; the SW does
                // NOT decode the JSON. We hand the raw `data` through and
                // let the page's provider adapter build the user-visible
                // text. For storage/broadcast purposes we treat the raw
                // SSE data line as the "delta" — tasks #6/7 will refine
                // this once provider adapters land.
                delta = ev && typeof ev.data === 'string' ? ev.data : '';
            } else {
                // NDJSON: pass-through as a stringified line.
                delta = typeof ev === 'string' ? ev : JSON.stringify(ev);
            }

            if (delta.length === 0) continue;

            pending += delta;
            pendingCount += 1;
            state.tokensReceived += 1;

            // Broadcast every chunk immediately so the UI feels live;
            // IndexedDB writes are debounced separately.
            broadcast({
                type: 'chat_chunk',
                conversationId,
                messageId,
                delta,
                raw: format === 'sse' ? { event: ev.event, data: ev.data } : ev,
            }).catch(() => {});

            await maybeFlush();
        }

        // Final flush before the done message.
        await flush(true);

        // Stamp final updatedAt + persisted state.
        try {
            await putMessage({
                conversationId,
                messageId,
                role: 'assistant',
                content: undefined, // appendMessageChunk owns content; don't clobber
                updatedAt: Date.now(),
                completedAt: Date.now(),
                tokensReceived: state.tokensReceived,
            });
        } catch (e) {
            // Final stamp is best-effort; the chunk-appended row is the source of truth.
            log('putMessage final stamp failed', e && e.message);
        }

        broadcast({
            type: 'chat_done',
            conversationId,
            messageId,
            usage,
            tokensReceived: state.tokensReceived,
        }).catch(() => {});
        replyTo(event, { type: 'chat_done', conversationId, messageId, usage });

        // Background notification: only when no foreground window is alive.
        const visibleClients = await self.clients.matchAll({ type: 'window' });
        if (visibleClients.length === 0 && self.registration && self.registration.showNotification) {
            try {
                await self.registration.showNotification('Brainwires Chat', {
                    body: 'Response ready',
                    tag: messageId,
                    icon: './icons/icon-192.png',
                    badge: './icons/icon-192.png',
                    data: { conversationId, messageId },
                });
            } catch (e) {
                log('showNotification failed', e && e.message);
            }
        }
    } catch (err) {
        await flush(true);
        const errorText = err && err.message ? err.message : String(err);
        const aborted = abortController.signal.aborted || (err && err.name === 'AbortError');
        broadcast({
            type: aborted ? 'chat_aborted' : 'chat_error',
            conversationId,
            messageId,
            error: aborted ? 'aborted' : errorText,
        }).catch(() => {});
        if (!aborted) {
            replyTo(event, { type: 'chat_error', conversationId, messageId, error: errorText });
        }
    } finally {
        activeStreams.delete(messageId);
        // Best-effort: clear the in-memory plaintext API key reference.
        apiKey = null;
    }
}

// ── Notification click ─────────────────────────────────────────

self.addEventListener('notificationclick', (event) => {
    event.notification.close();
    const data = event.notification.data || {};
    event.waitUntil((async () => {
        const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
        for (const c of clients) {
            try {
                c.postMessage({ type: 'open_chat', ...data });
                if ('focus' in c) return c.focus();
            } catch (_) {}
        }
        if (self.clients.openWindow) {
            return self.clients.openWindow('./index.html');
        }
    })());
});
