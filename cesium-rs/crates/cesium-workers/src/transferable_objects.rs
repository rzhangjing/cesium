//! Ported from `packages/engine/Source/Workers/transferableObjects.js`.
//!
//! Utilities for transferring data between workers.
//!
//! In CesiumJS, TransferableObjects manage the Web Worker Transferable API
//! for zero-copy data movement. In native Rust, data is moved by ownership
//! so this is largely a no-op. In wasm builds, this maps to the JS
//! Transferable API for web worker communication.

/// Marks a buffer as transferable (zero-copy move between workers).
///
/// In native Rust, this is a no-op since data is moved by ownership.
/// In wasm, this maps to the Transferable API.
pub struct TransferableObjects;

impl TransferableObjects {
    /// Creates a transferable buffer.
    ///
    /// In native Rust, returns a standard `Vec<u8>`.
    /// In wasm, this would create a SharedArrayBuffer or similar.
    pub fn create_buffer(size: usize) -> Vec<u8> {
        vec![0u8; size]
    }

    /// Marks a buffer as transferable for sending to a worker.
    ///
    /// In native Rust, this is a no-op (the buffer is moved by ownership).
    /// In wasm, this would add the buffer to the transfer list.
    pub fn mark_transferable(_buffer: &mut Vec<u8>) {
        // No-op in native Rust
    }

    /// Returns the list of transferable objects for a message.
    ///
    /// In CesiumJS, this returns an array of Transferable objects.
    /// In native Rust, this always returns an empty list.
    pub fn get_transferables() -> Vec<()> {
        Vec::new()
    }
}

/// Packs a typed array into a byte buffer for transfer.
///
/// # Arguments
/// * `data` - The data to pack.
///
/// Returns the packed bytes.
pub fn pack_typed_array(data: &[f64]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(data.len() * 8);
    for &val in data {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Unpacks a byte buffer into a typed array.
///
/// # Arguments
/// * `bytes` - The packed bytes.
///
/// Returns the unpacked f64 values.
pub fn unpack_typed_array(bytes: &[u8]) -> Vec<f64> {
    bytes
        .chunks_exact(8)
        .map(|chunk| {
            let arr: [u8; 8] = chunk.try_into().unwrap();
            f64::from_le_bytes(arr)
        })
        .collect()
}
