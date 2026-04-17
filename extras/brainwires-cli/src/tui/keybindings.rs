//! User-configurable keybindings for top-level global shortcuts.
//!
//! The TUI has ~50 hardcoded key-match sites across per-mode handlers.
//! Remapping all of them is out of scope; this module covers the small
//! set of truly global shortcuts users want to customize — the ones that
//! live in [`crate::tui::app::events::core::App::handle_event`] before
//! per-mode dispatch.
//!
//! ## Configuration
//!
//! Under `settings.json`:
//!
//! ```json
//! {
//!   "keybindings": {
//!     "global": {
//!       "console_view": "Ctrl+K",
//!       "plan_mode_toggle": "Alt+P"
//!     }
//!   }
//! }
//! ```
//!
//! Keys not listed fall back to the built-in defaults, so partial
//! configuration is fine.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::tui::Event;

/// Parsed key spec — a `(KeyCode, KeyModifiers)` pair.
pub type KeyBinding = (KeyCode, KeyModifiers);

/// Default bindings for every action this module knows about. Names are
/// stable and match the `Event::is_*` predicate stems for discoverability.
fn default_bindings() -> HashMap<String, KeyBinding> {
    let mut m = HashMap::new();
    m.insert(
        "console_view".to_string(),
        (KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    m.insert(
        "plan_mode_toggle".to_string(),
        (KeyCode::Char('p'), KeyModifiers::CONTROL),
    );
    m.insert(
        "task_viewer".to_string(),
        (KeyCode::Char('t'), KeyModifiers::CONTROL),
    );
    m.insert(
        "reverse_search".to_string(),
        (KeyCode::Char('r'), KeyModifiers::CONTROL),
    );
    m.insert(
        "sub_agent_viewer".to_string(),
        (KeyCode::Char('b'), KeyModifiers::CONTROL),
    );
    m.insert(
        "file_explorer".to_string(),
        (
            KeyCode::Char('f'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
    );
    m
}

/// A resolved keybinding table, built from defaults + optional user
/// overrides. Cheap to clone — it's a `HashMap<String, (KeyCode, KeyModifiers)>`.
#[derive(Debug, Clone)]
pub struct KeybindingMap {
    bindings: HashMap<String, KeyBinding>,
}

impl Default for KeybindingMap {
    fn default() -> Self {
        Self {
            bindings: default_bindings(),
        }
    }
}

impl KeybindingMap {
    /// Build a map from the `Keybindings` settings section (if any).
    /// Unknown actions are silently ignored; unparseable specs are
    /// logged and the default for that action is kept.
    pub fn from_settings(cfg: Option<&Keybindings>) -> Self {
        let mut bindings = default_bindings();
        if let Some(cfg) = cfg {
            for (action, spec) in &cfg.global {
                // Only accept actions we know about — typos would otherwise
                // silently add no-op entries.
                if !bindings.contains_key(action) {
                    tracing::warn!(
                        "Ignoring unknown keybinding action '{}' in settings",
                        action
                    );
                    continue;
                }
                match parse_key_spec(spec) {
                    Some(kb) => {
                        bindings.insert(action.clone(), kb);
                    }
                    None => {
                        tracing::warn!(
                            "Failed to parse key spec '{}' for action '{}'; keeping default",
                            spec,
                            action
                        );
                    }
                }
            }
        }
        Self { bindings }
    }

    /// Does `event` match the key bound to `action`?
    pub fn matches(&self, action: &str, event: &Event) -> bool {
        let Some((want_code, want_mods)) = self.bindings.get(action) else {
            return false;
        };
        match event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => code == want_code && modifiers == want_mods,
            _ => false,
        }
    }

    /// Every action name known to this map. Useful for help output.
    pub fn action_names(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.bindings.keys().map(|s| s.as_str()).collect();
        v.sort();
        v
    }
}

/// Parse `"Ctrl+D"`, `"Alt+Shift+F4"`, `"Esc"`, `"F1"`, etc. into a
/// `(KeyCode, KeyModifiers)`. Case-insensitive on modifier + named key
/// names; single-character keys are lowercased.
pub fn parse_key_spec(spec: &str) -> Option<KeyBinding> {
    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }

    let mut modifiers = KeyModifiers::NONE;
    let parts: Vec<&str> = spec.split('+').map(str::trim).collect();
    let (key_part, mod_parts) = parts.split_last()?;

    for m in mod_parts {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            _ => return None,
        }
    }

    let code = match *key_part {
        k if k.eq_ignore_ascii_case("esc") || k.eq_ignore_ascii_case("escape") => KeyCode::Esc,
        k if k.eq_ignore_ascii_case("enter") || k.eq_ignore_ascii_case("return") => {
            KeyCode::Enter
        }
        k if k.eq_ignore_ascii_case("tab") => KeyCode::Tab,
        k if k.eq_ignore_ascii_case("space") => KeyCode::Char(' '),
        k if k.eq_ignore_ascii_case("backspace") => KeyCode::Backspace,
        k if k.eq_ignore_ascii_case("delete") => KeyCode::Delete,
        k if k.eq_ignore_ascii_case("up") => KeyCode::Up,
        k if k.eq_ignore_ascii_case("down") => KeyCode::Down,
        k if k.eq_ignore_ascii_case("left") => KeyCode::Left,
        k if k.eq_ignore_ascii_case("right") => KeyCode::Right,
        k if k.eq_ignore_ascii_case("home") => KeyCode::Home,
        k if k.eq_ignore_ascii_case("end") => KeyCode::End,
        k if k.eq_ignore_ascii_case("pageup") => KeyCode::PageUp,
        k if k.eq_ignore_ascii_case("pagedown") => KeyCode::PageDown,
        k if k.len() >= 2 && (k.starts_with('F') || k.starts_with('f')) => {
            let n: u8 = k[1..].parse().ok()?;
            if (1..=24).contains(&n) {
                KeyCode::F(n)
            } else {
                return None;
            }
        }
        k => {
            let mut chars = k.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(c.to_ascii_lowercase())
        }
    };

    Some((code, modifiers))
}

/// The `keybindings` section of `settings.json`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Keybindings {
    /// Map from action name (e.g. `"console_view"`) to key spec
    /// (e.g. `"Ctrl+D"`). Arbitrary spec strings are accepted; parsing
    /// happens when the map is built in [`KeybindingMap::from_settings`].
    #[serde(default)]
    pub global: HashMap<String, String>,
}

impl Keybindings {
    /// Merge `other` into `self` — per-action `other` wins on key collision.
    pub fn merge(&mut self, other: Keybindings) {
        for (k, v) in other.global {
            self.global.insert(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind};

    fn key_event(code: KeyCode, mods: KeyModifiers) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        })
    }

    #[test]
    fn parse_round_trips_common_forms() {
        assert_eq!(
            parse_key_spec("Ctrl+D"),
            Some((KeyCode::Char('d'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key_spec("ctrl+d"),
            Some((KeyCode::Char('d'), KeyModifiers::CONTROL))
        );
        assert_eq!(
            parse_key_spec("Alt+Shift+F4"),
            Some((KeyCode::F(4), KeyModifiers::ALT | KeyModifiers::SHIFT))
        );
        assert_eq!(parse_key_spec("Esc"), Some((KeyCode::Esc, KeyModifiers::NONE)));
        assert_eq!(
            parse_key_spec("Space"),
            Some((KeyCode::Char(' '), KeyModifiers::NONE))
        );
        assert_eq!(parse_key_spec("F1"), Some((KeyCode::F(1), KeyModifiers::NONE)));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_key_spec("").is_none());
        assert!(parse_key_spec("not-a-key").is_none());
        assert!(parse_key_spec("Ctrl+").is_none());
        assert!(parse_key_spec("Ctrl+hello").is_none());
        assert!(parse_key_spec("F99").is_none());
    }

    #[test]
    fn defaults_match_built_in_behaviour() {
        let map = KeybindingMap::default();
        let ctrl_d = key_event(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(map.matches("console_view", &ctrl_d));
        let ctrl_p = key_event(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(map.matches("plan_mode_toggle", &ctrl_p));
        // Miss on wrong mods.
        let d = key_event(KeyCode::Char('d'), KeyModifiers::NONE);
        assert!(!map.matches("console_view", &d));
    }

    #[test]
    fn user_override_replaces_default() {
        let mut cfg = Keybindings::default();
        cfg.global
            .insert("console_view".to_string(), "Ctrl+K".to_string());
        let map = KeybindingMap::from_settings(Some(&cfg));

        let ctrl_k = key_event(KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert!(map.matches("console_view", &ctrl_k));
        // Old binding no longer fires.
        let ctrl_d = key_event(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(!map.matches("console_view", &ctrl_d));
        // Non-overridden bindings keep the default.
        let ctrl_p = key_event(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(map.matches("plan_mode_toggle", &ctrl_p));
    }

    #[test]
    fn unknown_action_is_ignored_not_fatal() {
        let mut cfg = Keybindings::default();
        cfg.global
            .insert("no_such_action".to_string(), "Ctrl+X".to_string());
        let map = KeybindingMap::from_settings(Some(&cfg));
        // Defaults still work.
        let ctrl_d = key_event(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert!(map.matches("console_view", &ctrl_d));
    }

    #[test]
    fn keybindings_merge_later_wins() {
        let mut a = Keybindings::default();
        a.global
            .insert("console_view".to_string(), "Ctrl+D".to_string());
        let mut b = Keybindings::default();
        b.global
            .insert("console_view".to_string(), "Ctrl+K".to_string());
        b.global
            .insert("plan_mode_toggle".to_string(), "Ctrl+J".to_string());
        a.merge(b);
        assert_eq!(a.global.get("console_view").unwrap(), "Ctrl+K");
        assert_eq!(a.global.get("plan_mode_toggle").unwrap(), "Ctrl+J");
    }
}
