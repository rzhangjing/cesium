//! Mirror of `packages/engine/Specs/Core/PolylineVolumeOutlineGeometrySpec.js`.
//!
//! Ports the `createGeometry`-returns-undefined and computes-positions tests.
//!
//! DEVIATION: JS uses an options object; the Rust port uses
//! `PolylineVolumeOutlineGeometry::new` with positional parameters.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::corner_type::CornerType;
use cesium_core::polyline_volume_outline_geometry::{
    create_geometry, PolylineVolumeOutlineGeometry,
};

fn box_shape() -> Vec<Cartesian2> {
    vec![
        Cartesian2::new(-100.0, -100.0),
        Cartesian2::new(100.0, -100.0),
        Cartesian2::new(100.0, 100.0),
        Cartesian2::new(-100.0, 100.0),
    ]
}

#[test]
fn create_geometry_returns_undefined_without_2_unique_polyline_positions() {
    let geometry = create_geometry(&PolylineVolumeOutlineGeometry::new(
        vec![Cartesian3::default()],
        box_shape(),
        None,
        None,
        None,
    ));
    assert!(geometry.is_none());
}

#[test]
fn create_geometry_returns_undefined_without_3_unique_shape_positions() {
    let geometry = create_geometry(&PolylineVolumeOutlineGeometry::new(
        vec![Cartesian3::UNIT_X, Cartesian3::UNIT_Y],
        vec![Cartesian2::UNIT_X, Cartesian2::UNIT_X, Cartesian2::UNIT_X],
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
    let m = create_geometry(&PolylineVolumeOutlineGeometry::new(
        positions,
        box_shape(),
        None,
        Some(CornerType::Mitered),
        None,
    ));
    let m = m.expect("geometry should not be None");

    // 6 polyline positions * 4 box positions
    assert_eq!(m.attributes.get("position").unwrap().values.len(), 24 * 3);
    // 4 lines * 5 positions + 4 lines * 2 end caps
    assert_eq!(m.indices.as_ref().unwrap().len(), 28 * 2);
}
