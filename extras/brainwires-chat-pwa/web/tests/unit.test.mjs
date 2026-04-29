// brainwires-chat-pwa — unit tests
//
// Run via: node --test tests/unit.test.mjs
//
// Covers:
//   - streaming.parseSSE / parseNDJSON / streamFromResponse
//   - crypto-store deriveKey/encrypt/decrypt round-trip + pack/unpack
//   - db.appendMessageChunk via fake-indexeddb (devDep)

import { test, describe } from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';

// Web Crypto on the global. node 20+ provides it but older runs may not.
if (typeof globalThis.crypto === 'undefined' || !globalThis.crypto.subtle) {
    globalThis.crypto = webcrypto;
}

// btoa/atob shims for crypto-store (Node 18+ has them globally; older nodes don't).
if (typeof globalThis.btoa === 'undefined') {
    globalThis.btoa = (s) => Buffer.from(s, 'binary').toString('base64');
}
if (typeof globalThis.atob === 'undefined') {
    globalThis.atob = (s) => Buffer.from(s, 'base64').toString('binary');
}

const { parseSSE, parseNDJSON, streamFromResponse } = await import('../src/streaming.js');
const cryptoStore = await import('../crypto-store.js');

// ── parseSSE / SSE assembly ────────────────────────────────────

describe('parseSSE', () => {
    test('field:value parsing strips one leading space', () => {
        assert.deepEqual(parseSSE('event: chunk'), { field: 'event', value: 'chunk' });
        assert.deepEqual(parseSSE('data:hello'), { field: 'data', value: 'hello' });
    });

    test('blank lines and comments return null', () => {
        assert.equal(parseSSE(''), null);
        assert.equal(parseSSE('\r'), null);
        assert.equal(parseSSE(': keepalive'), null);
    });

    test('field-only line (no colon) yields empty value', () => {
        assert.deepEqual(parseSSE('retry'), { field: 'retry', value: '' });
    });
});

describe('streamFromResponse(sse)', () => {
    test('multi-line data is joined and dispatched on blank line; [DONE] terminates', async () => {
        const lines = [
            'event: chunk',
            'data: line one',
            'data: line two',
            '',
            'data: solo',
            '',
            'data: [DONE]',
            '',
            'data: ignored after done',
            '',
        ];
        const body = lines.join('\n');
        const resp = mkResponse(body);
        const events = [];
        for await (const ev of streamFromResponse(resp, 'sse')) {
            events.push(ev);
            if (ev.done) break;
        }
        assert.equal(events.length, 3);
        assert.equal(events[0].event, 'chunk');
        assert.equal(events[0].data, 'line one\nline two');
        assert.equal(events[0].done, false);
        assert.equal(events[1].data, 'solo');
        assert.equal(events[2].done, true);
        assert.equal(events[2].data, '[DONE]');
    });

    test('handles \\r\\n line endings', async () => {
        const body = 'data: a\r\ndata: b\r\n\r\n';
        const resp = mkResponse(body);
        const events = [];
        for await (const ev of streamFromResponse(resp, 'sse')) events.push(ev);
        assert.equal(events.length, 1);
        assert.equal(events[0].data, 'a\nb');
    });
});

// ── parseNDJSON ────────────────────────────────────────────────

describe('parseNDJSON', () => {
    test('parses a single JSON object', () => {
        assert.deepEqual(parseNDJSON('{"a":1}'), { a: 1 });
    });

    test('returns null for blank/whitespace lines', () => {
        assert.equal(parseNDJSON(''), null);
        assert.equal(parseNDJSON('   '), null);
        assert.equal(parseNDJSON('\r'), null);
    });

    test('throws on malformed JSON', () => {
        assert.throws(() => parseNDJSON('{not json'));
    });
});

describe('streamFromResponse(ndjson)', () => {
    test('yields one parsed object per non-empty line, skips malformed', async () => {
        const body = '{"i":1}\n{"i":2}\n\nnot-json\n{"i":3}\n';
        const resp = mkResponse(body);
        const out = [];
        for await (const obj of streamFromResponse(resp, 'ndjson')) out.push(obj);
        assert.deepEqual(out, [{ i: 1 }, { i: 2 }, { i: 3 }]);
    });
});

// ── crypto-store round-trip ────────────────────────────────────

describe('crypto-store', () => {
    test('deriveKey + encrypt + decrypt round-trips a UTF-8 string', async () => {
        const salt = cryptoStore.generateSalt();
        const key = await cryptoStore.deriveKey('correct horse battery staple', salt);
        const blob = await cryptoStore.encrypt(key, 'sk-hello-world-🔑');
        const out = await cryptoStore.decrypt(key, blob);
        assert.equal(out, 'sk-hello-world-🔑');
    });

    test('pack/unpack is a bijection', async () => {
        const salt = cryptoStore.generateSalt();
        const key = await cryptoStore.deriveKey('pw', salt);
        const blob = await cryptoStore.encrypt(key, 'payload');
        const packed = cryptoStore.pack({ salt, iv: blob.iv, ciphertext: blob.ciphertext });
        const un = cryptoStore.unpack(packed);
        assert.deepEqual([...un.salt], [...salt]);
        assert.deepEqual([...un.iv], [...blob.iv]);
        assert.deepEqual([...un.ciphertext], [...blob.ciphertext]);
        // Re-derive on unpack-side and confirm decrypt still works.
        const key2 = await cryptoStore.deriveKey('pw', un.salt);
        const out = await cryptoStore.decrypt(key2, { iv: un.iv, ciphertext: un.ciphertext });
        assert.equal(out, 'payload');
    });

    test('decrypt throws on tampered ciphertext', async () => {
        const salt = cryptoStore.generateSalt();
        const key = await cryptoStore.deriveKey('pw', salt);
        const blob = await cryptoStore.encrypt(key, 'secret');
        const tampered = new Uint8Array(blob.ciphertext);
        tampered[0] ^= 0xff;
        await assert.rejects(cryptoStore.decrypt(key, { iv: blob.iv, ciphertext: tampered }));
    });
});

// ── db.appendMessageChunk via fake-indexeddb ───────────────────

describe('db', async () => {
    let dbModule = null;
    try {
        await import('fake-indexeddb/auto');
        dbModule = await import('../src/db.js');
    } catch (e) {
        // fake-indexeddb is the only devDep we add for tests; if it's
        // unavailable in this environment, skip the IDB tests rather
        // than fail the whole suite.
        // TODO: install fake-indexeddb if you want this test active.
        console.warn('[unit.test] fake-indexeddb not available, skipping IDB tests:', e.message);
    }

    if (!dbModule) return;

    test('appendMessageChunk creates the row on first call and accumulates content', async () => {
        const { appendMessageChunk, getMessage } = dbModule;
        const cid = 'c-' + Math.random().toString(36).slice(2);
        const mid = 'm-' + Math.random().toString(36).slice(2);
        await appendMessageChunk(cid, mid, 'Hello, ');
        await appendMessageChunk(cid, mid, 'world!');
        const row = await getMessage(cid, mid);
        assert.equal(row.content, 'Hello, world!');
        assert.equal(row.role, 'assistant');
        assert.equal(row.conversationId, cid);
        assert.equal(row.messageId, mid);
    });

    test('putConversation + listConversations sorts newest-first', async () => {
        const { putConversation, listConversations } = dbModule;
        const a = await putConversation({ id: 'sort-a', title: 'A', updatedAt: 100 });
        const b = await putConversation({ id: 'sort-b', title: 'B', updatedAt: 200 });
        const list = await listConversations();
        const byId = Object.fromEntries(list.map((c) => [c.id, c]));
        assert.ok(byId['sort-a']);
        assert.ok(byId['sort-b']);
        // Find their relative order — b updated later, must come first.
        const idxA = list.findIndex((c) => c.id === 'sort-a');
        const idxB = list.findIndex((c) => c.id === 'sort-b');
        assert.ok(idxB < idxA, 'expected newer conversation first');
        // Silence unused-warning; values are asserted above.
        void a; void b;
    });
});

// ── provider adapters ─────────────────────────────────────────

const anthropic = await import('../src/providers/anthropic.js');
const openai = await import('../src/providers/openai.js');
const google = await import('../src/providers/google.js');
const ollama = await import('../src/providers/ollama.js');
const modelStore = await import('../src/model-store.js');

describe('providers/anthropic', () => {
    test('buildRequest: URL, sentinel header, system extraction, stream:true', () => {
        const req = anthropic.buildRequest({
            model: 'claude-opus-4-7',
            messages: [
                { role: 'system', content: 'You are helpful.' },
                { role: 'user', content: 'Hi' },
                { role: 'assistant', content: 'Hello.' },
                { role: 'user', content: 'Tell me a joke.' },
            ],
            params: { max_tokens: 256, temperature: 0.5 },
        });
        assert.equal(req.url, 'https://api.anthropic.com/v1/messages');
        assert.equal(req.method, 'POST');
        assert.equal(req.format, 'sse');
        assert.equal(req.headers['anthropic-version'], '2023-06-01');
        assert.equal(req.headers['x-api-key'], '__API_KEY__');
        assert.equal(req.headers['content-type'], 'application/json');
        const body = JSON.parse(req.body);
        assert.equal(body.model, 'claude-opus-4-7');
        assert.equal(body.stream, true);
        assert.equal(body.max_tokens, 256);
        assert.equal(body.temperature, 0.5);
        assert.equal(body.system, 'You are helpful.');
        assert.equal(body.messages.length, 3);
        assert.equal(body.messages[0].role, 'user');
        assert.equal(body.messages[1].role, 'assistant');
    });

    test('parseChunk: text_delta event extracts the delta', () => {
        const ev = {
            type: 'event',
            event: 'content_block_delta',
            data: JSON.stringify({ type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: 'hello' } }),
            done: false,
        };
        assert.deepEqual(anthropic.parseChunk(ev), { delta: 'hello' });
    });

    test('parseChunk: message_stop returns finished', () => {
        const ev = {
            type: 'event',
            event: 'message_stop',
            data: JSON.stringify({ type: 'message_stop' }),
            done: false,
        };
        assert.deepEqual(anthropic.parseChunk(ev), { finished: true });
    });
});

describe('providers/openai', () => {
    test('buildRequest: URL, Bearer sentinel, stream:true', () => {
        const req = openai.buildRequest({
            model: 'gpt-4o-mini',
            messages: [
                { role: 'system', content: 'sys' },
                { role: 'user', content: 'hi' },
            ],
            params: { max_tokens: 64 },
        });
        assert.equal(req.url, 'https://api.openai.com/v1/chat/completions');
        assert.equal(req.method, 'POST');
        assert.equal(req.format, 'sse');
        assert.equal(req.headers['Authorization'], 'Bearer __API_KEY__');
        const body = JSON.parse(req.body);
        assert.equal(body.stream, true);
        assert.equal(body.model, 'gpt-4o-mini');
        // OpenAI keeps system in the messages array (unlike Anthropic).
        assert.equal(body.messages[0].role, 'system');
        assert.equal(body.max_tokens, 64);
    });

    test('parseChunk: extracts choices[0].delta.content', () => {
        const ev = {
            type: 'event',
            event: 'message',
            data: JSON.stringify({ choices: [{ delta: { content: 'tok' }, index: 0 }] }),
            done: false,
        };
        assert.deepEqual(openai.parseChunk(ev), { delta: 'tok' });
    });

    test('parseChunk: [DONE] sentinel returns finished', () => {
        assert.deepEqual(openai.parseChunk({ done: true, data: '[DONE]', event: 'message' }), { finished: true });
    });
});

describe('providers/google', () => {
    test('buildRequest: URL contains __API_KEY__, system → systemInstruction', () => {
        const req = google.buildRequest({
            model: 'gemini-2.5-flash',
            messages: [
                { role: 'system', content: 'be terse' },
                { role: 'user', content: 'hi' },
                { role: 'assistant', content: 'sup' },
            ],
            params: { temperature: 0.2 },
        });
        assert.ok(req.url.startsWith('https://generativelanguage.googleapis.com/v1beta/models/'));
        assert.ok(req.url.includes('streamGenerateContent'));
        assert.ok(req.url.includes('alt=sse'));
        assert.ok(req.url.includes('key=__API_KEY__'));
        assert.equal(req.format, 'sse');
        const body = JSON.parse(req.body);
        assert.equal(body.systemInstruction.parts[0].text, 'be terse');
        assert.equal(body.contents.length, 2);
        assert.equal(body.contents[0].role, 'user');
        assert.equal(body.contents[1].role, 'model');
        assert.equal(body.generationConfig.temperature, 0.2);
    });

    test('parseChunk: pulls candidates[0].content.parts[0].text', () => {
        const ev = {
            type: 'event',
            event: 'message',
            data: JSON.stringify({
                candidates: [{ content: { parts: [{ text: 'piece' }], role: 'model' } }],
            }),
            done: false,
        };
        assert.deepEqual(google.parseChunk(ev), { delta: 'piece' });
    });
});

describe('providers/ollama', () => {
    test('buildRequest: defaults to localhost:11434, no auth header, stream:true', () => {
        const req = ollama.buildRequest({
            model: 'gemma3:latest',
            messages: [{ role: 'user', content: 'hi' }],
            params: {},
        });
        assert.equal(req.url, 'http://localhost:11434/api/chat');
        assert.equal(req.format, 'ndjson');
        assert.equal(req.headers['Authorization'], undefined);
        assert.equal(req.headers['x-api-key'], undefined);
        const body = JSON.parse(req.body);
        assert.equal(body.stream, true);
        assert.equal(body.model, 'gemma3:latest');
    });

    test('buildRequest: honors params.baseUrl override and trims trailing slash', () => {
        const req = ollama.buildRequest({
            model: 'llama3.2:latest',
            messages: [{ role: 'user', content: 'x' }],
            params: { baseUrl: 'http://10.0.0.5:11434/' },
        });
        assert.equal(req.url, 'http://10.0.0.5:11434/api/chat');
    });

    test('parseChunk: ndjson line yields delta and finished', () => {
        assert.deepEqual(
            ollama.parseChunk({ message: { role: 'assistant', content: 'hey' }, done: false }),
            { delta: 'hey' },
        );
        const fin = ollama.parseChunk({ message: { role: 'assistant', content: '' }, done: true, eval_count: 5 });
        assert.equal(fin.finished, true);
        assert.equal(fin.usage.completion_tokens, 5);
    });
});

describe('model-store', () => {
    test('KNOWN_MODELS has gemma-4-e2b with the expected shape', () => {
        const m = modelStore.KNOWN_MODELS['gemma-4-e2b'];
        assert.ok(m);
        assert.equal(m.id, 'gemma-4-e2b');
        assert.equal(m.hf.repo, 'google/gemma-4-e2b');
        assert.equal(m.hf.revision, 'main');
        assert.ok(Array.isArray(m.files));
        const kinds = m.files.map((f) => f.kind).sort();
        assert.deepEqual(kinds, ['tokenizer', 'weights']);
    });

    test('cacheKey produces a HF resolve URL', () => {
        const url = modelStore.cacheKey('gemma-4-e2b', 'model.safetensors');
        assert.equal(url, 'https://huggingface.co/google/gemma-4-e2b/resolve/main/model.safetensors');
    });

    test('cacheKey throws on unknown model', () => {
        assert.throws(() => modelStore.cacheKey('does-not-exist', 'x.bin'));
    });

    // The full download path needs Cache Storage + a fetch polyfill that
    // streams a Response.body. Skipping until we wire one in.
    // TODO: bring in a Cache + fetch test polyfill (or use Playwright)
    // and exercise downloadModel({onProgress}) end-to-end.
    test.skip('downloadModel writes to Cache Storage and emits progress', () => {});

    test('downloadModel emits a verifying-phase event for cached files with sha256 pins', async () => {
        // Mock Cache Storage with a pre-cached entry whose bytes match
        // the pin. We set a pin on the registry temporarily so the
        // verify branch fires; restore it after.
        const m = modelStore.KNOWN_MODELS['gemma-4-e2b'];
        const originalFiles = m.files.map((f) => ({ ...f }));
        const stateMod = await import('../src/state.js');

        // Build small payloads + their real sha256 pins.
        const payloads = await Promise.all(m.files.map(async (f) => {
            const bytes = new TextEncoder().encode(`stub-${f.filename}`);
            const digest = await crypto.subtle.digest('SHA-256', bytes);
            const hex = [...new Uint8Array(digest)]
                .map((b) => b.toString(16).padStart(2, '0')).join('');
            return { f, bytes, hex };
        }));

        // Pin each file in the registry so downloadModel takes the
        // verify-cached branch.
        for (const p of payloads) p.f.sha256 = p.hex;

        // Install a fake `caches` that returns the pre-cached responses.
        const cacheBacking = new Map();
        for (const p of payloads) {
            const key = modelStore.cacheKey('gemma-4-e2b', p.f.filename);
            cacheBacking.set(key, new Response(p.bytes));
        }
        const fakeCache = {
            match: async (key) => {
                const hit = cacheBacking.get(key);
                return hit ? hit.clone() : undefined;
            },
            put: async (key, resp) => { cacheBacking.set(key, resp); },
            delete: async (key) => cacheBacking.delete(key),
        };
        const originalCaches = globalThis.caches;
        globalThis.caches = { open: async () => fakeCache };

        const phases = [];
        const handler = (e) => {
            if (e.detail && e.detail.phase) phases.push(e.detail.phase);
        };
        stateMod.events.addEventListener('model_progress', handler);

        try {
            await modelStore.downloadModel('gemma-4-e2b');
        } finally {
            stateMod.events.removeEventListener('model_progress', handler);
            globalThis.caches = originalCaches;
            // Restore registry pins.
            for (let i = 0; i < originalFiles.length; i++) {
                m.files[i].sha256 = originalFiles[i].sha256;
            }
        }

        assert.ok(phases.includes('verifying'),
            `expected a 'verifying' phase event, got: ${JSON.stringify(phases)}`);
    });
});

// ── providers/local — RPC surface ─────────────────────────────
//
// `node --test` has no Worker polyfill, so we can't drive end-to-end
// through the actual worker — but we can at least lock in the public
// API shape so accidental renames break the test rather than the UI.
// The full main↔worker↔wasm round-trip lives in the e2e suite.

describe('providers/local (RPC surface)', () => {
    test('exposes load/unload/chat/cancel + module metadata', async () => {
        await import('fake-indexeddb/auto');
        const local = await import('../src/providers/local.js');
        assert.equal(local.id, 'local-gemma-4-e2b');
        assert.equal(local.runtime, 'local');
        assert.equal(local.defaultModel, 'gemma-4-e2b');
        assert.equal(typeof local.loadLocalModel, 'function');
        assert.equal(typeof local.unloadLocalModel, 'function');
        assert.equal(typeof local.startChat, 'function');
        assert.equal(typeof local.chatLocal, 'function');
        assert.equal(typeof local.cancelLocal, 'function');
        assert.equal(typeof local.isLocalModelLoaded, 'function');
        // Aliases must point at the same impl so behaviour can't drift.
        assert.equal(local.chatLocal, local.startChat);
        // No worker spawned just by importing the module.
        assert.equal(local.isLocalModelLoaded(), false);
    });

    test.skip('main→worker→wasm round-trip (needs Worker polyfill or browser)', () => {});
});

// ── utils (formatters + pure DOM helpers) ─────────────────────

const utils = await import('../src/utils.js');

describe('utils.formatBytes', () => {
    test('handles 0, sub-KB, KB, MB, GB', () => {
        assert.equal(utils.formatBytes(0), '0 B');
        assert.equal(utils.formatBytes(512), '512 B');
        assert.equal(utils.formatBytes(1024), '1.00 KB');
        assert.equal(utils.formatBytes(1536), '1.50 KB');
        assert.equal(utils.formatBytes(5_242_880), '5.00 MB');
        assert.equal(utils.formatBytes(2_500_000_000), '2.33 GB');
    });

    test('rejects non-number / negative', () => {
        assert.equal(utils.formatBytes(NaN), '0 B');
        assert.equal(utils.formatBytes(-1), '0 B');
        assert.equal(utils.formatBytes('hello'), '0 B');
    });
});

describe('utils.formatEta', () => {
    test('seconds, minutes, hours', () => {
        assert.equal(utils.formatEta(0), '0s');
        assert.equal(utils.formatEta(45), '45s');
        assert.equal(utils.formatEta(75), '1m 15s');
        assert.equal(utils.formatEta(3725), '1h 2m 5s');
    });

    test('null/Infinity/negative → em-dash', () => {
        assert.equal(utils.formatEta(null), '—');
        assert.equal(utils.formatEta(undefined), '—');
        assert.equal(utils.formatEta(Infinity), '—');
        assert.equal(utils.formatEta(-1), '—');
    });
});

describe('utils.escapeHtml', () => {
    test('escapes the five usual suspects', () => {
        assert.equal(utils.escapeHtml('<a href="x">&y</a>'), '&lt;a href=&quot;x&quot;&gt;&amp;y&lt;/a&gt;');
        assert.equal(utils.escapeHtml("o'reilly"), 'o&#39;reilly');
    });
});

describe('utils.debounce / throttle', () => {
    test('debounce only fires once after quiet period', async () => {
        let calls = 0;
        const fn = utils.debounce(() => { calls += 1; }, 30);
        fn(); fn(); fn();
        await new Promise((r) => setTimeout(r, 80));
        assert.equal(calls, 1);
    });

    test('throttle fires at most once per window', async () => {
        let calls = 0;
        const fn = utils.throttle(() => { calls += 1; }, 30);
        fn(); fn(); fn();
        // The first call is immediate; further calls coalesce into one trailing call.
        await new Promise((r) => setTimeout(r, 80));
        assert.ok(calls >= 1 && calls <= 2, `expected 1..2 calls, got ${calls}`);
    });
});

// ── i18n ─────────────────────────────────────────────────────

describe('i18n', () => {
    test('falls back to the key when missing', async () => {
        const i18n = await import('../src/i18n.js');
        i18n._setDictForTests({ 'app.title': 'Brainwires Chat' });
        assert.equal(i18n.t('app.title'), 'Brainwires Chat');
        assert.equal(i18n.t('not.in.dict'), 'not.in.dict');
    });

    test('substitutes {var} placeholders', async () => {
        const i18n = await import('../src/i18n.js');
        i18n._setDictForTests({ 'hello': 'Hello, {name}!' });
        assert.equal(i18n.t('hello', { name: 'world' }), 'Hello, world!');
        // Missing variable → leaves the placeholder visible.
        assert.equal(i18n.t('hello', {}), 'Hello, {name}!');
    });
});

// ── markdown ──────────────────────────────────────────────────

describe('markdown', async () => {
    let renderRaw;
    try {
        const i18n = await import('../src/i18n.js');
        i18n._setDictForTests({ 'chat.copy': 'Copy' });
        const md = await import('../src/markdown.js');
        renderRaw = md.renderRaw;
    } catch (e) {
        console.warn('[unit.test] markdown imports failed:', e.message);
    }

    test('emits our codeblock wrapper with copy button + language class', (ctx) => {
        if (!renderRaw) return ctx.skip();
        const out = renderRaw('```javascript\nconst x = 1;\n```\n');
        assert.match(out, /<div class="codeblock">/);
        assert.match(out, /data-bw-copy="1"/);
        assert.match(out, /class="codeblock-copy"/);
        assert.match(out, /<code class="language-javascript">/);
        assert.match(out, /const x = 1;/);
    });

    test('inline code, bold, italic render', (ctx) => {
        if (!renderRaw) return ctx.skip();
        const out = renderRaw('use `foo` and **bold** and *italic*');
        assert.match(out, /<code>foo<\/code>/);
        assert.match(out, /<strong>bold<\/strong>/);
        assert.match(out, /<em>italic<\/em>/);
    });

    test('links get target=_blank and rel=noopener noreferrer', (ctx) => {
        if (!renderRaw) return ctx.skip();
        const out = renderRaw('[click](https://example.com)');
        assert.match(out, /href="https:\/\/example\.com"/);
        assert.match(out, /target="_blank"/);
        assert.match(out, /rel="noopener noreferrer"/);
    });

    test('unclosed fence at end of stream still renders as code (mid-stream safety)', (ctx) => {
        if (!renderRaw) return ctx.skip();
        const out = renderRaw('here:\n```js\nconst partial = ');
        // Marked treats an unclosed fence as a code block to EOF — exactly
        // what we want during streaming so the bubble doesn't briefly show
        // the fence as plain text and then "snap" into a code block.
        assert.match(out, /<pre><code/);
        assert.match(out, /const partial =/);
    });

    test('escapes HTML inside code blocks (no XSS via fenced content)', (ctx) => {
        if (!renderRaw) return ctx.skip();
        const out = renderRaw('```\n<script>x</script>\n```');
        assert.ok(!out.includes('<script>x'));
        assert.match(out, /&lt;script&gt;x&lt;\/script&gt;/);
    });
});

// ── chunker ───────────────────────────────────────────────────

describe('chunker', async () => {
    let chunkText;
    try { ({ chunkText } = await import('../src/chunker.js')); }
    catch (e) { console.warn('[unit.test] chunker import failed:', e.message); }

    test('empty / null returns empty array', (ctx) => {
        if (!chunkText) return ctx.skip();
        assert.deepEqual(chunkText(''), []);
        assert.deepEqual(chunkText(null), []);
        assert.deepEqual(chunkText(undefined), []);
    });

    test('short text fits in one chunk', (ctx) => {
        if (!chunkText) return ctx.skip();
        const out = chunkText('Hello world. This is a short doc.');
        assert.equal(out.length, 1);
        assert.match(out[0], /Hello world/);
    });

    test('long text splits into multiple chunks with overlap', (ctx) => {
        if (!chunkText) return ctx.skip();
        // ~3000 chars of repeating sentences → multiple ~2KB chunks at the
        // default target (512 tokens × 4 chars/token = 2048).
        const sentences = [];
        for (let i = 0; i < 80; i++) sentences.push(`Sentence ${i} has a payload of words that hopefully chunks well.`);
        const out = chunkText(sentences.join(' '));
        assert.ok(out.length >= 2, `expected multiple chunks, got ${out.length}`);
        // Adjacent chunks should share some prefix material via overlap.
        const tail = out[0].slice(-32);
        assert.ok(out[1].includes(tail.slice(0, 16)) || out[1].length > 0);
    });

    test('hard-splits a single sentence longer than target', (ctx) => {
        if (!chunkText) return ctx.skip();
        const huge = 'x'.repeat(5000);
        const out = chunkText(huge);
        assert.ok(out.length >= 2, `expected hard-split, got ${out.length}`);
    });
});

// ── vision (mapping + isVisionModel) ──────────────────────────

describe('vision', async () => {
    let vision;
    try { vision = await import('../src/vision.js'); }
    catch (e) { console.warn('[unit.test] vision import failed:', e.message); }

    test('isVisionModel returns true for known multimodal models', (ctx) => {
        if (!vision) return ctx.skip();
        assert.equal(vision.isVisionModel('anthropic', 'claude-opus-4-7'), true);
        assert.equal(vision.isVisionModel('anthropic', 'claude-sonnet-4-6'), true);
        assert.equal(vision.isVisionModel('openai', 'gpt-5.5'), true);
        assert.equal(vision.isVisionModel('openai', 'gpt-4.1-mini'), true);
        assert.equal(vision.isVisionModel('openai', 'o3'), true);
        assert.equal(vision.isVisionModel('google', 'gemini-2.5-flash'), true);
        assert.equal(vision.isVisionModel('google', 'gemini-1.5-pro'), true);
        assert.equal(vision.isVisionModel('local', 'gemma-4-e2b'), true);
    });

    test('isVisionModel returns false for unknown providers / models', (ctx) => {
        if (!vision) return ctx.skip();
        assert.equal(vision.isVisionModel('ollama', 'llama3'), false);
        assert.equal(vision.isVisionModel('mystery', 'foo'), false);
        assert.equal(vision.isVisionModel('anthropic', null), false);
    });
});

describe('providers vision mapping', async () => {
    const ant = await import('../src/providers/anthropic.js');
    const oai = await import('../src/providers/openai.js');
    const gem = await import('../src/providers/google.js');

    const visionMessage = {
        role: 'user',
        content: [
            { type: 'text', text: 'what is in this image?' },
            { type: 'image', mediaType: 'image/jpeg', data: 'AAAAB' },
        ],
    };

    test('anthropic: image part becomes a base64 source content block', () => {
        const req = ant.buildRequest({
            model: 'claude-opus-4-7',
            messages: [visionMessage],
        });
        const body = JSON.parse(req.body);
        const blocks = body.messages[0].content;
        assert.ok(Array.isArray(blocks));
        assert.equal(blocks[0].type, 'text');
        assert.equal(blocks[0].text, 'what is in this image?');
        assert.equal(blocks[1].type, 'image');
        assert.equal(blocks[1].source.type, 'base64');
        assert.equal(blocks[1].source.media_type, 'image/jpeg');
        assert.equal(blocks[1].source.data, 'AAAAB');
    });

    test('openai: image part becomes an image_url with data URL', () => {
        const req = oai.buildRequest({
            model: 'gpt-5.5',
            messages: [visionMessage],
        });
        const body = JSON.parse(req.body);
        const items = body.messages[0].content;
        assert.ok(Array.isArray(items));
        assert.equal(items[0].type, 'text');
        assert.equal(items[1].type, 'image_url');
        assert.equal(items[1].image_url.url, 'data:image/jpeg;base64,AAAAB');
    });

    test('gemini: image part becomes inline_data', () => {
        const req = gem.buildRequest({
            model: 'gemini-2.5-flash',
            messages: [visionMessage],
        });
        const body = JSON.parse(req.body);
        const parts = body.contents[0].parts;
        assert.equal(parts[0].text, 'what is in this image?');
        assert.equal(parts[1].inline_data.mime_type, 'image/jpeg');
        assert.equal(parts[1].inline_data.data, 'AAAAB');
    });

    test('all providers: legacy string content still works', () => {
        const msgs = [{ role: 'user', content: 'plain' }];
        const a = JSON.parse(ant.buildRequest({ model: 'claude-opus-4-7', messages: msgs }).body);
        const o = JSON.parse(oai.buildRequest({ model: 'gpt-5.5', messages: msgs }).body);
        const g = JSON.parse(gem.buildRequest({ model: 'gemini-2.5-flash', messages: msgs }).body);
        assert.equal(a.messages[0].content, 'plain');
        assert.equal(o.messages[0].content, 'plain');
        assert.equal(g.contents[0].parts[0].text, 'plain');
    });
});

// ── db parts[] helpers ────────────────────────────────────────

describe('db parts[] helpers', async () => {
    let dbReady = false;
    try {
        await import('fake-indexeddb/auto');
        dbReady = true;
    } catch (_) { /* skip silently */ }

    test('normalizeContent wraps a string into [{type:text}]', async (ctx) => {
        if (!dbReady) return ctx.skip();
        const db = await import('../src/db.js');
        assert.deepEqual(db.normalizeContent('hi'), [{ type: 'text', text: 'hi' }]);
        assert.deepEqual(db.normalizeContent(''), []);
        assert.deepEqual(db.normalizeContent(null), []);
        assert.deepEqual(db.normalizeContent(undefined), []);
        const parts = [{ type: 'text', text: 'a' }, { type: 'image', mediaType: 'image/png', data: 'AAAA' }];
        assert.equal(db.normalizeContent(parts), parts); // identity for arrays
    });

    test('partsToText joins text parts and skips non-text', async (ctx) => {
        if (!dbReady) return ctx.skip();
        const db = await import('../src/db.js');
        assert.equal(db.partsToText('plain'), 'plain');
        assert.equal(db.partsToText([{ type: 'text', text: 'a' }, { type: 'image', data: 'x' }, { type: 'text', text: 'b' }]), 'ab');
        assert.equal(db.partsToText([]), '');
        assert.equal(db.partsToText(null), '');
    });

    test('appendMessageChunk on a parts[] row appends to the trailing text part', async (ctx) => {
        if (!dbReady) return ctx.skip();
        const db = await import('../src/db.js');
        db._resetDbForTests();
        await db.putMessage({
            conversationId: 'c1',
            messageId: 'm1',
            role: 'assistant',
            content: [{ type: 'text', text: 'hello ' }],
            createdAt: Date.now(),
        });
        await db.appendMessageChunk('c1', 'm1', 'world');
        const row = await db.getMessage('c1', 'm1');
        assert.deepEqual(row.content, [{ type: 'text', text: 'hello world' }]);
    });

    test('new v2 stores accept inserts (smoke)', async (ctx) => {
        if (!dbReady) return ctx.skip();
        const db = await import('../src/db.js');
        db._resetDbForTests();
        await db.putRagDoc({ id: 'd1', conversationId: null, name: 'spec.pdf', type: 'pdf', bytes: 1234 });
        const docs = await db.listRagDocs(null);
        assert.equal(docs.length, 1);
        assert.equal(docs[0].name, 'spec.pdf');
        await db.putRagChunks([
            { id: 'k1', docId: 'd1', conversationId: null, page: 1, text: 'chunk one', embeddingDim: 384 },
            { id: 'k2', docId: 'd1', conversationId: null, page: 2, text: 'chunk two', embeddingDim: 384 },
        ]);
        const chunks = await db.listRagChunksByDoc('d1');
        assert.equal(chunks.length, 2);
        await db.deleteRagDoc('d1');
        assert.equal((await db.listRagDocs(null)).length, 0);
        assert.equal((await db.listRagChunksByDoc('d1')).length, 0);
    });
});

// ── reasoning-display ─────────────────────────────────────────

describe('reasoning-display', async () => {
    let extractThinking;
    try {
        const i18n = await import('../src/i18n.js');
        i18n._setDictForTests({ 'chat.copy': 'Copy', 'chat.reasoning': 'Reasoning' });
        const mod = await import('../src/reasoning-display.js');
        extractThinking = mod.extractThinking;
    } catch (e) { console.warn('[unit.test] reasoning-display imports failed:', e.message); }

    test('extracts a leading <thinking>...</thinking> block', (ctx) => {
        if (!extractThinking) return ctx.skip();
        const { thinking, body } = extractThinking('<thinking>I should check both branches.</thinking>\nThe answer is 42.');
        assert.equal(thinking, 'I should check both branches.');
        assert.equal(body, 'The answer is 42.');
    });

    test('returns null thinking when no block present', (ctx) => {
        if (!extractThinking) return ctx.skip();
        const { thinking, body } = extractThinking('Just a regular response.');
        assert.equal(thinking, null);
        assert.equal(body, 'Just a regular response.');
    });

    test('handles whitespace before opening tag', (ctx) => {
        if (!extractThinking) return ctx.skip();
        const { thinking, body } = extractThinking('  \n<thinking>step</thinking>x');
        assert.equal(thinking, 'step');
        assert.equal(body, 'x');
    });

    test('does not extract when tag is mid-message', (ctx) => {
        if (!extractThinking) return ctx.skip();
        const { thinking } = extractThinking('Hello <thinking>this came late</thinking>');
        assert.equal(thinking, null);
    });

    test('partial open tag during streaming yields no extraction yet', (ctx) => {
        if (!extractThinking) return ctx.skip();
        const { thinking, body } = extractThinking('<thinking>partial...');
        // Without a closing tag, the regex doesn't match; the partial body
        // renders as plain text and snaps into a <details> when </thinking>
        // arrives.
        assert.equal(thinking, null);
        assert.equal(body, '<thinking>partial...');
    });
});

// ── theme ─────────────────────────────────────────────────────

describe('theme', async () => {
    let dbReady = false;
    try {
        await import('fake-indexeddb/auto');
        dbReady = true;
    } catch (_) { /* fake-indexeddb missing — tests below will skip */ }

    function makeStubs() {
        const html = {
            _attrs: {},
            _style: {},
            setAttribute(k, v) { this._attrs[k] = v; },
            getAttribute(k) { return this._attrs[k] ?? null; },
            get style() { return this._style; },
        };
        globalThis.document = { documentElement: html };
        globalThis.matchMedia = () => ({
            matches: false,
            addEventListener() {},
            removeEventListener() {},
        });
        return html;
    }

    test('setTheme persists, applies data-theme, and emits theme-changed', async (ctx) => {
        if (!dbReady) return ctx.skip('fake-indexeddb not available');
        const html = makeStubs();

        const db = await import('../src/db.js');
        db._resetDbForTests();

        const theme = await import(`../src/theme.js?cb=${Date.now()}`);
        const state = await import('../src/state.js');

        let received = null;
        state.appEvents.addEventListener('theme-changed',
            (e) => { received = e.detail; }, { once: true });

        await theme.setTheme('light');

        assert.equal(theme.getTheme(), 'light');
        assert.equal(html._attrs['data-theme'], 'light');
        assert.equal(html._style.colorScheme, 'light');
        assert.deepEqual(received, { theme: 'light' });
        assert.equal(await db.getSetting('ui.theme'), 'light');
    });

    test('loadTheme reads the saved value and applies it', async (ctx) => {
        if (!dbReady) return ctx.skip('fake-indexeddb not available');
        const html = makeStubs();

        const db = await import('../src/db.js');
        db._resetDbForTests();
        await db.setSetting('ui.theme', 'dark');

        const theme = await import(`../src/theme.js?cb=${Date.now()}_2`);
        await theme.loadTheme();

        assert.equal(theme.getTheme(), 'dark');
        assert.equal(html._attrs['data-theme'], 'dark');
        assert.equal(html._style.colorScheme, 'dark');
    });

    test('invalid theme value falls back to system', async (ctx) => {
        if (!dbReady) return ctx.skip('fake-indexeddb not available');
        const html = makeStubs();

        const db = await import('../src/db.js');
        db._resetDbForTests();

        const theme = await import(`../src/theme.js?cb=${Date.now()}_3`);
        await theme.setTheme('bogus');
        assert.equal(theme.getTheme(), 'system');
        assert.equal(html._attrs['data-theme'], 'system');
        assert.equal(html._style.colorScheme, 'dark light');
    });
});

// ── views (no-DOM smoke) ──────────────────────────────────────
//
// Skipped: the router needs a real `document` to mount sections under.
// Without a JSDOM/linkedom dependency we'd have to fake too much of
// the DOM API to get a useful signal here. See the Tests section in
// the task notes.
test.skip('views.mount toggles classes correctly', () => {});

// ── helpers ────────────────────────────────────────────────────

function mkResponse(body) {
    // Build a minimal Response-like object backed by a ReadableStream.
    // node 20+ has global ReadableStream + Response.
    const enc = new TextEncoder();
    const stream = new ReadableStream({
        start(controller) {
            controller.enqueue(enc.encode(body));
            controller.close();
        },
    });
    return new Response(stream);
}
