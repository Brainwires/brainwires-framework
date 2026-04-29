// brainwires-chat-pwa — chat view
//
// Mobile-first chat surface. Vanilla DOM, no framework. Renders:
//   - header (drawer toggle, title, overflow menu)
//   - off-canvas conversation drawer
//   - messages list (user / assistant bubbles)
//   - composer with mic + textarea + send + provider chip
//
// Streaming is fed from `state.events`:
//   - 'chat_chunk' { conversationId, messageId, delta }
//   - 'chat_done'  { conversationId, messageId, usage }
//   - 'chat_error' { conversationId, messageId, error }
// The same payloads also surface on `appEvents` as 'chat-chunk' /
// 'chat-done' / 'chat-error' (see boot.js fan-out). We listen on
// `state.events` because the underscore form is closer to source-of-truth.

import { events as stateEvents, getSessionKey, isSessionUnlocked } from './state.js';
import {
    listConversations,
    putConversation,
    deleteConversation,
    listMessages,
    putMessage,
    setSetting,
    getSetting,
} from './db.js';
import { listProviders, startChat } from './providers/index.js';
import * as localProvider from './providers/local.js';
import { isDownloaded } from './model-store.js';
import { el, clear, toast, isMobile, genId, escapeHtml } from './utils.js';
import { t } from './i18n.js';
import { renderMarkdown } from './markdown.js';
import * as voice from './voice.js';
import { isDownloadActive, activeModelId } from './ui-download-banner.js';
import * as cryptoStore from '../crypto-store.js';
import { mount as mountView } from './views.js';

// ── Module state ───────────────────────────────────────────────

let _root = null;
let _conversationId = null;
let _conversation = null;          // { id, title, providerId, ... }
let _messages = [];                // array of { messageId, role, content, ... }
let _streaming = null;             // { messageId, bubble, contentNode, finalized }
let _autoScroll = true;
let _activeProviderId = null;

// DOM cache
const _ui = {
    titleEl: null,
    listEl: null,
    drawerEl: null,
    drawerListEl: null,
    composer: null,
    textarea: null,
    sendBtn: null,
    micBtn: null,
    providerChip: null,
    listening: false,
};

// ── Public render ──────────────────────────────────────────────

export async function render(root) {
    _root = root;
    clear(root);
    root.appendChild(buildLayout());
    subscribeStreams();
    bindVisualViewport();
    await refreshConversations();
    await refreshActiveProvider();

    // Pick the most recent conversation (or create a new one).
    let id = await getSetting('chat.activeConversationId');
    if (!id) {
        const all = await listConversations();
        id = all && all.length ? all[0].id : null;
    }
    if (!id) id = await newConversation(/* silent */ true);
    await loadConversation(id);
}

export function onShow() {
    if (_ui.textarea && !isMobile()) {
        try { _ui.textarea.focus(); } catch (_) {}
    }
}

// ── Layout ─────────────────────────────────────────────────────

function buildLayout() {
    // Header
    const titleEl = el('h1', { class: 'chat-title', attrs: { 'aria-live': 'polite' } }, t('chat.title'));
    _ui.titleEl = titleEl;

    const drawerToggle = el('button', {
        class: 'icon-btn',
        attrs: { type: 'button', 'aria-label': t('nav.menu'), 'aria-controls': 'conversation-drawer' },
        onClick: () => toggleDrawer(true),
    }, glyph('menu'));

    const overflowBtn = el('button', {
        class: 'icon-btn',
        attrs: { type: 'button', 'aria-label': 'More', 'aria-haspopup': 'menu' },
        onClick: (e) => openOverflowMenu(e.currentTarget),
    }, glyph('dots'));

    const settingsBtn = el('button', {
        class: 'icon-btn',
        attrs: { type: 'button', 'aria-label': t('nav.settings') },
        onClick: () => mountView('settings'),
    }, glyph('gear'));

    const header = el('header', { class: 'chat-header' },
        drawerToggle,
        titleEl,
        settingsBtn,
        overflowBtn,
    );

    // Drawer (off-canvas)
    const drawerListEl = el('ol', { class: 'drawer-list', attrs: { 'aria-label': 'Conversations' } });
    const drawerEl = el('nav', {
        id: 'conversation-drawer',
        class: 'drawer',
        attrs: { 'aria-label': 'Conversations', 'aria-hidden': 'true' },
    },
        el('div', { class: 'drawer-header' },
            el('button', {
                class: 'icon-btn',
                attrs: { type: 'button', 'aria-label': t('nav.close') },
                onClick: () => toggleDrawer(false),
            }, glyph('x')),
            el('strong', { class: 'drawer-title' }, t('app.title')),
        ),
        el('button', {
            class: 'btn drawer-new',
            attrs: { type: 'button' },
            onClick: async () => { const id = await newConversation(); toggleDrawer(false); await loadConversation(id); },
        }, '+ ' + t('chat.newChat')),
        drawerListEl,
    );
    _ui.drawerEl = drawerEl;
    _ui.drawerListEl = drawerListEl;
    const scrim = el('div', {
        class: 'drawer-scrim',
        attrs: { 'aria-hidden': 'true' },
        onClick: () => toggleDrawer(false),
    });

    // Messages
    const listEl = el('ol', { class: 'chat-messages', attrs: { 'aria-live': 'polite' } });
    _ui.listEl = listEl;
    listEl.addEventListener('scroll', () => {
        const nearBottom = listEl.scrollTop + listEl.clientHeight >= listEl.scrollHeight - 24;
        _autoScroll = nearBottom;
    }, { passive: true });

    // Composer
    const textarea = el('textarea', {
        class: 'composer-input',
        attrs: {
            'aria-label': t('chat.placeholder'),
            placeholder: t('chat.placeholder'),
            rows: 1,
            spellcheck: 'true',
            autocomplete: 'off',
            autocapitalize: 'sentences',
        },
    });
    _ui.textarea = textarea;

    textarea.addEventListener('input', () => {
        autoSizeTextarea(textarea);
        updateSendDisabled();
    });
    textarea.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' && !e.shiftKey && !isMobile()) {
            e.preventDefault();
            handleSend();
        }
    });

    const micBtn = el('button', {
        class: 'icon-btn mic-btn',
        attrs: { type: 'button', 'aria-label': t('voice.start') },
    }, glyph('mic'));
    _ui.micBtn = micBtn;
    bindMic(micBtn);

    const sendBtn = el('button', {
        class: 'icon-btn send-btn',
        attrs: { type: 'button', 'aria-label': t('chat.send'), disabled: '' },
        onClick: handleSend,
    }, glyph('send'));
    _ui.sendBtn = sendBtn;

    const providerChip = el('button', {
        class: 'provider-chip',
        attrs: { type: 'button', 'aria-label': t('chat.provider') },
        onClick: cycleProvider,
    }, t('chat.provider'));
    _ui.providerChip = providerChip;

    const composer = el('form', {
        class: 'composer',
        attrs: { 'aria-label': 'Composer' },
        onSubmit: (e) => { e.preventDefault(); handleSend(); },
    },
        el('div', { class: 'composer-row' },
            micBtn,
            textarea,
            sendBtn,
        ),
        el('div', { class: 'composer-meta' },
            providerChip,
        ),
    );
    _ui.composer = composer;

    return el('div', { class: 'chat-shell' },
        header,
        scrim,
        drawerEl,
        listEl,
        composer,
    );
}

// ── Conversations & messages ───────────────────────────────────

async function refreshConversations() {
    const conversations = await listConversations();
    const list = _ui.drawerListEl;
    if (!list) return;
    clear(list);
    if (!conversations.length) {
        list.appendChild(el('li', { class: 'drawer-empty' }, t('chat.empty')));
        return;
    }
    for (const c of conversations) {
        const item = el('li', { class: 'drawer-item' },
            el('button', {
                class: 'drawer-link' + (c.id === _conversationId ? ' is-active' : ''),
                attrs: { type: 'button' },
                onClick: async () => { toggleDrawer(false); await loadConversation(c.id); },
            }, c.title || t('chat.title')),
            el('button', {
                class: 'icon-btn drawer-del',
                attrs: { type: 'button', 'aria-label': t('chat.delete') },
                onClick: async (e) => {
                    e.stopPropagation();
                    if (!confirm(t('chat.confirmDelete'))) return;
                    await deleteConversation(c.id);
                    if (c.id === _conversationId) {
                        const all = await listConversations();
                        const next = all && all.length ? all[0].id : await newConversation(true);
                        await loadConversation(next);
                    } else {
                        await refreshConversations();
                    }
                    toast(t('chat.deleted'), 'success');
                },
            }, glyph('x')),
        );
        list.appendChild(item);
    }
}

async function newConversation(silent = false) {
    const id = genId('conv');
    const row = await putConversation({ id, title: t('chat.newChat'), providerId: _activeProviderId });
    _conversation = row;
    _conversationId = id;
    _messages = [];
    await setSetting('chat.activeConversationId', id);
    if (!silent) {
        await refreshConversations();
        renderMessages();
        setTitle(row.title);
    }
    return id;
}

async function loadConversation(id) {
    _conversationId = id;
    await setSetting('chat.activeConversationId', id);
    const all = await listConversations();
    _conversation = all.find((c) => c.id === id) || null;
    setTitle(_conversation ? (_conversation.title || t('chat.title')) : t('chat.title'));
    _messages = await listMessages(id);
    await refreshConversations();
    renderMessages();
}

function setTitle(title) {
    if (_ui.titleEl) _ui.titleEl.textContent = title || t('chat.title');
}

function renderMessages() {
    if (!_ui.listEl) return;
    clear(_ui.listEl);
    if (_messages.length === 0) {
        const emptyState = el('li', { class: 'chat-empty-state' },
            el('img', { class: 'chat-logo', attrs: { src: 'icons/icon-192.png', alt: 'Brainwires Chat' } }),
            el('h2', {}, 'Brainwires Chat'),
            el('p', { class: 'build-stamp', id: 'build-stamp' }),
        );
        _ui.listEl.appendChild(emptyState);
        import('../build-info.js').then(info => {
            const stamp = document.getElementById('build-stamp');
            if (!stamp || !info) return;
            const parts = [info.BUILD_GIT, info.BUILD_TIME].filter(Boolean);
            stamp.textContent = parts.join(' — ') || 'dev';
            wireHardRefresh(stamp);
        }).catch(() => {});
        return;
    }
    for (const m of _messages) {
        _ui.listEl.appendChild(buildBubble(m));
    }
    requestAnimationFrame(() => scrollToBottom(true));
}

function buildBubble(m) {
    const isUser = m.role === 'user';
    const cls = isUser ? 'bubble bubble-user' : 'bubble bubble-assistant';
    const contentNode = el('div', { class: 'bubble-content' });
    contentNode.innerHTML = renderMarkdown(m.content || '');

    const actions = el('div', { class: 'bubble-actions' });
    actions.appendChild(el('button', {
        class: 'icon-btn',
        attrs: { type: 'button', 'aria-label': t('chat.copy') },
        onClick: () => copyText(m.content || ''),
    }, glyph('copy')));
    if (!isUser) {
        actions.appendChild(el('button', {
            class: 'icon-btn',
            attrs: { type: 'button', 'aria-label': t('chat.speak') },
            onClick: async () => { try { await voice.speak(m.content || ''); } catch (_) {} },
        }, glyph('speaker')));
        actions.appendChild(el('button', {
            class: 'icon-btn',
            attrs: { type: 'button', 'aria-label': t('chat.regenerate') },
            onClick: () => regenerateAt(m.messageId),
        }, glyph('refresh')));
    }

    const li = el('li', { class: cls, attrs: { 'data-msg-id': m.messageId } },
        contentNode,
        actions,
    );
    return li;
}

// ── Send + streaming ───────────────────────────────────────────

async function handleSend() {
    const text = _ui.textarea ? _ui.textarea.value.trim() : '';
    if (!text) return;
    if (!_activeProviderId) { toast(t('error.noProvider'), 'error'); return; }
    if (!await canUseProvider(_activeProviderId)) return;

    if (!_conversationId) await newConversation(true);

    const userMsg = {
        conversationId: _conversationId,
        messageId: genId('msg'),
        role: 'user',
        content: text,
        createdAt: Date.now(),
        updatedAt: Date.now(),
    };
    await putMessage(userMsg);
    _messages.push(userMsg);
    _ui.listEl.querySelector('.chat-empty-state')?.remove();
    _ui.listEl.appendChild(buildBubble(userMsg));

    _ui.textarea.value = '';
    autoSizeTextarea(_ui.textarea);
    updateSendDisabled();

    // If this is the first user message, set the conversation title to a snippet.
    if (_messages.filter((m) => m.role === 'user').length === 1) {
        const snip = text.length > 48 ? text.slice(0, 45) + '…' : text;
        await putConversation({ ..._conversation, id: _conversationId, title: snip });
        setTitle(snip);
        await refreshConversations();
    }

    await runProvider(_messages.map((m) => ({ role: m.role, content: m.content })));
}

async function runProvider(messages) {
    const messageId = genId('msg');
    const placeholder = {
        conversationId: _conversationId,
        messageId,
        role: 'assistant',
        content: '',
        createdAt: Date.now(),
        updatedAt: Date.now(),
    };
    _messages.push(placeholder);
    const bubble = buildBubble(placeholder);
    _ui.listEl.appendChild(bubble);
    _streaming = {
        messageId,
        bubble,
        contentNode: bubble.querySelector('.bubble-content'),
        accum: '',
        finalized: false,
        userMessages: messages,
    };
    scrollToBottom(false);
    await putMessage(placeholder);

    // Resolve API key & session key for cloud providers.
    let apiKeyEncrypted = null;
    let sessionKey = null;
    const providerInfo = listProviders().find((p) => p.id === _activeProviderId);
    if (providerInfo && providerInfo.runtime === 'cloud' && _activeProviderId !== 'ollama') {
        try {
            const blob = await getSetting(`provider.${_activeProviderId}.apiKey`);
            if (blob && blob.encrypted) {
                apiKeyEncrypted = blob.encrypted;
                sessionKey = getSessionKey();
                if (!sessionKey) {
                    streamingError(t('error.locked'));
                    return;
                }
            } else if (blob && blob.plaintext) {
                // We need the SW to substitute, so we have to pack this
                // as an encrypted blob. The SW always expects encrypted.
                // For unencrypted-storage users, we encrypt with an
                // ephemeral session key on the fly so the SW can decrypt.
                // Simpler path: complain and ask them to set a passphrase.
                streamingError('API key stored without encryption — set a passphrase in Settings to send via cloud providers.');
                return;
            } else {
                streamingError(t('error.noKey'));
                return;
            }
        } catch (e) {
            streamingError(e && e.message ? e.message : String(e));
            return;
        }
    }

    const params = {};
    if (providerInfo && _activeProviderId === 'ollama') {
        const baseUrl = await getSetting('provider.ollama.baseUrl');
        if (baseUrl) params.baseUrl = baseUrl;
    }
    const modelOverride = await getSetting(`provider.${_activeProviderId}.model`);
    if (modelOverride) params.model = modelOverride;

    const result = await startChat({
        provider: _activeProviderId,
        conversationId: _conversationId,
        messageId,
        messages,
        params,
        apiKeyEncrypted,
        sessionKey,
    });
    if (!result.ok) {
        streamingError(result.error || t('error.generic'));
    }
}

function streamingError(msg) {
    if (_streaming) {
        const node = _streaming.contentNode;
        node.innerHTML = `<em class="bubble-error">${escapeHtml(msg)}</em>`;
        _streaming.finalized = true;
        _streaming = null;
    }
    toast(msg, 'error');
}

function subscribeStreams() {
    stateEvents.addEventListener('chat_chunk', (e) => {
        const d = e.detail || {};
        if (!_streaming || _streaming.messageId !== d.messageId) return;
        if (typeof d.delta !== 'string') return;
        _streaming.accum += d.delta;
        _streaming.contentNode.innerHTML = renderMarkdown(_streaming.accum);
        if (_autoScroll) scrollToBottom(false);
    });
    stateEvents.addEventListener('chat_done', (e) => {
        const d = e.detail || {};
        if (!_streaming || _streaming.messageId !== d.messageId) return;
        finalizeStreaming();
    });
    stateEvents.addEventListener('chat_error', (e) => {
        const d = e.detail || {};
        if (!_streaming || _streaming.messageId !== d.messageId) return;
        const err = d.error || t('error.generic');
        const node = _streaming.contentNode;
        const prev = _streaming.accum || '';
        node.innerHTML = renderMarkdown(prev) + `<em class="bubble-error"> — ${escapeHtml(err)}</em>`;
        _streaming.finalized = true;
        _streaming = null;
        toast(err, 'error');
    });
}

async function finalizeStreaming() {
    if (!_streaming) return;
    const id = _streaming.messageId;
    const text = _streaming.accum || '';
    // Update local cache + db.
    const idx = _messages.findIndex((m) => m.messageId === id);
    if (idx >= 0) _messages[idx].content = text;
    try {
        await putMessage({
            conversationId: _conversationId,
            messageId: id,
            role: 'assistant',
            content: text,
            updatedAt: Date.now(),
        });
        await putConversation({ ..._conversation, id: _conversationId, updatedAt: Date.now() });
    } catch (_) { /* best-effort */ }
    _streaming.finalized = true;
    _streaming = null;
    refreshConversations();
}

async function regenerateAt(messageId) {
    // Find the user message that produced this assistant message and
    // resend up to (but not including) the assistant message.
    const idx = _messages.findIndex((m) => m.messageId === messageId);
    if (idx < 0) return;
    const slice = _messages.slice(0, idx);
    // Drop the old assistant from the array + DOM; we'll spawn a new one.
    _messages.splice(idx, 1);
    const node = _ui.listEl.querySelector(`[data-msg-id="${messageId}"]`);
    if (node) node.remove();
    await runProvider(slice.map((m) => ({ role: m.role, content: m.content })));
}

// ── Provider handling ──────────────────────────────────────────

async function refreshActiveProvider() {
    const stored = await getSetting('chat.activeProvider');
    const providers = listProviders();
    if (!providers.length) return;
    if (stored && providers.find((p) => p.id === stored)) {
        _activeProviderId = stored;
    } else {
        _activeProviderId = providers[0].id;
    }
    updateProviderChip();
    updateSendDisabled();
}

async function cycleProvider() {
    const providers = listProviders();
    if (!providers.length) return;
    const idx = providers.findIndex((p) => p.id === _activeProviderId);
    const next = providers[(idx + 1) % providers.length];
    _activeProviderId = next.id;
    await setSetting('chat.activeProvider', next.id);
    updateProviderChip();
    updateSendDisabled();
}

function updateProviderChip() {
    if (!_ui.providerChip) return;
    const providers = listProviders();
    const p = providers.find((x) => x.id === _activeProviderId);
    _ui.providerChip.textContent = p ? p.displayName : t('chat.provider');
}

async function canUseProvider(id) {
    const p = listProviders().find((x) => x.id === id);
    if (!p) return false;
    if (p.runtime === 'local') {
        // Need model downloaded + nothing in flight.
        if (isDownloadActive() && activeModelId() === p.defaultModel) {
            toast(t('chat.modelLoading'), 'info');
            return false;
        }
        const ok = await isDownloaded(p.defaultModel);
        if (!ok) {
            toast(t('chat.modelNotReady'), 'error');
            return false;
        }
        if (!localProvider.isLocalModelLoaded()) {
            // Lazy-load — runProvider will block on this otherwise.
            try { await localProvider.loadLocalModel(p.defaultModel); }
            catch (e) { toast(e && e.message ? e.message : String(e), 'error'); return false; }
        }
        return true;
    }
    if (id === 'ollama') return true;
    if (!isSessionUnlocked()) {
        toast(t('error.locked'), 'error');
        return false;
    }
    const blob = await getSetting(`provider.${id}.apiKey`);
    if (!blob || (!blob.encrypted && !blob.plaintext)) {
        toast(t('error.noKey'), 'error');
        return false;
    }
    return true;
}

function updateSendDisabled() {
    if (!_ui.sendBtn || !_ui.textarea) return;
    const hasText = _ui.textarea.value.trim().length > 0;
    let disabled = !hasText;
    // Local provider with active download → disable send.
    if (!disabled) {
        const p = listProviders().find((x) => x.id === _activeProviderId);
        if (p && p.runtime === 'local' && isDownloadActive() && activeModelId() === p.defaultModel) {
            disabled = true;
        }
    }
    if (disabled) _ui.sendBtn.setAttribute('disabled', '');
    else _ui.sendBtn.removeAttribute('disabled');
}

// ── Drawer ─────────────────────────────────────────────────────

function toggleDrawer(open) {
    if (!_ui.drawerEl) return;
    if (open) {
        _ui.drawerEl.classList.add('is-open');
        _ui.drawerEl.removeAttribute('aria-hidden');
        document.body.classList.add('drawer-open');
    } else {
        _ui.drawerEl.classList.remove('is-open');
        _ui.drawerEl.setAttribute('aria-hidden', 'true');
        document.body.classList.remove('drawer-open');
    }
}

// ── Overflow menu ──────────────────────────────────────────────

function openOverflowMenu(anchor) {
    let host = document.getElementById('overflow-menu-host');
    if (host) { host.remove(); host = null; }
    host = el('div', { id: 'overflow-menu-host', class: 'overflow-host', attrs: { role: 'menu' } });
    const close = () => { try { host.remove(); } catch (_) {} document.removeEventListener('click', closeOnOutside, true); };
    const closeOnOutside = (e) => { if (!host.contains(e.target) && e.target !== anchor) close(); };
    setTimeout(() => document.addEventListener('click', closeOnOutside, true), 0);

    host.appendChild(el('button', {
        class: 'menu-item',
        attrs: { type: 'button', role: 'menuitem' },
        onClick: async () => {
            close();
            const next = prompt(t('chat.renamePrompt'), _conversation && _conversation.title || '');
            if (!next || !_conversationId) return;
            await putConversation({ ..._conversation, id: _conversationId, title: next });
            _conversation = { ...(_conversation || {}), id: _conversationId, title: next };
            setTitle(next);
            await refreshConversations();
        },
    }, t('chat.rename')));

    host.appendChild(el('button', {
        class: 'menu-item',
        attrs: { type: 'button', role: 'menuitem' },
        onClick: async () => {
            close();
            if (!_conversationId) return;
            const data = { conversation: _conversation, messages: _messages };
            const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = el('a', { attrs: { href: url, download: `${(_conversation && _conversation.title) || 'chat'}.json` } });
            document.body.appendChild(a);
            a.click();
            a.remove();
            URL.revokeObjectURL(url);
            toast(t('chat.exported'), 'success');
        },
    }, t('chat.export')));

    host.appendChild(el('button', {
        class: 'menu-item menu-item-danger',
        attrs: { type: 'button', role: 'menuitem' },
        onClick: async () => {
            close();
            if (!_conversationId) return;
            if (!confirm(t('chat.confirmDelete'))) return;
            await deleteConversation(_conversationId);
            const all = await listConversations();
            const next = all && all.length ? all[0].id : await newConversation(true);
            await loadConversation(next);
            toast(t('chat.deleted'), 'success');
        },
    }, t('chat.delete')));

    // Position near the anchor.
    const rect = anchor.getBoundingClientRect();
    host.style.position = 'fixed';
    host.style.top = `${Math.round(rect.bottom + 4)}px`;
    host.style.right = `${Math.round(window.innerWidth - rect.right)}px`;
    document.body.appendChild(host);
}

// ── Mic / STT ──────────────────────────────────────────────────

function bindMic(btn) {
    let stopFn = null;
    let pressActive = false;
    let toggleMode = false;

    const begin = async () => {
        if (_ui.listening) return;
        try {
            stopFn = await voice.listen({
                onResult: (text, isFinal) => {
                    if (!isFinal) return;
                    if (!_ui.textarea) return;
                    const t0 = _ui.textarea.value;
                    _ui.textarea.value = (t0 ? t0 + ' ' : '') + text;
                    autoSizeTextarea(_ui.textarea);
                    updateSendDisabled();
                },
                onEnd: () => {
                    _ui.listening = false;
                    btn.classList.remove('is-listening');
                    btn.setAttribute('aria-label', t('voice.start'));
                },
                onError: (err) => {
                    _ui.listening = false;
                    btn.classList.remove('is-listening');
                    if (err === 'not-allowed' || err === 'service-not-allowed') {
                        toast(t('voice.permissionDenied'), 'error');
                    }
                },
            });
            _ui.listening = true;
            btn.classList.add('is-listening');
            btn.setAttribute('aria-label', t('voice.stop'));
        } catch (e) {
            if (e && e.name === 'STT_UNSUPPORTED') {
                toast(t('voice.unsupported'), 'error');
            } else {
                toast(e && e.message ? e.message : String(e), 'error');
            }
        }
    };
    const end = () => {
        if (stopFn) { try { stopFn(); } catch (_) {} stopFn = null; }
    };

    // Hold-to-talk via pointer events (covers mouse + touch + pen).
    btn.addEventListener('pointerdown', (e) => {
        if (e.button !== undefined && e.button !== 0) return;
        pressActive = true;
        toggleMode = false;
        // If the press never holds long enough, treat as toggle (tap).
        const start = Date.now();
        begin();
        const release = () => {
            if (!pressActive) return;
            pressActive = false;
            const held = Date.now() - start;
            if (held < 250) {
                // Treat as toggle: leave listening on; next tap stops.
                toggleMode = true;
                btn.removeEventListener('pointerup', release, true);
                btn.removeEventListener('pointerleave', release, true);
                btn.removeEventListener('pointercancel', release, true);
                return;
            }
            end();
            btn.removeEventListener('pointerup', release, true);
            btn.removeEventListener('pointerleave', release, true);
            btn.removeEventListener('pointercancel', release, true);
        };
        btn.addEventListener('pointerup', release, true);
        btn.addEventListener('pointerleave', release, true);
        btn.addEventListener('pointercancel', release, true);
    });

    // Click handler for the toggle-mode tap-to-stop.
    btn.addEventListener('click', () => {
        if (toggleMode && _ui.listening) {
            toggleMode = false;
            end();
        }
    });
}

// ── Code block copy button (event delegation) ─────────────────
// Wire the copy buttons in code blocks via event delegation. The button
// markup is produced by markdown.js with [data-bw-copy].
document.addEventListener('click', (e) => {
    const t0 = e.target;
    if (!(t0 instanceof Element)) return;
    if (!t0.matches('[data-bw-copy]')) return;
    const codeBlock = t0.parentElement && t0.parentElement.querySelector('pre code');
    if (!codeBlock) return;
    const text = codeBlock.textContent || '';
    copyText(text);
});

async function copyText(text) {
    try {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(text);
        } else {
            const ta = el('textarea', {});
            ta.value = text;
            document.body.appendChild(ta);
            ta.select();
            document.execCommand('copy');
            ta.remove();
        }
        toast(t('chat.copied'), 'success', 1200);
    } catch (_) {
        toast(t('error.generic'), 'error');
    }
}

// ── Composer / scroll plumbing ─────────────────────────────────

function autoSizeTextarea(ta) {
    if (!ta) return;
    ta.style.height = 'auto';
    const max = parseFloat(getComputedStyle(ta).lineHeight) * 6 || 144;
    ta.style.height = Math.min(ta.scrollHeight, max) + 'px';
}

function scrollToBottom(force = false) {
    if (!_ui.listEl) return;
    if (!force && !_autoScroll) return;
    _ui.listEl.scrollTop = _ui.listEl.scrollHeight;
}

function bindVisualViewport() {
    if (typeof window === 'undefined' || !window.visualViewport) return;
    const apply = () => {
        const vv = window.visualViewport;
        const offset = Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
        document.documentElement.style.setProperty('--vv-bottom', `${Math.round(offset)}px`);
        if (_autoScroll) scrollToBottom(false);
    };
    window.visualViewport.addEventListener('resize', apply);
    window.visualViewport.addEventListener('scroll', apply);
    apply();
}

// ── Glyphs (inline SVG; no remote icon font) ───────────────────

function glyph(name) {
    const svgNS = 'http://www.w3.org/2000/svg';
    const paths = {
        menu: 'M3 6h18M3 12h18M3 18h18',
        x: 'M6 6l12 12M6 18L18 6',
        gear: 'M12 8a4 4 0 100 8 4 4 0 000-8z M19 12c0-.6-.06-1.2-.18-1.74l1.92-1.5-2-3.46-2.32.94c-.86-.7-1.86-1.22-2.94-1.5L13 2h-4l-.48 2.74c-1.08.28-2.08.8-2.94 1.5l-2.32-.94-2 3.46 1.92 1.5C3.06 10.8 3 11.4 3 12s.06 1.2.18 1.74l-1.92 1.5 2 3.46 2.32-.94c.86.7 1.86 1.22 2.94 1.5L9 22h4l.48-2.74c1.08-.28 2.08-.8 2.94-1.5l2.32.94 2-3.46-1.92-1.5c.12-.54.18-1.14.18-1.74z',
        send: 'M2 12l20-9-9 20-2-9-9-2z',
        mic: 'M12 14a3 3 0 003-3V6a3 3 0 10-6 0v5a3 3 0 003 3z M5 11a7 7 0 0014 0M12 18v3',
        copy: 'M9 9h11v11H9z M5 5h11v3M5 5v11h3',
        speaker: 'M5 9v6h4l5 5V4L9 9H5z M16 8a5 5 0 010 8',
        refresh: 'M4 12a8 8 0 0114-5l2-2v6h-6l3-3a6 6 0 10-1 9',
        dots: 'M5 12a1 1 0 102 0 1 1 0 10-2 0z M11 12a1 1 0 102 0 1 1 0 10-2 0z M17 12a1 1 0 102 0 1 1 0 10-2 0z',
    };
    const d = paths[name] || paths.dots;
    const svg = document.createElementNS(svgNS, 'svg');
    svg.setAttribute('viewBox', '0 0 24 24');
    svg.setAttribute('width', '20');
    svg.setAttribute('height', '20');
    svg.setAttribute('fill', 'none');
    svg.setAttribute('stroke', 'currentColor');
    svg.setAttribute('stroke-width', '2');
    svg.setAttribute('stroke-linecap', 'round');
    svg.setAttribute('stroke-linejoin', 'round');
    svg.setAttribute('aria-hidden', 'true');
    const p = document.createElementNS(svgNS, 'path');
    p.setAttribute('d', d);
    svg.appendChild(p);
    return svg;
}

// Mark crypto-store import as used (download banner already imports it
// directly; we re-import here only because future encryption-on-send
// support will live in this module).
void cryptoStore;

// ── Build-stamp hard refresh (dblclick + long-press) ─────────────────

function wireHardRefresh(stamp) {
    let pressTimer = null;
    let fired = false;
    stamp.addEventListener('dblclick', (e) => { e.preventDefault(); hardRefresh(); });
    stamp.addEventListener('pointerdown', () => {
        fired = false;
        pressTimer = setTimeout(() => { fired = true; hardRefresh(); }, 800);
    });
    stamp.addEventListener('pointerup', () => clearTimeout(pressTimer));
    stamp.addEventListener('pointercancel', () => clearTimeout(pressTimer));
    stamp.addEventListener('pointermove', () => clearTimeout(pressTimer));
    stamp.addEventListener('contextmenu', (e) => e.preventDefault());
    stamp.addEventListener('click', (e) => { if (fired) { e.preventDefault(); e.stopPropagation(); } });
}

async function hardRefresh() {
    if ('caches' in self) {
        const ks = await caches.keys();
        await Promise.all(ks.map(k => caches.delete(k)));
    }
    if ('serviceWorker' in navigator) {
        const rs = await navigator.serviceWorker.getRegistrations();
        await Promise.all(rs.map(r => r.unregister()));
    }
    location.reload();
}
