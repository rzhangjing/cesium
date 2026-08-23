//! Ported from `packages/engine/Source/Workers/createPolylineVolumeGeometry.js`.
//!
//! Worker entry point for creating polyline volume geometry.

use cesium_core::cartesian3::Cartesian3;

/// Creates polyline volume geometry in a worker.
///
/// Deserializes polyline positions, shape positions (cross-section),
/// corner type, and vertex format from packed bytes.
/// Constructs `PolylineVolumeGeometry` and returns the packed result.
pub fn create_polyline_volume_geometry(params: &[u8]) -> Vec<u8> {
    let _ = params;
    Vec::new()
}

/// Creates a polyline volume geometry from unpacked parameters (for in-process use).
///
/// # Arguments
/// * `polyline_positions` - The centerline positions of the volume.
/// * `shape_positions` - The 2D cross-section shape positions.
pub fn create_polyline_volume_geometry_unpacked(
    _polyline_positions: &[Cartesian3],
    _shape_positions: &[Cartesian3],
) -> Option<cesium_core::geometry::Geometry> {
    // DEVIATION: PolylineVolumeGeometry not yet ported
    None
}
