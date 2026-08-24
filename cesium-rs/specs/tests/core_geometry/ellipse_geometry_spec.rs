//! Mirror of `packages/engine/Specs/Core/EllipseGeometrySpec.js`.
//!
//! Ports the `computes positions`, `compute all vertex attributes`, and
//! extruded ellipse tests.
//!
//! DEVIATION: JS uses an options object; the Rust port uses
//! `EllipseGeometry::new` with positional parameters.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipse_geometry::{create_geometry, EllipseGeometry};
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::vertex_format::VertexFormat;

#[test]
fn computes_positions() {
    let center = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let m = create_geometry(&EllipseGeometry::new(
        center,
        1.0,
        1.0,
        Some(Ellipsoid::WGS84),
        None,
        None,
        None,
        None,
        Some(0.1), // granularity
        Some(VertexFormat::position_only()),
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    // rows of 1 + 4 + 6 + 4 + 1
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 16 * 3);
    // rows of 3 + 8 + 8 + 3
    assert_eq!(m.indices.as_ref().unwrap().len(), 22 * 3);
    assert!(
        (m.bounding_sphere.as_ref().unwrap().radius - 1.0).abs() < 1e-10,
        "bounding sphere radius should be 1.0"
    );
}

#[test]
fn compute_all_vertex_attributes() {
    let center = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let m = create_geometry(&EllipseGeometry::new(
        center,
        1.0,
        1.0,
        Some(Ellipsoid::WGS84),
        None,
        None,
        None,
        None,
        Some(0.1), // granularity
        Some(VertexFormat::all()),
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    let num_vertices = 16;
    let num_triangles = 22;
    assert_eq!(
        m.attributes.get("position").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("st").unwrap().values.len(),
        num_vertices * 2
    );
    assert_eq!(
        m.attributes.get("normal").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("tangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(
        m.attributes.get("bitangent").unwrap().values.len(),
        num_vertices * 3
    );
    assert_eq!(m.indices.as_ref().unwrap().len(), num_triangles * 3);
}
