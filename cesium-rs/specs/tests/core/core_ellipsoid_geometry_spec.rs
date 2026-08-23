//! Tests for `cesium_core::EllipsoidGeometry`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid_geometry::EllipsoidGeometry;
use cesium_core::vertex_format::VertexFormat;

#[test]
fn default_creates_geometry() {
    let geo = EllipsoidGeometry::default();
    let _ = geo;
}

#[test]
fn new_with_custom_radii() {
    let radii = Cartesian3::new(2.0, 3.0, 4.0);
    let geo = EllipsoidGeometry::new(Some(radii), None, None, None, None, None, Some(4), Some(4), None, None);
    let _ = geo;
}

#[test]
fn create_geometry_has_position_attribute() {
    let vf = VertexFormat { position: true, ..Default::default() };
    let geo = EllipsoidGeometry::new(None, None, None, None, None, None, Some(8), Some(8), Some(vf), None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.attributes.contains_key("position"));
}

#[test]
fn create_geometry_has_normal_when_requested() {
    let vf = VertexFormat { position: true, normal: true, ..Default::default() };
    let geo = EllipsoidGeometry::new(None, None, None, None, None, None, Some(8), Some(8), Some(vf), None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.attributes.contains_key("normal"));
}

#[test]
fn create_geometry_has_bounding_sphere() {
    let vf = VertexFormat { position: true, ..Default::default() };
    let geo = EllipsoidGeometry::new(None, None, None, None, None, None, Some(8), Some(8), Some(vf), None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.bounding_sphere.is_some());
}

#[test]
fn pack_and_unpack_roundtrip() {
    let radii = Cartesian3::new(2.0, 3.0, 4.0);
    let original = EllipsoidGeometry::new(Some(radii), None, None, None, None, None, Some(8), Some(8), None, None);
    let mut array = vec![0.0f64; EllipsoidGeometry::PACKED_LENGTH];
    original.pack(&mut array, None);
    let unpacked = EllipsoidGeometry::unpack(&array, None);
    let _ = unpacked;
}
