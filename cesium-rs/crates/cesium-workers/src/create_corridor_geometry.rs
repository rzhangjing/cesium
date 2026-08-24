//! Ported from `packages/engine/Source/Workers/createCorridorGeometry.js`.
//!
//! Worker entry point for creating corridor geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::corridor_geometry::CorridorGeometry;
use cesium_core::geometry::Geometry;

/// Creates corridor geometry in a worker.
///
/// In CesiumJS, this unpacks a `CorridorGeometry` from the packed
/// parameters and returns `CorridorGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_corridor_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_corridor_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCorridorGeometry"))
}

/// Creates a corridor geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `CorridorGeometry` and
/// delegates to the ported `CorridorGeometry.createGeometry`.
///
/// # Arguments
/// * `positions` - The centerline positions of the corridor.
/// * `width` - The corridor width in meters.
/// * `height` - Height above the ellipsoid.
/// * `extruded_height` - Extruded height (0.0 for flat).
pub fn create_corridor_geometry_unpacked(
    positions: &[Cartesian3],
    width: f64,
    height: f64,
    extruded_height: f64,
) -> Option<Geometry> {
    let corridor = CorridorGeometry::new(
        positions.to_vec(),
        width,
        None,
        Some(cesium_core::vertex_format::VertexFormat::position_only()),
        Some(height),
        Some(extruded_height),
        None,
        None,
        None,
        None,
    );
    cesium_core::corridor_geometry::create_geometry(&corridor)
}
