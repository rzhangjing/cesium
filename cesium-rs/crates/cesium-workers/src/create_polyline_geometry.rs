//! Ported from `packages/engine/Source/Workers/createPolylineGeometry.js`.
//!
//! Worker entry point for creating polyline geometry on the ellipsoid.

use cesium_core::arc_type::ArcType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::polyline_geometry::PolylineGeometry;

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
/// Mirror of the JS worker body: builds `PolylineGeometry` from the
/// unpacked parameters and delegates `createGeometry` to the core port.
///
/// # Arguments
/// * `positions` - The polyline vertex positions.
/// * `width` - The polyline width in pixels.
/// * `follow_surface` - Whether the polyline follows the ellipsoid surface
///   (maps to the JS `arcType` option: geodesic when true, none otherwise).
pub fn create_polyline_geometry_unpacked(
    positions: &[Cartesian3],
    width: f64,
    follow_surface: bool,
) -> Option<cesium_core::geometry::Geometry> {
    let arc_type = if follow_surface {
        ArcType::Geodesic
    } else {
        ArcType::None
    };
    let polyline_geometry = PolylineGeometry::new(
        positions.to_vec(),
        Some(width),
        None,
        None,
        Some(arc_type),
        None,
        None,
    );
    PolylineGeometry::create_geometry(&polyline_geometry)
}
