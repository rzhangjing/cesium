//! Mock test worker: return_non_cloneable.
//!
//! Ported from `Specs/TestWorkers/returnNonCloneable.js`.
//! Returns a non-cloneable value (a function in JS, an error in Rust).

/// A mock worker function that attempts to return a non-cloneable value.
///
/// In CesiumJS, this returns a function (which is not cloneable via structured clone).
/// In Rust, we simulate this by returning an error indicating the value is not serializable.
/// Used in tests to verify serialization error handling.
pub fn return_non_cloneable(_params: &[u8]) -> Result<Vec<u8>, String> {
    Err("Cannot serialize non-cloneable value (function equivalent)".to_string())
}
