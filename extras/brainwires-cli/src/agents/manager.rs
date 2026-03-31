use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::providers::Provider;
use crate::types::agent::{AgentContext, AgentResponse, PermissionMode, Task};

use super::{
    communication::{AgentMessage, CommunicationHub},
    orchestrator::OrchestratorAgent,
    task_queue::{TaskPriority, TaskQueue},
    worker::WorkerAgent,
};

/// Agent manager - coordinates orchestrator and worker agents
pub struct AgentManager {
    orchestrator: Arc<RwLock<OrchestratorAgent>>,
    workers: Arc<RwLock<HashMap<String, Arc<WorkerAgent>>>>,
    task_queue: Arc<TaskQueue>,
    communication_hub: Arc<CommunicationHub>,
    max_workers: usize,
}

impl AgentManager {
    /// Create a new agent manager
    pub async fn new(
        provider: Arc<dyn Provider>,
        permission_mode: PermissionMode,
        max_workers: usize,
    ) -> Result<Self> {
        let orchestrator = Arc::new(RwLock::new(OrchestratorAgent::new(
            provider.clone(),
            permission_mode,
        )));
        let task_queue = Arc::new(TaskQueue::new(1000)); // Max 1000 queued tasks
        let communication_hub = Arc::new(CommunicationHub::new());

        // Register orchestrator with communication hub
        communication_hub
            .register_agent("orchestrator".to_string())
            .await?;

        Ok(Self {
            orchestrator,
            workers: Arc::new(RwLock::new(HashMap::new())),
            task_queue,
            communication_hub,
            max_workers,
        })
    }

    /// Execute a task using the orchestrator
    pub async fn execute_task(
        &self,
        description: &str,
        context: &mut AgentContext,
    ) -> Result<AgentResponse> {
        self.orchestrator
            .write()
            .await
            .execute(description, context)
            .await
    }

    /// Queue a task for later execution
    pub async fn queue_task(&self, task: Task, priority: TaskPriority) -> Result<()> {
        self.task_queue.enqueue(task, priority).await
    }

    /// Create and register a new worker agent
    pub async fn spawn_worker(
        &self,
        worker_id: String,
        provider: Arc<dyn Provider>,
        permission_mode: PermissionMode,
    ) -> Result<String> {
        let workers_count = self.workers.read().await.len();
        if workers_count >= self.max_workers {
            anyhow::bail!("Maximum number of workers ({}) reached", self.max_workers);
        }

        let worker = Arc::new(WorkerAgent::new(provider, permission_mode));
        self.workers.write().await.insert(worker_id.clone(), worker);

        // Register worker with communication hub
        self.communication_hub
            .register_agent(worker_id.clone())
            .await?;

        Ok(worker_id)
    }

    /// Remove a worker agent
    pub async fn remove_worker(&self, worker_id: &str) -> Result<()> {
        self.workers.write().await.remove(worker_id);
        self.communication_hub.unregister_agent(worker_id).await?;
        Ok(())
    }

    /// Delegate a task to a specific worker
    pub async fn delegate_to_worker(
        &self,
        worker_id: &str,
        task_description: &str,
        context: &mut AgentContext,
    ) -> Result<AgentResponse> {
        let workers = self.workers.read().await;
        let worker = workers
            .get(worker_id)
            .ok_or_else(|| anyhow::anyhow!("Worker {} not found", worker_id))?;

        worker.execute(task_description, context).await
    }

    /// Delegate a task to any available worker
    pub async fn delegate_to_any_worker(
        &self,
        task_description: &str,
        context: &mut AgentContext,
    ) -> Result<AgentResponse> {
        let workers = self.workers.read().await;
        if workers.is_empty() {
            anyhow::bail!("No workers available");
        }

        // Get first available worker
        let (worker_id, worker) = workers.iter().next().unwrap();
        let worker_id = worker_id.clone();
        let worker = worker.clone();
        drop(workers);

        // Send task request message
        self.communication_hub
            .send_message(
                "orchestrator".to_string(),
                worker_id.clone(),
                AgentMessage::TaskRequest {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    description: task_description.to_string(),
                    priority: 5,
                },
            )
            .await?;

        worker.execute(task_description, context).await
    }

    /// Process tasks from the queue
    pub async fn process_queue(&self, context: &mut AgentContext) -> Result<Vec<AgentResponse>> {
        let mut responses = Vec::new();

        while let Some(queued_task) = self.task_queue.dequeue().await {
            // Try to delegate to a worker, or use orchestrator if no workers
            let response = if self.workers.read().await.is_empty() {
                self.orchestrator
                    .write()
                    .await
                    .execute(&queued_task.task.description, context)
                    .await?
            } else {
                self.delegate_to_any_worker(&queued_task.task.description, context)
                    .await?
            };

            responses.push(response);
        }

        Ok(responses)
    }

    /// Get the task queue
    pub fn task_queue(&self) -> Arc<TaskQueue> {
        self.task_queue.clone()
    }

    /// Get the communication hub
    pub fn communication_hub(&self) -> Arc<CommunicationHub> {
        self.communication_hub.clone()
    }

    /// Get the number of active workers
    pub async fn worker_count(&self) -> usize {
        self.workers.read().await.len()
    }

    /// Get list of worker IDs
    pub async fn list_workers(&self) -> Vec<String> {
        self.workers.read().await.keys().cloned().collect()
    }

    /// Send a broadcast message to all workers
    pub async fn broadcast_to_workers(&self, message: AgentMessage) -> Result<()> {
        self.communication_hub
            .broadcast("orchestrator".to_string(), message)
            .await
    }

    /// Get queue statistics
    pub async fn queue_stats(&self) -> (usize, (usize, usize, usize, usize)) {
        let total = self.task_queue.size().await;
        let by_priority = self.task_queue.size_by_priority().await;
        (total, by_priority)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::constants::DEFAULT_BACKEND_URL;
    use crate::providers::BrainwiresHttpProvider;

    async fn create_test_manager() -> AgentManager {
        let provider = Arc::new(BrainwiresHttpProvider::new(
            "bw_test_12345678901234567890123456789012".to_string(),
            DEFAULT_BACKEND_URL.to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ));
        AgentManager::new(provider, PermissionMode::Auto, 5)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = create_test_manager().await;
        assert_eq!(manager.worker_count().await, 0);
    }

    #[tokio::test]
    async fn test_spawn_worker() {
        let manager = create_test_manager().await;
        let provider = Arc::new(BrainwiresHttpProvider::new(
            "bw_test_12345678901234567890123456789012".to_string(),
            DEFAULT_BACKEND_URL.to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ));

        let worker_id = manager
            .spawn_worker("worker-1".to_string(), provider, PermissionMode::Auto)
            .await
            .unwrap();

        assert_eq!(worker_id, "worker-1");
        assert_eq!(manager.worker_count().await, 1);
        assert!(
            manager
                .list_workers()
                .await
                .contains(&"worker-1".to_string())
        );
    }

    #[tokio::test]
    async fn test_max_workers_limit() {
        let manager = create_test_manager().await;
        let provider = Arc::new(BrainwiresHttpProvider::new(
            "bw_test_12345678901234567890123456789012".to_string(),
            DEFAULT_BACKEND_URL.to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ));

        // Spawn max workers (5)
        for i in 0..5 {
            manager
                .spawn_worker(
                    format!("worker-{}", i),
                    provider.clone(),
                    PermissionMode::Auto,
                )
                .await
                .unwrap();
        }

        // Try to spawn one more - should fail
        let result = manager
            .spawn_worker("worker-6".to_string(), provider, PermissionMode::Auto)
            .await;

        assert!(result.is_err());
        assert_eq!(manager.worker_count().await, 5);
    }

    #[tokio::test]
    async fn test_remove_worker() {
        let manager = create_test_manager().await;
        let provider = Arc::new(BrainwiresHttpProvider::new(
            "bw_test_12345678901234567890123456789012".to_string(),
            DEFAULT_BACKEND_URL.to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ));

        manager
            .spawn_worker("worker-1".to_string(), provider, PermissionMode::Auto)
            .await
            .unwrap();

        assert_eq!(manager.worker_count().await, 1);

        manager.remove_worker("worker-1").await.unwrap();
        assert_eq!(manager.worker_count().await, 0);
    }

    #[tokio::test]
    async fn test_queue_task() {
        let manager = create_test_manager().await;
        let task = Task::new("task-1".to_string(), "Test task".to_string());

        manager
            .queue_task(task, TaskPriority::Normal)
            .await
            .unwrap();

        let (total, _) = manager.queue_stats().await;
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let manager = create_test_manager().await;

        manager
            .queue_task(
                Task::new("1".to_string(), "Urgent task".to_string()),
                TaskPriority::Urgent,
            )
            .await
            .unwrap();
        manager
            .queue_task(
                Task::new("2".to_string(), "High task".to_string()),
                TaskPriority::High,
            )
            .await
            .unwrap();
        manager
            .queue_task(
                Task::new("3".to_string(), "Normal task".to_string()),
                TaskPriority::Normal,
            )
            .await
            .unwrap();

        let (total, (urgent, high, normal, low)) = manager.queue_stats().await;
        assert_eq!(total, 3);
        assert_eq!(urgent, 1);
        assert_eq!(high, 1);
        assert_eq!(normal, 1);
        assert_eq!(low, 0);
    }

    #[tokio::test]
    async fn test_list_workers_empty() {
        let manager = create_test_manager().await;
        let workers = manager.list_workers().await;
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn test_list_workers() {
        let manager = create_test_manager().await;
        let provider = Arc::new(BrainwiresHttpProvider::new(
            "bw_test_12345678901234567890123456789012".to_string(),
            DEFAULT_BACKEND_URL.to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
        ));

        manager
            .spawn_worker(
                "worker-1".to_string(),
                provider.clone(),
                PermissionMode::Auto,
            )
            .await
            .unwrap();
        manager
            .spawn_worker("worker-2".to_string(), provider, PermissionMode::Auto)
            .await
            .unwrap();

        let workers = manager.list_workers().await;
        assert_eq!(workers.len(), 2);
        assert!(workers.contains(&"worker-1".to_string()));
        assert!(workers.contains(&"worker-2".to_string()));
    }

    #[tokio::test]
    async fn test_task_queue_accessor() {
        let manager = create_test_manager().await;
        let queue = manager.task_queue();
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_communication_hub_accessor() {
        let manager = create_test_manager().await;
        let _hub = manager.communication_hub();
        // Just verify we can get the hub without panic
    }

    #[tokio::test]
    async fn test_delegate_to_nonexistent_worker() {
        let manager = create_test_manager().await;
        let mut context = AgentContext::default();

        let result = manager
            .delegate_to_worker("nonexistent-worker", "Test task", &mut context)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_delegate_to_any_worker_no_workers() {
        let manager = create_test_manager().await;
        let mut context = AgentContext::default();

        let result = manager
            .delegate_to_any_worker("Test task", &mut context)
            .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No workers available")
        );
    }
}
