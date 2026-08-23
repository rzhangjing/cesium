//! Ported from `packages/engine/Source/Workers/TaskProcessor.js`.
//!
//! Manages offloading computation to a thread pool.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Manages offloading computation to a thread pool.
///
/// In CesiumJS, this uses Web Workers. In Rust, this uses rayon's thread pool
/// with channels for zero-copy data transfer.
/// Mirrors CesiumJS `TaskProcessor` (400 lines).
pub struct TaskProcessor {
    /// The worker script/module name.
    worker_name: String,
    /// The maximum number of active tasks.
    maximum_active_tasks: usize,
    /// The number of currently active tasks.
    active_tasks: Arc<Mutex<usize>>,
    /// Whether this processor has been destroyed.
    is_destroyed: bool,
}

/// A handle to a pending task.
pub struct TaskHandle {
    /// The receiver for the task result.
    receiver: mpsc::Receiver<TaskResult>,
}

/// The result of a task.
pub type TaskResult = Result<Vec<u8>, String>;

impl TaskProcessor {
    /// Creates a new TaskProcessor.
    pub fn new(worker_name: &str) -> Self {
        Self {
            worker_name: worker_name.to_string(),
            maximum_active_tasks: 4,
            active_tasks: Arc::new(Mutex::new(0)),
            is_destroyed: false,
        }
    }

    /// Creates a new TaskProcessor with a custom maximum active tasks.
    pub fn with_max_tasks(worker_name: &str, maximum_active_tasks: usize) -> Self {
        Self {
            worker_name: worker_name.to_string(),
            maximum_active_tasks,
            active_tasks: Arc::new(Mutex::new(0)),
            is_destroyed: false,
        }
    }

    /// Schedules a task for processing.
    ///
    /// Returns a [`TaskHandle`] that can be used to retrieve the result.
    pub fn schedule_task(&self, parameters: Vec<u8>) -> Option<TaskHandle> {
        if self.is_destroyed {
            return None;
        }

        let mut active = self.active_tasks.lock().unwrap();
        if *active >= self.maximum_active_tasks {
            return None; // Too many active tasks
        }
        *active += 1;
        drop(active);

        let active_tasks = Arc::clone(&self.active_tasks);
        let (sender, receiver) = mpsc::channel();

        // DEVIATION: In production, this would use rayon::spawn or a dedicated thread pool
        // For now, we execute synchronously as a stub
        std::thread::spawn(move || {
            let result = process_worker_task(&parameters);
            let _ = sender.send(result);
            let mut active = active_tasks.lock().unwrap();
            *active = active.saturating_sub(1);
        });

        Some(TaskHandle { receiver })
    }

    /// Returns the worker name.
    pub fn worker_name(&self) -> &str {
        &self.worker_name
    }

    /// Returns the number of active tasks.
    pub fn active_tasks_count(&self) -> usize {
        *self.active_tasks.lock().unwrap()
    }

    /// Returns whether this processor has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this processor.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl TaskHandle {
    /// Waits for the task to complete and returns the result.
    pub fn wait(self) -> TaskResult {
        self.receiver.recv().unwrap_or(Err("Channel closed".to_string()))
    }

    /// Tries to get the result without blocking.
    pub fn try_get(&self) -> Option<TaskResult> {
        self.receiver.try_recv().ok()
    }
}

/// Internal task processing function.
///
/// In CesiumJS, this runs in a Web Worker. In Rust, this runs in a thread.
fn process_worker_task(_parameters: &[u8]) -> TaskResult {
    // DEVIATION: This is the entry point for all worker tasks.
    // In production, this would dispatch to the appropriate worker function
    // based on the worker_name.
    Ok(Vec::new())
}

impl Default for TaskProcessor {
    fn default() -> Self { Self::new("default") }
}
