//! UNSAFE — no isolation; for development and testing only.
//!
//! `HostSandbox` spawns processes directly on the host with `tokio::process`.
//! It still enforces the policy's mount whitelist (so consumers can catch
//! mis-configured policies early) and the per-spec timeout, but does not
//! apply any resource limits, namespaces, or network isolation. Do not use
//! in production.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;

use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::error::{Result, SandboxError};
use crate::{ExecHandle, ExecOutput, ExecSpec, Sandbox, SandboxPolicy, SandboxRuntime};

struct Job {
    child: Child,
    started: Instant,
    timeout: std::time::Duration,
}

/// Host pass-through implementation — NO isolation.
pub struct HostSandbox {
    policy: SandboxPolicy,
    jobs: Arc<Mutex<HashMap<ExecHandle, Job>>>,
}

impl HostSandbox {
    /// Build a new host sandbox. The `policy` is used only for mount
    /// validation; resource limits and network rules are ignored.
    pub fn new(policy: SandboxPolicy) -> Self {
        Self {
            policy,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl Sandbox for HostSandbox {
    async fn spawn(&self, spec: ExecSpec) -> Result<ExecHandle> {
        for m in &spec.mounts {
            self.policy.validate_mount(m)?;
        }

        let program = spec
            .cmd
            .first()
            .ok_or_else(|| SandboxError::PolicyViolation("empty cmd".into()))?
            .clone();

        let mut command = Command::new(&program);
        command.args(&spec.cmd[1..]);
        command.env_clear();
        for (k, v) in &spec.env {
            command.env(k, v);
        }
        command.current_dir(&spec.workdir);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;

        if let Some(bytes) = spec.stdin
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(&bytes).await?;
            stdin.shutdown().await?;
        }

        let handle = ExecHandle::new();
        let job = Job {
            child,
            started: Instant::now(),
            timeout: spec.timeout,
        };
        self.jobs.lock().await.insert(handle, job);
        Ok(handle)
    }

    async fn wait(&self, handle: ExecHandle) -> Result<ExecOutput> {
        let job = self
            .jobs
            .lock()
            .await
            .remove(&handle)
            .ok_or_else(|| SandboxError::NotAvailable("unknown exec handle".into()))?;

        let timeout_dur = job.timeout;
        let started = job.started;

        let wait_fut = async { job.child.wait_with_output().await };
        let output = match timeout(timeout_dur, wait_fut).await {
            Ok(res) => res?,
            Err(_) => {
                // Timeout — best-effort kill. `wait_with_output` has already
                // consumed the child, so we cannot kill it from here; on
                // timeout the OS process will continue until it terminates
                // on its own. This is acceptable for dev mode; production
                // must use `DockerSandbox`.
                return Err(SandboxError::Timeout);
            }
        };

        Ok(ExecOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            wall_time: started.elapsed(),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        let mut jobs = self.jobs.lock().await;
        for (_, mut job) in jobs.drain() {
            let _ = job.child.start_kill();
        }
        Ok(())
    }

    fn runtime(&self) -> SandboxRuntime {
        SandboxRuntime::Host
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::time::Duration;

    #[tokio::test]
    async fn echo_hello() {
        let sandbox = HostSandbox::new(SandboxPolicy::default());
        let spec = ExecSpec {
            cmd: vec!["echo".into(), "hello".into()],
            env: BTreeMap::new(),
            workdir: PathBuf::from("/"),
            stdin: None,
            mounts: vec![],
            timeout: Duration::from_secs(5),
        };
        let handle = sandbox.spawn(spec).await.expect("spawn");
        let out = sandbox.wait(handle).await.expect("wait");
        assert_eq!(out.exit_code, 0);
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello");
    }
}
