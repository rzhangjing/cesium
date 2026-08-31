//! Ported from `packages/engine/Source/Workers/createSimplePolylineGeometry.js`.
//!
//! Worker entry point for creating simple (non-geodesic) polyline geometry.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::simple_polyline_geometry::SimplePolylineGeometry;

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
/// Mirror of the JS worker body: the JS `SimplePolylineGeometry` options
/// are positions/colors/arcType only (no width — width belongs to
/// `PolylineGeometry`), so the unpacked entry takes just the positions and
/// delegates `createGeometry` to the core port.
///
/// # Arguments
/// * `positions` - The polyline vertex positions (straight-line segments).
pub fn create_simple_polyline_geometry_unpacked(
    positions: &[Cartesian3],
) -> Option<cesium_core::geometry::Geometry> {
    let simple_polyline_geometry =
        SimplePolylineGeometry::new(positions.to_vec(), None, None, None, None, None);
    simple_polyline_geometry.create_geometry()
}
