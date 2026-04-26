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
