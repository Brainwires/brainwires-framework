// brainwires-chat-pwa — OpenAI provider adapter (chat completions, SSE)
//
// Substitution contract: `Authorization: Bearer __API_KEY__` is
// rewritten by the SW after AES-GCM decrypt. See providers/index.js
// for the full contract.

export const id = 'openai';
export const displayName = 'OpenAI';
export const runtime = 'cloud';
export const format = 'sse';
export const defaultModel = 'gpt-5.5';
export const models = [
    'gpt-5.5',
    'gpt-5.5-pro',
    'gpt-5.4',
    'gpt-5.4-mini',
    'gpt-5.4-nano',
    'gpt-5.4-pro',
    'gpt-5.2',
    'gpt-5.2-pro',
    'gpt-5.1',
    'gpt-5',
    'gpt-5-mini',
    'gpt-5-nano',
    'o4-mini',
    'o3',
    'o3-pro',
    'o3-mini',
    'o1',
    'gpt-4.1',
    'gpt-4.1-mini',
    'gpt-4.1-nano',
];

const ENDPOINT = 'https://api.openai.com/v1/chat/completions';

/**
 * Translate one of our parts to an OpenAI chat-completions content item.
 * Unknown / unsupported parts are dropped.
 */
function partToOpenAI(p) {
    if (!p || typeof p !== 'object') return null;
    if (p.type === 'text') {
        return typeof p.text === 'string' ? { type: 'text', text: p.text } : null;
    }
    if (p.type === 'image' && typeof p.data === 'string') {
        const mt = p.mediaType || 'image/jpeg';
        return { type: 'image_url', image_url: { url: `data:${mt};base64,${p.data}` } };
    }
    return null;
}

/**
 * Map our portable history to OpenAI's `messages` array. String content is
 * passed through; parts[] is expanded into the typed content-array shape
 * (chat-completions accepts both forms on a per-message basis).
 */
function mapMessages(messages) {
    const out = [];
    for (const m of messages) {
        if (!m || typeof m !== 'object') continue;
        const role = m.role === 'assistant' ? 'assistant'
            : m.role === 'system' ? 'system'
            : 'user';
        if (typeof m.content === 'string') {
            out.push({ role, content: m.content });
            continue;
        }
        if (Array.isArray(m.content)) {
            const items = m.content.map(partToOpenAI).filter(Boolean);
            if (items.length) out.push({ role, content: items });
            continue;
        }
        out.push({ role, content: '' });
    }
    return out;
}

export function buildRequest({ model, messages, params = {} }) {
    const body = {
        model: model || defaultModel,
        messages: mapMessages(messages),
        stream: true,
    };
    if (typeof params.temperature === 'number') body.temperature = params.temperature;
    if (typeof params.top_p === 'number') body.top_p = params.top_p;
    if (typeof params.max_tokens === 'number') body.max_tokens = params.max_tokens;
    else if (typeof params.maxTokens === 'number') body.max_tokens = params.maxTokens;

    return {
        url: ENDPOINT,
        method: 'POST',
        headers: {
            'content-type': 'application/json',
            'Authorization': 'Bearer __API_KEY__',
        },
        body: JSON.stringify(body),
        format: 'sse',
    };
}

/**
 * @param {{event?: string, data?: string, done?: boolean}} ev
 * @returns {{delta?: string, usage?: object, finished?: boolean} | null}
 */
export function parseChunk(ev) {
    if (!ev) return null;
    if (ev.done) return { finished: true }; // [DONE] sentinel from streaming.js
    if (!ev.data || ev.data === '') return null;
    let payload;
    try { payload = JSON.parse(ev.data); } catch (_) { return null; }

    const choices = Array.isArray(payload.choices) ? payload.choices : [];
    const c0 = choices[0];
    if (!c0) {
        if (payload.usage) return { usage: payload.usage };
        return null;
    }
    const delta = (c0.delta && typeof c0.delta.content === 'string') ? c0.delta.content : '';
    const finishReason = c0.finish_reason || c0.finishReason;
    const out = {};
    if (delta) out.delta = delta;
    if (finishReason) out.finished = true;
    if (payload.usage) out.usage = payload.usage;
    return Object.keys(out).length ? out : null;
}
