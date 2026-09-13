//! Tests for NodeTransformationProperty - ported from NodeTransformationPropertySpec.js
//!
//! Original: 7 it() → 5 A-class (2 C-class: result-param/definitionChanged omitted)

use cesium_datasource::node_transformation_property::NodeTransformationProperty;
use cesium_datasource::property_system::property::{
    ConstantProperty, DynProperty, TimeIntervalCollectionProperty,
};
use cesium_datasource::property_system::value::PropertyValue;
use cesium_time::{JulianDate, TimeInterval};
use glam::{DQuat, DVec3};
use std::sync::Arc;

fn jd(seconds: f64) -> JulianDate {
    JulianDate::new(0.0, seconds)
}

// === Default constructor ===

#[test]
fn test_node_transformation_default_constructor() {
    let property = NodeTransformationProperty::new();
    assert!(property.is_constant());
    assert!(property.translation().is_none());
    assert!(property.rotation().is_none());
    assert!(property.scale().is_none());

    let result = property.get_value(&jd(0.0));
    assert_eq!(result.translation, DVec3::ZERO);
    assert_eq!(result.rotation, DQuat::IDENTITY);
    assert_eq!(result.scale, DVec3::ONE);
}

// === Constructor with options ===

#[test]
fn test_node_transformation_constructor_with_options() {
    let translation = DVec3::Y;
    let rotation = DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5);
    let scale = DVec3::X;

    let property = NodeTransformationProperty::with_values(translation, rotation, scale);
    assert!(property.translation().is_some());
    assert!(property.rotation().is_some());
    assert!(property.scale().is_some());

    let result = property.get_value(&jd(0.0));
    assert_eq!(result.translation, translation);
    assert_eq!(result.rotation, rotation);
    assert_eq!(result.scale, scale);
}

// === Works with constant values ===

#[test]
fn test_node_transformation_constant_values() {
    let mut property = NodeTransformationProperty::new();
    property.set_translation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::Y),
    ))));
    property.set_rotation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Quaternion(DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5)),
    ))));
    property.set_scale(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::X),
    ))));

    let result = property.get_value(&jd(0.0));
    assert_eq!(result.translation, DVec3::Y);
    assert_eq!(result.rotation, DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5));
    assert_eq!(result.scale, DVec3::X);
}

// === Works with dynamic values ===

#[test]
fn test_node_transformation_dynamic_values() {
    let mut property = NodeTransformationProperty::new();

    let mut tic_translation = TimeIntervalCollectionProperty::new();
    let mut tic_rotation = TimeIntervalCollectionProperty::new();
    let mut tic_scale = TimeIntervalCollectionProperty::new();

    let start = jd(86400.0); // JulianDate(1, 0)
    let stop = jd(172800.0); // JulianDate(2, 0)

    tic_translation.add_interval(
        TimeInterval::new(start, stop, true, true),
        Some(PropertyValue::Cartesian3(DVec3::Y)),
    );
    tic_rotation.add_interval(
        TimeInterval::new(start, stop, true, true),
        Some(PropertyValue::Quaternion(DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5))),
    );
    tic_scale.add_interval(
        TimeInterval::new(start, stop, true, true),
        Some(PropertyValue::Cartesian3(DVec3::X)),
    );

    property.set_translation(Some(Arc::new(tic_translation)));
    property.set_rotation(Some(Arc::new(tic_rotation)));
    property.set_scale(Some(Arc::new(tic_scale)));

    assert!(!property.is_constant());

    let result = property.get_value(&start);
    assert_eq!(result.translation, DVec3::Y);
    assert_eq!(result.rotation, DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5));
    assert_eq!(result.scale, DVec3::X);
}

// === equals ===

#[test]
fn test_node_transformation_equals() {
    let mut left = NodeTransformationProperty::new();
    left.set_translation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::Y),
    ))));
    left.set_rotation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Quaternion(DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5)),
    ))));
    left.set_scale(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::X),
    ))));

    let mut right = NodeTransformationProperty::new();
    right.set_translation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::Y),
    ))));
    right.set_rotation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Quaternion(DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5)),
    ))));
    right.set_scale(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::X),
    ))));

    assert!(left.equals(&right));

    // Different scale
    right.set_scale(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::ZERO),
    ))));
    assert!(!left.equals(&right));

    // Restore scale, different translation
    right.set_scale(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::X),
    ))));
    right.set_translation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::ZERO),
    ))));
    assert!(!left.equals(&right));

    // Restore translation, different rotation
    right.set_translation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Cartesian3(DVec3::Y),
    ))));
    right.set_rotation(Some(Arc::new(ConstantProperty::new(
        PropertyValue::Quaternion(DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0)),
    ))));
    assert!(!left.equals(&right));
}
