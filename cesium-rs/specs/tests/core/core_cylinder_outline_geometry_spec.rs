//! Tests for `cesium_core::CylinderOutlineGeometry`.

use cesium_core::cylinder_outline_geometry::CylinderOutlineGeometry;

#[test]
fn new_creates_cylinder_outline() {
    let geo = CylinderOutlineGeometry::new(10.0, 5.0, 5.0, Some(32), None, None);
    let geom = geo.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn default_slices_and_vertical_lines() {
    let geo = CylinderOutlineGeometry::new(10.0, 5.0, 5.0, None, None, None);
    let geom = geo.create_geometry();
    assert!(geom.is_some());
}

#[test]
fn pack_and_unpack_roundtrip() {
    let original = CylinderOutlineGeometry::new(10.0, 3.0, 5.0, Some(16), Some(8), None);
    let mut array = vec![0.0f64; CylinderOutlineGeometry::PACKED_LENGTH];
    original.pack(&mut array, None);
    let unpacked = CylinderOutlineGeometry::unpack(&array, None);
    assert!(unpacked.create_geometry().is_some());
}

#[test]
fn create_geometry_has_bounding_sphere() {
    let geo = CylinderOutlineGeometry::new(10.0, 5.0, 5.0, Some(32), None, None);
    let geom = geo.create_geometry().unwrap();
    assert!(geom.bounding_sphere.is_some());
}
