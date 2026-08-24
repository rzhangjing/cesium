//! Ported from `packages/engine/Source/Workers/createPolylineVolumeGeometry.js`.
//!
//! Worker entry point for creating polyline volume geometry.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;
use cesium_core::polyline_volume_geometry::PolylineVolumeGeometry;

/// Creates polyline volume geometry in a worker.
///
/// In CesiumJS, this unpacks a `PolylineVolumeGeometry` from the packed
/// parameters and returns `PolylineVolumeGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_polyline_volume_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_polyline_volume_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPolylineVolumeGeometry"))
}

/// Creates a polyline volume geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `PolylineVolumeGeometry`
/// and delegates to the ported `PolylineVolumeGeometry.createGeometry`.
///
/// # Arguments
/// * `polyline_positions` - The centerline positions of the volume.
/// * `shape_positions` - The 2D cross-section shape positions.
pub fn create_polyline_volume_geometry_unpacked(
    polyline_positions: &[Cartesian3],
    shape_positions: &[Cartesian2],
) -> Option<Geometry> {
    let volume = PolylineVolumeGeometry::new(
        polyline_positions.to_vec(),
        shape_positions.to_vec(),
        None,
        None,
        Some(cesium_core::vertex_format::VertexFormat::position_only()),
        None,
    );
    cesium_core::polyline_volume_geometry::create_geometry(&volume)
}
