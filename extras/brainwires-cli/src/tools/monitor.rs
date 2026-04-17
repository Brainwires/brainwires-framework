//! Monitor Tool — watch a long-running shell process without blocking the turn.
//!
//! Bridges the gap between `execute_command` (fully blocking) and kicking off
//! a full background agent. The agent starts a process via `monitor_start`,
//! gets an opaque id back, and can then poll `monitor_read` between other
//! tool calls to see new stdout/stderr. When done, `monitor_stop` kills the
//! process.
//!
//! Inspired by Claude Code's Monitor tool. Kept intentionally narrow: no
//! pattern-based wait, no file descriptors, no stdin — those are already
//! solved by the shell itself.
//!
//! # State model
//!
//! One [`MonitorTool`] per session holds a registry of child processes,
//! each pushing stdout+stderr lines into a capped ring buffer. Reads drain
//! new lines since the caller's last read offset, so the agent only sees
//! lines it hasn't seen before.
//!
//! # Safety
//!
//! `monitor_start` runs arbitrary shell commands — it passes through the
//! same approval layer as the `execute_command` tool. We do NOT add a
//! second approval here; the tool executor is responsible.

use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::types::tool::{Tool, ToolInputSchema, ToolResult};

/// Hard cap on lines retained per watcher, regardless of reads. Protects
/// against runaway processes eating all our memory.
const MAX_BUFFERED_LINES: usize = 10_000;

/// Hard cap on lines returned by a single `monitor_read`. Prevents blowing
/// up the model's context window with a gigantic log dump.
const DEFAULT_READ_LIMIT: usize = 200;
const MAX_READ_LIMIT: usize = 2_000;

/// One watched process.
struct MonitorProcess {
    /// The command string as the agent supplied it, for display / listing.
    command: String,
    /// Working directory the process was launched in.
    cwd: String,
    /// When the process was started, used for the `age` field in listings.
    started_at: Instant,
    /// Lines buffered since start (stdout + stderr merged in timestamp order).
    /// Bounded at [`MAX_BUFFERED_LINES`] with FIFO eviction.
    buffer: Arc<Mutex<VecDeque<Line>>>,
    /// Running sequence number — every line gets a unique, monotonically
    /// increasing offset so callers can pass `since_offset` for idempotent
    /// reads. Higher than the last line's offset means no new data.
    next_offset: Arc<Mutex<u64>>,
    /// Count of lines evicted by the ring buffer cap. Surfaced on read so
    /// agents know when a chatty process has outrun them.
    dropped: Arc<Mutex<u64>>,
    /// Child handle; `None` after the process has been awaited.
    child: Arc<Mutex<Option<Child>>>,
    /// Join handle for the output-pumping task. We keep it so a future
    /// enhancement can `.abort()` on stop; today the pump exits naturally
    /// when stdout/stderr close after `kill()`.
    #[allow(dead_code)]
    pump: Option<JoinHandle<()>>,
    /// Terminal state, set when the process exits on its own or is stopped.
    final_status: Arc<Mutex<Option<FinalStatus>>>,
}

#[derive(Debug, Clone)]
struct Line {
    offset: u64,
    stream: LineStream,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineStream {
    Stdout,
    Stderr,
}

impl LineStream {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

#[derive(Debug, Clone)]
struct FinalStatus {
    exit_code: Option<i32>,
    /// `true` when stop was triggered by `monitor_stop`, not natural exit.
    killed: bool,
}

/// The Monitor tool. Cheap to clone — all state lives behind `Arc`.
#[derive(Clone, Default)]
pub struct MonitorTool {
    procs: Arc<Mutex<HashMap<String, Arc<MonitorProcess>>>>,
}

impl MonitorTool {
    pub fn new() -> Self {
        Self::default()
    }

    /// All tool definitions this tool exposes to the model.
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::start_tool(),
            Self::read_tool(),
            Self::stop_tool(),
            Self::list_tool(),
        ]
    }

    fn start_tool() -> Tool {
        let mut props = HashMap::new();
        props.insert(
            "command".to_string(),
            json!({
                "type": "string",
                "description": "Shell command to start. Runs through `bash -c`. The process runs until it exits or monitor_stop is called."
            }),
        );
        props.insert(
            "cwd".to_string(),
            json!({
                "type": "string",
                "description": "Working directory (absolute path). Defaults to the agent's current working directory."
            }),
        );
        Tool {
            name: "monitor_start".to_string(),
            description: "Start a long-running shell command in the background and return an opaque id. Use monitor_read to poll its stdout/stderr, monitor_stop to terminate. Useful for dev servers, watchers, tailing logs, or any command that streams output over time."
                .to_string(),
            input_schema: ToolInputSchema::object(props, vec!["command".to_string()]),
            requires_approval: true,
            defer_loading: false,
            ..Default::default()
        }
    }

    fn read_tool() -> Tool {
        let mut props = HashMap::new();
        props.insert(
            "id".to_string(),
            json!({"type": "string", "description": "Monitor id returned by monitor_start"}),
        );
        props.insert(
            "since_offset".to_string(),
            json!({
                "type": "integer",
                "minimum": 0,
                "description": "Only return lines with offset >= this value. Pass the `next_offset` from the previous read to resume exactly where you left off. Omit to get all buffered lines."
            }),
        );
        props.insert(
            "max_lines".to_string(),
            json!({
                "type": "integer",
                "minimum": 1,
                "maximum": MAX_READ_LIMIT as u64,
                "description": format!(
                    "Maximum number of lines to return (default {}, max {}).",
                    DEFAULT_READ_LIMIT, MAX_READ_LIMIT
                ),
            }),
        );
        Tool {
            name: "monitor_read".to_string(),
            description: "Read new stdout/stderr lines from a monitored process. Returns up to `max_lines` lines at or after `since_offset`, plus `next_offset`, `dropped_lines` (count of lines the ring buffer evicted before you read them), and status. Non-blocking — returns immediately with whatever is buffered."
                .to_string(),
            input_schema: ToolInputSchema::object(props, vec!["id".to_string()]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    fn stop_tool() -> Tool {
        let mut props = HashMap::new();
        props.insert(
            "id".to_string(),
            json!({"type": "string", "description": "Monitor id returned by monitor_start"}),
        );
        Tool {
            name: "monitor_stop".to_string(),
            description: "Terminate a monitored process. Sends SIGKILL on Unix / TerminateProcess on Windows. Safe to call on an already-exited process. Returns the final exit status."
                .to_string(),
            input_schema: ToolInputSchema::object(props, vec!["id".to_string()]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    fn list_tool() -> Tool {
        Tool {
            name: "monitor_list".to_string(),
            description: "List all active monitored processes for this session with their ids, commands, status, and buffered-line counts."
                .to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    /// Dispatch a `monitor_*` tool call.
    pub async fn execute(&self, tool_use_id: &str, tool_name: &str, input: &Value) -> ToolResult {
        match tool_name {
            "monitor_start" => self.do_start(tool_use_id, input).await,
            "monitor_read" => self.do_read(tool_use_id, input).await,
            "monitor_stop" => self.do_stop(tool_use_id, input).await,
            "monitor_list" => self.do_list(tool_use_id).await,
            other => ToolResult::error(
                tool_use_id.to_string(),
                format!("Unknown monitor tool: {}", other),
            ),
        }
    }

    async fn do_start(&self, tool_use_id: &str, input: &Value) -> ToolResult {
        #[derive(Deserialize)]
        struct Args {
            command: String,
            cwd: Option<String>,
        }
        let args: Args = match serde_json::from_value(input.clone()) {
            Ok(a) => a,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("invalid monitor_start input: {}", e),
                );
            }
        };

        if args.command.trim().is_empty() {
            return ToolResult::error(
                tool_use_id.to_string(),
                "command must not be empty".to_string(),
            );
        }

        let cwd = args
            .cwd
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });

        let mut cmd = Command::new("bash");
        cmd.arg("-o").arg("pipefail").arg("-c").arg(&args.command);
        cmd.current_dir(&cwd);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("failed to spawn: {}", e),
                );
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let id = new_id();
        let buffer: Arc<Mutex<VecDeque<Line>>> = Arc::new(Mutex::new(VecDeque::new()));
        let next_offset = Arc::new(Mutex::new(0u64));
        let dropped = Arc::new(Mutex::new(0u64));
        let final_status = Arc::new(Mutex::new(None));

        let proc = Arc::new(MonitorProcess {
            command: args.command.clone(),
            cwd: cwd.clone(),
            started_at: Instant::now(),
            buffer: buffer.clone(),
            next_offset: next_offset.clone(),
            dropped: dropped.clone(),
            child: Arc::new(Mutex::new(Some(child))),
            pump: None, // filled in below
            final_status: final_status.clone(),
        });

        // Pump stdout+stderr into the buffer, then wait for exit. We use a
        // single spawn because `Child` isn't `Clone`; we take it out of the
        // mutex for the await and put nothing back (the Option flips to None).
        let proc_for_pump = proc.clone();
        let pump = tokio::spawn(pump_output(
            proc_for_pump,
            stdout,
            stderr,
            buffer,
            next_offset,
            dropped,
            final_status,
        ));

        // Swap the freshly-constructed proc with one that owns the join
        // handle. Since Arc makes `proc` immutable we re-box it with the
        // handle attached.
        let proc = Arc::new(MonitorProcess {
            command: proc.command.clone(),
            cwd: proc.cwd.clone(),
            started_at: proc.started_at,
            buffer: proc.buffer.clone(),
            next_offset: proc.next_offset.clone(),
            dropped: proc.dropped.clone(),
            child: proc.child.clone(),
            pump: Some(pump),
            final_status: proc.final_status.clone(),
        });

        self.procs.lock().await.insert(id.clone(), proc);

        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({
                "id": id,
                "command": args.command,
                "cwd": cwd,
                "status": "running",
            }))
            .unwrap_or_default(),
        )
    }

    async fn do_read(&self, tool_use_id: &str, input: &Value) -> ToolResult {
        #[derive(Deserialize)]
        struct Args {
            id: String,
            #[serde(default)]
            since_offset: Option<u64>,
            #[serde(default)]
            max_lines: Option<usize>,
        }
        let args: Args = match serde_json::from_value(input.clone()) {
            Ok(a) => a,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("invalid monitor_read input: {}", e),
                );
            }
        };

        let max = args
            .max_lines
            .unwrap_or(DEFAULT_READ_LIMIT)
            .clamp(1, MAX_READ_LIMIT);
        let since = args.since_offset.unwrap_or(0);

        let proc = match self.procs.lock().await.get(&args.id).cloned() {
            Some(p) => p,
            None => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("no monitor with id: {}", args.id),
                );
            }
        };

        let buffer = proc.buffer.lock().await;
        let lines: Vec<&Line> = buffer
            .iter()
            .skip_while(|l| l.offset < since)
            .take(max)
            .collect();

        let rendered: Vec<Value> = lines
            .iter()
            .map(|l| {
                json!({
                    "offset": l.offset,
                    "stream": l.stream.as_str(),
                    "text": l.text,
                })
            })
            .collect();

        let next_offset = lines.last().map(|l| l.offset + 1).unwrap_or(since);
        let truncated = buffer
            .iter()
            .skip_while(|l| l.offset < since)
            .count()
            > lines.len();

        drop(buffer);

        let dropped = *proc.dropped.lock().await;
        let status = describe_status(&proc).await;

        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({
                "id": args.id,
                "lines": rendered,
                "next_offset": next_offset,
                "more_available": truncated,
                "dropped_lines": dropped,
                "status": status,
            }))
            .unwrap_or_default(),
        )
    }

    async fn do_stop(&self, tool_use_id: &str, input: &Value) -> ToolResult {
        #[derive(Deserialize)]
        struct Args {
            id: String,
        }
        let args: Args = match serde_json::from_value(input.clone()) {
            Ok(a) => a,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("invalid monitor_stop input: {}", e),
                );
            }
        };

        let proc = match self.procs.lock().await.get(&args.id).cloned() {
            Some(p) => p,
            None => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("no monitor with id: {}", args.id),
                );
            }
        };

        // Kill the child if still alive. `kill().await` issues SIGKILL on
        // Unix; we don't try SIGTERM first because monitor_stop is meant
        // to be immediate — if the model wanted graceful shutdown it would
        // send a signal via the shell.
        let mut guard = proc.child.lock().await;
        if let Some(mut child) = guard.take() {
            let _ = child.start_kill();
            // Best effort — we don't await full exit here to avoid blocking
            // if the process ignores SIGKILL (rare but happens with zombies).
            let _ = child.try_wait();
            *proc.final_status.lock().await = Some(FinalStatus {
                exit_code: None,
                killed: true,
            });
        }
        drop(guard);

        let status = describe_status(&proc).await;

        // Remove from registry so `list` no longer shows it.
        self.procs.lock().await.remove(&args.id);

        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({
                "id": args.id,
                "status": status,
            }))
            .unwrap_or_default(),
        )
    }

    async fn do_list(&self, tool_use_id: &str) -> ToolResult {
        let procs = self.procs.lock().await;
        let mut items = Vec::with_capacity(procs.len());
        for (id, proc) in procs.iter() {
            let buffer = proc.buffer.lock().await;
            let buffered = buffer.len();
            drop(buffer);
            let dropped = *proc.dropped.lock().await;
            let status = describe_status(proc).await;
            items.push(json!({
                "id": id,
                "command": proc.command,
                "cwd": proc.cwd,
                "age_seconds": proc.started_at.elapsed().as_secs(),
                "buffered_lines": buffered,
                "dropped_lines": dropped,
                "status": status,
            }));
        }

        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&json!({ "monitors": items })).unwrap_or_default(),
        )
    }
}

async fn describe_status(proc: &MonitorProcess) -> Value {
    match &*proc.final_status.lock().await {
        Some(FinalStatus {
            exit_code,
            killed: true,
        }) => json!({
            "state": "killed",
            "exit_code": exit_code,
        }),
        Some(FinalStatus {
            exit_code: Some(code),
            killed: false,
        }) => json!({
            "state": if *code == 0 { "exited_ok" } else { "exited_error" },
            "exit_code": code,
        }),
        Some(FinalStatus {
            exit_code: None,
            killed: false,
        }) => json!({ "state": "exited_unknown" }),
        None => json!({ "state": "running" }),
    }
}

/// Drain `stdout`+`stderr` into the ring buffer, then await child exit and
/// record the final status. Runs once per started process.
async fn pump_output(
    proc: Arc<MonitorProcess>,
    stdout: Option<tokio::process::ChildStdout>,
    stderr: Option<tokio::process::ChildStderr>,
    buffer: Arc<Mutex<VecDeque<Line>>>,
    next_offset: Arc<Mutex<u64>>,
    dropped: Arc<Mutex<u64>>,
    final_status: Arc<Mutex<Option<FinalStatus>>>,
) {
    // Spawn two reader tasks so stderr isn't starved by a chatty stdout.
    let push_line = {
        let buffer = buffer.clone();
        let next_offset = next_offset.clone();
        let dropped = dropped.clone();
        move |stream: LineStream, text: String| {
            let buffer = buffer.clone();
            let next_offset = next_offset.clone();
            let dropped = dropped.clone();
            async move {
                let mut off = next_offset.lock().await;
                let offset = *off;
                *off = offset + 1;
                drop(off);
                let mut b = buffer.lock().await;
                b.push_back(Line {
                    offset,
                    stream,
                    text,
                });
                let mut evicted: u64 = 0;
                while b.len() > MAX_BUFFERED_LINES {
                    b.pop_front();
                    evicted += 1;
                }
                drop(b);
                if evicted > 0 {
                    *dropped.lock().await += evicted;
                }
            }
        }
    };

    let stdout_task = stdout.map(|s| {
        let push = push_line.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                push(LineStream::Stdout, line).await;
            }
        })
    });

    let stderr_task = stderr.map(|s| {
        let push = push_line.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                push(LineStream::Stderr, line).await;
            }
        })
    });

    if let Some(t) = stdout_task {
        let _ = t.await;
    }
    if let Some(t) = stderr_task {
        let _ = t.await;
    }

    // Readers closed; now reap the child.
    let mut guard = proc.child.lock().await;
    if let Some(mut child) = guard.take() {
        match child.wait().await {
            Ok(status) => {
                *final_status.lock().await = Some(FinalStatus {
                    exit_code: status.code(),
                    killed: false,
                });
            }
            Err(_) => {
                *final_status.lock().await = Some(FinalStatus {
                    exit_code: None,
                    killed: false,
                });
            }
        }
    }
}

/// Generate a short unique id — `mon-` + 12 random hex chars.
fn new_id() -> String {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // Mix in a small counter to avoid collisions inside the same nanosecond
    // on systems with coarse clocks.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("mon-{:012x}", (nanos as u64) ^ c.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn wait_for_status(tool: &MonitorTool, id: &str, want_running: bool) {
        for _ in 0..50 {
            let result = tool.do_read("t", &json!({"id": id})).await;
            if !result.is_error {
                let val: Value = serde_json::from_str(&result.content).unwrap();
                let state = val["status"]["state"].as_str().unwrap_or("");
                if (state == "running") == want_running {
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    fn start_args(cmd: &str) -> Value {
        json!({"command": cmd})
    }

    #[tokio::test]
    async fn start_read_stop_lifecycle() {
        let tool = MonitorTool::new();

        let started = tool
            .do_start("t1", &start_args("echo hello; echo world"))
            .await;
        assert!(!started.is_error, "start failed: {:?}", started.content);
        let v: Value = serde_json::from_str(&started.content).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        // Wait for the short process to exit.
        wait_for_status(&tool, &id, false).await;

        let read = tool.do_read("t2", &json!({"id": id})).await;
        assert!(!read.is_error);
        let v: Value = serde_json::from_str(&read.content).unwrap();
        let lines = v["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["text"], "hello");
        assert_eq!(lines[1]["text"], "world");
        assert_eq!(v["next_offset"], 2);
        assert!(
            ["exited_ok", "exited_error"].contains(&v["status"]["state"].as_str().unwrap()),
            "unexpected state {:?}",
            v["status"]
        );
    }

    #[tokio::test]
    async fn since_offset_skips_seen_lines() {
        let tool = MonitorTool::new();
        let started = tool
            .do_start("t1", &start_args("echo one; echo two; echo three"))
            .await;
        let v: Value = serde_json::from_str(&started.content).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        wait_for_status(&tool, &id, false).await;

        // Read first line only.
        let first = tool
            .do_read("t2", &json!({"id": id, "max_lines": 1}))
            .await;
        let v: Value = serde_json::from_str(&first.content).unwrap();
        assert_eq!(v["lines"].as_array().unwrap().len(), 1);
        assert_eq!(v["next_offset"], 1);

        // Resume from next_offset.
        let rest = tool
            .do_read("t3", &json!({"id": id, "since_offset": 1}))
            .await;
        let v: Value = serde_json::from_str(&rest.content).unwrap();
        let lines = v["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["text"], "two");
        assert_eq!(lines[1]["text"], "three");
    }

    #[tokio::test]
    async fn stop_removes_from_registry() {
        let tool = MonitorTool::new();
        let started = tool.do_start("t1", &start_args("sleep 30")).await;
        let v: Value = serde_json::from_str(&started.content).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        // Confirm it's listed.
        let list = tool.do_list("l1").await;
        let v: Value = serde_json::from_str(&list.content).unwrap();
        assert_eq!(v["monitors"].as_array().unwrap().len(), 1);

        // Stop and confirm it's gone.
        let stopped = tool.do_stop("t2", &json!({"id": id})).await;
        assert!(!stopped.is_error);

        let list = tool.do_list("l2").await;
        let v: Value = serde_json::from_str(&list.content).unwrap();
        assert_eq!(v["monitors"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn read_unknown_id_is_error() {
        let tool = MonitorTool::new();
        let result = tool.do_read("t1", &json!({"id": "mon-nope"})).await;
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn empty_command_is_error() {
        let tool = MonitorTool::new();
        let result = tool.do_start("t1", &json!({"command": "   "})).await;
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn failed_command_records_nonzero_exit() {
        let tool = MonitorTool::new();
        let started = tool.do_start("t1", &start_args("exit 7")).await;
        assert!(!started.is_error);
        let v: Value = serde_json::from_str(&started.content).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        wait_for_status(&tool, &id, false).await;

        let read = tool.do_read("t2", &json!({"id": id})).await;
        let v: Value = serde_json::from_str(&read.content).unwrap();
        assert_eq!(v["status"]["state"], "exited_error");
        assert_eq!(v["status"]["exit_code"], 7);
        assert_eq!(v["dropped_lines"], 0);
    }

    #[tokio::test]
    async fn ring_buffer_records_dropped_lines() {
        // Emit more lines than MAX_BUFFERED_LINES so the ring evicts older
        // ones. Tested at the do_read level: `dropped_lines` must grow past
        // zero and the oldest surviving line must no longer be offset 0.
        let tool = MonitorTool::new();
        let overrun = MAX_BUFFERED_LINES + 50;
        let cmd = format!("for i in $(seq 1 {}); do echo \"$i\"; done", overrun);
        let started = tool.do_start("t1", &start_args(&cmd)).await;
        assert!(!started.is_error);
        let v: Value = serde_json::from_str(&started.content).unwrap();
        let id = v["id"].as_str().unwrap().to_string();

        wait_for_status(&tool, &id, false).await;

        let read = tool
            .do_read("t2", &json!({"id": id, "max_lines": 1}))
            .await;
        let v: Value = serde_json::from_str(&read.content).unwrap();
        assert!(
            v["dropped_lines"].as_u64().unwrap() >= 50,
            "expected at least 50 dropped lines, got {:?}",
            v["dropped_lines"]
        );
        // First surviving line's offset should be >= 50 (we evicted the oldest).
        let first_off = v["lines"][0]["offset"].as_u64().unwrap();
        assert!(
            first_off >= 50,
            "oldest surviving line should have been past offset 50, got {}",
            first_off
        );
    }
}
