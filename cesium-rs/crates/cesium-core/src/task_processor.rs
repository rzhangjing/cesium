//! Ported from `packages/engine/Source/Core/TaskProcessor.js`.
//!
//! Manages web worker tasks. Skeleton implementation.

/// Processes tasks using web workers (or equivalent in Rust).
pub struct TaskProcessor {
    worker_path: String,
}

impl TaskProcessor {
    /// Creates a new TaskProcessor.
    pub fn new(worker_path: String) -> Self {
        Self { worker_path }
    }

    /// Gets the worker path.
    pub fn worker_path(&self) -> &str {
        &self.worker_path
    }
}
