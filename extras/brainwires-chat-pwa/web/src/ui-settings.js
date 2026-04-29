// brainwires-chat-pwa — settings view
//
// Sections:
//   - Passphrase (set/change/lock)
//   - Cloud providers (per-provider card with API key + model + Test)
//   - Local model (Gemma 4 E2B card with progress mirror)
//   - Voice (TTS/STT pickers)
//   - About

import { el, clear, toast, escapeHtml } from './utils.js';
import { t } from './i18n.js';
import {
    getSetting,
    setSetting,
} from './db.js';
import { listProviders } from './providers/index.js';
import {
    KNOWN_MODELS,
    KNOWN_EMBEDDING_MODELS,
    isDownloaded,
    cancelDownload,
    deleteModel,
    getPartialInfo,
} from './model-store.js';
import * as banner from './ui-download-banner.js';
import * as cryptoStore from '../crypto-store.js';
import {
    getSessionKey,
    setSessionKey,
    isSessionUnlocked,
    getWasm,
    events as stateEvents,
} from './state.js';
import * as voice from './voice.js';
import { mount as mountView } from './views.js';
import { getTheme, setTheme } from './theme.js';
import { render as renderRagPanel } from './ui-rag-panel.js';
import { render as renderMcpPanel } from './ui-mcp-panel.js';
import { renderHomePairingCard } from './ui-home-pairing.js';

const PASSPHRASE_SETTING = 'passphraseConfig'; // { salt: base64, verify: encrypted("ok") }
const ENCRYPT_OPT_OUT_SETTING = 'encryptionOptOut';

let _root = null;

// ── Public render ──────────────────────────────────────────────

export async function render(root) {
    _root = root;
    clear(root);
    root.appendChild(buildHeader());
    const main = el('div', { class: 'settings-main' });
    root.appendChild(main);

    main.appendChild(await sectionPassphrase());
    main.appendChild(await sectionTheme());
    main.appendChild(await sectionHomeAgent());
    main.appendChild(await sectionProviders());
    main.appendChild(await sectionLocalModel());
    main.appendChild(await sectionEmbeddingModels());
    main.appendChild(await sectionRag());
    main.appendChild(await sectionMcp());
    main.appendChild(await sectionVoice());
    main.appendChild(await sectionAbout());

    // Partial update: refresh just the affected card when a download completes.
    stateEvents.addEventListener('model_progress', (e) => {
        const d = e.detail;
        if (d && d.phase === 'ready' && d.modelId) {
            refreshCard(d.modelId);
        }
    });
}

export function onShow() { /* could refresh dynamic state here */ }

// ── Header ─────────────────────────────────────────────────────

function buildHeader() {
    return el('header', { class: 'settings-header' },
        el('button', {
            class: 'icon-btn',
            attrs: { type: 'button', 'aria-label': t('nav.back') },
            onClick: () => mountView('chat'),
        }, '←'),
        el('h1', { class: 'settings-title' }, t('settings.title')),
    );
}

function sectionWrap(title, body) {
    return el('section', { class: 'settings-section' },
        el('h2', { class: 'settings-section-title' }, title),
        body,
    );
}

// ── Passphrase ─────────────────────────────────────────────────

async function sectionPassphrase() {
    const body = el('div', { class: 'settings-card' });
    await renderPassphrase(body);
    return sectionWrap(t('settings.passphrase.title'), body);
}

async function renderPassphrase(body) {
    clear(body);
    const cfg = await getSetting(PASSPHRASE_SETTING);
    const optOut = await getSetting(ENCRYPT_OPT_OUT_SETTING);

    if (cfg && cfg.salt && cfg.verify) {
        // Configured. Show change + lock buttons.
        body.appendChild(el('p', { class: 'settings-help' },
            isSessionUnlocked() ? '✓ Unlocked' : t('settings.passphrase.locked'),
        ));

        if (!isSessionUnlocked()) {
            const pp = el('input', { type: 'password', class: 'bw-input', attrs: { placeholder: t('settings.passphrase.placeholder'), autocomplete: 'current-password' } });
            const err = el('div', { class: 'settings-err' });
            const unlock = el('button', {
                class: 'bw-btn bw-btn-primary',
                attrs: { type: 'button' },
                onClick: async () => {
                    err.textContent = '';
                    try {
                        await unlockPassphrase(pp.value);
                        toast('Unlocked', 'success');
                        await renderPassphrase(body);
                    } catch (e) {
                        err.textContent = e && e.message ? e.message : String(e);
                    }
                },
            }, t('settings.passphrase.unlock'));
            body.appendChild(pp);
            body.appendChild(unlock);
            body.appendChild(err);
        } else {
            body.appendChild(el('button', {
                class: 'bw-btn bw-btn-secondary',
                attrs: { type: 'button' },
                onClick: () => { setSessionKey(null); toast('Locked', 'info'); renderPassphrase(body); },
            }, t('settings.passphrase.lock')));
            // Change passphrase form (collapsed).
            body.appendChild(buildChangePassphraseForm(() => renderPassphrase(body)));
        }
    } else {
        // Not yet configured.
        if (optOut) {
            body.appendChild(el('p', { class: 'settings-help settings-warn' }, t('settings.passphrase.skipWarn')));
        }
        body.appendChild(buildSetPassphraseForm(() => renderPassphrase(body)));
        body.appendChild(el('button', {
            class: 'bw-btn bw-btn-link',
            attrs: { type: 'button' },
            onClick: async () => {
                await setSetting(ENCRYPT_OPT_OUT_SETTING, true);
                toast(t('settings.passphrase.skipWarn'), 'warn', 5000);
                await renderPassphrase(body);
            },
        }, t('settings.passphrase.skip')));
    }
}

function buildSetPassphraseForm(onDone) {
    const pw1 = el('input', { type: 'password', class: 'bw-input', attrs: { placeholder: t('settings.passphrase.placeholder'), autocomplete: 'new-password' } });
    const pw2 = el('input', { type: 'password', class: 'bw-input', attrs: { placeholder: t('settings.passphrase.confirm'), autocomplete: 'new-password' } });
    const err = el('div', { class: 'settings-err' });
    const btn = el('button', {
        class: 'bw-btn bw-btn-primary',
        attrs: { type: 'button' },
        onClick: async () => {
            err.textContent = '';
            if (pw1.value.length < 8) { err.textContent = t('settings.passphrase.tooShort'); return; }
            if (pw1.value !== pw2.value) { err.textContent = t('settings.passphrase.mismatch'); return; }
            try {
                await configurePassphrase(pw1.value);
                toast(t('settings.saved'), 'success');
                if (onDone) onDone();
            } catch (e) {
                err.textContent = e && e.message ? e.message : String(e);
            }
        },
    }, t('settings.passphrase.set'));
    return el('div', { class: 'settings-form' }, pw1, pw2, btn, err);
}

function buildChangePassphraseForm(onDone) {
    const cur = el('input', { type: 'password', class: 'bw-input', attrs: { placeholder: 'Current ' + t('settings.passphrase.placeholder'), autocomplete: 'current-password' } });
    const pw1 = el('input', { type: 'password', class: 'bw-input', attrs: { placeholder: 'New passphrase', autocomplete: 'new-password' } });
    const pw2 = el('input', { type: 'password', class: 'bw-input', attrs: { placeholder: t('settings.passphrase.confirm'), autocomplete: 'new-password' } });
    const err = el('div', { class: 'settings-err' });
    const btn = el('button', {
        class: 'bw-btn bw-btn-secondary',
        attrs: { type: 'button' },
        onClick: async () => {
            err.textContent = '';
            if (pw1.value.length < 8) { err.textContent = t('settings.passphrase.tooShort'); return; }
            if (pw1.value !== pw2.value) { err.textContent = t('settings.passphrase.mismatch'); return; }
            try {
                await unlockPassphrase(cur.value); // verify
                await configurePassphrase(pw1.value);
                toast(t('settings.saved'), 'success');
                if (onDone) onDone();
            } catch (e) {
                err.textContent = e && e.message ? e.message : String(e);
            }
        },
    }, t('settings.passphrase.change'));
    return el('details', { class: 'settings-form-collapsible' },
        el('summary', {}, t('settings.passphrase.change')),
        cur, pw1, pw2, btn, err,
    );
}

async function configurePassphrase(passphrase) {
    const salt = cryptoStore.generateSalt();
    const key = await cryptoStore.deriveKey(passphrase, salt);
    const verifyBlob = await cryptoStore.encrypt(key, 'ok');
    const verifyPacked = cryptoStore.pack({ salt, iv: verifyBlob.iv, ciphertext: verifyBlob.ciphertext });
    await setSetting(PASSPHRASE_SETTING, { salt: b64Encode(salt), verify: verifyPacked });
    setSessionKey(key);
}

async function unlockPassphrase(passphrase) {
    const cfg = await getSetting(PASSPHRASE_SETTING);
    if (!cfg) throw new Error('Passphrase not configured');
    const parts = cryptoStore.unpack(cfg.verify);
    const key = await cryptoStore.deriveKey(passphrase, parts.salt);
    try {
        const out = await cryptoStore.decrypt(key, { iv: parts.iv, ciphertext: parts.ciphertext });
        if (out !== 'ok') throw new Error('verify mismatch');
    } catch (_) {
        throw new Error(t('unlock.wrong'));
    }
    setSessionKey(key);
}

// Tiny helpers — full base64 not base64url since we only stash the salt.
function b64Encode(bytes) {
    let s = '';
    for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    return btoa(s);
}

// ── Theme ──────────────────────────────────────────────────────

async function sectionTheme() {
    const body = el('div', { class: 'settings-card' });
    const current = getTheme();
    const sel = el('select', { class: 'bw-input', attrs: { 'aria-label': t('settings.theme.label') } });
    const options = [
        ['system', t('settings.theme.system')],
        ['light', t('settings.theme.light')],
        ['dark', t('settings.theme.dark')],
    ];
    for (const [val, label] of options) {
        const o = el('option', { attrs: { value: val } }, label);
        if (val === current) o.setAttribute('selected', '');
        sel.appendChild(o);
    }
    sel.addEventListener('change', async () => {
        try {
            await setTheme(sel.value);
            toast(t('settings.saved'), 'success', 1200);
        } catch (e) {
            toast(e && e.message ? e.message : String(e), 'error');
        }
    });
    body.appendChild(el('label', { class: 'bw-label' }, t('settings.theme.label'), sel));
    return sectionWrap(t('settings.theme.title'), body);
}

// ── Home agent (M8 pairing) ────────────────────────────────────

async function sectionHomeAgent() {
    const body = await renderHomePairingCard();
    return sectionWrap(t('settings.home.title'), body);
}

// ── Cloud providers ────────────────────────────────────────────

async function sectionProviders() {
    const body = el('div', { class: 'settings-card-list' });
    const providers = listProviders().filter((p) => p.runtime === 'cloud');
    for (const p of providers) {
        body.appendChild(await buildProviderCard(p));
    }
    return sectionWrap(t('settings.providers'), body);
}

async function buildProviderCard(p) {
    const id = p.id;
    const blob = await getSetting(`provider.${id}.apiKey`);
    const savedModel = await getSetting(`provider.${id}.model`);
    const baseUrl = id === 'ollama' ? await getSetting(`provider.${id}.baseUrl`) : null;

    const apiKeyInput = el('input', {
        type: 'password',
        class: 'bw-input',
        attrs: {
            placeholder: t('settings.apiKey'),
            autocomplete: 'off',
            'aria-label': `${p.displayName} ${t('settings.apiKey')}`,
        },
    });
    if (blob && (blob.encrypted || blob.plaintext)) {
        apiKeyInput.placeholder = '•••••••• (saved)';
    }

    const modelSelect = el('select', { class: 'bw-input', attrs: { 'aria-label': t('settings.model') } });
    for (const m of (p.models && p.models.length ? p.models : [p.defaultModel])) {
        const o = el('option', { attrs: { value: m } }, m);
        if (m === (savedModel || p.defaultModel)) o.setAttribute('selected', '');
        modelSelect.appendChild(o);
    }

    const baseUrlInput = id === 'ollama'
        ? el('input', {
            type: 'url',
            class: 'bw-input',
            value: baseUrl || 'http://localhost:11434',
            attrs: { placeholder: 'http://localhost:11434', 'aria-label': t('settings.baseUrl') },
        })
        : null;

    const status = el('div', { class: 'settings-status', attrs: { 'aria-live': 'polite' } });
    const testBtn = el('button', {
        class: 'bw-btn bw-btn-secondary',
        attrs: { type: 'button' },
        onClick: async () => {
            status.textContent = t('settings.testing');
            try {
                await testProvider(id, apiKeyInput.value, baseUrlInput ? baseUrlInput.value : null);
                status.textContent = t('settings.testOk');
                status.className = 'settings-status settings-status-ok';
            } catch (e) {
                status.textContent = t('settings.testFail', { error: e && e.message ? e.message : String(e) });
                status.className = 'settings-status settings-status-err';
            }
        },
    }, t('settings.test'));

    const saveBtn = el('button', {
        class: 'bw-btn bw-btn-primary',
        attrs: { type: 'button' },
        onClick: async () => {
            try {
                await saveProvider(id, apiKeyInput.value, modelSelect.value, baseUrlInput ? baseUrlInput.value : null);
                apiKeyInput.value = '';
                apiKeyInput.placeholder = '•••••••• (saved)';
                toast(t('settings.saved'), 'success');
            } catch (e) {
                toast(e && e.message ? e.message : String(e), 'error');
            }
        },
    }, t('settings.save'));

    const card = el('div', { class: 'settings-card' },
        el('h3', { class: 'settings-card-title' }, p.displayName),
        id !== 'ollama' && el('label', { class: 'bw-label' }, t('settings.apiKey'), apiKeyInput),
        baseUrlInput && el('label', { class: 'bw-label' }, t('settings.baseUrl'), baseUrlInput),
        el('label', { class: 'bw-label' }, t('settings.model'), modelSelect),
        el('div', { class: 'settings-actions' }, testBtn, saveBtn),
        status,
    );
    return card;
}

async function saveProvider(id, apiKey, model, baseUrl) {
    if (apiKey && apiKey.length > 0) {
        const sessionKey = getSessionKey();
        if (sessionKey) {
            const salt = cryptoStore.generateSalt();
            const blob = await cryptoStore.encrypt(sessionKey, apiKey);
            const packed = cryptoStore.pack({ salt, iv: blob.iv, ciphertext: blob.ciphertext });
            await setSetting(`provider.${id}.apiKey`, { encrypted: packed });
        } else {
            // No passphrase — store in plaintext per user opt-out.
            await setSetting(`provider.${id}.apiKey`, { plaintext: apiKey });
        }
    }
    if (model) await setSetting(`provider.${id}.model`, model);
    if (baseUrl) await setSetting(`provider.${id}.baseUrl`, baseUrl);
}

async function testProvider(id, apiKeyInline, baseUrlInline) {
    // Use the inline value if present, else the saved (decrypted) one.
    let key = apiKeyInline && apiKeyInline.length ? apiKeyInline : null;
    if (!key && id !== 'ollama') {
        const blob = await getSetting(`provider.${id}.apiKey`);
        if (blob && blob.plaintext) key = blob.plaintext;
        else if (blob && blob.encrypted) {
            const sk = getSessionKey();
            if (!sk) throw new Error(t('error.locked'));
            const parts = cryptoStore.unpack(blob.encrypted);
            key = await cryptoStore.decrypt(sk, { iv: parts.iv, ciphertext: parts.ciphertext });
        }
    }

    if (id === 'openai') {
        if (!key) throw new Error('No API key');
        const r = await fetch('https://api.openai.com/v1/models', {
            headers: { 'Authorization': `Bearer ${key}` },
        });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return;
    }
    if (id === 'anthropic') {
        if (!key) throw new Error('No API key');
        const r = await fetch('https://api.anthropic.com/v1/models', {
            headers: { 'x-api-key': key, 'anthropic-version': '2023-06-01' },
        });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return;
    }
    if (id === 'google') {
        if (!key) throw new Error('No API key');
        const r = await fetch(`https://generativelanguage.googleapis.com/v1beta/models?key=${encodeURIComponent(key)}`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return;
    }
    if (id === 'ollama') {
        const base = (baseUrlInline || 'http://localhost:11434').replace(/\/+$/, '');
        const r = await fetch(`${base}/api/tags`);
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return;
    }
    throw new Error(`Unknown provider: ${id}`);
}

// ── Local model ────────────────────────────────────────────────

async function buildLlmCard(modelId) {
    const m = KNOWN_MODELS[modelId];
    if (!m) return el('div');
    const downloaded = await isDownloaded(modelId).catch((e) => { console.error("[bw] swallowed:", e); return false; });
    const card = el('div', { class: 'settings-card', id: `model-card-${modelId}` });

    card.appendChild(el('div', { class: 'settings-card-header' },
        el('h3', { class: 'settings-card-title' }, m.displayName),
        el('span', { class: 'pill ' + (downloaded ? 'pill-ok' : 'pill-muted') },
            downloaded ? t('settings.localModel.ready') : formatSize(m.estimatedBytes)),
    ));
    card.appendChild(el('p', { class: 'settings-help' }, m.description));

    const partial = !downloaded ? await getPartialInfo(modelId).catch(() => ({ hasData: false })) : null;

    const actions = el('div', { class: 'settings-actions' });
    if (!downloaded) {
        actions.appendChild(el('button', {
            class: 'bw-btn bw-btn-primary bw-btn-sm',
            attrs: { type: 'button' },
            onClick: () => banner.startDownload(modelId),
        }, t('settings.localModel.download')));
        actions.appendChild(el('button', {
            class: 'bw-btn bw-btn-secondary bw-btn-sm',
            attrs: { type: 'button' },
            onClick: () => cancelDownload(modelId),
        }, t('settings.localModel.cancel')));
        if (partial && partial.hasData) {
            actions.appendChild(el('button', {
                class: 'bw-btn bw-btn-danger bw-btn-sm',
                attrs: { type: 'button' },
                onClick: async () => {
                    await deleteModel(modelId);
                    toast(`Cleared ${formatSize(partial.totalBytes)} partial data`);
                    await refreshCard(modelId);
                },
            }, `Clear partial (${formatSize(partial.totalBytes)})`));
        }
    } else {
        actions.appendChild(el('button', {
            class: 'bw-btn bw-btn-danger bw-btn-sm',
            attrs: { type: 'button' },
            onClick: async () => {
                if (!confirm(t('settings.localModel.confirmDelete'))) return;
                await banner.deleteActive(modelId);
                await refreshCard(modelId);
            },
        }, t('settings.localModel.delete')));
    }
    card.appendChild(actions);
    return card;
}

async function sectionLocalModel() {
    const body = el('div', { class: 'settings-card-list' });
    body.appendChild(await buildLlmCard('gemma-4-e2b'));
    return sectionWrap(t('settings.localModel.title'), body);
}

// ── Embedding models ──────────────────────────────────────────

function formatSize(bytes) {
    if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
    if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(0)} MB`;
    return `${(bytes / 1e3).toFixed(0)} KB`;
}

async function buildEmbeddingCard(m) {
    const downloaded = await isDownloaded(m.id).catch((e) => { console.error("[bw] swallowed:", e); return false; });
    const active = (await getSetting('embedding.activeModel')) === m.id;

    const card = el('div', { class: 'settings-card', id: `model-card-${m.id}` });
    card.appendChild(el('div', { class: 'settings-card-header' },
        el('h3', { class: 'settings-card-title' }, m.displayName),
        el('span', { class: 'pill ' + (downloaded ? 'pill-ok' : 'pill-muted') },
            downloaded ? (active ? 'Active' : 'Ready') : formatSize(m.estimatedBytes)),
    ));
    card.appendChild(el('p', { class: 'settings-help' },
        `${m.provider} · ${m.dimensions}-dim · ${m.maxTokens} max tokens`));
    card.appendChild(el('p', { class: 'settings-help' }, m.description));

    const partial = !downloaded ? await getPartialInfo(m.id).catch(() => ({ hasData: false })) : null;

    const actions = el('div', { class: 'settings-actions' });
    if (!downloaded) {
        actions.appendChild(el('button', {
            class: 'bw-btn bw-btn-primary bw-btn-sm',
            attrs: { type: 'button' },
            onClick: () => {
                banner.startDownload(m.id);
                toast(`Downloading ${m.displayName}…`);
            },
        }, 'Download'));
        if (partial && partial.hasData) {
            actions.appendChild(el('button', {
                class: 'bw-btn bw-btn-danger bw-btn-sm',
                attrs: { type: 'button' },
                onClick: async () => {
                    await deleteModel(m.id);
                    toast(`Cleared ${formatSize(partial.totalBytes)} partial data`);
                    await refreshCard(m.id);
                },
            }, `Clear partial (${formatSize(partial.totalBytes)})`));
        }
    } else {
        if (!active) {
            actions.appendChild(el('button', {
                class: 'bw-btn bw-btn-primary bw-btn-sm',
                attrs: { type: 'button' },
                onClick: async () => {
                    await setSetting('embedding.activeModel', m.id);
                    toast(`${m.displayName} set as active`);
                    await refreshCard(m.id);
                },
            }, 'Use'));
        } else {
            actions.appendChild(el('span', { class: 'pill pill-ok' }, '✓ Active'));
        }
        actions.appendChild(el('button', {
            class: 'bw-btn bw-btn-danger bw-btn-sm',
            attrs: { type: 'button' },
            onClick: async () => {
                if (!confirm(`Delete ${m.displayName}?`)) return;
                await deleteModel(m.id);
                if (active) await setSetting('embedding.activeModel', '');
                toast(`${m.displayName} deleted`);
                await refreshCard(m.id);
            },
        }, 'Delete'));
    }
    card.appendChild(actions);
    return card;
}

async function sectionEmbeddingModels() {
    const body = el('div', { class: 'settings-card-list' });
    const models = Object.values(KNOWN_EMBEDDING_MODELS);
    const categories = ['small', 'medium', 'large'];
    for (const cat of categories) {
        const catModels = models.filter((m) => m.category === cat);
        if (catModels.length === 0) continue;
        const catLabel = cat === 'small' ? 'Small (< 200 MB)' : cat === 'medium' ? 'Medium (200 MB – 1 GB)' : 'Large (> 1 GB)';
        body.appendChild(el('h4', { class: 'settings-subsection' }, catLabel));
        for (const m of catModels) {
            body.appendChild(await buildEmbeddingCard(m));
        }
    }
    return sectionWrap('Embedding models (local RAG)', body);
}

// ── Private RAG ────────────────────────────────────────────────

async function sectionRag() {
    const body = await renderRagPanel();
    return sectionWrap(t('settings.rag.title'), body);
}

// ── MCP servers ────────────────────────────────────────────────

async function sectionMcp() {
    const body = await renderMcpPanel();
    return sectionWrap(t('settings.mcp.title'), body);
}

// ── Partial card refresh (swap one card, keep scroll + rest) ──

async function refreshCard(modelId) {
    const existing = document.getElementById(`model-card-${modelId}`);
    if (!existing) return;
    let newCard;
    if (KNOWN_MODELS[modelId]) {
        newCard = await buildLlmCard(modelId);
    } else if (KNOWN_EMBEDDING_MODELS[modelId]) {
        newCard = await buildEmbeddingCard(KNOWN_EMBEDDING_MODELS[modelId]);
    }
    if (newCard) existing.replaceWith(newCard);
}

// ── Voice ──────────────────────────────────────────────────────

async function sectionVoice() {
    const body = el('div', { class: 'settings-card' });
    const enabled = await voice.voicePrefs.get('stt.enabled', true);

    // STT enable toggle.
    const enableLabel = el('label', { class: 'bw-label bw-label-row' },
        el('span', {}, t('settings.voice.enable')),
        el('input', {
            type: 'checkbox',
            checked: !!enabled,
            onChange: async (e) => { await voice.voicePrefs.set('stt.enabled', !!e.currentTarget.checked); toast(t('settings.saved'), 'success', 1200); },
        }),
    );
    body.appendChild(enableLabel);

    if (!voice.isSttSupported()) {
        body.appendChild(el('p', { class: 'settings-help settings-warn' }, t('settings.voice.unsupported')));
    }

    // TTS section.
    body.appendChild(el('h4', { class: 'settings-subsection' }, t('settings.voice.tts')));
    const ttsVoiceSel = el('select', { class: 'bw-input', attrs: { 'aria-label': t('settings.voice.voice') } });
    ttsVoiceSel.appendChild(el('option', { attrs: { value: '' } }, '(default)'));
    try {
        const voices = await voice.listVoices();
        const savedUri = await voice.voicePrefs.get('tts.voiceUri', '');
        for (const v of voices) {
            const opt = el('option', { attrs: { value: v.uri } }, `${v.name} — ${v.lang}`);
            if (v.uri === savedUri) opt.setAttribute('selected', '');
            ttsVoiceSel.appendChild(opt);
        }
    } catch (_err) { console.warn("[bw] caught:", _err); }
    ttsVoiceSel.addEventListener('change', () => voice.setTtsVoice(ttsVoiceSel.value || null));
    body.appendChild(el('label', { class: 'bw-label' }, t('settings.voice.voice'), ttsVoiceSel));

    body.appendChild(await buildSlider('tts.rate', t('settings.voice.rate'), 0.5, 2.0, 0.1, 1.0));
    body.appendChild(await buildSlider('tts.pitch', t('settings.voice.pitch'), 0.0, 2.0, 0.1, 1.0));
    body.appendChild(await buildSlider('tts.volume', t('settings.voice.volume'), 0.0, 1.0, 0.05, 1.0));

    body.appendChild(el('button', {
        class: 'bw-btn bw-btn-secondary',
        attrs: { type: 'button' },
        onClick: () => voice.speak(t('settings.voice.testText')).catch((e) => toast(e && e.message ? e.message : String(e), 'error')),
    }, t('settings.voice.test')));

    // STT section.
    body.appendChild(el('h4', { class: 'settings-subsection' }, t('settings.voice.stt')));
    const sttLang = el('input', {
        type: 'text',
        class: 'bw-input',
        value: await voice.voicePrefs.get('stt.lang', 'en-US'),
        attrs: { 'aria-label': t('settings.voice.lang'), placeholder: 'en-US' },
        onChange: async (e) => { await voice.setSttLang(e.currentTarget.value); toast(t('settings.saved'), 'success', 1200); },
    });
    body.appendChild(el('label', { class: 'bw-label' }, t('settings.voice.lang'), sttLang));

    const continuous = el('input', {
        type: 'checkbox',
        checked: !!(await voice.voicePrefs.get('stt.continuous', false)),
        onChange: async (e) => { await voice.voicePrefs.set('stt.continuous', !!e.currentTarget.checked); },
    });
    body.appendChild(el('label', { class: 'bw-label bw-label-row' },
        el('span', {}, t('settings.voice.continuous')), continuous));

    const interim = el('input', {
        type: 'checkbox',
        checked: !!(await voice.voicePrefs.get('stt.interim', true)),
        onChange: async (e) => { await voice.voicePrefs.set('stt.interim', !!e.currentTarget.checked); },
    });
    body.appendChild(el('label', { class: 'bw-label bw-label-row' },
        el('span', {}, t('settings.voice.interim')), interim));

    return sectionWrap(t('settings.voice.title'), body);
}

async function buildSlider(prefKey, label, min, max, step, fallback) {
    const value = await voice.voicePrefs.get(prefKey, fallback);
    const out = el('span', { class: 'bw-slider-value' }, String(value));
    const slider = el('input', {
        type: 'range',
        class: 'bw-slider',
        attrs: { min: String(min), max: String(max), step: String(step) },
        value: String(value),
        onInput: (e) => { out.textContent = e.currentTarget.value; },
        onChange: (e) => { voice.voicePrefs.set(prefKey, parseFloat(e.currentTarget.value)); },
    });
    return el('label', { class: 'bw-label' },
        el('span', {}, label),
        el('div', { class: 'bw-slider-row' }, slider, out),
    );
}

// ── About ──────────────────────────────────────────────────────

async function sectionAbout() {
    const body = el('div', { class: 'settings-card' });
    let version = 'unknown';
    try {
        const wasm = await getWasm();
        if (typeof wasm.version === 'function') version = wasm.version();
    } catch (_err) { console.warn("[bw] caught:", _err); }

    let buildTime = 'unknown';
    let buildGit = 'unknown';
    try {
        const info = await import('../build-info.js');
        buildTime = info.BUILD_TIME || buildTime;
        buildGit = info.BUILD_GIT || buildGit;
    } catch (_err) { console.warn("[bw] caught:", _err); }

    body.appendChild(el('p', {}, el('strong', {}, t('settings.about.version') + ': '), String(version)));
    body.appendChild(el('p', {}, el('strong', {}, t('settings.about.build') + ': '), `${buildTime} (${buildGit})`));
    return sectionWrap(t('settings.about.title'), body);
}

// Avoid unused warnings: escapeHtml is reserved for future use.
void escapeHtml;
