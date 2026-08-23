//! Mock test worker: transfer_array_buffer.
//!
//! Ported from `Specs/TestWorkers/transferArrayBuffer.js`.
//! Creates an ArrayBuffer of the requested size and marks it as transferable.

/// A mock worker function that creates a buffer and marks it as transferable.
///
/// In CesiumJS, this creates a new `ArrayBuffer(parameters.byteLength)` and pushes
/// it to the `transferableObjects` array. In native Rust, data is moved by ownership
/// so the transferable concept is a no-op.
pub fn transfer_array_buffer(params: &[u8]) -> Result<Vec<u8>, String> {
    // In CesiumJS, parameters.byteLength determines the output buffer size.
    // In Rust, we create a zero-filled buffer of the same size as the input.
    let size = params.len();
    Ok(vec![0u8; size])
}
