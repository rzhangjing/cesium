//! Ported from `packages/engine/Source/Workers/createEllipsoidGeometry.js`.
//!
//! Worker entry point for creating ellipsoid geometry.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid_geometry::EllipsoidGeometry;
use cesium_core::geometry::Geometry;
use cesium_core::vertex_format::VertexFormat;

/// Creates ellipsoid geometry in a worker.
///
/// In CesiumJS, this unpacks an `EllipsoidGeometry` from the packed
/// parameters and returns `EllipsoidGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_ellipsoid_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_ellipsoid_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createEllipsoidGeometry"))
}

/// Creates an ellipsoid geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs an `EllipsoidGeometry` and
/// delegates to the ported `EllipsoidGeometry.createGeometry`.
/// (CesiumJS `EllipsoidGeometry` is centered at the origin; it has no
/// center parameter.)
///
/// # Arguments
/// * `radii` - The radii along x, y, z axes.
pub fn create_ellipsoid_geometry_unpacked(radii: &Cartesian3) -> Option<Geometry> {
    let ellipsoid_geometry = EllipsoidGeometry::new(
        Some(*radii),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(VertexFormat::position_only()),
        None,
    );
    ellipsoid_geometry.create_geometry()
}
