// brainwires-chat-pwa — OpenAI provider adapter (chat completions, SSE)
//
// Substitution contract: `Authorization: Bearer __API_KEY__` is
// rewritten by the SW after AES-GCM decrypt. See providers/index.js
// for the full contract.

export const id = 'openai';
export const displayName = 'OpenAI';
export const runtime = 'cloud';
export const format = 'sse';
export const defaultModel = 'gpt-4o-mini';
export const models = [
    'gpt-4o-mini',
    'gpt-4o',
    'gpt-4.1-mini',
    'gpt-4.1',
];

const ENDPOINT = 'https://api.openai.com/v1/chat/completions';

/**
 * OpenAI's `messages` shape matches ours 1:1; we just normalize to
 * `{role, content}` strings (no tool calls in v1).
 */
function mapMessages(messages) {
    const out = [];
    for (const m of messages) {
        if (!m || typeof m !== 'object') continue;
        const role = m.role === 'assistant' ? 'assistant'
            : m.role === 'system' ? 'system'
            : 'user';
        const content = typeof m.content === 'string' ? m.content : '';
        out.push({ role, content });
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
