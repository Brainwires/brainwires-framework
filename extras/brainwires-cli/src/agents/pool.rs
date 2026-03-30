//! Agent Pool - Manages a pool of background task agents
//!
//! Provides lifecycle management for task agents including spawning,
//! monitoring, and cleanup.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::providers::Provider;
use crate::types::agent::{AgentContext, Task};

use super::communication::CommunicationHub;
use super::file_locks::FileLockManager;
use super::task_agent::{spawn_task_agent, TaskAgent, TaskAgentConfig, TaskAgentResult, TaskAgentStatus};

/// Handle for a running agent
struct AgentHandle {
    /// The agent instance
    agent: Arc<TaskAgent>,
    /// Join handle for the background task
    join_handle: JoinHandle<Result<TaskAgentResult>>,
}

/// Manages a pool of background task agents
pub struct AgentPool {
    /// Maximum number of concurrent agents
    max_agents: usize,
    /// Running agents
    agents: Arc<RwLock<HashMap<String, AgentHandle>>>,
    /// Communication hub for all agents
    communication_hub: Arc<CommunicationHub>,
    /// Shared file lock manager
    file_lock_manager: Arc<FileLockManager>,
    /// AI provider factory (creates providers for new agents)
    provider: Arc<dyn Provider>,
}

impl AgentPool {
    /// Create a new agent pool
    pub fn new(
        max_agents: usize,
        provider: Arc<dyn Provider>,
        communication_hub: Arc<CommunicationHub>,
        file_lock_manager: Arc<FileLockManager>,
    ) -> Self {
        Self {
            max_agents,
            agents: Arc::new(RwLock::new(HashMap::new())),
            communication_hub,
            file_lock_manager,
            provider,
        }
    }

    /// Spawn a new task agent
    ///
    /// Returns the agent ID if successful.
    pub async fn spawn_agent(
        &self,
        task: Task,
        context: AgentContext,
        config: Option<TaskAgentConfig>,
    ) -> Result<String> {
        let agents = self.agents.read().await;
        if agents.len() >= self.max_agents {
            return Err(anyhow!(
                "Agent pool is full ({}/{})",
                agents.len(),
                self.max_agents
            ));
        }
        drop(agents);

        let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
        let config = config.unwrap_or_default();

        let agent = Arc::new(TaskAgent::new(
            agent_id.clone(),
            task,
            Arc::clone(&self.provider),
            Arc::clone(&self.communication_hub),
            Arc::clone(&self.file_lock_manager),
            context,
            config,
        ));

        let handle = spawn_task_agent(Arc::clone(&agent));

        let mut agents = self.agents.write().await;
        agents.insert(
            agent_id.clone(),
            AgentHandle {
                agent,
                join_handle: handle,
            },
        );

        Ok(agent_id)
    }

    /// Get the status of an agent
    pub async fn get_status(&self, agent_id: &str) -> Option<TaskAgentStatus> {
        let agents = self.agents.read().await;
        if let Some(handle) = agents.get(agent_id) {
            Some(handle.agent.status().await)
        } else {
            None
        }
    }

    /// Get the task for an agent
    pub async fn get_task(&self, agent_id: &str) -> Option<Task> {
        let agents = self.agents.read().await;
        if let Some(handle) = agents.get(agent_id) {
            Some(handle.agent.task().await)
        } else {
            None
        }
    }

    /// Stop an agent
    pub async fn stop_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(handle) = agents.remove(agent_id) {
            // Abort the task
            handle.join_handle.abort();
            // Release all locks held by this agent
            self.file_lock_manager.release_all_locks(agent_id).await;
            Ok(())
        } else {
            Err(anyhow!("Agent {} not found", agent_id))
        }
    }

    /// Wait for an agent to complete and get its result
    pub async fn await_completion(&self, agent_id: &str) -> Result<TaskAgentResult> {
        // First get the join handle
        let handle = {
            let mut agents = self.agents.write().await;
            agents.remove(agent_id)
        };

        if let Some(handle) = handle {
            match handle.join_handle.await {
                Ok(result) => result,
                Err(e) => Err(anyhow!("Agent task panicked: {}", e)),
            }
        } else {
            Err(anyhow!("Agent {} not found", agent_id))
        }
    }

    /// List all active agents with their status
    pub async fn list_active(&self) -> Vec<(String, TaskAgentStatus)> {
        let agents = self.agents.read().await;
        let mut result = Vec::with_capacity(agents.len());

        for (id, handle) in agents.iter() {
            let status = handle.agent.status().await;
            result.push((id.clone(), status));
        }

        result
    }

    /// Get the number of active agents
    pub async fn active_count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Check if an agent is running
    pub async fn is_running(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;
        if let Some(handle) = agents.get(agent_id) {
            !handle.join_handle.is_finished()
        } else {
            false
        }
    }

    /// Cleanup completed agents and return their results
    pub async fn cleanup_completed(&self) -> Vec<(String, Result<TaskAgentResult>)> {
        let mut completed = Vec::new();
        let mut to_remove = Vec::new();

        // First identify completed agents
        {
            let agents = self.agents.read().await;
            for (id, handle) in agents.iter() {
                if handle.join_handle.is_finished() {
                    to_remove.push(id.clone());
                }
            }
        }

        // Then remove and collect results
        {
            let mut agents = self.agents.write().await;
            for id in to_remove {
                if let Some(handle) = agents.remove(&id) {
                    let result = match handle.join_handle.await {
                        Ok(r) => r,
                        Err(e) => Err(anyhow!("Agent task panicked: {}", e)),
                    };
                    completed.push((id, result));
                }
            }
        }

        completed
    }

    /// Wait for all agents to complete
    pub async fn await_all(&self) -> Vec<(String, Result<TaskAgentResult>)> {
        let mut results = Vec::new();

        // Get all agent IDs
        let agent_ids: Vec<String> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        // Await each one
        for id in agent_ids {
            let result = self.await_completion(&id).await;
            results.push((id, result));
        }

        results
    }

    /// Shutdown the pool, stopping all agents
    pub async fn shutdown(&self) {
        let mut agents = self.agents.write().await;
        for (agent_id, handle) in agents.drain() {
            handle.join_handle.abort();
            self.file_lock_manager.release_all_locks(&agent_id).await;
        }
    }

    /// Get pool statistics
    pub async fn stats(&self) -> AgentPoolStats {
        let agents = self.agents.read().await;
        let mut running = 0;
        let mut completed = 0;
        let failed = 0;

        for (_, handle) in agents.iter() {
            if handle.join_handle.is_finished() {
                // We don't know if it succeeded or failed without awaiting
                completed += 1;
            } else {
                running += 1;
            }
        }

        AgentPoolStats {
            max_agents: self.max_agents,
            total_agents: agents.len(),
            running,
            completed,
            failed,
        }
    }

    /// Get the shared file lock manager
    pub fn file_lock_manager(&self) -> Arc<FileLockManager> {
        Arc::clone(&self.file_lock_manager)
    }

    /// Get the shared communication hub
    pub fn communication_hub(&self) -> Arc<CommunicationHub> {
        Arc::clone(&self.communication_hub)
    }
}

/// Statistics about the agent pool
#[derive(Debug, Clone)]
pub struct AgentPoolStats {
    /// Maximum number of agents allowed
    pub max_agents: usize,
    /// Total agents (running + pending completion)
    pub total_agents: usize,
    /// Currently running agents
    pub running: usize,
    /// Completed but not yet cleaned up
    pub completed: usize,
    /// Failed agents
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::message::{ChatResponse, Message, MessageContent, Role, StreamChunk, Usage};
    use crate::types::provider::ChatOptions;
    use crate::types::tool::Tool;
    use async_trait::async_trait;
    use futures::stream::BoxStream;

    /// Mock provider for testing
    struct MockProvider {
        response: ChatResponse,
    }

    impl MockProvider {
        fn new(text: &str) -> Self {
            Self {
                response: ChatResponse {
                    message: Message {
                        role: Role::Assistant,
                        content: MessageContent::Text(text.to_string()),
                        name: None,
                        metadata: None,
                    },
                    finish_reason: Some("stop".to_string()),
                    usage: Usage::default(),
                },
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: &[Message],
            _tools: Option<&[Tool]>,
            _options: &ChatOptions,
        ) -> Result<ChatResponse> {
            Ok(self.response.clone())
        }

        fn stream_chat<'a>(
            &'a self,
            _messages: &'a [Message],
            _tools: Option<&'a [Tool]>,
            _options: &'a ChatOptions,
        ) -> BoxStream<'a, Result<StreamChunk>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let provider = Arc::new(MockProvider::new("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(5, provider, hub, locks);
        assert_eq!(pool.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_spawn_agent() {
        let provider = Arc::new(MockProvider::new("Task completed"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(5, provider, hub, locks);
        let task = Task::new("task-1", "Test task");
        let context = AgentContext::default();

        let agent_id = pool.spawn_agent(task, context, None).await.unwrap();
        assert!(agent_id.starts_with("agent-"));
        assert_eq!(pool.active_count().await, 1);
    }

    #[tokio::test]
    async fn test_max_agents_limit() {
        let provider = Arc::new(MockProvider::new("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(2, provider, hub, locks);

        // Spawn two agents
        let task1 = Task::new("task-1", "Task 1");
        let task2 = Task::new("task-2", "Task 2");
        let task3 = Task::new("task-3", "Task 3");

        pool.spawn_agent(task1, AgentContext::default(), None)
            .await
            .unwrap();
        pool.spawn_agent(task2, AgentContext::default(), None)
            .await
            .unwrap();

        // Third should fail
        let result = pool.spawn_agent(task3, AgentContext::default(), None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("full"));
    }

    #[tokio::test]
    async fn test_await_completion() {
        let provider = Arc::new(MockProvider::new("Success"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(5, provider, hub, locks);
        let task = Task::new("task-1", "Test task");

        let config = TaskAgentConfig {
            validation_config: None,
            ..Default::default()
        };
        let agent_id = pool
            .spawn_agent(task, AgentContext::default(), Some(config))
            .await
            .unwrap();

        let result = pool.await_completion(&agent_id).await.unwrap();
        assert!(result.success);
        assert_eq!(result.task_id, "task-1");
    }

    #[tokio::test]
    async fn test_list_active() {
        let provider = Arc::new(MockProvider::new("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(5, provider, hub, locks);

        let task1 = Task::new("task-1", "Task 1");
        let task2 = Task::new("task-2", "Task 2");

        pool.spawn_agent(task1, AgentContext::default(), None)
            .await
            .unwrap();
        pool.spawn_agent(task2, AgentContext::default(), None)
            .await
            .unwrap();

        let active = pool.list_active().await;
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_stop_agent() {
        let provider = Arc::new(MockProvider::new("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(5, provider, hub, locks);
        let task = Task::new("task-1", "Test task");

        let agent_id = pool
            .spawn_agent(task, AgentContext::default(), None)
            .await
            .unwrap();

        pool.stop_agent(&agent_id).await.unwrap();
        assert_eq!(pool.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let provider = Arc::new(MockProvider::new("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(5, provider, hub, locks);

        let task1 = Task::new("task-1", "Task 1");
        let task2 = Task::new("task-2", "Task 2");

        pool.spawn_agent(task1, AgentContext::default(), None)
            .await
            .unwrap();
        pool.spawn_agent(task2, AgentContext::default(), None)
            .await
            .unwrap();

        pool.shutdown().await;
        assert_eq!(pool.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let provider = Arc::new(MockProvider::new("Done"));
        let hub = Arc::new(CommunicationHub::new());
        let locks = Arc::new(FileLockManager::new());

        let pool = AgentPool::new(10, provider, hub, locks);

        let stats = pool.stats().await;
        assert_eq!(stats.max_agents, 10);
        assert_eq!(stats.total_agents, 0);
    }
}
