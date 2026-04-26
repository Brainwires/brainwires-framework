// brainwires-chat-pwa — minimal i18n
//
// Async loader for `lang/<code>.json`. Falls back to the key on miss.
// Only English ships today; the scaffold exists so a translator can drop
// in `lang/de.json` without touching JS.

let _dict = {};
let _code = 'en';

/**
 * Load a locale dictionary from `lang/<code>.json`. Subsequent calls
 * for the same code are cheap (re-fetch but the browser HTTP cache
 * will satisfy them). Falls back silently when the network/file is
 * unavailable so the UI never blanks out — `t(key)` will return the
 * key string instead.
 *
 * @param {string} code
 * @returns {Promise<Record<string,string>>}
 */
export async function loadLocale(code) {
    const target = code || 'en';
    try {
        const url = `./lang/${encodeURIComponent(target)}.json`;
        const resp = await fetch(url, { cache: 'force-cache' });
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
        const data = await resp.json();
        if (data && typeof data === 'object') {
            _dict = data;
            _code = target;
            return data;
        }
    } catch (_) {
        // Non-fatal: keep whatever dict we already had.
    }
    if (!_dict || Object.keys(_dict).length === 0) _dict = {};
    return _dict;
}

/**
 * Look up a key. Falls back to the key string when missing.
 *
 * @param {string} key
 * @param {Record<string, string|number>} [vars]  optional `{name}` substitutions
 * @returns {string}
 */
export function t(key, vars) {
    const raw = (_dict && Object.prototype.hasOwnProperty.call(_dict, key))
        ? _dict[key]
        : key;
    if (!vars || typeof vars !== 'object') return String(raw);
    return String(raw).replace(/\{(\w+)\}/g, (_, name) => {
        return Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : `{${name}}`;
    });
}

/** Current locale code (the last successfully-loaded one). */
export function currentLocale() { return _code; }

/** For tests — install a dictionary directly without fetching. */
export function _setDictForTests(dict, code = 'en') {
    _dict = dict || {};
    _code = code;
}
