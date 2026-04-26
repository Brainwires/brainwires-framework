// brainwires-chat-pwa — provider registry + dispatcher
//
// ── %KEY% / __API_KEY__ substitution contract ──────────────────
//
// Cloud providers MUST NOT embed the plaintext API key in the request
// envelope they hand to the service worker. Instead:
//   - The provider returns `requestPayload = { url, method, headers, body, format }`
//     where the literal string `__API_KEY__` is used wherever the API
//     key needs to land — typically a header value (`x-api-key`,
//     `Authorization: Bearer __API_KEY__`) or, for Gemini, the URL's
//     `?key=__API_KEY__` query parameter.
//   - The page additionally hands the SW the encrypted blob
//     (`apiKeyEncrypted`) and the imported `sessionKey` so the SW can
//     decrypt and substitute *after* the message has crossed the
//     postMessage boundary. The plaintext key never sits in the
//     envelope, never lands in cache, never gets logged.
//   - The SW (`sw.source.js`) walks `headers` and `url` and replaces
//     every literal `__API_KEY__` with the decrypted plaintext.
//
// Local providers (runtime: 'local') drive the WASM module directly
// from this module — no SW round-trip. They emit the same
// `chat_chunk` / `chat_done` / `chat_error` events on
// `state.events` so the UI is provider-agnostic.

import { appEvents, events, postToServiceWorker } from '../state.js';
import * as anthropic from './anthropic.js';
import * as openai from './openai.js';
import * as google from './google.js';
import * as ollama from './ollama.js';
import * as local from './local.js';

const REGISTRY = new Map();
function register(mod) { REGISTRY.set(mod.id, mod); }

register(anthropic);
register(openai);
register(google);
register(ollama);
register(local);

/**
 * @returns {Array<{id: string, runtime: 'cloud'|'local', defaultModel: string, models: string[], displayName?: string}>}
 */
export function listProviders() {
    return Array.from(REGISTRY.values()).map((p) => ({
        id: p.id,
        runtime: p.runtime,
        defaultModel: p.defaultModel,
        models: Array.isArray(p.models) ? p.models.slice() : [p.defaultModel],
        displayName: p.displayName || p.id,
    }));
}

/**
 * @param {string} id
 * @returns {object | null}
 */
export function getProvider(id) {
    return REGISTRY.get(id) || null;
}

/**
 * Single entry point for UI. Routes to the SW (cloud) or WASM (local)
 * depending on the provider's `runtime`.
 *
 * @param {object} args
 * @param {string} args.provider         provider id, e.g. 'anthropic'
 * @param {string} args.conversationId
 * @param {string} args.messageId
 * @param {Array<{role: string, content: string}>} args.messages
 * @param {object} [args.params]         provider-specific (model, max_tokens, temperature, ...)
 * @param {string} [args.apiKeyEncrypted] packed crypto-store blob for cloud providers
 * @param {CryptoKey | Uint8Array} [args.sessionKey] the AES-GCM key (or 32 raw bytes) for the SW to decrypt with
 * @returns {Promise<{ ok: true } | { ok: false, error: string }>}
 */
export async function startChat(args) {
    const { provider, conversationId, messageId, messages, params = {} } = args;
    if (!provider) return { ok: false, error: 'startChat: provider required' };
    if (!conversationId || !messageId) return { ok: false, error: 'startChat: conversationId + messageId required' };
    if (!Array.isArray(messages)) return { ok: false, error: 'startChat: messages must be an array' };

    const p = getProvider(provider);
    if (!p) return { ok: false, error: `startChat: unknown provider '${provider}'` };

    if (p.runtime === 'local') {
        // Local providers handle their own dispatching against state.events.
        try {
            await p.startChat({ conversationId, messageId, messages, params });
            return { ok: true };
        } catch (err) {
            const error = err && err.message ? err.message : String(err);
            events.dispatchEvent(new CustomEvent('chat_error', {
                detail: { conversationId, messageId, error },
            }));
            // Mirror to the legacy 'chat-error' channel boot.js wires.
            appEvents.dispatchEvent(new CustomEvent('chat-error', {
                detail: { type: 'chat_error', conversationId, messageId, error },
            }));
            return { ok: false, error };
        }
    }

    // Cloud path. Build the envelope and ship it to the SW.
    const model = params.model || p.defaultModel;
    let requestPayload;
    try {
        requestPayload = p.buildRequest({
            // We deliberately pass NO plaintext key. providers embed the
            // sentinel `__API_KEY__` so the SW substitutes after decrypt.
            model,
            messages,
            params,
        });
    } catch (err) {
        const error = err && err.message ? err.message : String(err);
        return { ok: false, error };
    }
    if (!requestPayload || !requestPayload.url) {
        return { ok: false, error: `provider '${provider}' produced an empty requestPayload` };
    }

    const ok = postToServiceWorker({
        type: 'chat_start',
        conversationId,
        messageId,
        provider: p.id,
        requestPayload,
        apiKeyEncrypted: args.apiKeyEncrypted || null,
        sessionKey: args.sessionKey || null,
    });
    if (!ok) {
        return { ok: false, error: 'no service worker controller; refresh the page after registration' };
    }
    return { ok: true };
}

/**
 * Helper: take an SSE event dict and the provider id, return the
 * provider's parseChunk result (or null). Used by tests and any UI
 * code that wants to render raw broadcasts directly without round-
 * tripping through `appendMessageChunk`.
 */
export function parseProviderChunk(providerId, ev) {
    const p = getProvider(providerId);
    if (!p || typeof p.parseChunk !== 'function') return null;
    try { return p.parseChunk(ev); } catch (_) { return null; }
}
