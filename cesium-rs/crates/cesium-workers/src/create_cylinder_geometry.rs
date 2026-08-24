//! Ported from `packages/engine/Source/Workers/createCylinderGeometry.js`.
//!
//! Worker entry point for creating cylinder geometry.

use cesium_core::cylinder_geometry::CylinderGeometry;
use cesium_core::geometry::Geometry;
use cesium_core::vertex_format::VertexFormat;

/// Creates cylinder geometry in a worker.
///
/// In CesiumJS, this unpacks a `CylinderGeometry` from the packed
/// parameters and returns `CylinderGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_cylinder_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_cylinder_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCylinderGeometry"))
}

/// Creates a cylinder geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `CylinderGeometry` and
/// delegates to the ported `CylinderGeometry.createGeometry`.
///
/// # Arguments
/// * `length` - The cylinder length.
/// * `top_radius` - Radius at the top cap.
/// * `bottom_radius` - Radius at the bottom cap.
/// * `slices` - Number of radial subdivisions.
pub fn create_cylinder_geometry_unpacked(
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: u32,
) -> Option<Geometry> {
    CylinderGeometry::new(
        length,
        top_radius,
        bottom_radius,
        Some(slices as usize),
        Some(VertexFormat::position_only()),
        None,
    )
    .create_geometry()
}
