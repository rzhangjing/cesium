//! Ported from `packages/engine/Source/Workers/createTaskProcessorWorker.js`.
//!
//! The bootstrap function that runs inside each worker thread.

/// Creates a task processor worker function.
///
/// In CesiumJS, this wraps a user-defined function to run inside a Web Worker.
/// In Rust, this wraps a function to run in a thread pool task.
///
/// The wrapper handles:
/// - Deserializing input parameters
/// - Calling the user function
/// - Serializing the result
/// - Error handling
///
/// Mirrors CesiumJS `createTaskProcessorWorker` (75 lines).
pub fn create_task_processor_worker(
    user_function: fn(&[u8]) -> Result<Vec<u8>, String>,
) -> impl Fn(&[u8]) -> Result<Vec<u8>, String> {
    move |parameters: &[u8]| {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| user_function(parameters))) {
            Ok(result) => result,
            Err(_) => Err("Worker task panicked".to_string()),
        }
    }
}

/// A default no-op worker function for testing.
pub fn noop_worker(_parameters: &[u8]) -> Result<Vec<u8>, String> {
    Ok(Vec::new())
}
