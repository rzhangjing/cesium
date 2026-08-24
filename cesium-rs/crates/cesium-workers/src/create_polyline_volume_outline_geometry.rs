//! Ported from `packages/engine/Source/Workers/createPolylineVolumeOutlineGeometry.js`.
//!
//! Worker entry point for creating polyline volume outline geometry.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;
use cesium_core::polyline_volume_outline_geometry::PolylineVolumeOutlineGeometry;

/// Creates polyline volume outline geometry in a worker.
///
/// In CesiumJS, this unpacks a `PolylineVolumeOutlineGeometry` from the
/// packed parameters and returns
/// `PolylineVolumeOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_polyline_volume_outline_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_polyline_volume_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPolylineVolumeOutlineGeometry"))
}

/// Creates a polyline volume outline from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `PolylineVolumeOutlineGeometry`
/// and delegates to the ported `PolylineVolumeOutlineGeometry.createGeometry`.
///
/// # Arguments
/// * `polyline_positions` - The centerline positions of the volume.
/// * `shape_positions` - The 2D cross-section shape positions.
pub fn create_polyline_volume_outline_geometry_unpacked(
    polyline_positions: &[Cartesian3],
    shape_positions: &[Cartesian2],
) -> Option<Geometry> {
    let volume = PolylineVolumeOutlineGeometry::new(
        polyline_positions.to_vec(),
        shape_positions.to_vec(),
        None,
        None,
        None,
    );
    cesium_core::polyline_volume_outline_geometry::create_geometry(&volume)
}
