use crate::types::{Id, Priority, Status, Task};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TaskEntry {
    pub task: Task,
    pub status: Status,
    pub submitted_at: std::time::Instant,
    pub started_at: Option<std::time::Instant>,
    pub completed_at: Option<std::time::Instant>,
}

impl TaskEntry {
    pub fn new(task: Task) -> Self {
        Self {
            task,
            status: Status::Pending,
            submitted_at: std::time::Instant::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.submitted_at.elapsed()
    }

    pub fn execution_time(&self) -> Option<std::time::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            (Some(start), None) => Some(start.elapsed()),
            _ => None,
        }
    }
}

impl Eq for TaskEntry {}

impl PartialEq for TaskEntry {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Ord for TaskEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let priority_ord = priority_value(self.task.priority).cmp(&priority_value(other.task.priority));
        if priority_ord != Ordering::Equal {
            return priority_ord;
        }
        other.submitted_at.cmp(&self.submitted_at)
    }
}

impl PartialOrd for TaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn priority_value(p: Priority) -> u8 {
    match p {
        Priority::Critical => 4,
        Priority::High => 3,
        Priority::Normal => 2,
        Priority::Low => 1,
    }
}

#[derive(Clone)]
pub struct TaskQueue {
    queue: Arc<Mutex<BinaryHeap<TaskEntry>>>,
    capacity: usize,
}

impl TaskQueue {
    pub fn new(capacity: usize) -> Self {
        debug_assert!(capacity > 0, "capacity must be positive");
        Self {
            queue: Arc::new(Mutex::new(BinaryHeap::with_capacity(capacity))),
            capacity,
        }
    }

    pub async fn push(&self, task: Task) -> Result<(), QueueError> {
        let mut queue = self.queue.lock().await;
        if queue.len() >= self.capacity {
            return Err(QueueError::Full);
        }
        queue.push(TaskEntry::new(task));
        Ok(())
    }

    pub async fn pop(&self) -> Option<TaskEntry> {
        let mut queue = self.queue.lock().await;
        queue.pop()
    }

    pub async fn peek(&self) -> Option<TaskEntry> {
        let queue = self.queue.lock().await;
        queue.peek().cloned()
    }

    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    pub async fn clear(&self) {
        self.queue.lock().await.clear();
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub async fn find(&self, id: Id) -> Option<TaskEntry> {
        let queue = self.queue.lock().await;
        queue.iter().find(|e| e.task.id == id).cloned()
    }

    pub async fn remove(&self, id: Id) -> bool {
        let mut queue = self.queue.lock().await;
        let original_len = queue.len();
        let entries: Vec<TaskEntry> = queue.drain().filter(|e| e.task.id != id).collect();
        let removed = original_len > entries.len();
        for entry in entries {
            queue.push(entry);
        }
        removed
    }

    pub async fn drain(&self) -> Vec<TaskEntry> {
        let mut queue = self.queue.lock().await;
        queue.drain().collect()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    Full,
    NotFound,
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::Full => write!(f, "Queue is full"),
            QueueError::NotFound => write!(f, "Task not found"),
        }
    }
}

impl std::error::Error for QueueError {}
