//! Tests for `cesium_core::spline` utilities.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::spline::{clamp_time, find_time_interval, wrap_time, SplinePoint};

#[test]
fn find_time_interval_returns_correct_index() {
    let times = vec![0.0, 1.0, 2.0, 3.0];
    assert_eq!(find_time_interval(&times, 0.5, None), Some(0));
    assert_eq!(find_time_interval(&times, 1.5, None), Some(1));
    assert_eq!(find_time_interval(&times, 2.5, None), Some(2));
}

#[test]
fn find_time_interval_returns_none_outside() {
    let times = vec![0.0, 1.0, 2.0];
    assert_eq!(find_time_interval(&times, -1.0, None), None);
    assert_eq!(find_time_interval(&times, 3.0, None), None);
}

#[test]
fn wrap_time_wraps_outside_range() {
    let times = vec![0.0, 1.0, 2.0];
    let wrapped = wrap_time(&times, 3.0);
    assert!(wrapped >= 0.0 && wrapped <= 2.0);
}

#[test]
fn clamp_time_clamps_to_bounds() {
    let times = vec![0.0, 1.0, 2.0];
    assert_eq!(clamp_time(&times, -5.0), 0.0);
    assert_eq!(clamp_time(&times, 5.0), 2.0);
    assert_eq!(clamp_time(&times, 1.0), 1.0);
}

#[test]
fn spline_point_lerp_scalar() {
    let a = SplinePoint::Scalar(0.0);
    let b = SplinePoint::Scalar(10.0);
    let mid = SplinePoint::lerp(&a, &b, 0.5);
    if let SplinePoint::Scalar(v) = mid {
        assert!((v - 5.0).abs() < 1e-10);
    } else {
        panic!("Expected Scalar");
    }
}

#[test]
fn spline_point_lerp_cartesian3() {
    let a = SplinePoint::Cartesian3(Cartesian3::new(0.0, 0.0, 0.0));
    let b = SplinePoint::Cartesian3(Cartesian3::new(2.0, 4.0, 6.0));
    let mid = SplinePoint::lerp(&a, &b, 0.5);
    if let SplinePoint::Cartesian3(v) = mid {
        assert!((v.x - 1.0).abs() < 1e-10);
        assert!((v.y - 2.0).abs() < 1e-10);
        assert!((v.z - 3.0).abs() < 1e-10);
    } else {
        panic!("Expected Cartesian3");
    }
}
