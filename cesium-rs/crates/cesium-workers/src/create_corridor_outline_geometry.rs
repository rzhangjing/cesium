//! Ported from `packages/engine/Source/Workers/createCorridorOutlineGeometry.js`.
//!
//! Worker entry point for creating corridor outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::corridor_outline_geometry::CorridorOutlineGeometry;
use cesium_core::geometry::Geometry;

/// Creates corridor outline geometry in a worker.
///
/// In CesiumJS, this unpacks a `CorridorOutlineGeometry` from the packed
/// parameters and returns `CorridorOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_corridor_outline_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_corridor_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCorridorOutlineGeometry"))
}

/// Creates a corridor outline from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `CorridorOutlineGeometry`
/// and delegates to the ported `CorridorOutlineGeometry.createGeometry`.
pub fn create_corridor_outline_geometry_unpacked(
    positions: &[Cartesian3],
    width: f64,
) -> Option<Geometry> {
    let corridor = CorridorOutlineGeometry::new(
        positions.to_vec(),
        width,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    cesium_core::corridor_outline_geometry::create_geometry(&corridor)
}
