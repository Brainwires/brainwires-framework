//! Cron / scheduled task runner for BrainClaw.
//!
//! # Overview
//!
//! Cron jobs are persisted as JSON files under `~/.brainclaw/cron/`.  At
//! startup the `CronStore` loads all jobs; the `CronRunner` background task
//! polls every 30 seconds and dispatches a synthetic `ChannelMessage` to the
//! `AgentInboundHandler` whenever a job is due.
//!
//! # Cron expression format
//!
//! Standard 5-field cron: `min hour day month weekday`
//! e.g. `0 9 * * *` = every day at 09:00 UTC.
//!
//! The `cron` crate (0.12) is used for parsing and next-run calculation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use brainwires_channels::message::{ChannelMessage, MessageContent, MessageId};
use brainwires_channels::ConversationId;
use brainwires_gateway::channel_registry::ChannelRegistry;
use brainwires_gateway::AgentInboundHandler;
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// A single scheduled cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Unique job identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// 5-field cron expression (min hour day month weekday).
    pub schedule: String,
    /// Prompt text that will be sent to the agent.
    pub prompt: String,
    /// Target platform (e.g. "discord", "telegram", "webchat").
    pub target_platform: String,
    /// Target channel ID within the platform (room, channel, DM peer, etc.).
    pub target_channel_id: String,
    /// Target user ID used to look up or create an agent session.
    pub target_user_id: String,
    /// Whether the job is active.
    pub enabled: bool,
    /// UTC timestamp of the last execution (None if never run).
    pub last_run: Option<DateTime<Utc>>,
}

impl CronJob {
    /// Parse this job's schedule and return the next scheduled run after `after`.
    pub fn next_run_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Prepend a wildcard seconds field so the cron crate gets a 6-field expression.
        let expr = format!("0 {}", self.schedule);
        let sched = Schedule::from_str(&expr).ok()?;
        sched.after(&after).next()
    }
}

/// Persistent store for cron jobs backed by JSON files.
pub struct CronStore {
    dir: PathBuf,
    jobs: RwLock<HashMap<Uuid, CronJob>>,
}

impl CronStore {
    /// Open (or create) the store at `dir`.
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create cron dir: {}", dir.display()))?;

        let mut jobs = HashMap::new();
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("Failed to read cron dir: {}", dir.display()))?
        {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                match std::fs::read_to_string(&path)
                    .and_then(|s| Ok(serde_json::from_str::<CronJob>(&s)?))
                {
                    Ok(job) => {
                        jobs.insert(job.id, job);
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "Failed to load cron job");
                    }
                }
            }
        }

        tracing::info!(count = jobs.len(), path = %dir.display(), "Cron jobs loaded");

        Ok(Self {
            dir,
            jobs: RwLock::new(jobs),
        })
    }

    /// Return a snapshot of all jobs.
    pub async fn list(&self) -> Vec<CronJob> {
        self.jobs.read().await.values().cloned().collect()
    }

    /// Get a single job by ID.
    pub async fn get(&self, id: Uuid) -> Option<CronJob> {
        self.jobs.read().await.get(&id).cloned()
    }

    /// Add or replace a job and persist it to disk.
    pub async fn upsert(&self, job: CronJob) -> Result<()> {
        let path = self.job_path(job.id);
        let json = serde_json::to_string_pretty(&job)
            .context("Failed to serialize cron job")?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write cron job: {}", path.display()))?;
        self.jobs.write().await.insert(job.id, job);
        Ok(())
    }

    /// Delete a job and remove its file.
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let removed = self.jobs.write().await.remove(&id).is_some();
        if removed {
            let path = self.job_path(id);
            if path.exists() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("Failed to delete cron job file: {}", path.display()))?;
            }
        }
        Ok(removed)
    }

    /// Update `last_run` for a job and persist the change.
    pub async fn record_run(&self, id: Uuid, at: DateTime<Utc>) -> Result<()> {
        if let Some(job) = self.jobs.write().await.get_mut(&id) {
            job.last_run = Some(at);
            let path = self.job_path(id);
            let json = serde_json::to_string_pretty(job)
                .context("Failed to serialize cron job")?;
            std::fs::write(&path, json)
                .with_context(|| format!("Failed to write cron job: {}", path.display()))?;
        }
        Ok(())
    }

    fn job_path(&self, id: Uuid) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }
}

/// Background task that polls cron jobs and fires them when due.
pub struct CronRunner {
    store: Arc<CronStore>,
    handler: Arc<AgentInboundHandler>,
    channels: Arc<ChannelRegistry>,
}

impl CronRunner {
    pub fn new(
        store: Arc<CronStore>,
        handler: Arc<AgentInboundHandler>,
        channels: Arc<ChannelRegistry>,
    ) -> Self {
        Self { store, handler, channels }
    }

    /// Start the cron runner as a background tokio task.
    ///
    /// Polls every 30 seconds, fires any jobs whose next scheduled time has
    /// passed since their `last_run`.
    pub fn spawn(self: Arc<Self>) {
        tokio::spawn(async move {
            tracing::info!("Cron runner started");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                if let Err(e) = self.tick().await {
                    tracing::error!(error = %e, "Cron tick error");
                }
            }
        });
    }

    async fn tick(&self) -> Result<()> {
        let now = Utc::now();
        let jobs = self.store.list().await;

        for job in jobs {
            if !job.enabled {
                continue;
            }

            let since = job.last_run.unwrap_or_else(|| now - chrono::Duration::days(1));
            let Some(next) = job.next_run_after(since) else {
                tracing::warn!(job = %job.name, "Could not compute next run; skipping");
                continue;
            };

            if next <= now {
                self.fire_job(&job, now).await;
            }
        }

        Ok(())
    }

    async fn fire_job(&self, job: &CronJob, now: DateTime<Utc>) {
        tracing::info!(job = %job.name, schedule = %job.schedule, "Firing cron job");

        // Find a connected channel adapter for the target platform.
        let channel_ids = self.channels.find_by_type(&job.target_platform);
        let channel_id = match channel_ids.first() {
            Some(id) => *id,
            None => {
                tracing::warn!(
                    job = %job.name,
                    platform = %job.target_platform,
                    "No connected channel adapter for cron job; using nil UUID (response will be dropped)"
                );
                Uuid::nil()
            }
        };

        let msg = ChannelMessage {
            id: MessageId::new(Uuid::new_v4().to_string()),
            conversation: ConversationId {
                platform: job.target_platform.clone(),
                channel_id: job.target_channel_id.clone(),
                server_id: None,
            },
            author: job.target_user_id.clone(),
            content: MessageContent::Text(job.prompt.clone()),
            thread_id: None,
            reply_to: None,
            timestamp: now,
            attachments: vec![],
            metadata: HashMap::new(),
        };

        if let Err(e) = self.handler.dispatch_message(channel_id, msg).await {
            tracing::error!(job = %job.name, error = %e, "Cron job dispatch failed");
        } else {
            if let Err(e) = self.store.record_run(job.id, now).await {
                tracing::warn!(job = %job.name, error = %e, "Failed to record cron run");
            }
        }
    }
}
