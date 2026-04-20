//! Docker / Podman sandbox backed by [bollard].
//!
//! Containers are created with resource limits, a read-only root filesystem,
//! and no inherited host environment. Only mounts permitted by the
//! [`SandboxPolicy`] whitelist are forwarded to the container runtime.
//!
//! # Known limitations
//!
//! - [`NetworkPolicy::Limited`] is not yet implemented and currently returns
//!   [`SandboxError::NotAvailable`]. Per-host egress allowlists require an
//!   out-of-band firewall (iptables, nftables, cilium) that bollard does not
//!   expose directly.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bollard::Docker;
use bollard::container::{
    AttachContainerOptions, AttachContainerResults, Config, CreateContainerOptions, LogOutput,
    RemoveContainerOptions, StartContainerOptions,
};
use bollard::models::HostConfig;
use futures::StreamExt;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::error::{Result, SandboxError};
use crate::{
    ExecHandle, ExecOutput, ExecSpec, Mount, NetworkPolicy, Sandbox, SandboxPolicy, SandboxRuntime,
};

struct Job {
    container_id: String,
    started: Instant,
    timeout: Duration,
    attach: AttachContainerResults,
}

/// Sandbox backed by a Docker- or Podman-compatible daemon.
pub struct DockerSandbox {
    client: Arc<Docker>,
    policy: SandboxPolicy,
    jobs: Arc<Mutex<HashMap<ExecHandle, Job>>>,
}

impl DockerSandbox {
    /// Connect to the configured daemon. For [`SandboxRuntime::Podman`] the
    /// socket path is taken from the `PODMAN_SOCKET` environment variable and
    /// falls back to `unix:///run/podman/podman.sock`.
    pub fn connect(policy: SandboxPolicy) -> Result<Self> {
        let client = match policy.runtime {
            SandboxRuntime::Docker => Docker::connect_with_socket_defaults()?,
            SandboxRuntime::Podman => {
                let socket = std::env::var("PODMAN_SOCKET")
                    .unwrap_or_else(|_| "unix:///run/podman/podman.sock".to_string());
                Docker::connect_with_socket(&socket, 120, bollard::API_DEFAULT_VERSION)?
            }
            SandboxRuntime::Host => {
                return Err(SandboxError::NotAvailable(
                    "DockerSandbox cannot run SandboxRuntime::Host; use HostSandbox instead".into(),
                ));
            }
        };

        if matches!(policy.network, NetworkPolicy::Limited(_)) {
            return Err(SandboxError::NotAvailable(
                "Limited network policy not yet implemented; set network = \"None\" or \"Full\""
                    .into(),
            ));
        }

        Ok(Self {
            client: Arc::new(client),
            policy,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn build_host_config(&self, mounts: &[Mount]) -> HostConfig {
        let memory = self
            .policy
            .memory_limit_mb
            .map(|mb| (mb as i64).saturating_mul(1024 * 1024));
        let nano_cpus = self
            .policy
            .cpu_limit
            .map(|cores| (cores * 1_000_000_000f64) as i64);
        let pids_limit = self.policy.pid_limit.map(|n| n as i64);

        let binds: Vec<String> = mounts
            .iter()
            .map(|m| {
                format!(
                    "{}:{}:{}",
                    m.source.display(),
                    m.target.display(),
                    if m.read_only { "ro" } else { "rw" }
                )
            })
            .collect();

        let network_mode = match &self.policy.network {
            NetworkPolicy::None => Some("none".to_string()),
            NetworkPolicy::Full => Some("bridge".to_string()),
            NetworkPolicy::Limited(_) => Some("none".to_string()),
        };

        HostConfig {
            memory,
            nano_cpus,
            pids_limit,
            network_mode,
            binds: if binds.is_empty() { None } else { Some(binds) },
            readonly_rootfs: Some(self.policy.read_only_rootfs),
            auto_remove: Some(false),
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl Sandbox for DockerSandbox {
    async fn spawn(&self, spec: ExecSpec) -> Result<ExecHandle> {
        for m in &spec.mounts {
            self.policy.validate_mount(m)?;
        }

        let env: Vec<String> = spec.env.iter().map(|(k, v)| format!("{k}={v}")).collect();

        let host_config = self.build_host_config(&spec.mounts);

        let config: Config<String> = Config {
            image: Some(self.policy.image.clone()),
            cmd: Some(spec.cmd.clone()),
            env: Some(env),
            working_dir: Some(spec.workdir.display().to_string()),
            attach_stdin: Some(spec.stdin.is_some()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            open_stdin: Some(spec.stdin.is_some()),
            stdin_once: Some(spec.stdin.is_some()),
            tty: Some(false),
            host_config: Some(host_config),
            ..Default::default()
        };

        let handle = ExecHandle::new();
        let name = format!("brainwires-sandbox-{}", handle.as_uuid());

        let create_opts = CreateContainerOptions {
            name: name.clone(),
            platform: None,
        };

        let created = self
            .client
            .create_container(Some(create_opts), config)
            .await?;
        let container_id = created.id;

        let mut attach = self
            .client
            .attach_container(
                &container_id,
                Some(AttachContainerOptions::<String> {
                    stdin: Some(spec.stdin.is_some()),
                    stdout: Some(true),
                    stderr: Some(true),
                    stream: Some(true),
                    logs: Some(true),
                    detach_keys: None,
                }),
            )
            .await?;

        self.client
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await?;

        if let Some(bytes) = spec.stdin.as_ref() {
            use tokio::io::AsyncWriteExt;
            attach.input.write_all(bytes).await?;
            attach.input.shutdown().await?;
        }

        let job = Job {
            container_id,
            started: Instant::now(),
            timeout: spec.timeout,
            attach,
        };
        self.jobs.lock().await.insert(handle, job);
        Ok(handle)
    }

    async fn wait(&self, handle: ExecHandle) -> Result<ExecOutput> {
        let Job {
            container_id,
            started,
            timeout: timeout_dur,
            mut attach,
        } = self
            .jobs
            .lock()
            .await
            .remove(&handle)
            .ok_or_else(|| SandboxError::NotAvailable("unknown exec handle".into()))?;

        let collect_and_wait = async {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            while let Some(frame) = attach.output.next().await {
                let frame = frame.map_err(|e| SandboxError::Docker(e.to_string()))?;
                match frame {
                    LogOutput::StdOut { message } => stdout.extend_from_slice(&message),
                    LogOutput::StdErr { message } => stderr.extend_from_slice(&message),
                    LogOutput::Console { message } => stdout.extend_from_slice(&message),
                    LogOutput::StdIn { .. } => {}
                }
            }

            let mut wait_stream = self.client.wait_container(
                &container_id,
                None::<bollard::container::WaitContainerOptions<String>>,
            );
            let mut exit_code: i64 = 0;
            while let Some(ev) = wait_stream.next().await {
                match ev {
                    Ok(resp) => exit_code = resp.status_code,
                    Err(bollard::errors::Error::DockerContainerWaitError { code, .. }) => {
                        exit_code = code;
                    }
                    Err(e) => return Err(SandboxError::Docker(e.to_string())),
                }
            }

            Ok::<_, SandboxError>(ExecOutput {
                exit_code: exit_code as i32,
                stdout,
                stderr,
                wall_time: started.elapsed(),
            })
        };

        let result = match timeout(timeout_dur, collect_and_wait).await {
            Ok(res) => res,
            Err(_) => Err(SandboxError::Timeout),
        };

        let _ = self
            .client
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    v: true,
                    link: false,
                }),
            )
            .await;

        result
    }

    async fn shutdown(&self) -> Result<()> {
        let jobs: Vec<_> = {
            let mut guard = self.jobs.lock().await;
            guard.drain().collect()
        };
        for (_, job) in jobs {
            let _ = self
                .client
                .remove_container(
                    &job.container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        v: true,
                        link: false,
                    }),
                )
                .await;
        }
        Ok(())
    }

    fn runtime(&self) -> SandboxRuntime {
        self.policy.runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[tokio::test]
    #[ignore = "requires a live Docker daemon"]
    async fn echo_hello_in_docker() {
        let mut policy = SandboxPolicy::default();
        policy.image = "alpine:3".into();
        policy.network = NetworkPolicy::None;
        let sandbox = DockerSandbox::connect(policy).expect("connect");

        let spec = ExecSpec {
            cmd: vec!["echo".into(), "hello".into()],
            env: BTreeMap::new(),
            workdir: PathBuf::from("/"),
            stdin: None,
            mounts: vec![],
            timeout: Duration::from_secs(30),
        };

        let handle = sandbox.spawn(spec).await.expect("spawn");
        let out = sandbox.wait(handle).await.expect("wait");
        assert_eq!(out.exit_code, 0);
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello");
    }
}
