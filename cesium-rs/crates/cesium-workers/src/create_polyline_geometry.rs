//! Ported from `packages/engine/Source/Workers/createPolylineGeometry.js`.
//!
//! Worker entry point for creating polyline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;

/// Creates polyline geometry in a worker.
///
/// Deserializes polyline positions, width, colors, and arc type
/// from packed bytes. Constructs `PolylineGeometry` and returns packed result.
pub fn create_polyline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPolylineGeometry"))
}

/// Creates a polyline geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `positions` - The polyline vertex positions.
/// * `width` - The polyline width in pixels.
/// * `follow_surface` - Whether the polyline follows the ellipsoid curvature.
pub fn create_polyline_geometry_unpacked(
    _positions: &[Cartesian3],
    _width: f64,
    _follow_surface: bool,
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PolylineGeometry not yet ported
    None
}
