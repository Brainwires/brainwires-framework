// brainwires-chat-pwa — local-WASM provider (Gemma 4 E2B)
//
// Drives the WASM module's `local_chat_stream` directly on the main
// thread. Consumes the returned `ReadableStream<Uint8Array>` line-by-
// line, parses each line as JSON in the `{delta?, usage?, error?,
// finished?}` convention, and dispatches the same `chat_chunk`,
// `chat_done`, `chat_error` events the SW emits — so UI is provider-
// agnostic.
//
// Lifecycle helpers (`loadLocalModel`, `unloadLocalModel`,
// `isLocalModelLoaded`) are exported for the Settings page to call.

import {
    getWasm,
    getLocalModelHandle,
    setLocalModelHandle,
    getLocalModelId,
    events,
    appEvents,
} from '../state.js';
import { appendMessageChunk, putMessage } from '../db.js';
import { getModelBytes, isDownloaded } from '../model-store.js';

export const id = 'local-gemma-4-e2b';
export const displayName = 'Gemma 4 E2B (on-device)';
export const runtime = 'local';
export const defaultModel = 'gemma-4-e2b';
export const models = ['gemma-4-e2b'];

// ── Lifecycle ──────────────────────────────────────────────────

/**
 * Load a model into the WASM runtime. Reads the cached HF assets via
 * `model-store`, then calls `wasm.init_local_model(weights, tokenizer, modelId)`.
 *
 * @param {string} [modelId='gemma-4-e2b']
 * @returns {Promise<object>} the wasm handle
 */
export async function loadLocalModel(modelId = defaultModel) {
    if (getLocalModelHandle() && getLocalModelId() === modelId) {
        return getLocalModelHandle();
    }
    if (!(await isDownloaded(modelId))) {
        throw new Error(`local model not downloaded: ${modelId}. Open Settings → Local model to download.`);
    }
    let { weights, tokenizer } = await getModelBytes(modelId);
    const wasm = await getWasm();
    if (typeof wasm.init_local_model !== 'function') {
        throw new Error('wasm.init_local_model() not available — rebuild the WASM crate');
    }
    // Indeterminate "loading into wasm" phase — wasm-bindgen copies the
    // bytes into linear memory inside this single call, which can block
    // the main thread for several seconds. We yield once so the banner
    // can flip to "Loading model into memory…" before that happens.
    events.dispatchEvent(new CustomEvent('model_progress', {
        detail: { phase: 'loading', modelId },
    }));
    await new Promise((r) => setTimeout(r, 0));
    let handle;
    try {
        handle = await wasm.init_local_model(weights, tokenizer, modelId);
    } finally {
        // Drop our refs — wasm has copied them into linear memory by now,
        // so the JS-side ArrayBuffers (~2.5 GB) can be GC'd. Halves peak
        // heap. The locals are function-scoped and would be reclaimed on
        // return anyway; we explicitly null them here for clarity.
        weights = null;
        tokenizer = null;
    }
    setLocalModelHandle(handle, modelId);
    events.dispatchEvent(new CustomEvent('model_progress', {
        detail: { phase: 'ready', modelId },
    }));
    return handle;
}

/** Drop the loaded model handle; lets the WASM allocator reclaim memory. */
export function unloadLocalModel() {
    const h = getLocalModelHandle();
    if (h && typeof h.free === 'function') {
        try { h.free(); } catch (_) { /* idempotent */ }
    }
    setLocalModelHandle(null, null);
}

export function isLocalModelLoaded() {
    return getLocalModelHandle() !== null;
}

// ── Streaming ──────────────────────────────────────────────────

/**
 * @param {object} args
 * @param {string} args.conversationId
 * @param {string} args.messageId
 * @param {Array<{role: string, content: string}>} args.messages
 * @param {object} [args.params]
 */
export async function startChat({ conversationId, messageId, messages, params = {} }) {
    const handle = getLocalModelHandle()
        || await loadLocalModel(params.model || defaultModel).catch((e) => { throw e; });
    if (!handle) {
        throw new Error('local model not loaded — please download Gemma 4 E2B in Settings');
    }
    const wasm = await getWasm();
    if (typeof wasm.local_chat_stream !== 'function') {
        throw new Error('wasm.local_chat_stream() not available — rebuild the WASM crate');
    }

    const messagesJson = JSON.stringify(messages);
    const paramsJson = JSON.stringify(params);
    const stream = await wasm.local_chat_stream(handle, messagesJson, paramsJson);
    if (!stream || typeof stream.getReader !== 'function') {
        throw new Error('local_chat_stream did not return a ReadableStream');
    }

    const reader = stream.getReader();
    const decoder = new TextDecoder('utf-8');
    let buffer = '';
    let usage = null;
    let tokensReceived = 0;
    let finished = false;

    // Buffered DB writes — match the SW's debounce policy.
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
            try { await appendMessageChunk(conversationId, messageId, delta); } catch (_) { /* best-effort */ }
        }
    };
    const maybeFlush = async () => {
        if (pendingCount >= FLUSH_TOKENS || (Date.now() - lastFlushAt) >= FLUSH_MS) {
            await flush(false);
        }
    };

    const dispatch = (type, detail) => {
        events.dispatchEvent(new CustomEvent(type, { detail }));
        // Mirror to the legacy hyphenated channel boot.js wires for SW msgs,
        // so existing listeners pick up local streams too.
        const hyphenType = type.replace(/_/g, '-');
        appEvents.dispatchEvent(new CustomEvent(hyphenType, { detail: { type, ...detail } }));
    };

    try {
        while (true) {
            const { value, done } = await reader.read();
            if (done) break;
            buffer += decoder.decode(value, { stream: true });
            // Split into complete NDJSON lines.
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
                    pending += obj.delta;
                    pendingCount += 1;
                    tokensReceived += 1;
                    dispatch('chat_chunk', { conversationId, messageId, delta: obj.delta });
                    await maybeFlush();
                }
                if (obj.usage && typeof obj.usage === 'object') {
                    usage = obj.usage;
                }
                if (obj.finished === true) {
                    finished = true;
                }
            }
        }
        // Flush any trailing partial line as a JSON object too.
        if (buffer.trim() !== '') {
            try {
                const obj = JSON.parse(buffer.trim());
                if (obj && typeof obj.delta === 'string' && obj.delta !== '') {
                    pending += obj.delta;
                    tokensReceived += 1;
                    dispatch('chat_chunk', { conversationId, messageId, delta: obj.delta });
                }
                if (obj && obj.usage) usage = obj.usage;
                if (obj && obj.finished === true) finished = true;
            } catch (_) { /* ignore */ }
            buffer = '';
        }

        await flush(true);

        try {
            await putMessage({
                conversationId,
                messageId,
                role: 'assistant',
                updatedAt: Date.now(),
                completedAt: Date.now(),
                tokensReceived,
            });
        } catch (_) { /* best-effort */ }

        dispatch('chat_done', { conversationId, messageId, usage, tokensReceived });
    } catch (err) {
        await flush(true);
        const error = err && err.message ? err.message : String(err);
        dispatch('chat_error', { conversationId, messageId, error });
        throw err;
    } finally {
        try { reader.releaseLock(); } catch (_) { /* already released */ }
        // `finished` is informational; UI should rely on chat_done.
        void finished;
    }
}
