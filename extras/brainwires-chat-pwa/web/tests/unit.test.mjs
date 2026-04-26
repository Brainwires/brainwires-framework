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

    // The download path needs Cache Storage + a fetch polyfill that
    // streams a Response.body. Skipping until we wire one in.
    // TODO: bring in a Cache + fetch test polyfill (or use Playwright)
    // and exercise downloadModel({onProgress}) end-to-end.
    test.skip('downloadModel writes to Cache Storage and emits progress', () => {});
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
