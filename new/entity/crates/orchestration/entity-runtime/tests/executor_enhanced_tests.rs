use entity_runtime::{Output, Priority, Status, Task};
use entity_runtime::{Executor, ExecutorConfig, ExecutorHandle, Scheduler, SchedulerConfig, TaskQueue};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

// -- Enhanced Executor Tests --

#[tokio::test]
async fn test_executor_shutdown_no_tasks() {
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
    assert_eq!(result.unwrap_err(), entity_runtime::ExecutorError::Shutdown);
}

#[tokio::test]
async fn test_executor_submit_checked_allows_before_shutdown() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Should be accepted");
    let result = executor.submit_checked(task).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_executor_submit_with_retry_success_first_try() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Retry test");
    let output = executor
        .submit_with_retry(
            task,
            |t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }),
            3,
            10,
        )
        .await;

    assert!(output.is_success());
    assert_eq!(output.result.unwrap(), "Done");
}

#[tokio::test]
async fn test_executor_submit_with_retry_eventual_success() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    let executor = Executor::new(ExecutorConfig::default());
    let task = Task::new("Eventually succeeds");

    let output = executor
        .submit_with_retry(
            task,
            move |t| {
                let count = call_count_clone.clone();
                Box::pin(async move {
                    let n = count.fetch_add(1, Ordering::SeqCst);
                    if n >= 2 {
                        Output::success(t.id, "Finally!".to_string())
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
            task,
            |t| Box::pin(async move { Output::failure(t.id, "Nope".to_string()) }),
            2,
            10,
        )
        .await;

    assert!(output.is_failure());
}

#[tokio::test]
async fn test_executor_wait_for_completion_empty() {
    let executor = Executor::new(ExecutorConfig::default());

    // No tasks, should return immediately
    let completed = executor.wait_for_completion(5).await;
    assert!(completed);
}

#[tokio::test]
async fn test_executor_wait_for_completion_with_pending() {
    let executor = Executor::new(ExecutorConfig::default());

    // Submit a task but don't execute it
    let task = Task::new("Pending");
    executor.submit(task).await.unwrap();

    // Timeout should occur since task is never processed
    let completed = executor.wait_for_completion(1).await;
    assert!(!completed);
}

// -- Advanced Queue Tests --

#[tokio::test]
async fn test_task_queue_drain() {
    let queue = TaskQueue::new(10);

    queue.push(Task::new("1")).await.unwrap();
    queue.push(Task::new("2")).await.unwrap();
    queue.push(Task::new("3")).await.unwrap();

    let drained = queue.drain().await;
    assert_eq!(drained.len(), 3);
    assert!(queue.is_empty().await);
}

#[tokio::test]
async fn test_task_queue_peek() {
    let queue = TaskQueue::new(10);

    let high = Task::new("High").with_priority(Priority::High);
    let low = Task::new("Low").with_priority(Priority::Low);

    queue.push(low).await.unwrap();
    queue.push(high).await.unwrap();

    // Peek should show highest priority without removing
    let peeked = queue.peek().await.unwrap();
    assert_eq!(peeked.task.input, "High");

    // Still in queue
    assert_eq!(queue.len().await, 2);
}

#[tokio::test]
async fn test_task_queue_clear() {
    let queue = TaskQueue::new(10);

    queue.push(Task::new("1")).await.unwrap();
    queue.push(Task::new("2")).await.unwrap();

    queue.clear().await;
    assert!(queue.is_empty().await);
    assert_eq!(queue.len().await, 0);
}

#[tokio::test]
async fn test_task_queue_critical_priority() {
    let queue = TaskQueue::new(10);

    let normal = Task::new("Normal").with_priority(Priority::Normal);
    let critical = Task::new("Critical").with_priority(Priority::Critical);
    let high = Task::new("High").with_priority(Priority::High);

    queue.push(normal).await.unwrap();
    queue.push(high).await.unwrap();
    queue.push(critical).await.unwrap();

    let first = queue.pop().await.unwrap();
    assert_eq!(first.task.input, "Critical");
}

// -- Executor Status Tracking Tests --

#[tokio::test]
async fn test_executor_status_tracking() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Track me");
    let id = executor.submit(task).await.unwrap();

    // Should be pending
    let status = executor.status(id).await.unwrap();
    assert_eq!(status.status, Status::Pending);
    assert!(status.completed_at.is_none());

    // Execute it
    let _ = executor
        .run_one(|t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }))
        .await;

    // Now should be completed
    let status = executor.status(id).await.unwrap();
    assert_eq!(status.status, Status::Completed);
    assert!(status.completed_at.is_some());
}

#[tokio::test]
async fn test_executor_output_retrieval() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Get output");
    let id = executor.submit(task).await.unwrap();

    // No output yet
    assert!(executor.output(id).await.is_none());

    // Execute
    let _ = executor
        .run_one(|t| Box::pin(async move { Output::success(t.id, "Result data".to_string()) }))
        .await;

    // Now has output
    let output = executor.output(id).await.unwrap();
    assert_eq!(output.result.unwrap(), "Result data");
}

#[tokio::test]
async fn test_executor_cancel_nonexistent() {
    let executor = Executor::new(ExecutorConfig::default());

    let fake_id = entity_runtime::new_id();
    let cancelled = executor.cancel(fake_id).await;
    assert!(!cancelled);
}

#[tokio::test]
async fn test_executor_multiple_submissions() {
    let executor = Executor::new(ExecutorConfig::new().queue_capacity(100));

    let mut ids = Vec::new();
    for i in 0..10 {
        let task = Task::new(format!("Task {}", i));
        ids.push(executor.submit(task).await.unwrap());
    }

    assert_eq!(executor.pending_count().await, 10);
    assert_eq!(ids.len(), 10);
}

#[tokio::test]
async fn test_executor_all_statuses() {
    let executor = Executor::new(ExecutorConfig::default());

    executor.submit(Task::new("A")).await.unwrap();
    executor.submit(Task::new("B")).await.unwrap();

    let all = executor.all_statuses().await;
    assert_eq!(all.len(), 2);
}

// -- Scheduler Enhanced Tests --

#[tokio::test]
async fn test_scheduler_schedule_and_cancel_delayed() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("Delayed cancel");
    let id = scheduler
        .schedule_delayed(task, Duration::from_secs(60))
        .await
        .unwrap();

    assert_eq!(scheduler.scheduled_count().await, 1);

    let cancelled = scheduler.cancel(id).await;
    assert!(cancelled);

    assert_eq!(scheduler.scheduled_count().await, 0);
}

#[tokio::test]
async fn test_scheduler_schedule_recurring() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("Recurring");
    let id = scheduler
        .schedule_recurring(task, Duration::from_secs(30))
        .await
        .unwrap();

    assert_eq!(scheduler.scheduled_count().await, 1);

    // Cancel the recurring task
    let cancelled = scheduler.cancel(id).await;
    assert!(cancelled);
}

#[tokio::test]
async fn test_scheduler_stop() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    assert!(!scheduler.is_running().await);

    // Start in background
    let sched = Arc::new(scheduler);
    let sched_clone = sched.clone();

    let handle = tokio::spawn(async move {
        sched_clone
            .run(|t| Box::pin(async move { Output::success(t.id, "ok".to_string()) }))
            .await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(sched.is_running().await);

    sched.stop().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!sched.is_running().await);

    // The run loop should have exited
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
}

#[tokio::test]
async fn test_scheduler_concurrency() {
    let config = SchedulerConfig::new().concurrency(2);
    let scheduler = Scheduler::new(config);

    assert_eq!(scheduler.concurrency(), 2);
}

// -- ExecutorHandle Tests --

#[tokio::test]
async fn test_executor_handle_all_operations() {
    let executor = Executor::new(ExecutorConfig::default());
    let handle = ExecutorHandle::new(executor);

    assert_eq!(handle.concurrency(), 4); // default

    let task = Task::new("Handle ops");
    let id = handle.submit(task).await.unwrap();

    assert_eq!(handle.pending_count().await, 1);
    assert_eq!(handle.active_count().await, 0);

    let status = handle.status(id).await;
    assert!(status.is_some());

    let output = handle.output(id).await;
    assert!(output.is_none()); // Not executed yet

    let cancelled = handle.cancel(id).await;
    assert!(cancelled);

    assert_eq!(handle.pending_count().await, 0);
}

// -- Edge Cases --

#[tokio::test]
async fn test_task_entry_timing() {
    let queue = TaskQueue::new(10);

    let task = Task::new("Timing");
    queue.push(task).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let entry = queue.pop().await.unwrap();
    let elapsed = entry.elapsed();
    assert!(elapsed.as_millis() >= 90); // Allow small tolerance

    // No execution time since not started
    assert!(entry.started_at.is_none());
    assert!(entry.execution_time().is_none());
}

#[tokio::test]
async fn test_executor_config_defaults() {
    let config = ExecutorConfig::default();
    assert_eq!(config.concurrency, 4);
    assert_eq!(config.queue_capacity, 1000);
    assert_eq!(config.shutdown_timeout_secs, 30);
}

#[tokio::test]
async fn test_scheduler_config_defaults() {
    let config = SchedulerConfig::default();
    assert_eq!(config.poll_interval_ms, 100);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.retry_delay_ms, 1000);
}
