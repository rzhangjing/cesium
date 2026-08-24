//! Ported from `packages/engine/Source/Workers/createBoxOutlineGeometry.js`.
//!
//! Worker entry point for creating box outline geometry.

use cesium_core::box_outline_geometry::BoxOutlineGeometry;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::geometry::Geometry;

/// Creates box outline geometry in a worker.
///
/// In CesiumJS, this unpacks a `BoxOutlineGeometry` from the packed
/// parameters and returns `BoxOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_box_outline_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_box_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createBoxOutlineGeometry"))
}

/// Creates a box outline geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs a `BoxOutlineGeometry` from the
/// min/max corners and delegates to the ported
/// `BoxOutlineGeometry.createGeometry`.
pub fn create_box_outline_geometry_unpacked(
    minimum: &Cartesian3,
    maximum: &Cartesian3,
) -> Option<Geometry> {
    BoxOutlineGeometry::new(minimum, maximum, None).create_geometry()
}
