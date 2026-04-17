//! Hook dispatcher — runs user-supplied shell commands at four well-known
//! points in the agent lifecycle:
//!
//! | Event              | When it fires                                              |
//! |--------------------|------------------------------------------------------------|
//! | `PreToolUse`       | After approval, before the tool's implementation runs      |
//! | `PostToolUse`      | After the tool finishes, before the audit record is logged |
//! | `UserPromptSubmit` | Right after the user's turn is pushed into the conversation|
//! | `Stop`             | After the assistant's final message of a turn is stored    |
//!
//! Hooks are configured under `Settings.hooks` — see [`crate::config::Hooks`].
//!
//! Each hook runs `bash -c <command>` with a JSON event payload piped to
//! stdin. Exit-code semantics match Claude Code:
//!
//! - `0`   → `Continue`  (allow)
//! - `2`   → `Block`     (stderr is passed back to the model as feedback)
//! - any other non-zero  → `SoftError` (logged, does not block)

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use crate::config::{HookCommand, HookMatcher, Hooks, Settings, SettingsManager};

/// Default timeout per hook command.
const DEFAULT_HOOK_TIMEOUT_MS: u64 = 5_000;

/// Load layered harness settings + a matching hook dispatcher for the
/// given working directory. Returns `(settings, dispatcher)` so callers
/// can attach both to a tool executor or dispatch lifecycle events
/// directly.
pub fn load_for_cwd(cwd: &std::path::Path) -> (Arc<Settings>, Arc<HookDispatcher>) {
    let merged = SettingsManager::load(cwd).merged;
    let hooks = merged.hooks.clone().unwrap_or_default();
    let settings = Arc::new(merged);
    let dispatcher = Arc::new(HookDispatcher::new(hooks, cwd.to_path_buf()));
    (settings, dispatcher)
}

/// What the dispatcher decided this turn should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOutcome {
    Continue,
    Block { reason: String },
    SoftError(String),
}

impl HookOutcome {
    pub fn is_block(&self) -> bool {
        matches!(self, HookOutcome::Block { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    UserPromptSubmit,
    Stop,
}

/// Lightweight dispatcher — cheap to clone via `Arc` wrapper.
pub struct HookDispatcher {
    hooks: Hooks,
    cwd: std::path::PathBuf,
}

impl HookDispatcher {
    pub fn new(hooks: Hooks, cwd: std::path::PathBuf) -> Self {
        Self { hooks, cwd }
    }

    pub fn empty(cwd: std::path::PathBuf) -> Self {
        Self {
            hooks: Hooks::default(),
            cwd,
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    pub fn has_any(&self) -> bool {
        !(self.hooks.pre_tool_use.is_empty()
            && self.hooks.post_tool_use.is_empty()
            && self.hooks.user_prompt_submit.is_empty()
            && self.hooks.stop.is_empty())
    }

    /// Fire PreToolUse. Block result is surfaced as an error the caller
    /// should short-circuit on (matches Claude Code: exit 2 on PreToolUse
    /// means "do not run this tool").
    pub async fn dispatch_pre_tool(&self, tool_name: &str, tool_args: &Value) -> HookOutcome {
        let matchers = &self.hooks.pre_tool_use;
        if matchers.is_empty() {
            return HookOutcome::Continue;
        }
        let payload = json!({
            "event": "PreToolUse",
            "tool_name": tool_name,
            "tool_args": tool_args,
            "cwd": self.cwd,
        });
        self.run_matchers(matchers, tool_name, payload).await
    }

    pub async fn dispatch_post_tool(
        &self,
        tool_name: &str,
        tool_args: &Value,
        tool_result: &Value,
        is_error: bool,
    ) -> HookOutcome {
        let matchers = &self.hooks.post_tool_use;
        if matchers.is_empty() {
            return HookOutcome::Continue;
        }
        let payload = json!({
            "event": "PostToolUse",
            "tool_name": tool_name,
            "tool_args": tool_args,
            "tool_result": tool_result,
            "is_error": is_error,
            "cwd": self.cwd,
        });
        self.run_matchers(matchers, tool_name, payload).await
    }

    pub async fn dispatch_user_prompt(&self, prompt: &str) -> HookOutcome {
        let matchers = &self.hooks.user_prompt_submit;
        if matchers.is_empty() {
            return HookOutcome::Continue;
        }
        let payload = json!({
            "event": "UserPromptSubmit",
            "prompt": prompt,
            "cwd": self.cwd,
        });
        self.run_matchers(matchers, "", payload).await
    }

    pub async fn dispatch_stop(&self, final_message: &str) -> HookOutcome {
        let matchers = &self.hooks.stop;
        if matchers.is_empty() {
            return HookOutcome::Continue;
        }
        let payload = json!({
            "event": "Stop",
            "final_message": final_message,
            "cwd": self.cwd,
        });
        self.run_matchers(matchers, "", payload).await
    }

    async fn run_matchers(
        &self,
        matchers: &[HookMatcher],
        tool_name: &str,
        payload: Value,
    ) -> HookOutcome {
        for m in matchers {
            if !matcher_applies(&m.matcher, tool_name) {
                continue;
            }
            for hook in &m.hooks {
                match run_hook_command(hook, &payload).await {
                    HookOutcome::Continue => {}
                    blocking => return blocking,
                }
            }
        }
        HookOutcome::Continue
    }
}

fn matcher_applies(matcher: &Option<String>, tool_name: &str) -> bool {
    match matcher {
        None => true,
        Some(s) if s == "*" => true,
        Some(s) => {
            // Support same short-name aliases as the permission matcher.
            let canon = match s.as_str() {
                "Bash" => "execute_command",
                "Read" => "read_file",
                "Write" => "write_file",
                "Edit" => "edit_file",
                "Glob" => "glob",
                "Grep" => "search",
                "WebFetch" => "web_fetch",
                "WebSearch" => "web_search",
                other => other,
            };
            if let Some(prefix) = canon.strip_suffix('*') {
                tool_name.starts_with(prefix)
            } else {
                canon == tool_name
            }
        }
    }
}

async fn run_hook_command(hook: &HookCommand, payload: &Value) -> HookOutcome {
    if hook.kind != "command" {
        return HookOutcome::SoftError(format!("unknown hook kind: {}", hook.kind));
    }

    let timeout_ms = hook.timeout_ms.unwrap_or(DEFAULT_HOOK_TIMEOUT_MS);
    let payload_bytes = match serde_json::to_vec(payload) {
        Ok(b) => b,
        Err(e) => return HookOutcome::SoftError(format!("failed to serialise payload: {}", e)),
    };

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(&hook.command);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return HookOutcome::SoftError(format!("failed to spawn hook: {}", e)),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&payload_bytes).await;
        drop(stdin);
    }

    let wait_result = timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await;

    let output = match wait_result {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return HookOutcome::SoftError(format!("hook error: {}", e)),
        Err(_) => {
            return HookOutcome::SoftError(format!(
                "hook timed out after {}ms: {}",
                timeout_ms, hook.command
            ));
        }
    };

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match code {
        0 => HookOutcome::Continue,
        2 => HookOutcome::Block {
            reason: if stderr.is_empty() {
                "blocked by hook".to_string()
            } else {
                stderr
            },
        },
        _ => HookOutcome::SoftError(format!(
            "hook exited with code {}: {}",
            code,
            if stderr.is_empty() {
                "(no stderr)".to_string()
            } else {
                stderr
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HookCommand, HookMatcher, Hooks};
    use std::path::PathBuf;

    fn single_hook(event: &str, cmd: &str) -> Hooks {
        let matcher = HookMatcher {
            matcher: None,
            hooks: vec![HookCommand {
                kind: "command".into(),
                command: cmd.into(),
                timeout_ms: Some(2_000),
            }],
        };
        let mut h = Hooks::default();
        match event {
            "pre" => h.pre_tool_use.push(matcher),
            "post" => h.post_tool_use.push(matcher),
            "prompt" => h.user_prompt_submit.push(matcher),
            "stop" => h.stop.push(matcher),
            _ => {}
        }
        h
    }

    #[tokio::test]
    async fn zero_exit_continues() {
        let d = HookDispatcher::new(single_hook("pre", "exit 0"), PathBuf::from("/"));
        assert_eq!(
            d.dispatch_pre_tool("x", &json!({})).await,
            HookOutcome::Continue
        );
    }

    #[tokio::test]
    async fn exit_two_blocks_with_stderr_message() {
        let d = HookDispatcher::new(
            single_hook("pre", "echo nope 1>&2; exit 2"),
            PathBuf::from("/"),
        );
        match d.dispatch_pre_tool("x", &json!({})).await {
            HookOutcome::Block { reason } => assert_eq!(reason, "nope"),
            other => panic!("expected Block, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn other_nonzero_is_soft_error_not_block() {
        let d = HookDispatcher::new(single_hook("pre", "exit 1"), PathBuf::from("/"));
        match d.dispatch_pre_tool("x", &json!({})).await {
            HookOutcome::SoftError(_) => {}
            other => panic!("expected SoftError, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn timeout_is_soft_error() {
        let hook = HookCommand {
            kind: "command".into(),
            command: "sleep 5".into(),
            timeout_ms: Some(100),
        };
        let m = HookMatcher {
            matcher: None,
            hooks: vec![hook],
        };
        let mut hooks = Hooks::default();
        hooks.pre_tool_use.push(m);
        let d = HookDispatcher::new(hooks, PathBuf::from("/"));
        assert!(matches!(
            d.dispatch_pre_tool("x", &json!({})).await,
            HookOutcome::SoftError(_)
        ));
    }

    #[tokio::test]
    async fn no_matchers_continues_silently() {
        let d = HookDispatcher::empty(PathBuf::from("/"));
        assert_eq!(
            d.dispatch_pre_tool("x", &json!({})).await,
            HookOutcome::Continue
        );
        assert_eq!(
            d.dispatch_user_prompt("hi").await,
            HookOutcome::Continue
        );
    }

    #[tokio::test]
    async fn matcher_only_fires_for_named_tool() {
        let mut hooks = Hooks::default();
        hooks.pre_tool_use.push(HookMatcher {
            matcher: Some("Bash".into()),
            hooks: vec![HookCommand {
                kind: "command".into(),
                command: "exit 2".into(),
                timeout_ms: Some(1000),
            }],
        });
        let d = HookDispatcher::new(hooks, PathBuf::from("/"));
        // Matcher says "Bash" (→ execute_command) — a read_file call must pass through.
        assert_eq!(
            d.dispatch_pre_tool("read_file", &json!({})).await,
            HookOutcome::Continue
        );
        // execute_command is the canonical alias — should block.
        assert!(
            d.dispatch_pre_tool("execute_command", &json!({}))
                .await
                .is_block()
        );
    }

    #[tokio::test]
    async fn event_payload_reaches_hook_stdin() {
        use std::io::Read;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let out = tmp.path().to_string_lossy().to_string();
        let hook_cmd = format!("cat > {}", out);
        let d = HookDispatcher::new(
            single_hook("prompt", &hook_cmd),
            PathBuf::from("/tmp"),
        );
        assert_eq!(
            d.dispatch_user_prompt("hello world").await,
            HookOutcome::Continue
        );
        let mut contents = String::new();
        std::fs::File::open(tmp.path())
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
        assert!(contents.contains("\"event\":\"UserPromptSubmit\""));
        assert!(contents.contains("\"prompt\":\"hello world\""));
    }
}
