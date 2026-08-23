//! Ported from `packages/engine/Source/Core/TaskProcessor.js`.

/// Processes tasks in web workers.
pub struct TaskProcessor {
    _private: (),
}

impl TaskProcessor {
    /// Creates a new TaskProcessor.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for TaskProcessor {
    fn default() -> Self { Self::new() }
}
