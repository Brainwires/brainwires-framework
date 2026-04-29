// brainwires-chat-pwa — Private RAG orchestrator
//
// Pipeline:
//   ingest(file, conversationId?) →
//     extract text (PDF via pdf.js, txt as-is) →
//     chunk into ~512-token windows with 64-token overlap →
//     embed each chunk via the local worker's embed_text →
//     persist chunks + insert vectors into the conversation's HNSW index →
//     save the index to OPFS
//
// retrieve(query, opts) →
//     embed query →
//     index.search(vec, k) →
//     return top-k chunks (text, page, docId, score)
//
// All steps run on the user's device. No network calls past the embedding
// model download (handled separately by Settings → Embedding models).

import { getWasm } from './state.js';
import { getSetting, putRagDoc, listRagDocs, putRagChunks, listRagChunksByDoc, deleteRagDoc as dbDeleteRagDoc } from './db.js';
import { loadModel, embed, embedBatch, loadedDim } from './embeddings.js';
import { saveIndex, loadIndex } from './vector-store.js';
import { chunkText } from './chunker.js';
import { genId } from './utils.js';

const _indexCache = new Map(); // indexName → wasm LocalVectorIndex

function indexNameFor(conversationId) {
    return conversationId ? `rag-${conversationId}` : 'rag-global';
}

async function getOrCreateIndex(conversationId) {
    const name = indexNameFor(conversationId);
    if (_indexCache.has(name)) return _indexCache.get(name);
    const wasm = await getWasm();
    if (!wasm || typeof wasm.LocalVectorIndex !== 'function') {
        throw new Error('wasm.LocalVectorIndex not available — rebuild the WASM crate');
    }
    let idx = await loadIndex(wasm, name);
    if (!idx) {
        const dim = loadedDim() || 384;
        idx = new wasm.LocalVectorIndex(name, dim);
    }
    _indexCache.set(name, idx);
    return idx;
}

async function activeEmbeddingModel() {
    const m = await getSetting('embedding.activeModel');
    if (!m) throw new Error('No embedding model selected. Open Settings → Embedding models to choose one.');
    return m;
}

async function readFileAsText(file) {
    if (!file) return '';
    const t = file.type || '';
    if (t === 'application/pdf' || /\.pdf$/i.test(file.name || '')) {
        const { extractText } = await import('./pdf-text.js');
        const { pages } = await extractText(file);
        return { kind: 'pdf', pages };
    }
    // Text or unknown — read as UTF-8.
    const text = await file.text();
    return { kind: 'text', pages: [{ page: 1, text }] };
}

/**
 * Ingest a File or Blob: extract text, chunk, embed, persist, index, save.
 *
 * @param {File} file
 * @param {object} [opts]
 * @param {string|null} [opts.conversationId=null]   null → global library
 * @param {(p: { phase: string, current?: number, total?: number }) => void} [opts.onProgress]
 * @returns {Promise<{ docId: string, chunkCount: number }>}
 */
export async function ingest(file, opts = {}) {
    const conversationId = opts.conversationId ?? null;
    const onProgress = typeof opts.onProgress === 'function' ? opts.onProgress : () => {};

    onProgress({ phase: 'extract' });
    const extracted = await readFileAsText(file);

    onProgress({ phase: 'chunk' });
    const allChunks = []; // { id, page, text }
    for (const p of extracted.pages) {
        const pieces = chunkText(p.text);
        for (const piece of pieces) {
            allChunks.push({ id: genId('chunk'), page: p.page, text: piece });
        }
    }
    if (allChunks.length === 0) {
        throw new Error('Document had no extractable text.');
    }

    const modelId = await activeEmbeddingModel();
    onProgress({ phase: 'embed_load' });
    await loadModel(modelId);

    const docId = genId('doc');
    await putRagDoc({
        id: docId,
        conversationId,
        name: file.name || 'document',
        type: extracted.kind,
        bytes: file.size || 0,
    });

    const dim = loadedDim();
    const idx = await getOrCreateIndex(conversationId);

    // Embed in small batches with progress so the UI can show a meaningful bar.
    const BATCH = 16;
    const persistedRows = [];
    for (let i = 0; i < allChunks.length; i += BATCH) {
        const batch = allChunks.slice(i, i + BATCH);
        onProgress({ phase: 'embed', current: i, total: allChunks.length });
        const vectors = await embedBatch(batch.map((c) => c.text));
        for (let j = 0; j < batch.length; j++) {
            const c = batch[j];
            const v = vectors[j];
            const meta = JSON.stringify({ chunkId: c.id, docId, page: c.page });
            try { idx.insert(v, meta); }
            catch (e) { console.warn('[rag] vector insert failed:', e && e.message); }
            persistedRows.push({
                id: c.id, docId, conversationId, page: c.page, text: c.text, embeddingDim: dim,
            });
        }
    }
    onProgress({ phase: 'persist', current: allChunks.length, total: allChunks.length });
    await putRagChunks(persistedRows);

    onProgress({ phase: 'save_index' });
    try { await saveIndex(idx); }
    catch (e) { console.warn('[rag] saveIndex failed:', e && e.message); }

    onProgress({ phase: 'done' });
    return { docId, chunkCount: allChunks.length };
}

/**
 * Retrieve the top-k chunks for `query`. Returns hits sorted best-first.
 *
 * @param {string} query
 * @param {object} [opts]
 * @param {string|null} [opts.conversationId=null]
 * @param {number} [opts.k=4]
 * @returns {Promise<Array<{ text: string, docId: string, page: number, chunkId: string, score: number }>>}
 */
export async function retrieve(query, opts = {}) {
    const conversationId = opts.conversationId ?? null;
    const k = opts.k || 4;
    if (typeof query !== 'string' || query.trim().length === 0) return [];

    // No docs ingested yet → fast path.
    const docs = await listRagDocs(conversationId);
    if (docs.length === 0) return [];

    const modelId = await activeEmbeddingModel();
    await loadModel(modelId);
    const qvec = await embed(query);
    const idx = await getOrCreateIndex(conversationId);
    if (!idx || typeof idx.search !== 'function') return [];

    let raw;
    try { raw = idx.search(qvec, k); }
    catch (e) { console.warn('[rag] search failed:', e && e.message); return []; }

    // The wasm `search` shape isn't pinned across versions — defensively
    // accept either an array of {meta, score} objects or a JSON string.
    let hits = [];
    if (typeof raw === 'string') { try { hits = JSON.parse(raw); } catch (_) {} }
    else if (Array.isArray(raw)) hits = raw;

    const out = [];
    for (const h of hits) {
        let meta;
        try { meta = typeof h.meta === 'string' ? JSON.parse(h.meta) : (h.meta || {}); }
        catch (_) { meta = {}; }
        if (!meta.chunkId) continue;
        // Look up the chunk text from IDB. We already have it in memory at
        // ingest time but cross-session retrieval needs the round-trip.
        const chunks = await listRagChunksByDoc(meta.docId);
        const chunk = chunks.find((c) => c.id === meta.chunkId);
        if (!chunk) continue;
        out.push({
            text: chunk.text,
            docId: meta.docId,
            page: meta.page || 1,
            chunkId: chunk.id,
            score: h.score || 0,
        });
    }
    return out;
}

/**
 * Format retrieved hits into a synthetic system message that prepends
 * cite-tagged sources. Returns an empty string when there are no hits.
 *
 * @param {Array<{ text: string, page: number }>} hits
 * @returns {string}
 */
export function formatRetrievalAsSystem(hits) {
    if (!Array.isArray(hits) || hits.length === 0) return '';
    const parts = ['Use the following sources to answer. Cite them as [1], [2], etc.\n'];
    hits.forEach((h, i) => {
        parts.push(`[${i + 1}] (page ${h.page}) ${h.text}`);
    });
    return parts.join('\n\n');
}

/**
 * Delete a RAG doc and rebuild the conversation's vector index without it.
 * The vector index has no per-row delete, so we drop and rebuild from
 * remaining chunks. Cheap when collections are small.
 *
 * @param {string} docId
 * @param {string|null} conversationId
 */
export async function deleteRagDoc(docId, conversationId = null) {
    await dbDeleteRagDoc(docId);
    // Rebuild the index for this conversation from the surviving docs.
    const wasm = await getWasm();
    if (!wasm || typeof wasm.LocalVectorIndex !== 'function') return;
    const name = indexNameFor(conversationId);
    const dim = loadedDim() || 384;
    const fresh = new wasm.LocalVectorIndex(name, dim);
    const remaining = await listRagDocs(conversationId);
    for (const d of remaining) {
        const chunks = await listRagChunksByDoc(d.id);
        for (const c of chunks) {
            // We only have stored text, not vectors. Re-embedding a small
            // residual library is cheap and avoids an additional store.
            try {
                await loadModel(await activeEmbeddingModel());
                const v = await embed(c.text);
                fresh.insert(v, JSON.stringify({ chunkId: c.id, docId: d.id, page: c.page }));
            } catch (e) { console.warn('[rag] reindex failed:', e && e.message); }
        }
    }
    _indexCache.set(name, fresh);
    try { await saveIndex(fresh); } catch (_) { /* best effort */ }
}
