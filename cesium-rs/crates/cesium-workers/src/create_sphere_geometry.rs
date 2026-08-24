//! Ported from `packages/engine/Source/Workers/createSphereGeometry.js`.
//!
//! Worker entry point for creating sphere geometry.

use cesium_core::geometry::Geometry;
use cesium_core::sphere_geometry::SphereGeometry;
use cesium_core::vertex_format::VertexFormat;

/// Creates sphere geometry in a worker.
///
/// In CesiumJS, this unpacks a `SphereGeometry` from the packed
/// parameters and returns `SphereGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_sphere_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_sphere_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createSphereGeometry"))
}

/// Creates a sphere geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `SphereGeometry` and
/// delegates to the ported `SphereGeometry.createGeometry`.
/// (CesiumJS `SphereGeometry` is centered at the origin; it has no
/// center parameter.)
///
/// # Arguments
/// * `radius` - The sphere radius in meters.
pub fn create_sphere_geometry_unpacked(radius: f64) -> Option<Geometry> {
    SphereGeometry::new(
        Some(radius),
        None,
        None,
        Some(VertexFormat::position_only()),
    )
    .create_geometry()
}
