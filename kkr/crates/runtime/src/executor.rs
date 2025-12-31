//! Task executor


/// Executor for running tasks
pub struct Executor {
    // TODO: Add thread pool, task queue, etc.
}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
