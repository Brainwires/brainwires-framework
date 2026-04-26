// brainwires-chat-pwa — Anthropic provider adapter
//
// Cloud, SSE. The SW does the actual fetch; this module only:
//   - shapes the request envelope (with `__API_KEY__` sentinel)
//   - maps `{role, content}` chat history → Anthropic's expected shape,
//     including system-message extraction into the top-level `system` field
//   - parses individual SSE events into a portable `{delta, usage, finished}`

export const id = 'anthropic';
export const displayName = 'Anthropic Claude';
export const runtime = 'cloud';
export const format = 'sse';
export const defaultModel = 'claude-opus-4-7';
export const models = [
    'claude-opus-4-7',
    'claude-sonnet-4-6',
    'claude-haiku-4-5',
];

const ENDPOINT = 'https://api.anthropic.com/v1/messages';
const API_VERSION = '2023-06-01';

/**
 * Extract a single concatenated system prompt from `system` messages.
 * Returns `''` when there are none.
 */
function extractSystem(messages) {
    return messages
        .filter((m) => m && m.role === 'system' && typeof m.content === 'string')
        .map((m) => m.content)
        .join('\n\n');
}

/**
 * Map our portable history shape to Anthropic's `messages` array.
 * Anthropic only accepts `user` and `assistant`; system is extracted.
 * Tool/function-call shapes are out of scope for v1.
 */
function mapMessages(messages) {
    const out = [];
    for (const m of messages) {
        if (!m || typeof m !== 'object') continue;
        if (m.role === 'system') continue;
        const role = m.role === 'assistant' ? 'assistant' : 'user';
        const content = typeof m.content === 'string' ? m.content : '';
        out.push({ role, content });
    }
    return out;
}

/**
 * @param {object} args
 * @param {string} args.model
 * @param {Array<{role: string, content: string}>} args.messages
 * @param {object} args.params
 * @returns {{url: string, method: string, headers: object, body: string, format: 'sse'}}
 */
export function buildRequest({ model, messages, params = {} }) {
    const body = {
        model: model || defaultModel,
        messages: mapMessages(messages),
        max_tokens: params.max_tokens || params.maxTokens || 1024,
        stream: true,
    };
    const sys = extractSystem(messages);
    if (sys) body.system = sys;
    if (typeof params.temperature === 'number') body.temperature = params.temperature;
    if (typeof params.top_p === 'number') body.top_p = params.top_p;

    return {
        url: ENDPOINT,
        method: 'POST',
        headers: {
            'content-type': 'application/json',
            'anthropic-version': API_VERSION,
            // SW substitutes the literal `__API_KEY__` after decrypting
            // the encrypted blob it receives in the same `chat_start`.
            'x-api-key': '__API_KEY__',
        },
        body: JSON.stringify(body),
        format: 'sse',
    };
}

/**
 * Parse a single SSE event from the streaming.js generator.
 *
 * The streaming.js shape is `{ type: 'event', event, data, done }`.
 * Anthropic's relevant events:
 *   - `content_block_delta` with `delta.type === 'text_delta'` → text
 *   - `message_delta` with `usage` → token counts
 *   - `message_stop` → end of message
 *
 * @param {{event?: string, data?: string, done?: boolean}} ev
 * @returns {{delta?: string, usage?: object, finished?: boolean} | null}
 */
export function parseChunk(ev) {
    if (!ev) return null;
    if (ev.done) return { finished: true };
    if (!ev.data || ev.data === '') return null;
    let payload;
    try { payload = JSON.parse(ev.data); } catch (_) { return null; }
    const t = payload.type || ev.event;

    if (t === 'content_block_delta') {
        const d = payload.delta || {};
        if (d.type === 'text_delta' && typeof d.text === 'string') {
            return { delta: d.text };
        }
        return null;
    }
    if (t === 'message_delta') {
        const usage = payload.usage || (payload.delta && payload.delta.usage);
        if (usage) return { usage };
        return null;
    }
    if (t === 'message_stop') {
        return { finished: true };
    }
    // message_start / content_block_start / ping / etc. — ignore.
    return null;
}
