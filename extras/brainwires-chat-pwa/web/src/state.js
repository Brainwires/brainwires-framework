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

// ── App-wide pub/sub ───────────────────────────────────────────
//
// Known event types (consumers should listen for these, dispatchers fire
// CustomEvent('name', { detail: ... }) where applicable):
//   - 'session-unlocked'  / 'session-locked'   (Event)
//   - 'chat-chunk'   { conversationId, messageId, delta } (CustomEvent.detail)
//   - 'chat-done'    { conversationId, messageId, usage }
//   - 'chat-error'   { conversationId, messageId, error }
//   - 'sw-ready'     { registration }
export const appEvents = new EventTarget();
