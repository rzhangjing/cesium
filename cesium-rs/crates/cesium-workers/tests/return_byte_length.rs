//! Mock test worker: return_byte_length.
//!
//! Ported from `Specs/TestWorkers/returnByteLength.js`.
//! Returns the byte length of the input parameters.

/// A mock worker function that returns the byte length of its input.
///
/// In CesiumJS, this returns `parameters.byteLength`.
/// Used in tests to verify that data is correctly passed to workers.
pub fn return_byte_length(params: &[u8]) -> Result<Vec<u8>, String> {
    let len = params.len() as u64;
    Ok(len.to_le_bytes().to_vec())
}
