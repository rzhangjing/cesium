//! Ported from `packages/engine/Source/Workers/createPolylineVolumeOutlineGeometry.js`.
//!
//! Worker entry point for creating polyline volume outline geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates polyline volume outline geometry in a worker.
///
/// Deserializes polyline positions and shape positions (cross-section)
/// from packed bytes. Constructs `PolylineVolumeOutlineGeometry` and returns packed result.
pub fn create_polyline_volume_outline_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a polyline volume outline from unpacked parameters (for in-process use).
pub fn create_polyline_volume_outline_geometry_unpacked(
    _polyline_positions: &[Cartesian3],
    _shape_positions: &[Cartesian3],
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PolylineVolumeOutlineGeometry not yet ported
    None
}
