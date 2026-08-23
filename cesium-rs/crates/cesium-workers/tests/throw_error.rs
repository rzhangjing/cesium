//! Mock test worker: throw_error.
//!
//! Ported from `Specs/TestWorkers/throwError.js`.
//! Throws an error with a message from the parameters.

/// A mock worker function that throws an error with a message from parameters.
///
/// In CesiumJS, this throws `new Error(parameters.message)`.
/// Used in tests to verify error propagation through the TaskProcessor pipeline.
pub fn throw_error(params: &[u8]) -> Result<Vec<u8>, String> {
    let message = String::from_utf8_lossy(params);
    Err(format!("Worker error: {}", message))
}
