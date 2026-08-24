//! Ported from `packages/engine/Source/Workers/createPolygonOutlineGeometry.js`.
//!
//! Worker entry point for creating polygon outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;
use cesium_core::polygon_outline_geometry::PolygonOutlineGeometry;

/// Creates polygon outline geometry in a worker.
///
/// In CesiumJS, this unpacks a `PolygonOutlineGeometry` from the packed
/// parameters and returns `PolygonOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_polygon_outline_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_polygon_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPolygonOutlineGeometry"))
}

/// Creates a polygon outline from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `PolygonOutlineGeometry`
/// and delegates to the ported `PolygonOutlineGeometry.createGeometry`.
pub fn create_polygon_outline_geometry_unpacked(
    polygon_hierarchy: &[Cartesian3],
    height: f64,
    extruded_height: f64,
) -> Option<Geometry> {
    let polygon = PolygonOutlineGeometry::new(
        polygon_hierarchy.to_vec(),
        None,
        Some(height),
        Some(extruded_height),
        None,
        None,
        None,
        None,
    );
    cesium_core::polygon_outline_geometry::create_geometry(&polygon)
}
