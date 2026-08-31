//! Ported from `packages/engine/Source/Workers/createEllipsoidOutlineGeometry.js`.
//!
//! Worker entry point for creating ellipsoid outline geometry.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid_outline_geometry::EllipsoidOutlineGeometry;

/// Creates ellipsoid outline geometry in a worker.
///
/// Deserializes center and radii from packed bytes.
/// Constructs `EllipsoidOutlineGeometry` and returns the packed result.
pub fn create_ellipsoid_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createEllipsoidOutlineGeometry"))
}

/// Creates an ellipsoid outline from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: the JS `EllipsoidOutlineGeometry` options
/// carry `radii` (the geometry is centered at the origin), so the unpacked
/// entry takes just the radii and delegates `createGeometry` to the core
/// port (remaining options default).
pub fn create_ellipsoid_outline_geometry_unpacked(
    radii: &Cartesian3,
) -> Option<cesium_core::geometry::Geometry> {
    let ellipsoid_outline_geometry = EllipsoidOutlineGeometry::new(
        Some(radii.clone()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    ellipsoid_outline_geometry.create_geometry()
}
