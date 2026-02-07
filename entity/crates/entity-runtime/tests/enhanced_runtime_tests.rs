use entity_runtime::{Output, Priority, Status, Task};
use entity_runtime::{Executor, ExecutorConfig, ExecutorHandle, Scheduler, SchedulerConfig, TaskQueue};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// -- Executor Shutdown Tests --

#[tokio::test]
async fn test_executor_graceful_shutdown_no_tasks() {
    let executor = Executor::new(ExecutorConfig::default());

    assert!(!executor.is_shutdown().await);

    executor.shutdown().await;

    assert!(executor.is_shutdown().await);
}

#[tokio::test]
async fn test_executor_submit_checked_rejects_after_shutdown() {
    let executor = Executor::new(ExecutorConfig::default());

    executor.shutdown().await;

    let task = Task::new("Should be rejected");
    let result = executor.submit_checked(task).await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        entity_runtime::ExecutorError::Shutdown
    );
}

#[tokio::test]
async fn test_executor_submit_checked_works_before_shutdown() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Before shutdown");
    let result = executor.submit_checked(task).await;

    assert!(result.is_ok());
    assert_eq!(executor.pending_count().await, 1);
}

// -- Executor Retry Tests --

#[tokio::test]
async fn test_executor_submit_with_retry_success_first_try() {
    let executor = Executor::new(ExecutorConfig::default());
    let task = Task::new("Retry test");

    let output = executor
        .submit_with_retry(
            task.clone(),
            |t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }),
            3,
            10,
        )
        .await;

    assert!(output.is_success());
    assert_eq!(output.result.unwrap(), "Done");
}

#[tokio::test]
async fn test_executor_submit_with_retry_succeeds_after_failures() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let count_clone = call_count.clone();

    let executor = Executor::new(ExecutorConfig::default());
    let task = Task::new("Eventually succeeds");

    let output = executor
        .submit_with_retry(
            task.clone(),
            move |t| {
                let c = count_clone.clone();
                Box::pin(async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n >= 2 {
                        Output::success(t.id, "Finally".to_string())
                    } else {
                        Output::failure(t.id, format!("Attempt {} failed", n))
                    }
                })
            },
            5,
            10,
        )
        .await;

    assert!(output.is_success());
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_executor_submit_with_retry_all_fail() {
    let executor = Executor::new(ExecutorConfig::default());
    let task = Task::new("Always fails");

    let output = executor
        .submit_with_retry(
            task.clone(),
            |t| Box::pin(async move { Output::failure(t.id, "Nope".to_string()) }),
            2,
            10,
        )
        .await;

    assert!(output.is_failure());
}

// -- Wait for Completion Tests --

#[tokio::test]
async fn test_executor_wait_for_completion_empty() {
    let executor = Executor::new(ExecutorConfig::default());

    let completed = executor.wait_for_completion(1).await;
    assert!(completed);
}

#[tokio::test]
async fn test_executor_wait_for_completion_with_pending() {
    let executor = Executor::new(ExecutorConfig::default());

    executor.submit(Task::new("Pending")).await.unwrap();

    // Won't complete because nothing is processing
    let completed = executor.wait_for_completion(1).await;
    assert!(!completed); // Times out because task is still pending
}

// -- Priority Queue Comprehensive Tests --

#[tokio::test]
async fn test_queue_critical_priority_first() {
    let queue = TaskQueue::new(10);

    queue
        .push(Task::new("Normal").with_priority(Priority::Normal))
        .await
        .unwrap();
    queue
        .push(Task::new("Critical").with_priority(Priority::Critical))
        .await
        .unwrap();
    queue
        .push(Task::new("Low").with_priority(Priority::Low))
        .await
        .unwrap();
    queue
        .push(Task::new("High").with_priority(Priority::High))
        .await
        .unwrap();

    let first = queue.pop().await.unwrap();
    assert_eq!(first.task.input, "Critical");

    let second = queue.pop().await.unwrap();
    assert_eq!(second.task.input, "High");

    let third = queue.pop().await.unwrap();
    assert_eq!(third.task.input, "Normal");

    let fourth = queue.pop().await.unwrap();
    assert_eq!(fourth.task.input, "Low");
}

#[tokio::test]
async fn test_queue_fifo_within_same_priority() {
    let queue = TaskQueue::new(10);

    queue
        .push(Task::new("First"))
        .await
        .unwrap();
    // Tiny delay so timestamps differ
    tokio::time::sleep(Duration::from_millis(1)).await;
    queue
        .push(Task::new("Second"))
        .await
        .unwrap();

    let first = queue.pop().await.unwrap();
    assert_eq!(first.task.input, "First");
}

#[tokio::test]
async fn test_queue_drain() {
    let queue = TaskQueue::new(10);

    for i in 0..5 {
        queue.push(Task::new(format!("Task {}", i))).await.unwrap();
    }

    assert_eq!(queue.len().await, 5);

    let drained = queue.drain().await;
    assert_eq!(drained.len(), 5);
    assert!(queue.is_empty().await);
}

#[tokio::test]
async fn test_queue_clear() {
    let queue = TaskQueue::new(10);

    queue.push(Task::new("1")).await.unwrap();
    queue.push(Task::new("2")).await.unwrap();
    queue.push(Task::new("3")).await.unwrap();

    queue.clear().await;
    assert!(queue.is_empty().await);
    assert_eq!(queue.len().await, 0);
}

#[tokio::test]
async fn test_queue_peek_doesnt_remove() {
    let queue = TaskQueue::new(10);
    queue.push(Task::new("Peek me")).await.unwrap();

    let peeked = queue.peek().await;
    assert!(peeked.is_some());
    assert_eq!(peeked.unwrap().task.input, "Peek me");

    // Still in queue
    assert_eq!(queue.len().await, 1);
}

// -- TaskEntry Timing Tests --

#[tokio::test]
async fn test_task_entry_execution_time() {
    let queue = TaskQueue::new(10);
    queue.push(Task::new("Timing")).await.unwrap();

    let mut entry = queue.pop().await.unwrap();

    // Not started yet
    assert!(entry.execution_time().is_none());

    // Simulate start
    entry.started_at = Some(std::time::Instant::now());
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Running time (no completion yet)
    let running_time = entry.execution_time();
    assert!(running_time.is_some());
    assert!(running_time.unwrap().as_millis() >= 10);

    // Simulate completion
    entry.completed_at = Some(std::time::Instant::now());
    let completed_time = entry.execution_time();
    assert!(completed_time.is_some());
}

// -- Scheduler Enhanced Tests --

#[tokio::test]
async fn test_scheduler_schedule_and_cancel() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("Cancel target");
    let id = scheduler.schedule_task(task).await.unwrap();

    // Should be in pending queue
    assert!(scheduler.pending_count().await > 0 || scheduler.scheduled_count().await > 0);

    let cancelled = scheduler.cancel(id).await;
    assert!(cancelled);
}

#[tokio::test]
async fn test_scheduler_recurring_creates_scheduled_entry() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("Recurring");
    let _id = scheduler
        .schedule_recurring(task, Duration::from_secs(60))
        .await
        .unwrap();

    assert_eq!(scheduler.scheduled_count().await, 1);
}

#[tokio::test]
async fn test_scheduler_stop() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    assert!(!scheduler.is_running().await);

    scheduler.stop().await;

    assert!(!scheduler.is_running().await);
}

// -- ExecutorConfig Edge Cases --

#[tokio::test]
async fn test_executor_config_clone() {
    let config = ExecutorConfig::new()
        .concurrency(16)
        .queue_capacity(2000)
        .shutdown_timeout(120);

    let cloned = config.clone();
    assert_eq!(cloned.concurrency, 16);
    assert_eq!(cloned.queue_capacity, 2000);
    assert_eq!(cloned.shutdown_timeout_secs, 120);
}

#[tokio::test]
async fn test_scheduler_config_clone() {
    let config = SchedulerConfig::new()
        .concurrency(8)
        .poll_interval(200)
        .max_retries(10)
        .retry_delay(5000);

    let cloned = config.clone();
    assert_eq!(cloned.executor_config.concurrency, 8);
    assert_eq!(cloned.poll_interval_ms, 200);
    assert_eq!(cloned.max_retries, 10);
    assert_eq!(cloned.retry_delay_ms, 5000);
}

// -- Executor Status Management --

#[tokio::test]
async fn test_executor_all_statuses() {
    let executor = Executor::new(ExecutorConfig::default());

    executor.submit(Task::new("Task A")).await.unwrap();
    executor.submit(Task::new("Task B")).await.unwrap();

    let statuses = executor.all_statuses().await;
    assert_eq!(statuses.len(), 2);
    assert!(statuses.iter().all(|s| s.status == Status::Pending));
}

#[tokio::test]
async fn test_executor_run_one_and_check_status() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Check status");
    let id = task.id;
    executor.submit(task).await.unwrap();

    let _output = executor
        .run_one(|t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }))
        .await;

    let status = executor.status(id).await;
    assert!(status.is_some());
    let status = status.unwrap();
    assert_eq!(status.status, Status::Completed);
    assert!(status.completed_at.is_some());
}

#[tokio::test]
async fn test_executor_clear_completed_keeps_pending() {
    let executor = Executor::new(ExecutorConfig::default());

    let task1 = Task::new("Will complete");
    let task2 = Task::new("Will remain pending");

    executor.submit(task1).await.unwrap();

    // Complete the first one
    let _ = executor
        .run_one(|t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }))
        .await;

    executor.submit(task2).await.unwrap();

    executor.clear_completed().await;

    // Only the pending task's status should remain
    let statuses = executor.all_statuses().await;
    assert!(statuses.iter().all(|s| !s.status.is_terminal()));
}

// -- ExecutorHandle Tests --

#[tokio::test]
async fn test_executor_handle_concurrency() {
    let config = ExecutorConfig::new().concurrency(16);
    let executor = Executor::new(config);
    let handle = ExecutorHandle::new(executor);

    assert_eq!(handle.concurrency(), 16);
}

#[tokio::test]
async fn test_executor_handle_full_lifecycle() {
    let executor = Executor::new(ExecutorConfig::default());
    let handle = ExecutorHandle::new(executor);

    let task = Task::new("Handle lifecycle");
    let id = handle.submit(task).await.unwrap();

    assert!(handle.pending_count().await > 0);

    let status = handle.status(id).await;
    assert!(status.is_some());
    assert_eq!(status.unwrap().status, Status::Pending);

    let cancelled = handle.cancel(id).await;
    assert!(cancelled);
}

// -- Error Display Tests --

#[test]
fn test_executor_error_display() {
    assert_eq!(
        format!("{}", entity_runtime::ExecutorError::QueueFull),
        "Task queue is full"
    );
    assert_eq!(
        format!("{}", entity_runtime::ExecutorError::Shutdown),
        "Executor is shutting down"
    );
    assert_eq!(
        format!("{}", entity_runtime::ExecutorError::TaskNotFound),
        "Task not found"
    );
}

#[test]
fn test_executor_error_eq() {
    assert_eq!(
        entity_runtime::ExecutorError::QueueFull,
        entity_runtime::ExecutorError::QueueFull
    );
    assert_ne!(
        entity_runtime::ExecutorError::QueueFull,
        entity_runtime::ExecutorError::Shutdown
    );
}
