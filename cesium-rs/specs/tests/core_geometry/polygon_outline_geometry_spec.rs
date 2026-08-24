//! Mirror of `packages/engine/Specs/Core/PolygonOutlineGeometrySpec.js`.
//!
//! Only the `createGeometry`-returns-undefined tests are ported, as the
//! full vertex-count assertions depend on `EllipsoidTangentPlane` which
//! is DEVIATED (see module docs of `polygon_outline_geometry`).
//!
//! DEVIATION: JS uses `PolygonOutlineGeometry.fromPositions` factory;
//! the Rust port uses `PolygonOutlineGeometry::new` directly.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::polygon_outline_geometry::{create_geometry, PolygonOutlineGeometry};

#[test]
fn returns_undefined_with_less_than_three_positions() {
    let geometry = create_geometry(&PolygonOutlineGeometry::new(
        vec![Cartesian3::default()],
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    assert!(geometry.is_none());
}

#[test]
fn create_geometry_returns_undefined_due_to_duplicate_positions() {
    let positions =
        Cartesian3::from_degrees_array(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0], None, None);
    let geometry = create_geometry(&PolygonOutlineGeometry::new(
        positions,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    assert!(geometry.is_none());
}
