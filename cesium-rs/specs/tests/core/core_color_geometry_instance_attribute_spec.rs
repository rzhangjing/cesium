//! Port of `Core/ColorGeometryInstanceAttributeSpec.js`.

use cesium_core::color_geometry_instance_attribute::ColorGeometryInstanceAttribute;
use cesium_core::component_datatype::ComponentDatatype;

#[test]
fn constructor_sets_values() {
    let attr = ColorGeometryInstanceAttribute::new(Some(1.0), Some(1.0), Some(0.0), Some(0.5));
    assert_eq!(attr.value, vec![1.0, 1.0, 0.0, 0.5]);
}

#[test]
fn component_datatype_is_unsigned_byte() {
    assert_eq!(
        ColorGeometryInstanceAttribute::component_datatype(),
        ComponentDatatype::UnsignedByte
    );
}

#[test]
fn components_per_attribute_is_four() {
    assert_eq!(ColorGeometryInstanceAttribute::components_per_attribute(), 4);
}

#[test]
fn normalize_is_true() {
    assert!(ColorGeometryInstanceAttribute::normalize());
}

#[test]
fn equals_works() {
    let a = ColorGeometryInstanceAttribute::new(Some(0.1), Some(0.2), Some(0.3), Some(0.4));
    let b = ColorGeometryInstanceAttribute::new(Some(0.1), Some(0.2), Some(0.3), Some(0.4));
    let c = ColorGeometryInstanceAttribute::new(Some(0.5), Some(0.2), Some(0.3), Some(0.4));
    assert!(ColorGeometryInstanceAttribute::equals(&a, &b));
    assert!(!ColorGeometryInstanceAttribute::equals(&a, &c));
}
