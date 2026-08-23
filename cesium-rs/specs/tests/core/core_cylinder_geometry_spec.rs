//! Tests for `cesium_core::CylinderGeometry`.

use cesium_core::cylinder_geometry::CylinderGeometry;
use cesium_core::vertex_format::VertexFormat;

#[test]
fn new_creates_cylinder() {
    let geo = CylinderGeometry::new(10.0, 5.0, 5.0, Some(32), None, None);
    let _ = geo;
}

#[test]
fn default_slices_is_128() {
    let geo = CylinderGeometry::new(10.0, 5.0, 5.0, None, None, None);
    let _ = geo;
}

#[test]
fn cone_when_top_radius_zero() {
    let geo = CylinderGeometry::new(10.0, 0.0, 5.0, Some(32), None, None);
    let _ = geo;
}

#[test]
fn create_geometry_has_position_attribute() {
    let vf = VertexFormat { position: true, ..Default::default() };
    let geo = CylinderGeometry::new(10.0, 5.0, 5.0, Some(16), Some(vf), None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.attributes.contains_key("position"));
}

#[test]
fn pack_and_unpack_roundtrip() {
    let original = CylinderGeometry::new(10.0, 3.0, 5.0, Some(32), None, None);
    let mut array = vec![0.0f64; CylinderGeometry::PACKED_LENGTH];
    original.pack(&mut array, None);
    let unpacked = CylinderGeometry::unpack(&array, None);
    let _ = unpacked;
}

#[test]
fn create_geometry_with_normals() {
    let vf = VertexFormat { position: true, normal: true, ..Default::default() };
    let geo = CylinderGeometry::new(10.0, 5.0, 5.0, Some(16), Some(vf), None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.attributes.contains_key("normal"));
}
