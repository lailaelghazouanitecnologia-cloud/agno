use kkr_core::{Output, Priority, Status, Task};
use kkr_runtime::{
    Executor, ExecutorConfig, ExecutorHandle, Scheduler, SchedulerConfig, TaskQueue,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_task_queue_push_pop() {
    let queue = TaskQueue::new(10);

    let task1 = Task::new("Task 1");
    let task2 = Task::new("Task 2");

    queue.push(task1.clone()).await.unwrap();
    queue.push(task2.clone()).await.unwrap();

    assert_eq!(queue.len().await, 2);

    let popped = queue.pop().await.unwrap();
    assert_eq!(popped.task.input, "Task 1");
    assert_eq!(queue.len().await, 1);
}

#[tokio::test]
async fn test_task_queue_priority() {
    let queue = TaskQueue::new(10);

    let low = Task::new("Low").with_priority(Priority::Low);
    let high = Task::new("High").with_priority(Priority::High);
    let normal = Task::new("Normal").with_priority(Priority::Normal);

    queue.push(low).await.unwrap();
    queue.push(high).await.unwrap();
    queue.push(normal).await.unwrap();

    let first = queue.pop().await.unwrap();
    assert_eq!(first.task.input, "High");

    let second = queue.pop().await.unwrap();
    assert_eq!(second.task.input, "Normal");

    let third = queue.pop().await.unwrap();
    assert_eq!(third.task.input, "Low");
}

#[tokio::test]
async fn test_task_queue_capacity() {
    let queue = TaskQueue::new(2);

    queue.push(Task::new("1")).await.unwrap();
    queue.push(Task::new("2")).await.unwrap();

    let result = queue.push(Task::new("3")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_task_queue_find() {
    let queue = TaskQueue::new(10);

    let task = Task::new("Find me");
    let id = task.id;

    queue.push(task).await.unwrap();

    let found = queue.find(id).await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().task.input, "Find me");
}

#[tokio::test]
async fn test_task_queue_remove() {
    let queue = TaskQueue::new(10);

    let task = Task::new("Remove me");
    let id = task.id;

    queue.push(task).await.unwrap();
    assert_eq!(queue.len().await, 1);

    let removed = queue.remove(id).await;
    assert!(removed);
    assert_eq!(queue.len().await, 0);
}

#[tokio::test]
async fn test_executor_config() {
    let config = ExecutorConfig::new()
        .concurrency(8)
        .queue_capacity(500)
        .shutdown_timeout(60);

    assert_eq!(config.concurrency, 8);
    assert_eq!(config.queue_capacity, 500);
    assert_eq!(config.shutdown_timeout_secs, 60);
}

#[tokio::test]
async fn test_executor_submit() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Test task");
    let id = executor.submit(task).await.unwrap();

    assert_eq!(executor.pending_count().await, 1);

    let status = executor.status(id).await;
    assert!(status.is_some());
    assert_eq!(status.unwrap().status, Status::Pending);
}

#[tokio::test]
async fn test_executor_cancel() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Cancel me");
    let id = executor.submit(task).await.unwrap();

    let cancelled = executor.cancel(id).await;
    assert!(cancelled);

    let status = executor.status(id).await;
    assert!(status.is_some());
    assert_eq!(status.unwrap().status, Status::Cancelled);
}

#[tokio::test]
async fn test_executor_run_one() {
    let executor = Executor::new(ExecutorConfig::default());

    let task = Task::new("Run me");
    let id = task.id;
    executor.submit(task).await.unwrap();

    let output = executor
        .run_one(|t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }))
        .await;

    assert!(output.is_some());
    let output = output.unwrap();
    assert!(output.is_success());
    assert_eq!(output.result.unwrap(), "Done");
}

#[tokio::test]
async fn test_executor_handle() {
    let executor = Executor::new(ExecutorConfig::default());
    let handle = ExecutorHandle::new(executor);

    let task = Task::new("Handle test");
    let id = handle.submit(task).await.unwrap();

    assert_eq!(handle.pending_count().await, 1);

    let status = handle.status(id).await;
    assert!(status.is_some());
}

#[tokio::test]
async fn test_scheduler_config() {
    let config = SchedulerConfig::new()
        .concurrency(4)
        .poll_interval(50)
        .max_retries(5)
        .retry_delay(2000);

    assert_eq!(config.executor_config.concurrency, 4);
    assert_eq!(config.poll_interval_ms, 50);
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.retry_delay_ms, 2000);
}

#[tokio::test]
async fn test_scheduler_schedule_task() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("Scheduled");
    let id = scheduler.schedule_task(task).await.unwrap();

    assert_eq!(scheduler.pending_count().await, 1);
}

#[tokio::test]
async fn test_scheduler_schedule_delayed() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("Delayed");
    let id = scheduler
        .schedule_delayed(task, Duration::from_secs(10))
        .await
        .unwrap();

    assert_eq!(scheduler.scheduled_count().await, 1);
    assert_eq!(scheduler.pending_count().await, 0);
}

#[tokio::test]
async fn test_scheduler_cancel() {
    let scheduler = Scheduler::new(SchedulerConfig::default());

    let task = Task::new("To cancel");
    let id = scheduler.schedule_task(task).await.unwrap();

    let cancelled = scheduler.cancel(id).await;
    assert!(cancelled);
}

#[tokio::test]
async fn test_task_entry_timing() {
    let queue = TaskQueue::new(10);

    let task = Task::new("Timing test");
    queue.push(task).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let entry = queue.pop().await.unwrap();
    assert!(entry.elapsed().as_millis() >= 50);
}

#[tokio::test]
async fn test_executor_clear_completed() {
    let executor = Executor::new(ExecutorConfig::default());

    let task1 = Task::new("Task 1");
    let id1 = task1.id;
    executor.submit(task1).await.unwrap();

    let _ = executor
        .run_one(|t| Box::pin(async move { Output::success(t.id, "Done".to_string()) }))
        .await;

    let all = executor.all_statuses().await;
    assert!(!all.is_empty());

    executor.clear_completed().await;

    let remaining = executor.all_statuses().await;
    assert!(remaining.is_empty() || remaining.iter().all(|s| !s.status.is_terminal()));
}

#[tokio::test]
async fn test_default_impls() {
    let _queue = TaskQueue::default();
    let _executor = Executor::default();
    let _scheduler = Scheduler::default();
    let _config = ExecutorConfig::default();
    let _sched_config = SchedulerConfig::default();
}
