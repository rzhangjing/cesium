//! Ported from `packages/engine/Source/Workers/createBoxGeometry.js`.
//!
//! Worker entry point for creating box geometry.

use cesium_core::box_geometry::BoxGeometry;
use cesium_core::cartesian3::Cartesian3;

/// Parameters for box geometry creation.
#[derive(Debug, Clone)]
pub struct CreateBoxGeometryParameters {
    /// Minimum corner of the box.
    pub minimum: Cartesian3,
    /// Maximum corner of the box.
    pub maximum: Cartesian3,
}

/// Creates box geometry in a worker.
///
/// In CesiumJS, this receives packed parameters from the main thread,
/// constructs a `BoxGeometry`, calls `createGeometry()`, and returns
/// the packed result. The Rust packed byte entry is not implemented yet
/// (no binary pack format for geometry parameters/results), so it
/// returns an explicit error; use [`create_box_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_box_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createBoxGeometry"))
}

/// Creates a `BoxGeometry` from unpacked parameters (for in-process use).
pub fn create_box_geometry_unpacked(minimum: &Cartesian3, maximum: &Cartesian3) -> Option<cesium_core::geometry::Geometry> {
    let box_geom = BoxGeometry::new(minimum, maximum, None, None);
    box_geom.create_geometry()
}
