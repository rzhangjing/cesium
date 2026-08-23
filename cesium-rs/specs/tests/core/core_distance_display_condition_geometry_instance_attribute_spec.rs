//! Port of `Core/DistanceDisplayConditionGeometryInstanceAttributeSpec.js`.
use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::distance_display_condition_geometry_instance_attribute::DistanceDisplayConditionGeometryInstanceAttribute;

#[test]
fn constructor() {
    let attr = DistanceDisplayConditionGeometryInstanceAttribute::new(Some(10.0), Some(100.0));
    assert_eq!(DistanceDisplayConditionGeometryInstanceAttribute::component_datatype(), ComponentDatatype::Float);
    assert_eq!(DistanceDisplayConditionGeometryInstanceAttribute::components_per_attribute(), 2);
    assert!(!DistanceDisplayConditionGeometryInstanceAttribute::normalize());
    assert_eq!(attr.value, vec![10.0, 100.0]);
}

#[test]
fn to_value() {
    let v = DistanceDisplayConditionGeometryInstanceAttribute::to_value(10.0, 200.0);
    assert_eq!(v, vec![10.0, 200.0]);
}
