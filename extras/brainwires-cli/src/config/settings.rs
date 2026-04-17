//! Layered settings — `settings.json` for the Claude-Code-shaped harness
//! config (permissions, hooks, env). Kept separate from `config.json` so
//! provider/model state stays clean.
//!
//! Merge order (later wins for scalars, arrays concatenate for
//! `permissions.allow/deny/ask`):
//!
//! 1. `~/.brainwires/settings.json` — user-wide
//! 2. `~/.claude/settings.json` — migrator compatibility (if present)
//! 3. `<project-root>/.brainwires/settings.json` — committed project rules
//! 4. `<project-root>/.brainwires/settings.local.json` — local overrides
//!
//! Pattern syntax for `permissions`:
//! - `"Read"` — tool name match, any args
//! - `"Bash(ls:*)"` — tool `execute_command`; `command` field prefix-matches `ls `
//! - `"Edit(src/**/*.rs)"` — tool `edit_file`; `file_path` matches the glob
//! - `"mcp__<server>__<tool>"` — exact MCP tool name

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Full merged harness settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Hooks>,
    /// Optional user-remapped TUI keybindings. See
    /// `crate::tui::keybindings` for action names + key-spec grammar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keybindings: Option<crate::tui::keybindings::Keybindings>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Permissions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ask: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Hooks {
    #[serde(rename = "PreToolUse", default, skip_serializing_if = "Vec::is_empty")]
    pub pre_tool_use: Vec<HookMatcher>,
    #[serde(rename = "PostToolUse", default, skip_serializing_if = "Vec::is_empty")]
    pub post_tool_use: Vec<HookMatcher>,
    #[serde(
        rename = "UserPromptSubmit",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub user_prompt_submit: Vec<HookMatcher>,
    #[serde(rename = "Stop", default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<HookMatcher>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HookMatcher {
    /// Tool-name glob. `None` means fire on every event (useful for
    /// `UserPromptSubmit` / `Stop` where there is no tool name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    #[serde(default)]
    pub hooks: Vec<HookCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookCommand {
    /// For now only `"command"` is supported — room to grow to `"mcp"` later.
    #[serde(rename = "type", default = "default_hook_type")]
    pub kind: String,
    /// Shell command piped to `bash -c`. Event JSON is written to stdin.
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

fn default_hook_type() -> String {
    "command".to_string()
}

impl Settings {
    /// Merge `other` into `self` following the documented precedence.
    /// Scalars: `other` wins when present. Arrays: concatenated.
    pub fn merge(&mut self, other: Settings) {
        // permissions — concatenate arrays if both sides present; else take the other.
        match (&mut self.permissions, other.permissions) {
            (Some(me), Some(them)) => {
                me.allow.extend(them.allow);
                me.deny.extend(them.deny);
                me.ask.extend(them.ask);
            }
            (slot @ None, Some(them)) => *slot = Some(them),
            _ => {}
        }
        // hooks — concatenate matchers per event.
        match (&mut self.hooks, other.hooks) {
            (Some(me), Some(them)) => {
                me.pre_tool_use.extend(them.pre_tool_use);
                me.post_tool_use.extend(them.post_tool_use);
                me.user_prompt_submit.extend(them.user_prompt_submit);
                me.stop.extend(them.stop);
            }
            (slot @ None, Some(them)) => *slot = Some(them),
            _ => {}
        }
        // keybindings — per-action later-wins on key collision.
        match (&mut self.keybindings, other.keybindings) {
            (Some(me), Some(them)) => me.merge(them),
            (slot @ None, Some(them)) => *slot = Some(them),
            _ => {}
        }
        // env — later wins on key collision.
        for (k, v) in other.env {
            self.env.insert(k, v);
        }
    }
}

/// What the permission layer decided for a single tool call.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionDecision {
    /// No matching rule — fall through to existing policy engine / approval.
    Unset,
    /// Allow without approval prompt (but still audit).
    Allow,
    /// Prompt user via the approval channel.
    Ask,
    /// Hard deny — overrides everything else including `PermissionMode::Full`.
    Deny(String),
}

impl Permissions {
    /// Decide what to do for a tool call. `deny` wins over `ask` wins over `allow`.
    pub fn decide(&self, tool_name: &str, args: &Value) -> PermissionDecision {
        if let Some(rule) = self
            .deny
            .iter()
            .find(|r| PermissionMatcher::new(r).matches(tool_name, args))
        {
            return PermissionDecision::Deny(format!("denied by settings rule '{}'", rule));
        }
        if self
            .ask
            .iter()
            .any(|r| PermissionMatcher::new(r).matches(tool_name, args))
        {
            return PermissionDecision::Ask;
        }
        if self
            .allow
            .iter()
            .any(|r| PermissionMatcher::new(r).matches(tool_name, args))
        {
            return PermissionDecision::Allow;
        }
        PermissionDecision::Unset
    }
}

/// Claude-Code-style permission rule pattern matcher.
///
/// Accepts: `Tool`, `Tool(argspec)`, and `mcp__server__tool`.
#[derive(Debug, Clone)]
pub struct PermissionMatcher<'a> {
    raw: &'a str,
}

impl<'a> PermissionMatcher<'a> {
    pub fn new(pattern: &'a str) -> Self {
        Self { raw: pattern }
    }

    /// Does this rule match the given tool call?
    pub fn matches(&self, tool_name: &str, args: &Value) -> bool {
        let (name_pat, arg_spec) = split_pattern(self.raw);

        if !tool_name_matches(name_pat, tool_name) {
            return false;
        }

        match arg_spec {
            None => true,
            Some(spec) => arg_matches(tool_name, spec, args),
        }
    }
}

fn split_pattern(raw: &str) -> (&str, Option<&str>) {
    if let Some(open) = raw.find('(')
        && let Some(close_rel) = raw[open + 1..].rfind(')')
    {
        let close = open + 1 + close_rel;
        return (&raw[..open], Some(&raw[open + 1..close]));
    }
    (raw, None)
}

/// Map a Claude-Code-style short name to the brainwires tool name when we
/// know the canonical alias. Unknown names pass through unchanged so users
/// can target brainwires-native tool names directly.
fn canonical_tool_name(pat: &str) -> &str {
    match pat {
        "Bash" => "execute_command",
        "Read" => "read_file",
        "Write" => "write_file",
        "Edit" => "edit_file",
        "Glob" => "glob",
        "Grep" => "search",
        "WebFetch" => "web_fetch",
        "WebSearch" => "web_search",
        other => other,
    }
}

fn tool_name_matches(pattern: &str, tool_name: &str) -> bool {
    let canon = canonical_tool_name(pattern);
    // Simple wildcard: trailing `*` matches any suffix. Everything else is exact.
    if let Some(prefix) = canon.strip_suffix('*') {
        tool_name.starts_with(prefix)
    } else {
        canon == tool_name
    }
}

/// Match the arg-spec part of a rule against the tool's input.
///
/// Current heuristics — chosen to match Claude Code's published rule style:
/// - `execute_command` / `run_command`: `command:*` prefix-matches the
///   `command` field, so `Bash(ls:*)` matches `ls -la`.
/// - `read_file` / `write_file` / `edit_file` / `delete_file`: glob against
///   the `file_path` (or `path`) field, so `Edit(src/**/*.rs)` works.
/// - MCP tools: no arg-spec support yet; bare names only.
/// - Unknown tool: no arg-spec support; returns `false`.
fn arg_matches(tool_name: &str, spec: &str, args: &Value) -> bool {
    match tool_name {
        "execute_command" | "run_command" => {
            if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                command_matches(spec, command)
            } else {
                false
            }
        }
        "read_file" | "write_file" | "edit_file" | "delete_file" => {
            let path = args
                .get("file_path")
                .and_then(|v| v.as_str())
                .or_else(|| args.get("path").and_then(|v| v.as_str()));
            match path {
                Some(p) => glob_matches(spec, p),
                None => false,
            }
        }
        _ => false,
    }
}

/// `"ls:*"` → prefix `"ls "`; `"ls"` → exact `"ls"`; `"*"` → always.
fn command_matches(spec: &str, command: &str) -> bool {
    if spec == "*" {
        return true;
    }
    if let Some(prefix) = spec.strip_suffix(":*") {
        // Match either "ls" alone or "ls " followed by args.
        return command == prefix || command.starts_with(&format!("{} ", prefix));
    }
    spec == command
}

/// Minimal glob: supports `*` (single path component) and `**` (any depth).
/// Anything fancier (brace expansion, character classes) is intentionally
/// left out — add it later if rules get richer.
fn glob_matches(pattern: &str, path: &str) -> bool {
    // Quick escape hatch: `*` matches everything.
    if pattern == "*" {
        return true;
    }

    // Build a regex from the glob. We don't want a full glob crate here — the
    // CLI already carries too many dependencies. Translate `**`, `*`, and `?`.
    let mut re = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    // Consume an optional trailing '/' after '**'.
                    if chars.peek() == Some(&'/') {
                        chars.next();
                    }
                    re.push_str(".*");
                } else {
                    re.push_str("[^/]*");
                }
            }
            '?' => re.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                re.push('\\');
                re.push(c);
            }
            other => re.push(other),
        }
    }
    re.push('$');
    match regex::Regex::new(&re) {
        Ok(r) => r.is_match(path),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_concatenates_permission_arrays() {
        let mut a = Settings {
            permissions: Some(Permissions {
                allow: vec!["Read".into()],
                deny: vec!["Bash(rm:*)".into()],
                ask: vec![],
            }),
            ..Default::default()
        };
        let b = Settings {
            permissions: Some(Permissions {
                allow: vec!["Edit".into()],
                deny: vec![],
                ask: vec!["WebFetch".into()],
            }),
            ..Default::default()
        };
        a.merge(b);
        let p = a.permissions.unwrap();
        assert_eq!(p.allow, vec!["Read", "Edit"]);
        assert_eq!(p.deny, vec!["Bash(rm:*)"]);
        assert_eq!(p.ask, vec!["WebFetch"]);
    }

    #[test]
    fn merge_env_later_wins() {
        let mut a = Settings::default();
        a.env.insert("K".into(), "1".into());
        let mut b = Settings::default();
        b.env.insert("K".into(), "2".into());
        a.merge(b);
        assert_eq!(a.env.get("K").map(String::as_str), Some("2"));
    }

    #[test]
    fn deny_wins_over_allow() {
        let p = Permissions {
            allow: vec!["Bash".into()],
            deny: vec!["Bash(rm:*)".into()],
            ask: vec![],
        };
        let decision = p.decide("execute_command", &json!({"command": "rm -rf /"}));
        assert!(matches!(decision, PermissionDecision::Deny(_)));
    }

    #[test]
    fn bash_prefix_pattern() {
        let p = Permissions {
            allow: vec!["Bash(ls:*)".into()],
            deny: vec![],
            ask: vec![],
        };
        assert_eq!(
            p.decide("execute_command", &json!({"command": "ls -la"})),
            PermissionDecision::Allow
        );
        assert_eq!(
            p.decide("execute_command", &json!({"command": "ls"})),
            PermissionDecision::Allow
        );
        assert_eq!(
            p.decide("execute_command", &json!({"command": "cat /etc/passwd"})),
            PermissionDecision::Unset
        );
    }

    #[test]
    fn edit_glob_pattern() {
        let p = Permissions {
            allow: vec!["Edit(src/**/*.rs)".into()],
            deny: vec![],
            ask: vec![],
        };
        assert_eq!(
            p.decide("edit_file", &json!({"file_path": "src/tools/mod.rs"})),
            PermissionDecision::Allow
        );
        assert_eq!(
            p.decide("edit_file", &json!({"file_path": "docs/README.md"})),
            PermissionDecision::Unset
        );
    }

    #[test]
    fn bare_tool_name_matches_any_args() {
        let p = Permissions {
            allow: vec!["Read".into()],
            deny: vec![],
            ask: vec![],
        };
        assert_eq!(
            p.decide("read_file", &json!({"path": "anything"})),
            PermissionDecision::Allow
        );
    }

    #[test]
    fn mcp_tool_exact_match() {
        let p = Permissions {
            allow: vec!["mcp__github__create_pr".into()],
            deny: vec![],
            ask: vec![],
        };
        assert_eq!(
            p.decide("mcp__github__create_pr", &json!({})),
            PermissionDecision::Allow
        );
        assert_eq!(
            p.decide("mcp__github__list_issues", &json!({})),
            PermissionDecision::Unset
        );
    }

    #[test]
    fn ask_beats_allow_but_loses_to_deny() {
        let p = Permissions {
            allow: vec!["Read".into()],
            ask: vec!["Read".into()],
            deny: vec![],
        };
        assert_eq!(
            p.decide("read_file", &json!({})),
            PermissionDecision::Ask
        );
    }

    #[test]
    fn settings_round_trip_json() {
        let s = Settings {
            permissions: Some(Permissions {
                allow: vec!["Read".into()],
                deny: vec!["Bash(rm:*)".into()],
                ask: vec![],
            }),
            hooks: Some(Hooks {
                pre_tool_use: vec![HookMatcher {
                    matcher: Some("Bash".into()),
                    hooks: vec![HookCommand {
                        kind: "command".into(),
                        command: "echo pre".into(),
                        timeout_ms: Some(3000),
                    }],
                }],
                ..Default::default()
            }),
            keybindings: None,
            env: {
                let mut m = HashMap::new();
                m.insert("FOO".into(), "bar".into());
                m
            },
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
