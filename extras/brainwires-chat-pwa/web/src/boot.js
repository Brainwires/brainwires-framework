// brainwires-chat-pwa — entry point
//
// Boot order:
//   1. open IndexedDB + register the SW (parallel)
//   2. wire SW → page chat IPC into the appEvents bus
//   3. wire local-provider events to the same hyphenated channel
//   4. mount the persistent download banner above the view region
//   5. initialize i18n
//   6. set up the view router and register chat / settings / unlock
//   7. route to 'unlock' if a passphrase is configured but the
//      session key isn't loaded; otherwise route to 'chat'
//   8. lazy-init the wasm module in the background — TTS/STT/local
//      providers wait on `getWasm()` themselves; first paint must not.

import { openDb } from './sql-db.js';
import {
    getWasm,
    setSwRegistration,
    appEvents,
    events as stateEvents,
    isSessionUnlocked,
} from './state.js';
import { isDownloaded } from './model-store.js';
import { getSetting } from './sql-db.js';
import * as views from './views.js';
import { mountBanner } from './ui-download-banner.js';
import { initI18n } from './i18n.js';
import { loadTheme } from './theme.js';
import * as uiChat from './ui-chat.js';
import * as uiSettings from './ui-settings.js';
import * as uiUnlock from './ui-unlock.js';
import { maybeInstallDevToggle as maybeInstallHomeDevToggle } from './home-dev-toggle.js';

const PASSPHRASE_SETTING = 'passphraseConfig';


async function isDevMode() {
    try {
        const info = await import('../build-info.js');
        return info.DEV_MODE === true;
    } catch (_) {
        return false;
    }
}

async function registerServiceWorker() {
    if (!('serviceWorker' in navigator)) return null;
    try {
        const reg = await navigator.serviceWorker.register('./sw.js', { scope: './' });
        setSwRegistration(reg);
        appEvents.dispatchEvent(new CustomEvent('sw-ready', { detail: { registration: reg } }));

        // In dev mode: tell the SW to use network-first (no cache/SRI)
        // so live-editing works, but keep the SW alive for model downloads.
        if (await isDevMode()) {
            const ctrl = navigator.serviceWorker.controller || reg.active;
            if (ctrl) ctrl.postMessage({ type: 'set_dev_mode', enabled: true });
            console.log('DEV_MODE: SW registered (network-first, no cache)');
        }
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
                stateEvents.dispatchEvent(new CustomEvent('chat_chunk', { detail: msg }));
                appEvents.dispatchEvent(new CustomEvent('chat-chunk', { detail: msg }));
                break;
            case 'chat_tool_use':
                // MCP plumbing (Follow-up 2): SW broadcasts a fully
                // reassembled tool invocation; the picker / execution
                // loop / bubble rendering land in the next commit.
                stateEvents.dispatchEvent(new CustomEvent('chat_tool_use', { detail: msg }));
                appEvents.dispatchEvent(new CustomEvent('chat-tool-use', { detail: msg }));
                break;
            case 'chat_done':
                stateEvents.dispatchEvent(new CustomEvent('chat_done', { detail: msg }));
                appEvents.dispatchEvent(new CustomEvent('chat-done', { detail: msg }));
                break;
            case 'chat_error':
            case 'chat_aborted':
                stateEvents.dispatchEvent(new CustomEvent('chat_error', { detail: msg }));
                appEvents.dispatchEvent(new CustomEvent('chat-error', { detail: msg }));
                break;
            // open_chat / chat_status / sri_table — not handled until UI lands.
        }
    });
}

// Mirror local-provider events from `state.events` (which providers/local.js
// dispatches under 'chat_chunk' etc) into `appEvents` 'chat-chunk' etc so
// any code that prefers the hyphenated channel still works.
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

async function shouldStartLocked() {
    try {
        const cfg = await getSetting(PASSPHRASE_SETTING);
        if (cfg && cfg.salt && cfg.verify && !isSessionUnlocked()) return true;
    } catch (_) { /* no idb yet */ }
    return false;
}

async function boot() {
    const app = document.getElementById('app');
    const bannerSlot = document.getElementById('download-banner-slot');

    // i18n: read the saved language (settings store), fall back to the
    // detected system locale on first run. Awaiting before view mount so
    // the first paint uses translated strings and `<html lang/dir>` is
    // already set when stylesheets evaluate `[dir="rtl"]` selectors.
    let savedLang = null;
    try { savedLang = (await getSetting('language')) || null; } catch { /* db not open yet */ }
    await initI18n(savedLang).catch(() => {});

    // Mount the persistent footer after i18n so the idle label renders
    // translated on the first paint.
    if (bannerSlot) mountBanner(bannerSlot);

    // DB + SW in parallel.
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

    // Apply the saved theme before any view paints. Falls through to
    // 'system' (the pre-switcher behavior) if nothing is saved.
    await loadTheme().catch((err) => console.warn('theme load failed:', err));

    wireServiceWorkerMessages();
    wireLocalProviderEvents();

    // Set up view router. Each view's render() runs on first activation.
    if (app) {
        views.init(app);
        views.register('chat', uiChat);
        views.register('settings', uiSettings);
        views.register('unlock', uiUnlock);
    }

    // Decide initial view. Respect ?page= query param so reloads stay
    // on the same view (e.g. /?page=settings).
    const requestedPage = new URL(location.href).searchParams.get('page');
    if (await shouldStartLocked()) {
        views.mount('unlock');
    } else if (requestedPage && ['settings', 'chat', 'unlock'].includes(requestedPage)) {
        views.mount(requestedPage);
    } else {
        views.mount('chat');
    }

    // Lazy-init the wasm module in the background. The chat composer
    // uses `voice.getTts()` / `voice.getStt()` which both await
    // `getWasm()` themselves; this just warms the cache so the first
    // user interaction doesn't pay the load cost.
    getWasm().catch((err) => {
        console.warn('wasm warmup failed:', err && err.message ? err.message : err);
    });

    // Hidden developer dial-home toggle (Phase 2 M5). No-op unless the
    // ?home=<url> query param or `bw_dial_home_url` localStorage key
    // is set. M9 will replace this with a real chat-UI integration.
    try { maybeInstallHomeDevToggle(); }
    catch (e) { console.warn('home-dev-toggle install failed:', e && e.message ? e.message : e); }

    // Probe whether the default local model is already cached.
    try {
        const cached = await isDownloaded('gemma-4-e2b-it');
        appEvents.dispatchEvent(new CustomEvent('local-model-cached-status', {
            detail: { modelId: 'gemma-4-e2b-it', cached },
        }));
    } catch (_) { /* Cache Storage may be unavailable in tests/SSR */ }
}

boot().catch((err) => {
    console.error('boot failed:', err);
    const app = document.getElementById('app');
    if (app) app.textContent = `Boot failed: ${err && err.message ? err.message : err}`;
});
