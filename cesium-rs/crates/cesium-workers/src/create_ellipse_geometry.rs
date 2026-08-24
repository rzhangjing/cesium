//! Ported from `packages/engine/Source/Workers/createEllipseGeometry.js`.
//!
//! Worker entry point for creating ellipse geometry on the ellipsoid.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipse_geometry::EllipseGeometry;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geometry::Geometry;
use cesium_core::vertex_format::VertexFormat;

/// Creates ellipse geometry in a worker.
///
/// In CesiumJS, this unpacks an `EllipseGeometry` from the packed
/// parameters and returns `EllipseGeometry.createGeometry(...)`.
/// The Rust packed byte entry is not implemented yet (no binary pack
/// format for geometry parameters/results), so it returns an explicit
/// error; use [`create_ellipse_geometry_unpacked`] for in-process
/// geometry creation.
pub fn create_ellipse_geometry(params: &[u8]) -> Result<Vec<u8>, String> {
    let _ = params;
    Err(crate::not_yet_ported_error("createEllipseGeometry"))
}

/// Creates an ellipse geometry from unpacked parameters (for in-process use).
///
/// Mirrors the JS worker body: constructs an `EllipseGeometry` and
/// delegates to the ported `EllipseGeometry.createGeometry`.
pub fn create_ellipse_geometry_unpacked(
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Option<Ellipsoid>,
) -> Option<Geometry> {
    let ellipse = EllipseGeometry::new(
        center,
        semi_major_axis,
        semi_minor_axis,
        ellipsoid,
        None,
        None,
        None,
        None,
        None,
        Some(VertexFormat::position_only()),
        None,
        None,
    );
    cesium_core::ellipse_geometry::create_geometry(&ellipse)
}
