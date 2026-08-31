//! Ported from `packages/engine/Source/Workers/createSphereOutlineGeometry.js`.
//!
//! Worker entry point for creating sphere outline geometry.

use cesium_core::sphere_outline_geometry::SphereOutlineGeometry;

/// Creates sphere outline geometry in a worker.
///
/// Deserializes sphere radius and center from packed bytes.
/// Constructs `SphereOutlineGeometry` and returns the packed result.
pub fn create_sphere_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createSphereOutlineGeometry"))
}

/// Creates a sphere outline from unpacked parameters (for in-process use).
///
/// Mirror of the JS worker body: the JS `SphereOutlineGeometry` options are
/// `radius`/partition counts only (the geometry is centered at the origin),
/// so the unpacked entry takes just the radius and delegates
/// `createGeometry` to the core port (partition defaults apply).
pub fn create_sphere_outline_geometry_unpacked(
    radius: f64,
) -> Option<cesium_core::geometry::Geometry> {
    let sphere_outline_geometry = SphereOutlineGeometry::new(Some(radius), None, None, None);
    SphereOutlineGeometry::create_geometry(&sphere_outline_geometry)
}
