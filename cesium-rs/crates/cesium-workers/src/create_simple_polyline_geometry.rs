//! Ported from `packages/engine/Source/Workers/createSimplePolylineGeometry.js`.
//!
//! Worker entry point for creating simple (non-geodesic) polyline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates simple polyline geometry in a worker.
///
/// Deserializes polyline positions and width from packed bytes.
/// Constructs `SimplePolylineGeometry` and returns the packed result.
pub fn create_simple_polyline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createSimplePolylineGeometry"))
}

/// Creates a simple polyline geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `positions` - The polyline vertex positions (straight-line segments).
/// * `width` - The polyline width in pixels.
pub fn create_simple_polyline_geometry_unpacked(
    _positions: &[Cartesian3],
    _width: f64,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: SimplePolylineGeometry not yet ported
    None
}
