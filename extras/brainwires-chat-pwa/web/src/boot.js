// brainwires-chat-pwa entry point (scaffold).
//
// Boots the wasm module and renders a one-line "loaded" indicator into
// #app. Real chat/streaming/voice UI lands in task #4–#7.

const PKG_URL = './pkg/brainwires_chat_pwa.js';

async function boot() {
    const app = document.getElementById('app');
    try {
        const wasm = await import(PKG_URL);
        await wasm.default(); // wasm-pack init
        wasm.init();
        const v = wasm.version();
        if (app) app.textContent = `Brainwires Chat v${v}`;
    } catch (err) {
        console.error('boot failed:', err);
        if (app) app.textContent = `Boot failed: ${err && err.message ? err.message : err}`;
    }
}

boot();
