//! Tests for `cesium_core::BoxGeometry`.

use cesium_core::box_geometry::BoxGeometry;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::vertex_format::VertexFormat;

#[test]
fn new_creates_box_with_min_max() {
    let min = Cartesian3::new(-1.0, -1.0, -1.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let box_geo = BoxGeometry::new(&min, &max, None, None);
    let geom = box_geo.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn from_dimensions_creates_centered_box() {
    let dims = Cartesian3::new(2.0, 2.0, 2.0);
    let box_geo = BoxGeometry::from_dimensions(&dims, None, None);
    let geom = box_geo.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn create_geometry_returns_none_for_zero_box() {
    let min = Cartesian3::new(0.0, 0.0, 0.0);
    let max = Cartesian3::new(0.0, 0.0, 0.0);
    let box_geo = BoxGeometry::new(&min, &max, None, None);
    assert!(box_geo.create_geometry().is_none());
}

#[test]
fn create_geometry_has_position_attribute() {
    let dims = Cartesian3::new(1.0, 1.0, 1.0);
    let vf = VertexFormat { position: true, ..Default::default() };
    let box_geo = BoxGeometry::from_dimensions(&dims, Some(vf), None);
    let geom = box_geo.create_geometry().unwrap();
    assert!(geom.attributes.contains_key("position"));
}

#[test]
fn create_geometry_has_bounding_sphere() {
    let dims = Cartesian3::new(2.0, 4.0, 6.0);
    let box_geo = BoxGeometry::from_dimensions(&dims, None, None);
    let geom = box_geo.create_geometry().unwrap();
    assert!(geom.bounding_sphere.is_some());
}

#[test]
fn pack_and_unpack_roundtrip() {
    let min = Cartesian3::new(-1.0, -2.0, -3.0);
    let max = Cartesian3::new(1.0, 2.0, 3.0);
    let original = BoxGeometry::new(&min, &max, None, None);
    let mut array = vec![0.0f64; BoxGeometry::PACKED_LENGTH];
    original.pack(&mut array, None);
    let unpacked = BoxGeometry::unpack(&array, None, None);
    let geom = unpacked.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn create_geometry_with_normal_attribute() {
    let dims = Cartesian3::new(1.0, 1.0, 1.0);
    let vf = VertexFormat { position: true, normal: true, ..Default::default() };
    let box_geo = BoxGeometry::from_dimensions(&dims, Some(vf), None);
    let geom = box_geo.create_geometry().unwrap();
    assert!(geom.attributes.contains_key("normal"));
}
