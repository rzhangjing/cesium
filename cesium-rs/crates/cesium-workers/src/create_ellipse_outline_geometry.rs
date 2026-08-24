//! Ported from `packages/engine/Source/Workers/createEllipseOutlineGeometry.js`.
//!
//! Worker entry point for creating ellipse outline geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipse_outline_geometry::EllipseOutlineGeometry;
use cesium_core::geometry::Geometry;

/// Creates ellipse outline geometry in a worker.
///
/// In CesiumJS, this unpacks an `EllipseOutlineGeometry` from the packed
/// parameters and returns `EllipseOutlineGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_ellipse_outline_geometry_unpacked`] for
/// in-process geometry creation.
pub fn create_ellipse_outline_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createEllipseOutlineGeometry"))
}

/// Creates an ellipse outline from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs an `EllipseOutlineGeometry`
/// and delegates to the ported `EllipseOutlineGeometry.createGeometry`.
pub fn create_ellipse_outline_geometry_unpacked(
    center: &Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
) -> Option<Geometry> {
    let ellipse = EllipseOutlineGeometry::new(
        *center,
        semi_major_axis,
        semi_minor_axis,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    cesium_core::ellipse_outline_geometry::create_geometry(&ellipse)
}
