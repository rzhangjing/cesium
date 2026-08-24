//! Ported from `packages/engine/Source/Workers/createPolygonGeometry.js`.
//!
//! Worker entry point for creating polygon geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;
use cesium_core::polygon_geometry::PolygonGeometry;

/// Creates polygon geometry in a worker.
///
/// In CesiumJS, this unpacks a `PolygonGeometry` from the packed
/// parameters and returns `PolygonGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_polygon_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_polygon_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPolygonGeometry"))
}

/// Creates a polygon geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `PolygonGeometry` and
/// delegates to the ported `PolygonGeometry.createGeometry`.
///
/// # Arguments
/// * `polygon_hierarchy` - Flattened polygon positions (outer ring).
/// * `height` - Height above the ellipsoid surface.
/// * `extruded_height` - Extruded height (0.0 for flat polygon).
pub fn create_polygon_geometry_unpacked(
    polygon_hierarchy: &[Cartesian3],
    height: f64,
    extruded_height: f64,
) -> Option<Geometry> {
    let polygon = PolygonGeometry::new(
        polygon_hierarchy.to_vec(),
        None,
        Some(cesium_core::vertex_format::VertexFormat::position_only()),
        Some(height),
        Some(extruded_height),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    cesium_core::polygon_geometry::create_geometry(&polygon)
}
