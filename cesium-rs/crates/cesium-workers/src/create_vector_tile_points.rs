//! Ported from `packages/engine/Source/Workers/createVectorTilePoints.js`.
//!
//! Worker entry point for creating vector tile point features.

/// Creates vector tile points.
///
/// In CesiumJS, this receives vector tile data and extracts point features,
/// creating point primitives for rendering.
pub fn create_vector_tile_points(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createVectorTilePoints"))
}

/// Creates vector tile points (for in-process use).
///
/// # Arguments
/// * `tile_data` - Vector tile binary data.
///
/// Returns serialized point feature data.
pub fn create_vector_tile_points_unpacked(_tile_data: &[u8]) -> Vec<u8> {
    // DEVIATION: Vector tile point extraction not yet implemented
    Vec::new()
}
