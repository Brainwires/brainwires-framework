// brainwires-chat-pwa — entry point
//
// On boot we:
//   1. register the service worker (sw.js, scope = pwa root)
//   2. open IndexedDB so the schema is created on first run
//   3. lazy-init the wasm module + write a status line into #app
//   4. wire SW → page chat IPC into the appEvents bus (tasks #6/#7
//      will subscribe and render UI; for now we just console-log)

import { openDb } from './db.js';
import {
    getWasm,
    setSwRegistration,
    appEvents,
} from './state.js';

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

    try {
        const wasm = await getWasm();
        const v = typeof wasm.version === 'function' ? wasm.version() : 'unknown';
        if (app) app.textContent = `Brainwires Chat v${v}`;
    } catch (err) {
        console.error('boot failed:', err);
        if (app) app.textContent = `Boot failed: ${err && err.message ? err.message : err}`;
    }
}

boot();
