//! Mirror of `packages/engine/Specs/Core/PolygonGeometrySpec.js`.
//!
//! Only the `createGeometry`-returns-undefined tests are ported, as the
//! full vertex-count assertions depend on `EllipsoidTangentPlane` which
//! is DEVIATED (see module docs of `polygon_geometry`).
//!
//! DEVIATION: JS uses `PolygonGeometry.fromPositions` factory;
//! the Rust port uses `PolygonGeometry::new` directly.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::polygon_geometry::{create_geometry, PolygonGeometry};

#[test]
fn returns_undefined_with_less_than_three_positions() {
    let geometry = create_geometry(&PolygonGeometry::new(
        vec![Cartesian3::default()],
        None,
        None,
        None,
        None,
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
    let geometry = create_geometry(&PolygonGeometry::new(
        positions,
        None,
        None,
        None,
        None,
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
