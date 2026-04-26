// brainwires-chat-pwa service worker (scaffold).
//
// Task #5 replaces this with the real background-resilient streaming SW.
// For now: skipWaiting on install, claim on activate, network-passthrough
// on fetch. No caching, no offline support yet.
//
// build.mjs substitutes the __SRI_HASHES__ token below at bundle time
// with a JSON object mapping each pinned static asset to its
// "sha256-<base64>" digest. The table is unused in this scaffold but
// the placeholder is kept so the build pipeline has a target.

const RESOURCE_HASHES = __SRI_HASHES__;
// Reference RESOURCE_HASHES so the bundler keeps the substitution in
// the emitted file even though no caching code consumes it yet.
self.__BW_CHAT_PWA_HASHES__ = RESOURCE_HASHES;

self.addEventListener('install', (event) => {
    event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (_event) => {
    // Network passthrough — the browser handles the request normally.
});
