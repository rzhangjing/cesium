//! Tests for `cesium_core::BoxOutlineGeometry`.

use cesium_core::box_outline_geometry::BoxOutlineGeometry;
use cesium_core::cartesian3::Cartesian3;

#[test]
fn new_creates_outline_box() {
    let min = Cartesian3::new(-1.0, -1.0, -1.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let geo = BoxOutlineGeometry::new(&min, &max, None);
    let geom = geo.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn from_dimensions_creates_centered() {
    let dims = Cartesian3::new(2.0, 2.0, 2.0);
    let geo = BoxOutlineGeometry::from_dimensions(&dims, None);
    let geom = geo.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn create_geometry_has_bounding_sphere() {
    let dims = Cartesian3::new(2.0, 4.0, 6.0);
    let geo = BoxOutlineGeometry::from_dimensions(&dims, None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.bounding_sphere.is_some());
}

#[test]
fn pack_and_unpack_roundtrip() {
    let min = Cartesian3::new(-1.0, -2.0, -3.0);
    let max = Cartesian3::new(1.0, 2.0, 3.0);
    let original = BoxOutlineGeometry::new(&min, &max, None);
    let mut array = vec![0.0f64; BoxOutlineGeometry::PACKED_LENGTH];
    original.pack(&mut array, None);
    let unpacked = BoxOutlineGeometry::unpack(&array, None, None);
    assert!(unpacked.create_geometry().is_some());
}

#[test]
fn create_geometry_returns_none_for_zero_box() {
    let min = Cartesian3::new(0.0, 0.0, 0.0);
    let max = Cartesian3::new(0.0, 0.0, 0.0);
    let geo = BoxOutlineGeometry::new(&min, &max, None);
    assert!(geo.create_geometry().is_none());
}
