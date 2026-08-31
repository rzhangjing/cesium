//! Ported from `packages/engine/Source/Workers/createCircleOutlineGeometry.js`.
//!
//! Worker entry point for creating circle outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::circle_outline_geometry::CircleOutlineGeometry;

/// Creates circle outline geometry in a worker.
///
/// Deserializes center, radius, and ellipsoid from packed bytes.
/// Constructs `CircleOutlineGeometry` and returns the packed result.
pub fn create_circle_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCircleOutlineGeometry"))
}

/// Creates a circle outline geometry from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: builds the outline from center/radius
/// (remaining options default as in the JS constructor) and delegates
/// `createGeometry` to the core port.
pub fn create_circle_outline_geometry_unpacked(
    center: &Cartesian3,
    radius: f64,
) -> Option<cesium_core::geometry::Geometry> {
    let circle_outline_geometry =
        CircleOutlineGeometry::new(center.clone(), radius, None, None, None, None, None);
    CircleOutlineGeometry::create_geometry(&circle_outline_geometry)
}
