//! Ported from `packages/engine/Source/Workers/transferTypedArrayTest.js`.
//!
//! Worker entry point for testing typed array transfer between workers.
//! This is used during initialization to verify that transferable objects
//! work correctly in the current environment.

/// Tests typed array transfer between workers.
///
/// In CesiumJS, this sends a typed array to a worker and back to verify
/// that the Transferable API is working correctly. In native Rust, this
/// is a no-op since data is moved by ownership.
pub fn transfer_typed_array_test(params: &[u8]) -> Result<Vec<u8>, String> {
    // Echo back the input to verify round-trip
    Ok(params.to_vec())
}

/// Tests typed array transfer (for in-process use).
///
/// In native Rust, this simply returns the input data unchanged.
pub fn transfer_typed_array_test_unpacked(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}
