//! Ported from `packages/engine/Source/Workers/createFrustumGeometry.js`.
//!
//! Worker entry point for creating frustum geometry.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::frustum_geometry::FrustumGeometry;

/// Creates frustum geometry in a worker.
///
/// Deserializes frustum parameters (origin, direction, up, fov, aspect ratio,
/// near, far) from packed bytes, constructs `FrustumGeometry`, and returns
/// the packed result.
pub fn create_frustum_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createFrustumGeometry"))
}

/// Creates a frustum geometry from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: the JS `FrustumGeometry` options are
/// `frustum`/`origin`/`orientation` (a unit quaternion packed as a
/// Cartesian4), so the unpacked entry takes the orientation directly and
/// delegates `createGeometry` to the core port.
pub fn create_frustum_geometry_unpacked(
    origin: &Cartesian3,
    orientation: &Cartesian4,
    near: f64,
    far: f64,
    fov: f64,
    aspect_ratio: f64,
) -> Option<cesium_core::geometry::Geometry> {
    let frustum_geometry = FrustumGeometry::new(
        origin.clone(),
        orientation.clone(),
        near,
        far,
        fov,
        aspect_ratio,
    );
    FrustumGeometry::create_geometry(&frustum_geometry)
}
