use crate::types::{Id, Output, Task};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;
use tracing::{debug, info};

use crate::executor::{Executor, ExecutorConfig, ExecutorError, TaskFuture, TaskStatus};

const DEFAULT_POLL_INTERVAL_MS: u64 = 100;

#[derive(Clone)]
pub struct SchedulerConfig {
    pub executor_config: ExecutorConfig,
    pub poll_interval_ms: u64,
    pub max_retries: usize,
    pub retry_delay_ms: u64,
}

impl SchedulerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        debug_assert!(n > 0, "concurrency must be positive");
        self.executor_config = self.executor_config.concurrency(n);
        self
    }

    pub fn poll_interval(mut self, ms: u64) -> Self {
        debug_assert!(ms > 0, "poll_interval must be positive");
        self.poll_interval_ms = ms;
        self
    }

    pub fn max_retries(mut self, n: usize) -> Self {
        self.max_retries = n;
        self
    }

    pub fn retry_delay(mut self, ms: u64) -> Self {
        debug_assert!(ms > 0, "retry_delay must be positive");
        self.retry_delay_ms = ms;
        self
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            executor_config: ExecutorConfig::default(),
            poll_interval_ms: DEFAULT_POLL_INTERVAL_MS,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub task: Task,
    pub scheduled_at: Instant,
    pub run_at: Option<Instant>,
    pub interval: Option<Duration>,
    pub retry_count: usize,
}

impl ScheduledTask {
    pub fn new(task: Task) -> Self {
        Self {
            task,
            scheduled_at: Instant::now(),
            run_at: None,
            interval: None,
            retry_count: 0,
        }
    }

    pub fn delay(mut self, duration: Duration) -> Self {
        self.run_at = Some(Instant::now() + duration);
        self
    }

    pub fn at(mut self, instant: Instant) -> Self {
        self.run_at = Some(instant);
        self
    }

    pub fn repeat(mut self, interval: Duration) -> Self {
        debug_assert!(!interval.is_zero(), "interval must be positive");
        self.interval = Some(interval);
        self
    }

    pub fn is_due(&self) -> bool {
        match self.run_at {
            Some(t) => Instant::now() >= t,
            None => true,
        }
    }
}

pub struct Scheduler {
    config: SchedulerConfig,
    executor: Executor,
    scheduled: Arc<RwLock<HashMap<Id, ScheduledTask>>>,
    running: Arc<RwLock<bool>>,
    /// Tracks tasks for retry: id -> (original_task, retry_count)
    retry_tracker: Arc<RwLock<HashMap<Id, (Task, usize)>>>,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        let executor = Executor::new(config.executor_config.clone());

        Self {
            config,
            executor,
            scheduled: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            retry_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn schedule(&self, scheduled_task: ScheduledTask) -> Result<Id, ExecutorError> {
        let id = scheduled_task.task.id;

        if scheduled_task.is_due() && scheduled_task.interval.is_none() {
            return self.executor.submit(scheduled_task.task).await;
        }

        {
            let mut scheduled = self.scheduled.write().await;
            scheduled.insert(id, scheduled_task);
        }

        debug!(task_id = %id, "Task scheduled");
        Ok(id)
    }

    pub async fn schedule_task(&self, task: Task) -> Result<Id, ExecutorError> {
        self.schedule(ScheduledTask::new(task)).await
    }

    pub async fn schedule_delayed(&self, task: Task, delay: Duration) -> Result<Id, ExecutorError> {
        self.schedule(ScheduledTask::new(task).delay(delay)).await
    }

    pub async fn schedule_recurring(
        &self,
        task: Task,
        interval: Duration,
    ) -> Result<Id, ExecutorError> {
        self.schedule(ScheduledTask::new(task).repeat(interval)).await
    }

    pub async fn cancel(&self, id: Id) -> bool {
        {
            let mut scheduled = self.scheduled.write().await;
            if scheduled.remove(&id).is_some() {
                return true;
            }
        }
        self.executor.cancel(id).await
    }

    pub async fn status(&self, id: Id) -> Option<TaskStatus> {
        self.executor.status(id).await
    }

    pub async fn output(&self, id: Id) -> Option<Output> {
        self.executor.output(id).await
    }

    pub async fn run<F>(&self, handler: F)
    where
        F: Fn(Task) -> TaskFuture + Send + Sync + Clone + 'static,
    {
        {
            let mut running = self.running.write().await;
            *running = true;
        }

        let poll_interval = Duration::from_millis(self.config.poll_interval_ms);
        let scheduled = self.scheduled.clone();
        let executor = &self.executor;
        let running = self.running.clone();

        loop {
            {
                let is_running = *running.read().await;
                if !is_running {
                    break;
                }
            }

            let mut due_tasks = Vec::new();
            let mut reschedule = Vec::new();

            {
                let mut scheduled_map = scheduled.write().await;
                let mut to_remove = Vec::new();

                for (id, task) in scheduled_map.iter() {
                    if task.is_due() {
                        due_tasks.push(task.clone());
                        if task.interval.is_some() {
                            reschedule.push(*id);
                        } else {
                            to_remove.push(*id);
                        }
                    }
                }

                for id in to_remove {
                    scheduled_map.remove(&id);
                }

                for id in reschedule {
                    if let Some(task) = scheduled_map.get_mut(&id) {
                        if let Some(interval) = task.interval {
                            task.run_at = Some(Instant::now() + interval);
                        }
                    }
                }
            }

            for scheduled_task in due_tasks {
                // Track task for potential retry
                if self.config.max_retries > 0 {
                    self.retry_tracker.write().await
                        .entry(scheduled_task.task.id)
                        .or_insert_with(|| (scheduled_task.task.clone(), 0));
                }
                let _ = executor.submit(scheduled_task.task).await;
            }

            let h = handler.clone();
            if let Some(output) = executor.run_one(move |task| h(task)).await {
                if output.is_failure() {
                    let mut tracker = self.retry_tracker.write().await;
                    if let Some((task, count)) = tracker.get_mut(&output.task_id) {
                        if *count < self.config.max_retries {
                            *count += 1;
                            let attempt = *count;
                            let delay_ms = self.config.retry_delay_ms * 2u64.pow((attempt - 1) as u32);
                            let retry_task = task.clone();
                            drop(tracker);
                            let _ = self.schedule(
                                ScheduledTask::new(retry_task).delay(Duration::from_millis(delay_ms))
                            ).await;
                            debug!(
                                task_id = %output.task_id,
                                attempt = attempt,
                                delay_ms = delay_ms,
                                "Retrying failed task with exponential backoff"
                            );
                        } else {
                            debug!(task_id = %output.task_id, "Task failed, max retries exhausted");
                            tracker.remove(&output.task_id);
                        }
                    }
                } else {
                    // Success: clean up retry tracker
                    self.retry_tracker.write().await.remove(&output.task_id);
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        info!("Scheduler stopped");
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    pub async fn pending_count(&self) -> usize {
        self.executor.pending_count().await
    }

    pub async fn scheduled_count(&self) -> usize {
        self.scheduled.read().await.len()
    }

    pub async fn active_count(&self) -> usize {
        self.executor.active_count().await
    }

    pub fn concurrency(&self) -> usize {
        self.executor.concurrency()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(SchedulerConfig::default())
    }
}
