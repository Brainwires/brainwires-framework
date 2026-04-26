// brainwires-chat-pwa — entry point
//
// On boot we:
//   1. register the service worker (sw.js, scope = pwa root)
//   2. open IndexedDB so the schema is created on first run
//   3. lazy-init the wasm module + write a status line into #app
//   4. wire SW → page chat IPC into the appEvents bus (tasks #6/#7
//      will subscribe and render UI; for now we just console-log)
//   5. mirror local-provider chat events (state.events) onto the same
//      hyphenated channel UI subscribes to, so the UI is provider-agnostic

import { openDb } from './db.js';
import {
    getWasm,
    setSwRegistration,
    appEvents,
    events as stateEvents,
} from './state.js';
import { isDownloaded } from './model-store.js';

async function registerServiceWorker() {
    if (!('serviceWorker' in navigator)) return null;
    try {
        const reg = await navigator.serviceWorker.register('./sw.js', { scope: './' });
        setSwRegistration(reg);
        appEvents.dispatchEvent(new CustomEvent('sw-ready', { detail: { registration: reg } }));
        return reg;
    } catch (err) {
        console.warn('SW registration failed:', err && err.message ? err.message : err);
        return null;
    }
}

function wireServiceWorkerMessages() {
    if (!('serviceWorker' in navigator)) return;
    navigator.serviceWorker.addEventListener('message', (event) => {
        const msg = event.data;
        if (!msg || typeof msg !== 'object') return;
        switch (msg.type) {
            case 'chat_chunk':
                appEvents.dispatchEvent(new CustomEvent('chat-chunk', { detail: msg }));
                break;
            case 'chat_done':
                appEvents.dispatchEvent(new CustomEvent('chat-done', { detail: msg }));
                break;
            case 'chat_error':
            case 'chat_aborted':
                appEvents.dispatchEvent(new CustomEvent('chat-error', { detail: msg }));
                break;
            // open_chat / chat_status / sri_table — not handled until UI lands.
        }
    });
}

// Mirror local-provider events into the same hyphenated channel the SW
// path uses. Local providers dispatch `chat_chunk` / `chat_done` /
// `chat_error` (with underscores) on `state.events`. UI code can pick
// either form, but consolidating into 'chat-chunk' / 'chat-done' /
// 'chat-error' on appEvents matches the SW path so UI doesn't branch.
function wireLocalProviderEvents() {
    const fwd = (underscore, hyphen) => {
        stateEvents.addEventListener(underscore, (e) => {
            appEvents.dispatchEvent(new CustomEvent(hyphen, { detail: { type: underscore, ...(e.detail || {}) } }));
        });
    };
    fwd('chat_chunk', 'chat-chunk');
    fwd('chat_done', 'chat-done');
    fwd('chat_error', 'chat-error');
}

async function boot() {
    const app = document.getElementById('app');

    // Fire DB open and SW registration in parallel; neither needs the other.
    const [dbResult, swResult] = await Promise.allSettled([
        openDb(),
        registerServiceWorker(),
    ]);
    if (dbResult.status === 'rejected') {
        console.warn('IndexedDB open failed:', dbResult.reason);
    }
    if (swResult.status === 'rejected') {
        console.warn('SW registration error:', swResult.reason);
    }

    wireServiceWorkerMessages();
    wireLocalProviderEvents();

    try {
        const wasm = await getWasm();
        const v = typeof wasm.version === 'function' ? wasm.version() : 'unknown';
        if (app) app.textContent = `Brainwires Chat v${v}`;
    } catch (err) {
        console.error('boot failed:', err);
        if (app) app.textContent = `Boot failed: ${err && err.message ? err.message : err}`;
    }

    // Probe whether the default local model is already cached. We do
    // NOT auto-load it (a 2.5 GB ArrayBuffer read on every page load
    // would defeat the PWA's snappy-cold-start design); the UI's
    // Settings page is responsible for explicit load/unload.
    try {
        const cached = await isDownloaded('gemma-4-e2b');
        appEvents.dispatchEvent(new CustomEvent('local-model-cached-status', {
            detail: { modelId: 'gemma-4-e2b', cached },
        }));
    } catch (_) { /* Cache Storage may be unavailable in tests/SSR */ }
}

boot();
