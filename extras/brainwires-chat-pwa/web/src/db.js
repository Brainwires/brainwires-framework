// brainwires-chat-pwa — IndexedDB wrapper
//
// One DB, four stores. Streaming chunks are persisted via
// `appendMessageChunk` from both the SW (network streams) and the page
// (local-wasm streams). All operations are async; no external deps.

const DB_NAME = 'bw-chat-db';
const DB_VERSION = 1;

// ── Schema ─────────────────────────────────────────────────────
//
// conversations: { id, title, createdAt, updatedAt, ... }
//   - keyPath: 'id' (string)
//   - index byUpdatedAt → updatedAt
//
// messages: { conversationId, messageId, role, content, usage, ... }
//   - keyPath: ['conversationId', 'messageId']
//   - index byConversation → conversationId
//
// settings: { key, value }
//   - keyPath: 'key'
//
// voicePrefs: { key, value }
//   - keyPath: 'key'

let _dbPromise = null;

/**
 * Open (or create) the brainwires chat database. Subsequent calls return
 * the cached connection — IndexedDB sessions are cheap to reuse.
 *
 * @returns {Promise<IDBDatabase>}
 */
export function openDb() {
    if (_dbPromise) return _dbPromise;
    _dbPromise = new Promise((resolve, reject) => {
        let resolved = false;
        const timeout = setTimeout(() => {
            if (!resolved) { resolved = true; reject(new Error('IndexedDB open timeout')); }
        }, 5000);

        const finish = (fn, value) => {
            if (resolved) return;
            resolved = true;
            clearTimeout(timeout);
            fn(value);
        };

        try {
            const req = indexedDB.open(DB_NAME, DB_VERSION);
            req.onupgradeneeded = (e) => {
                const db = e.target.result;
                if (!db.objectStoreNames.contains('conversations')) {
                    const s = db.createObjectStore('conversations', { keyPath: 'id' });
                    s.createIndex('byUpdatedAt', 'updatedAt', { unique: false });
                }
                if (!db.objectStoreNames.contains('messages')) {
                    const s = db.createObjectStore('messages', {
                        keyPath: ['conversationId', 'messageId'],
                    });
                    s.createIndex('byConversation', 'conversationId', { unique: false });
                }
                if (!db.objectStoreNames.contains('settings')) {
                    db.createObjectStore('settings', { keyPath: 'key' });
                }
                if (!db.objectStoreNames.contains('voicePrefs')) {
                    db.createObjectStore('voicePrefs', { keyPath: 'key' });
                }
            };
            req.onsuccess = () => finish(resolve, req.result);
            req.onerror = () => finish(reject, req.error);
            req.onblocked = () => finish(reject, new Error('IndexedDB blocked'));
        } catch (e) {
            finish(reject, e);
        }
    });
    return _dbPromise;
}

/**
 * Reset the cached connection. Test-only; production code should never need this.
 */
export function _resetDbForTests() {
    _dbPromise = null;
}

// ── Generic helpers (internal) ─────────────────────────────────

function txPromise(tx) {
    return new Promise((resolve, reject) => {
        tx.oncomplete = () => resolve();
        tx.onerror = () => reject(tx.error);
        tx.onabort = () => reject(tx.error || new Error('Transaction aborted'));
    });
}

function reqPromise(req) {
    return new Promise((resolve, reject) => {
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

// ── Conversations ──────────────────────────────────────────────

/**
 * Upsert a conversation row. `updatedAt` is stamped to `Date.now()` if absent
 * so the byUpdatedAt index is always populated.
 *
 * @param {{ id: string, [k: string]: any }} conversation
 */
export async function putConversation(conversation) {
    if (!conversation || !conversation.id) {
        throw new Error('putConversation: id required');
    }
    const row = { ...conversation };
    if (row.updatedAt === undefined) row.updatedAt = Date.now();
    if (row.createdAt === undefined) row.createdAt = row.updatedAt;
    const db = await openDb();
    const tx = db.transaction('conversations', 'readwrite');
    tx.objectStore('conversations').put(row);
    await txPromise(tx);
    return row;
}

/**
 * @param {string} id
 * @returns {Promise<object | undefined>}
 */
export async function getConversation(id) {
    const db = await openDb();
    const tx = db.transaction('conversations', 'readonly');
    return reqPromise(tx.objectStore('conversations').get(id));
}

/**
 * List all conversations, newest-first by updatedAt.
 * @returns {Promise<object[]>}
 */
export async function listConversations() {
    const db = await openDb();
    const tx = db.transaction('conversations', 'readonly');
    const idx = tx.objectStore('conversations').index('byUpdatedAt');
    // openCursor with 'prev' walks the index in reverse (newest first).
    return new Promise((resolve, reject) => {
        const out = [];
        const req = idx.openCursor(null, 'prev');
        req.onsuccess = () => {
            const cur = req.result;
            if (!cur) { resolve(out); return; }
            out.push(cur.value);
            cur.continue();
        };
        req.onerror = () => reject(req.error);
    });
}

/**
 * Delete a conversation and cascade-delete its messages.
 *
 * @param {string} id
 */
export async function deleteConversation(id) {
    const db = await openDb();
    const tx = db.transaction(['conversations', 'messages'], 'readwrite');
    tx.objectStore('conversations').delete(id);
    const msgIdx = tx.objectStore('messages').index('byConversation');
    const cursorReq = msgIdx.openCursor(IDBKeyRange.only(id));
    await new Promise((resolve, reject) => {
        cursorReq.onsuccess = () => {
            const cur = cursorReq.result;
            if (!cur) { resolve(); return; }
            cur.delete();
            cur.continue();
        };
        cursorReq.onerror = () => reject(cursorReq.error);
    });
    await txPromise(tx);
}

// ── Messages ───────────────────────────────────────────────────

/**
 * Append `delta` text to the message identified by [conversationId, messageId].
 * Read-modify-write under one readwrite transaction so concurrent SW writes
 * don't lose data. Returns the message row after the append.
 *
 * @param {string} conversationId
 * @param {string} messageId
 * @param {string} delta
 * @returns {Promise<object>}
 */
export async function appendMessageChunk(conversationId, messageId, delta) {
    const db = await openDb();
    const tx = db.transaction('messages', 'readwrite');
    const store = tx.objectStore('messages');
    const existing = await reqPromise(store.get([conversationId, messageId]));
    const row = existing || {
        conversationId,
        messageId,
        role: 'assistant',
        content: '',
        createdAt: Date.now(),
        updatedAt: Date.now(),
    };
    row.content = (row.content || '') + (delta || '');
    row.updatedAt = Date.now();
    store.put(row);
    await txPromise(tx);
    return row;
}

/**
 * @param {string} conversationId
 * @param {string} messageId
 */
export async function getMessage(conversationId, messageId) {
    const db = await openDb();
    const tx = db.transaction('messages', 'readonly');
    return reqPromise(tx.objectStore('messages').get([conversationId, messageId]));
}

/**
 * Replace a message row wholesale (e.g. final write at stream end).
 *
 * @param {object} row
 */
export async function putMessage(row) {
    if (!row || !row.conversationId || !row.messageId) {
        throw new Error('putMessage: conversationId + messageId required');
    }
    const next = { ...row };
    if (next.updatedAt === undefined) next.updatedAt = Date.now();
    const db = await openDb();
    const tx = db.transaction('messages', 'readwrite');
    tx.objectStore('messages').put(next);
    await txPromise(tx);
    return next;
}

/**
 * @param {string} conversationId
 * @returns {Promise<object[]>} messages for the conversation, oldest first by createdAt.
 */
export async function listMessages(conversationId) {
    const db = await openDb();
    const tx = db.transaction('messages', 'readonly');
    const idx = tx.objectStore('messages').index('byConversation');
    const req = idx.getAll(IDBKeyRange.only(conversationId));
    const rows = await reqPromise(req);
    rows.sort((a, b) => (a.createdAt || 0) - (b.createdAt || 0));
    return rows;
}

// ── Settings / voicePrefs ──────────────────────────────────────

export async function setSetting(key, value) {
    const db = await openDb();
    const tx = db.transaction('settings', 'readwrite');
    tx.objectStore('settings').put({ key, value });
    await txPromise(tx);
}

export async function getSetting(key) {
    const db = await openDb();
    const tx = db.transaction('settings', 'readonly');
    const row = await reqPromise(tx.objectStore('settings').get(key));
    return row ? row.value : undefined;
}

export async function setVoicePref(key, value) {
    const db = await openDb();
    const tx = db.transaction('voicePrefs', 'readwrite');
    tx.objectStore('voicePrefs').put({ key, value });
    await txPromise(tx);
}

export async function getVoicePref(key) {
    const db = await openDb();
    const tx = db.transaction('voicePrefs', 'readonly');
    const row = await reqPromise(tx.objectStore('voicePrefs').get(key));
    return row ? row.value : undefined;
}
