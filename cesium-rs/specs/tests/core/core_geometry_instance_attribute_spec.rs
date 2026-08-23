//! Port of `Core/GeometryInstanceAttributeSpec.js`.
use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::geometry_instance_attribute::GeometryInstanceAttribute;

#[test]
fn constructor() {
    let attr = GeometryInstanceAttribute::new(
        ComponentDatatype::UnsignedByte,
        4,
        Some(true),
        vec![255.0, 255.0, 0.0, 255.0],
    );
    assert_eq!(attr.component_datatype(), ComponentDatatype::UnsignedByte);
    assert_eq!(attr.components_per_attribute(), 4);
    assert!(attr.normalize());
    assert_eq!(attr.value(), &[255.0, 255.0, 0.0, 255.0]);
}
