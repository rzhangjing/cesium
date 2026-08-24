//! Ported from `packages/engine/Source/Workers/createGroundPolylineGeometry.js`.
//!
//! Worker entry point for creating ground-clamped polyline geometry.
//! This generates geometry that is draped onto the terrain surface.

use cesium_core::cartesian3::Cartesian3;

/// Creates ground polyline geometry in a worker.
///
/// Deserializes polyline positions, width, and arc type from packed bytes.
/// Constructs `GroundPolylineGeometry` and returns the packed result.
pub fn create_ground_polyline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createGroundPolylineGeometry"))
}

/// Creates a ground polyline geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `positions` - The polyline positions (will be clamped to ground).
/// * `width` - The polyline width in pixels.
pub fn create_ground_polyline_geometry_unpacked(
    _positions: &[Cartesian3],
    _width: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: GroundPolylineGeometry not yet ported
    None
}
