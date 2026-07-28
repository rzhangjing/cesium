//! Tests for VelocityOrientationProperty - ported from VelocityOrientationPropertySpec.js
//!
//! Original: 14 it() → 7 A-class (7 C-class: events/spy/system-time omitted)

use cesium_datasource::property_system::position::SampledPositionProperty;
use cesium_datasource::property_system::property::{ConstantProperty, DynProperty};
use cesium_datasource::property_system::value::{PropertyValue, ReferenceFrame};
use cesium_datasource::velocity_orientation_property::VelocityOrientationProperty;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::transforms::rotation_matrix_from_position_velocity;
use cesium_time::JulianDate;
use glam::{DQuat, DVec3};
use std::sync::Arc;

fn jd(seconds: f64) -> JulianDate {
    JulianDate::new(0.0, seconds)
}

fn from_degrees(lon_deg: f64, lat_deg: f64, height: f64) -> DVec3 {
    Ellipsoid::WGS84.cartographic_to_cartesian(&Cartographic::from_degrees(lon_deg, lat_deg, height))
}

// === Constructor ===

#[test]
fn test_velocity_orientation_default_construct() {
    let property = VelocityOrientationProperty::new();
    assert!(property.is_constant());
    assert!(property.position().is_none());
    assert_eq!(*property.ellipsoid(), Ellipsoid::WGS84);
}

#[test]
fn test_velocity_orientation_construct_with_args() {
    let position = Arc::new(ConstantProperty::new(PropertyValue::Cartesian3(
        DVec3::X,
    )));
    let property =
        VelocityOrientationProperty::with_position(position.clone(), Ellipsoid::UNIT_SPHERE);
    assert!(property.position().is_some());
    assert_eq!(*property.ellipsoid(), Ellipsoid::UNIT_SPHERE);
}

// === getValue ===

#[test]
fn test_velocity_orientation_get_value() {
    // Position moving east along equator
    let times = vec![jd(0.0), jd(1.0 / 60.0)];
    let values = vec![from_degrees(0.0, 0.0, 0.0), from_degrees(1.0, 0.0, 0.0)];

    let velocity = (values[1] - values[0]).normalize();

    let mut position = SampledPositionProperty::new(ReferenceFrame::Fixed, 0);
    position.add_samples(&times, &values, None);

    let property =
        VelocityOrientationProperty::with_position(Arc::new(position), Ellipsoid::WGS84);

    let pos_at_t0 = from_degrees(0.0, 0.0, 0.0);
    let matrix = rotation_matrix_from_position_velocity(pos_at_t0, velocity, &Ellipsoid::WGS84);
    let expected = DQuat::from_mat3(&matrix);

    let result = property.get_value(&times[0]);
    assert!(result.is_some());
    let q = result.unwrap();
    // Quaternions may differ by sign (q and -q represent same rotation)
    let dot = q.dot(expected).abs();
    assert!(
        (dot - 1.0).abs() < 1e-10,
        "quaternion mismatch: dot={}",
        dot
    );
}

// === Zero velocity ===

#[test]
fn test_velocity_orientation_zero_velocity() {
    // Constant position → zero velocity → undefined
    let position = Arc::new(ConstantProperty::new(PropertyValue::Cartesian3(
        from_degrees(0.0, 0.0, 0.0),
    )));
    let property = VelocityOrientationProperty::with_position(position, Ellipsoid::WGS84);
    let result = property.get_value(&jd(0.0));
    assert!(result.is_none());
}

// === Undefined position ===

#[test]
fn test_velocity_orientation_undefined_position() {
    // No position property → undefined
    let property = VelocityOrientationProperty::new();
    let result = property.get_value(&jd(0.0));
    assert!(result.is_none());
}

// === Single sample (cannot compute velocity) ===

#[test]
fn test_velocity_orientation_single_sample() {
    // With extrapolation NONE, querying outside the single sample returns undefined
    let mut position = SampledPositionProperty::new(ReferenceFrame::Fixed, 0);
    position.add_samples(&[jd(1.0)], &[from_degrees(0.0, 0.0, 0.0)], None);
    // Query at time 0 (before the sample) - with default extrapolation it may still work
    // but the velocity will be zero since both evaluations return the same value
    let property =
        VelocityOrientationProperty::with_position(Arc::new(position), Ellipsoid::WGS84);
    // At the exact sample time, finite diff forward gives same value → zero velocity
    let result = property.get_value(&jd(1.0));
    // With linear extrapolation, both t and t+dt evaluate to same constant → zero velocity
    assert!(result.is_none());
}

// === equals ===

#[test]
fn test_velocity_orientation_equals() {
    let position = Arc::new(ConstantProperty::new(PropertyValue::Cartesian3(DVec3::X)));

    let left = VelocityOrientationProperty::new();
    let right = VelocityOrientationProperty::new();
    assert!(left.equals(&right));

    let mut left2 = VelocityOrientationProperty::with_position(position.clone(), Ellipsoid::WGS84);
    assert!(!left2.equals(&right));

    let right2 = VelocityOrientationProperty::with_position(position.clone(), Ellipsoid::WGS84);
    assert!(left2.equals(&right2));

    // Different ellipsoid
    left2.set_ellipsoid(Ellipsoid::UNIT_SPHERE);
    assert!(!left2.equals(&right2));
}
