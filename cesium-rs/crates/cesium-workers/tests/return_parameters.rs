//! Mock test worker: return_parameters.
//!
//! Ported from `Specs/TestWorkers/returnParameters.js`.
//! Echoes back the input parameters unchanged.

/// A mock worker function that returns its input parameters unchanged.
///
/// In CesiumJS, this returns `parameters` directly.
/// Used in tests to verify round-trip data transfer through the worker pipeline.
pub fn return_parameters(params: &[u8]) -> Result<Vec<u8>, String> {
    Ok(params.to_vec())
}
