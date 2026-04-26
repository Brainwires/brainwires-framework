// brainwires-chat-pwa — singleton app state
//
// Module-scope singletons. Safe to import from anywhere on the page; the
// service worker has its own runtime and does NOT share these.
//
// Holds:
//   - lazy WASM module reference (initialized via getWasm())
//   - session crypto key (post-passphrase-unlock; never persisted)
//   - active service-worker registration accessor
//   - app-wide pub/sub via EventTarget

const PKG_URL = './pkg/brainwires_chat_pwa.js';

// ── WASM lazy loader ───────────────────────────────────────────

let _wasm = null;
let _wasmPromise = null;

/**
 * Load and initialize the wasm-pack module the first time it's needed.
 * Subsequent calls return the same module instance.
 *
 * @returns {Promise<object>} the wasm module exports
 */
export function getWasm() {
    if (_wasm) return Promise.resolve(_wasm);
    if (_wasmPromise) return _wasmPromise;
    _wasmPromise = (async () => {
        const mod = await import(PKG_URL);
        if (typeof mod.default === 'function') {
            await mod.default();
        }
        if (typeof mod.init === 'function') {
            try { mod.init(); } catch (_) { /* idempotent or already-initialized */ }
        }
        _wasm = mod;
        return mod;
    })();
    return _wasmPromise;
}

// ── Session key (in-memory only) ───────────────────────────────

let _sessionKey = null;

/** @returns {CryptoKey | null} */
export function getSessionKey() {
    return _sessionKey;
}

/** @param {CryptoKey | null} key */
export function setSessionKey(key) {
    const wasUnlocked = _sessionKey !== null;
    _sessionKey = key;
    if (key && !wasUnlocked) appEvents.dispatchEvent(new Event('session-unlocked'));
    if (!key && wasUnlocked) appEvents.dispatchEvent(new Event('session-locked'));
}

export function lockSession() { setSessionKey(null); }
export function isSessionUnlocked() { return _sessionKey !== null; }

// ── Service-worker registration ────────────────────────────────

let _swRegistration = null;

/** @param {ServiceWorkerRegistration | null} reg */
export function setSwRegistration(reg) { _swRegistration = reg; }

/** @returns {ServiceWorkerRegistration | null} */
export function getSwRegistration() { return _swRegistration; }

/**
 * Convenience: post a message to the active service worker, if one is
 * controlling the page. Returns `false` when there's no controller.
 *
 * @param {any} msg
 * @returns {boolean}
 */
export function postToServiceWorker(msg) {
    const ctl = (typeof navigator !== 'undefined' && navigator.serviceWorker)
        ? navigator.serviceWorker.controller
        : null;
    if (!ctl) return false;
    ctl.postMessage(msg);
    return true;
}

// ── Local model handle (in-memory only) ────────────────────────
//
// Holds the wasm-side `LocalModelHandle` returned by
// `init_local_model(weights, tokenizer, modelId)`. Owned exclusively
// by `providers/local.js` — UI code and other providers should treat
// this as opaque.

let _localModelHandle = null;
let _localModelId = null;

/** @returns {object | null} */
export function getLocalModelHandle() {
    return _localModelHandle;
}

/** @returns {string | null} */
export function getLocalModelId() {
    return _localModelId;
}

/**
 * @param {object | null} handle the wasm handle, or null to clear
 * @param {string | null} [modelId] the registered id (e.g. 'gemma-4-e2b')
 */
export function setLocalModelHandle(handle, modelId = null) {
    const wasLoaded = _localModelHandle !== null;
    _localModelHandle = handle;
    _localModelId = handle ? modelId : null;
    if (handle && !wasLoaded) {
        appEvents.dispatchEvent(new CustomEvent('local-model-loaded', { detail: { modelId: _localModelId } }));
    }
    if (!handle && wasLoaded) {
        appEvents.dispatchEvent(new Event('local-model-unloaded'));
    }
}

// ── Decrypted session key (in-memory only) ─────────────────────
//
// Convenience alias for `getSessionKey()` so the providers layer can
// reach for a more descriptive name. The underlying slot is shared.

/** @returns {CryptoKey | null} */
export function getDecryptedSessionKey() {
    return _sessionKey;
}

/** @param {CryptoKey | null} key */
export function setDecryptedSessionKey(key) {
    setSessionKey(key);
}

// ── App-wide pub/sub ───────────────────────────────────────────
//
// Known event types (consumers should listen for these, dispatchers fire
// CustomEvent('name', { detail: ... }) where applicable):
//   - 'session-unlocked'  / 'session-locked'   (Event)
//   - 'chat-chunk'   { conversationId, messageId, delta } (CustomEvent.detail)
//   - 'chat-done'    { conversationId, messageId, usage }
//   - 'chat-error'   { conversationId, messageId, error }
//   - 'chat_chunk' / 'chat_done' / 'chat_error' (mirrors of the SW
//      message types; `providers/local.js` dispatches these so the UI
//      doesn't care whether a stream is cloud-via-SW or local-via-WASM)
//   - 'model_progress' { modelId, file, fileBytesDone, ... }
//   - 'model_deleted'  { modelId }
//   - 'local-model-loaded' / 'local-model-unloaded'
//   - 'sw-ready'     { registration }
export const appEvents = new EventTarget();

// Alias for code that prefers `state.events` over `state.appEvents`.
// They reference the same `EventTarget` — pick whichever reads better.
export const events = appEvents;
