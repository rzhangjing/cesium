//! Faithful port of CesiumJS DataSources/ReferencePropertySpec.js A-class tests.
//!
//! Original: 24 it() tests. A-class (pure logic, no events/spy): 10 tests.
//! Event-based tests (definitionChanged tracking) are B-class.
//! Throws tests are C-class (Rust uses type system / Option instead).

use cesium_datasource::property_system::{
    ConstantProperty, DynProperty, MapPropertyResolver, PropertyValue, ReferenceProperty,
};
use cesium_time::JulianDate;
use std::sync::Arc;

fn jd(day: f64, seconds: f64) -> JulianDate {
    JulianDate::new(day, seconds)
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ===========================================================================
// Constructor
// ===========================================================================

#[test]
fn reference_property_constructor_sets_expected_values() {
    // "constructor sets expected values"
    let resolver = Arc::new(MapPropertyResolver::new());
    let property = ReferenceProperty::new(
        resolver,
        "testId",
        names(&["foo", "bar", "baz"]),
    );

    assert_eq!(property.target_id(), "testId");
    assert_eq!(
        property.target_property_names(),
        &["foo".to_string(), "bar".to_string(), "baz".to_string()]
    );
}

// ===========================================================================
// fromString
// ===========================================================================

#[test]
fn reference_property_from_string_sets_expected_values() {
    // "fromString sets expected values"
    let resolver = Arc::new(MapPropertyResolver::new());
    let property = ReferenceProperty::from_string(resolver, "testId#foo.bar.baz");

    assert_eq!(property.target_id(), "testId");
    assert_eq!(
        property.target_property_names(),
        &["foo".to_string(), "bar".to_string(), "baz".to_string()]
    );
}

#[test]
fn reference_property_from_string_works_with_escaped_values() {
    // "fromString works with escaped values"
    let resolver = Arc::new(MapPropertyResolver::new());
    let property = ReferenceProperty::from_string(
        resolver,
        r"\#identif\\\#ier\.#propertyName.\.abc\\.def",
    );

    assert_eq!(property.target_id(), "#identif\\#ier.");
    assert_eq!(
        property.target_property_names(),
        &[
            "propertyName".to_string(),
            ".abc\\".to_string(),
            "def".to_string()
        ]
    );
}

// ===========================================================================
// getValue / isConstant with resolution
// ===========================================================================

#[test]
fn reference_property_get_value_returns_undefined_if_target_not_resolved() {
    // "getValue returns undefined if target entity can not be resolved"
    let resolver = Arc::new(MapPropertyResolver::new());
    let property = ReferenceProperty::from_string(resolver, "testId#foo.bar");
    let time = jd(2451545.0, 0.0);

    assert_eq!(property.get_value(&time), PropertyValue::Undefined);
}

#[test]
fn reference_property_get_value_returns_undefined_if_property_not_resolved() {
    // "getValue returns undefined if target property can not be resolved"
    // Register a property at "testId#billboard" but query "testId#billboard.scale"
    let mut r = MapPropertyResolver::new();
    r.insert(
        "testId",
        &names(&["billboard"]),
        Arc::new(ConstantProperty::new(PropertyValue::Number(5.0))),
    );
    let resolver = Arc::new(r);

    let property = ReferenceProperty::from_string(resolver, "testId#billboard.scale");
    let time = jd(2451545.0, 0.0);
    assert_eq!(property.get_value(&time), PropertyValue::Undefined);
}

#[test]
fn reference_property_is_constant_true_when_unresolved() {
    // "isConstant returns true when target entity does not exist"
    let resolver = Arc::new(MapPropertyResolver::new());
    let property = ReferenceProperty::from_string(resolver, "nonExistent#foo");

    assert!(property.is_constant());
}

#[test]
fn reference_property_properly_tracks_resolved_property() {
    // "properly tracks resolved property" (A-class subset: getValue/isConstant)
    let mut resolver = MapPropertyResolver::new();
    resolver.insert(
        "testId",
        &names(&["billboard", "scale"]),
        Arc::new(ConstantProperty::new(PropertyValue::Number(5.0))),
    );
    let resolver = Arc::new(resolver);

    let property = ReferenceProperty::from_string(resolver, "testId#billboard.scale");
    let time = jd(2451545.0, 0.0);

    assert!(property.is_constant());
    assert_eq!(
        property.get_value(&time),
        PropertyValue::Number(5.0)
    );

    // resolved_property returns the underlying property
    let resolved = property.resolved_property();
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().get_value(&time), PropertyValue::Number(5.0));
}

#[test]
fn reference_property_resolved_property_none_when_unresolvable() {
    let resolver = Arc::new(MapPropertyResolver::new());
    let property = ReferenceProperty::from_string(resolver, "missing#foo.bar");

    assert!(property.resolved_property().is_none());
}

// ===========================================================================
// equals
// ===========================================================================

#[test]
fn reference_property_equals_works() {
    // "equals works"
    let resolver1 = Arc::new(MapPropertyResolver::new());
    let resolver2 = Arc::new(MapPropertyResolver::new());

    let left = ReferenceProperty::from_string(resolver1.clone(), "objectId#foo.bar");
    let right = ReferenceProperty::from_string(resolver1.clone(), "objectId#foo.bar");
    assert!(left.equals(&right));

    // collection (resolver) differs
    let right2 = ReferenceProperty::from_string(resolver2.clone(), "objectId#foo.bar");
    assert!(!left.equals(&right2));

    // target id differs
    let right3 = ReferenceProperty::from_string(resolver1.clone(), "otherObjectId#foo.bar");
    assert!(!left.equals(&right3));

    // number of sub-properties differ
    let right4 = ReferenceProperty::from_string(resolver1.clone(), "objectId#foo");
    assert!(!left.equals(&right4));

    // sub-properties of same length differ
    let right5 = ReferenceProperty::from_string(resolver1.clone(), "objectId#foo.baz");
    assert!(!left.equals(&right5));
}

// ===========================================================================
// reference_frame delegation
// ===========================================================================

#[test]
fn reference_property_reference_frame_delegates_to_resolved() {
    // "works with position properties" (A-class subset: referenceFrame)
    use cesium_datasource::property_system::{ConstantPositionProperty, ReferenceFrame};
    use glam::DVec3;

    let mut resolver = MapPropertyResolver::new();
    let pos_prop = ConstantPositionProperty::new(DVec3::new(1.0, 2.0, 3.0));
    resolver.insert(
        "testId",
        &names(&["position"]),
        Arc::new(pos_prop),
    );
    let resolver = Arc::new(resolver);

    let property = ReferenceProperty::from_string(resolver.clone(), "testId#position");
    assert_eq!(property.reference_frame(), Some(ReferenceFrame::Fixed));

    // Non-existent reference has no frame
    let property2 = ReferenceProperty::from_string(resolver, "nonExistent#position");
    assert_eq!(property2.reference_frame(), None);
}
