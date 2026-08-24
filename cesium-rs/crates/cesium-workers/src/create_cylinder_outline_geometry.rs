//! Ported from `packages/engine/Source/Workers/createCylinderOutlineGeometry.js`.
//!
//! Worker entry point for creating cylinder outline geometry.

use cesium_core::cylinder_outline_geometry::CylinderOutlineGeometry;
use cesium_core::geometry::Geometry;

/// Creates cylinder outline geometry in a worker.
///
/// In CesiumJS, this unpacks a `CylinderOutlineGeometry` from the packed
/// parameters and returns `CylinderOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_cylinder_outline_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_cylinder_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createCylinderOutlineGeometry"))
}

/// Creates a cylinder outline from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `CylinderOutlineGeometry`
/// and delegates to the ported `CylinderOutlineGeometry.createGeometry`.
pub fn create_cylinder_outline_geometry_unpacked(
    length: f64,
    top_radius: f64,
    bottom_radius: f64,
    slices: u32,
) -> Option<Geometry> {
    CylinderOutlineGeometry::new(
        length,
        top_radius,
        bottom_radius,
        Some(slices as usize),
        None,
        None,
    )
    .create_geometry()
}
