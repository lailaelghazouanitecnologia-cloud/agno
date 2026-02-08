use crate::types::{Id, Output, Status, Task};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
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
    shutdown: Arc<RwLock<bool>>,
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
            shutdown: Arc::new(RwLock::new(false)),
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
            // Check shutdown flag before processing next task
            if *self.shutdown.read().await {
                info!("Executor received shutdown signal, stopping execution loop");
                return Ok(());
            }

            let Some(mut entry) = self.queue.pop().await else {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
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

    /// Initiate graceful shutdown: no new tasks accepted, waits for active tasks.
    pub async fn shutdown(&self) {
        {
            let mut shutdown = self.shutdown.write().await;
            *shutdown = true;
        }
        info!("Executor shutting down, waiting for active tasks to complete");

        let timeout = std::time::Duration::from_secs(self.config.shutdown_timeout_secs);
        let start = std::time::Instant::now();

        loop {
            let active = *self.active_tasks.lock().await;
            if active == 0 {
                info!("All active tasks completed, shutdown successful");
                break;
            }
            if start.elapsed() >= timeout {
                warn!(
                    active_tasks = active,
                    "Shutdown timeout reached, {} tasks still running",
                    active
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    /// Check if the executor is shutting down.
    pub async fn is_shutdown(&self) -> bool {
        *self.shutdown.read().await
    }

    /// Submit a task, rejecting if shutdown is in progress.
    pub async fn submit_checked(&self, task: Task) -> Result<Id, ExecutorError> {
        if *self.shutdown.read().await {
            return Err(ExecutorError::Shutdown);
        }
        self.submit(task).await
    }

    /// Submit a task with retry on failure. Retries with exponential backoff.
    pub async fn submit_with_retry<F>(
        &self,
        task: Task,
        handler: F,
        max_retries: usize,
        base_delay_ms: u64,
    ) -> Output
    where
        F: Fn(Task) -> TaskFuture + Send + Sync + 'static,
    {
        let task_id = task.id;
        let mut current_task = task;
        let mut last_output = None;

        for attempt in 0..=max_retries {
            let _permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let output = handler(current_task.clone()).await;

            if output.is_success() || attempt == max_retries {
                // Update status
                {
                    let mut statuses = self.statuses.write().await;
                    statuses.insert(
                        task_id,
                        TaskStatus {
                            id: task_id,
                            status: output.status,
                            output: Some(output.clone()),
                            submitted_at: std::time::Instant::now(),
                            completed_at: Some(std::time::Instant::now()),
                        },
                    );
                }
                return output;
            }

            last_output = Some(output);

            let delay = base_delay_ms * 2u64.pow(attempt as u32);
            debug!(
                task_id = %task_id,
                attempt = attempt + 1,
                delay_ms = delay,
                "Retrying failed task"
            );
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

            current_task = current_task.clone();
        }

        last_output.unwrap_or_else(|| Output::failure(task_id, "All retries exhausted".to_string()))
    }

    /// Wait until all pending and active tasks are complete (or timeout).
    pub async fn wait_for_completion(&self, timeout_secs: u64) -> bool {
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        loop {
            let pending = self.queue.len().await;
            let active = *self.active_tasks.lock().await;

            if pending == 0 && active == 0 {
                return true;
            }
            if start.elapsed() >= timeout {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
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

/// Circuit breaker for protecting against cascading failures.
/// States: Closed (normal) -> Open (failing) -> HalfOpen (testing recovery)
#[derive(Debug)]
pub struct CircuitBreaker {
    state: AtomicU8, // 0=Closed, 1=Open, 2=HalfOpen
    failure_count: AtomicU64,
    success_count: AtomicU64,
    failure_threshold: u64,
    success_threshold: u64,
    last_failure: Arc<RwLock<Option<std::time::Instant>>>,
    reset_timeout: Duration,
}

const CB_CLOSED: u8 = 0;
const CB_OPEN: u8 = 1;
const CB_HALF_OPEN: u8 = 2;

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, success_threshold: u64, reset_timeout: Duration) -> Self {
        Self {
            state: AtomicU8::new(CB_CLOSED),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            failure_threshold,
            success_threshold,
            last_failure: Arc::new(RwLock::new(None)),
            reset_timeout,
        }
    }

    pub fn state(&self) -> &str {
        match self.state.load(Ordering::Relaxed) {
            CB_CLOSED => "closed",
            CB_OPEN => "open",
            CB_HALF_OPEN => "half_open",
            _ => "unknown",
        }
    }

    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::Relaxed) == CB_OPEN
    }

    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }

    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Check if a request should be allowed through.
    pub async fn allow_request(&self) -> bool {
        let state = self.state.load(Ordering::Relaxed);
        match state {
            CB_CLOSED => true,
            CB_OPEN => {
                // Check if reset timeout has elapsed
                let last = self.last_failure.read().await;
                if let Some(last_fail) = *last {
                    if last_fail.elapsed() >= self.reset_timeout {
                        drop(last);
                        self.state.store(CB_HALF_OPEN, Ordering::Relaxed);
                        self.success_count.store(0, Ordering::Relaxed);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CB_HALF_OPEN => true, // Allow test requests
            _ => false,
        }
    }

    /// Record a successful operation.
    pub fn record_success(&self) {
        let state = self.state.load(Ordering::Relaxed);
        match state {
            CB_HALF_OPEN => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.success_threshold {
                    self.state.store(CB_CLOSED, Ordering::Relaxed);
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CB_CLOSED => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record a failed operation.
    pub async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let state = self.state.load(Ordering::Relaxed);

        match state {
            CB_CLOSED => {
                if count >= self.failure_threshold {
                    self.state.store(CB_OPEN, Ordering::Relaxed);
                    let mut last = self.last_failure.write().await;
                    *last = Some(std::time::Instant::now());
                }
            }
            CB_HALF_OPEN => {
                // Any failure in half-open goes back to open
                self.state.store(CB_OPEN, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
                let mut last = self.last_failure.write().await;
                *last = Some(std::time::Instant::now());
            }
            _ => {
                let mut last = self.last_failure.write().await;
                *last = Some(std::time::Instant::now());
            }
        }
    }

    /// Reset the circuit breaker to closed state.
    pub fn reset(&self) {
        self.state.store(CB_CLOSED, Ordering::Relaxed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }
}
