//! Mock test worker: create_bad_geometry.
//!
//! Ported from `Specs/TestWorkers/createBadGeometry.js`.
//! Always throws an error to simulate geometry creation failure.

/// A mock worker function that always throws a BadGeometry error.
///
/// In CesiumJS, this wraps a function that throws `Error("BadGeometry.createGeometry")`.
/// Used in tests to verify error handling in the TaskProcessor pipeline.
pub fn create_bad_geometry(_params: &[u8]) -> Result<Vec<u8>, String> {
    Err("BadGeometry.createGeometry".to_string())
}
