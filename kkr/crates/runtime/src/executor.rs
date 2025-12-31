use kkr_core::{Id, Output, Status, Task};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::{debug, info, warn};

use crate::queue::TaskQueue;

const DEFAULT_CONCURRENCY: usize = 4;
const DEFAULT_QUEUE_CAPACITY: usize = 1000;

pub type TaskFuture = Pin<Box<dyn Future<Output = Output> + Send>>;
pub type TaskHandler = Box<dyn Fn(Task) -> TaskFuture + Send + Sync>;

#[derive(Clone)]
pub struct ExecutorConfig {
    pub concurrency: usize,
    pub queue_capacity: usize,
    pub shutdown_timeout_secs: u64,
}

impl ExecutorConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        debug_assert!(n > 0, "concurrency must be positive");
        self.concurrency = n;
        self
    }

    pub fn queue_capacity(mut self, n: usize) -> Self {
        debug_assert!(n > 0, "queue_capacity must be positive");
        self.queue_capacity = n;
        self
    }

    pub fn shutdown_timeout(mut self, secs: u64) -> Self {
        debug_assert!(secs > 0, "shutdown_timeout must be positive");
        self.shutdown_timeout_secs = secs;
        self
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            concurrency: DEFAULT_CONCURRENCY,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            shutdown_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskStatus {
    pub id: Id,
    pub status: Status,
    pub output: Option<Output>,
    pub submitted_at: std::time::Instant,
    pub completed_at: Option<std::time::Instant>,
}

pub struct Executor {
    config: ExecutorConfig,
    queue: TaskQueue,
    semaphore: Arc<Semaphore>,
    statuses: Arc<RwLock<HashMap<Id, TaskStatus>>>,
    active_tasks: Arc<Mutex<usize>>,
}

impl Executor {
    pub fn new(config: ExecutorConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.concurrency));
        let queue = TaskQueue::new(config.queue_capacity);

        Self {
            config,
            queue,
            semaphore,
            statuses: Arc::new(RwLock::new(HashMap::new())),
            active_tasks: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn submit(&self, task: Task) -> Result<Id, ExecutorError> {
        let id = task.id;

        {
            let mut statuses = self.statuses.write().await;
            statuses.insert(
                id,
                TaskStatus {
                    id,
                    status: Status::Pending,
                    output: None,
                    submitted_at: std::time::Instant::now(),
                    completed_at: None,
                },
            );
        }

        self.queue
            .push(task)
            .await
            .map_err(|_| ExecutorError::QueueFull)?;

        debug!(task_id = %id, "Task submitted");
        Ok(id)
    }

    pub async fn execute<F>(&self, handler: F) -> Result<(), ExecutorError>
    where
        F: Fn(Task) -> TaskFuture + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);

        loop {
            let Some(mut entry) = self.queue.pop().await else {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            };

            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let handler = handler.clone();
            let statuses = self.statuses.clone();
            let active = self.active_tasks.clone();

            entry.status = Status::Running;
            entry.started_at = Some(std::time::Instant::now());

            {
                let mut statuses = statuses.write().await;
                if let Some(status) = statuses.get_mut(&entry.task.id) {
                    status.status = Status::Running;
                }
            }

            {
                let mut count = active.lock().await;
                *count += 1;
            }

            let task_id = entry.task.id;
            debug!(task_id = %task_id, "Task started");

            tokio::spawn(async move {
                let output = handler(entry.task).await;

                {
                    let mut statuses = statuses.write().await;
                    if let Some(status) = statuses.get_mut(&task_id) {
                        status.status = output.status;
                        status.output = Some(output.clone());
                        status.completed_at = Some(std::time::Instant::now());
                    }
                }

                {
                    let mut count = active.lock().await;
                    *count = count.saturating_sub(1);
                }

                if output.is_success() {
                    info!(task_id = %task_id, "Task completed successfully");
                } else {
                    warn!(task_id = %task_id, error = ?output.error, "Task failed");
                }

                drop(permit);
            });
        }
    }

    pub async fn run_one<F>(&self, handler: F) -> Option<Output>
    where
        F: FnOnce(Task) -> TaskFuture + Send + 'static,
    {
        let Some(mut entry) = self.queue.pop().await else {
            return None;
        };

        let _permit = self.semaphore.clone().acquire_owned().await.unwrap();

        entry.status = Status::Running;
        entry.started_at = Some(std::time::Instant::now());

        let task_id = entry.task.id;
        let output = handler(entry.task).await;

        {
            let mut statuses = self.statuses.write().await;
            if let Some(status) = statuses.get_mut(&task_id) {
                status.status = output.status;
                status.output = Some(output.clone());
                status.completed_at = Some(std::time::Instant::now());
            }
        }

        Some(output)
    }

    pub async fn status(&self, id: Id) -> Option<TaskStatus> {
        self.statuses.read().await.get(&id).cloned()
    }

    pub async fn output(&self, id: Id) -> Option<Output> {
        self.statuses.read().await.get(&id).and_then(|s| s.output.clone())
    }

    pub async fn cancel(&self, id: Id) -> bool {
        if self.queue.remove(id).await {
            let mut statuses = self.statuses.write().await;
            if let Some(status) = statuses.get_mut(&id) {
                status.status = Status::Cancelled;
                status.completed_at = Some(std::time::Instant::now());
            }
            true
        } else {
            false
        }
    }

    pub async fn pending_count(&self) -> usize {
        self.queue.len().await
    }

    pub async fn active_count(&self) -> usize {
        *self.active_tasks.lock().await
    }

    pub fn concurrency(&self) -> usize {
        self.config.concurrency
    }

    pub async fn clear_completed(&self) {
        let mut statuses = self.statuses.write().await;
        statuses.retain(|_, s| !s.status.is_terminal());
    }

    pub async fn all_statuses(&self) -> Vec<TaskStatus> {
        self.statuses.read().await.values().cloned().collect()
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new(ExecutorConfig::default())
    }
}

#[derive(Clone)]
pub struct ExecutorHandle {
    executor: Arc<Executor>,
}

impl ExecutorHandle {
    pub fn new(executor: Executor) -> Self {
        Self {
            executor: Arc::new(executor),
        }
    }

    pub async fn submit(&self, task: Task) -> Result<Id, ExecutorError> {
        self.executor.submit(task).await
    }

    pub async fn status(&self, id: Id) -> Option<TaskStatus> {
        self.executor.status(id).await
    }

    pub async fn output(&self, id: Id) -> Option<Output> {
        self.executor.output(id).await
    }

    pub async fn cancel(&self, id: Id) -> bool {
        self.executor.cancel(id).await
    }

    pub async fn pending_count(&self) -> usize {
        self.executor.pending_count().await
    }

    pub async fn active_count(&self) -> usize {
        self.executor.active_count().await
    }

    pub fn concurrency(&self) -> usize {
        self.executor.concurrency()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    QueueFull,
    Shutdown,
    TaskNotFound,
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorError::QueueFull => write!(f, "Task queue is full"),
            ExecutorError::Shutdown => write!(f, "Executor is shutting down"),
            ExecutorError::TaskNotFound => write!(f, "Task not found"),
        }
    }
}

impl std::error::Error for ExecutorError {}
