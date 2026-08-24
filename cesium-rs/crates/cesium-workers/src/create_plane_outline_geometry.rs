//! Ported from `packages/engine/Source/Workers/createPlaneOutlineGeometry.js`.
//!
//! Worker entry point for creating plane outline geometry.

use cesium_core::geometry::Geometry;
use cesium_core::plane_outline_geometry::PlaneOutlineGeometry;

/// Creates plane outline geometry in a worker.
///
/// In CesiumJS, this returns `PlaneOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_plane_outline_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_plane_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createPlaneOutlineGeometry"))
}

/// Creates a plane outline from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: `PlaneOutlineGeometry.createGeometry()`.
/// CesiumJS `PlaneOutlineGeometry` takes no parameters (unit plane
/// outline centered at the origin).
pub fn create_plane_outline_geometry_unpacked() -> Option<Geometry> {
    Some(PlaneOutlineGeometry::create_geometry())
}
