//! Mirror of `packages/engine/Specs/Core/EllipseOutlineGeometrySpec.js`.
//!
//! Ports the `computes positions` and `computes positions extruded` tests.
//!
//! DEVIATION: JS uses an options object; the Rust port uses
//! `EllipseOutlineGeometry::new` with positional parameters.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipse_outline_geometry::{create_geometry, EllipseOutlineGeometry};
use cesium_core::ellipsoid::Ellipsoid;

#[test]
fn computes_positions() {
    let center = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let m = create_geometry(&EllipseOutlineGeometry::new(
        center,
        1.0,
        1.0,
        Some(Ellipsoid::WGS84),
        None,
        None,
        None,
        Some(0.1), // granularity
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    assert_eq!(m.attributes.get("position").unwrap().values.len(), 8 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 8 * 2);
    assert!(
        (m.bounding_sphere.as_ref().unwrap().radius - 1.0).abs() < 1e-10,
        "bounding sphere radius should be 1.0"
    );
}

#[test]
fn computes_positions_extruded() {
    let center = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let m = create_geometry(&EllipseOutlineGeometry::new(
        center,
        1.0,
        1.0,
        Some(Ellipsoid::WGS84),
        None,
        Some(5.0), // extrudedHeight
        None,
        Some(0.1), // granularity
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 8 top + 8 bottom
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 16 * 3);
    // 8 top + 8 bottom + 8 sides
    assert_eq!(m.indices.as_ref().unwrap().len(), 24 * 2);
}
