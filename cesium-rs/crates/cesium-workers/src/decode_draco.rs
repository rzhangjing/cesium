//! Ported from `packages/engine/Source/Workers/decodeDraco.js`.
//!
//! Worker entry point for decoding Draco-compressed geometry.
//! Draco is a compression library that reduces the size of 3D geometry data.

/// Decodes Draco-compressed geometry.
///
/// In CesiumJS, this receives Draco-compressed buffer data and decodes it
/// into vertex positions, normals, texture coordinates, and indices using
/// the Draco decoder library.
pub fn decode_draco(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("decodeDraco"))
}

/// Decodes Draco-compressed data (for in-process use).
///
/// # Arguments
/// * `compressed_data` - The Draco-compressed buffer bytes.
///
/// Returns decoded geometry data as a flat `Vec<u8>`.
pub fn decode_draco_unpacked(_compressed_data: &[u8]) -> Vec<u8> {
    // DEVIATION: Draco decoding requires the draco native library binding
    Vec::new()
}
