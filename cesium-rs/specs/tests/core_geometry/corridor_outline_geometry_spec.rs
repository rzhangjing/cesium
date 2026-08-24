//! Mirror of `packages/engine/Specs/Core/CorridorOutlineGeometrySpec.js`.
//!
//! Ports the `createGeometry`-returns-undefined, positions, and
//! extruded positions tests.
//!
//! DEVIATION: JS uses an options object; the Rust port uses
//! `CorridorOutlineGeometry::new` with positional parameters.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::corner_type::CornerType;
use cesium_core::corridor_outline_geometry::{create_geometry, CorridorOutlineGeometry};

#[test]
fn create_geometry_returns_undefined_without_2_unique_positions() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -30.0], None, None);
    let geometry = create_geometry(&CorridorOutlineGeometry::new(
        positions,
        10000.0,
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
fn computes_positions() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let m = create_geometry(&CorridorOutlineGeometry::new(
        positions,
        30000.0,
        None,
        None,
        None,
        Some(CornerType::Mitered),
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 6 left + 6 right
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 12 * 3);
    assert_eq!(m.indices.as_ref().unwrap().len(), 12 * 2);
}

#[test]
fn computes_positions_extruded() {
    let positions =
        Cartesian3::from_degrees_array(&[90.0, -30.0, 90.0, -35.0], None, None);
    let m = create_geometry(&CorridorOutlineGeometry::new(
        positions,
        30000.0,
        None,
        None,
        Some(30000.0), // extrudedHeight
        Some(CornerType::Mitered),
        None,
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 6 positions * 4 for a box at each position
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 24 * 3);
    // 5 segments * 4 lines per segment + 4 lines * 2 ends
    assert_eq!(m.indices.as_ref().unwrap().len(), 28 * 2);
}
