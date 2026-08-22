//! Mirrors packages/engine/Specs/Core/RaySpec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::ray::Ray;
use cesium_test_utils::expect_to_throw_dev_error;

#[test]
fn default_constructor_creates_zero_valued_ray() {
    let ray = Ray::default();
    assert_eq!(ray.origin, Cartesian3::ZERO);
    assert_eq!(ray.direction, Cartesian3::ZERO);
}

#[test]
fn constructor_sets_expected_properties() {
    let origin = Cartesian3::UNIT_Y;
    let direction = Cartesian3::UNIT_X;
    let ray = Ray::new(Some(&origin), Some(&direction));
    assert_eq!(ray.origin, origin);
    assert_eq!(ray.direction, direction);
}

#[test]
fn constructor_normalizes_direction() {
    let origin = Cartesian3::UNIT_Y;
    let direction = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, 18.0);
    let ray = Ray::new(Some(&origin), Some(&direction));
    assert_eq!(ray.origin, origin);
    assert_eq!(ray.direction, Cartesian3::UNIT_X);
}

#[test]
fn clone_without_result_parameter() {
    let dir = Cartesian3::normalize_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let ray = Ray::new(Some(&Cartesian3::UNIT_X), Some(&dir));
    let returned = Ray::clone_new(Some(&ray));
    assert!(returned.is_some());
    let returned = returned.unwrap();
    assert_ne!(std::ptr::addr_of!(ray), std::ptr::addr_of!(returned));
    assert_eq!(ray, returned);
}

#[test]
fn clone_with_result_parameter() {
    let dir = Cartesian3::normalize_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let ray = Ray::new(Some(&Cartesian3::UNIT_X), Some(&dir));
    let mut result = Ray::default();
    let returned = Ray::clone(Some(&ray), Some(&mut result));
    // JS: result === returnedResult (but Rust returns None when result is mutated)
    assert!(returned.is_none());
    assert_eq!(ray, result);
}

#[test]
fn clone_returns_none_if_ray_is_none() {
    let returned = Ray::clone_new(None);
    assert!(returned.is_none());
}

#[test]
fn get_point_along_ray_without_result_parameter() {
    let dir = Cartesian3::normalize_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let ray = Ray::new(Some(&Cartesian3::UNIT_X), Some(&dir));
    for i in -10..=10 {
        let expected = Cartesian3::add_new(
            &Cartesian3::multiply_by_scalar_new(&dir, i as f64),
            &Cartesian3::UNIT_X,
        );
        let returned = Ray::get_point_new(&ray, Some(i as f64));
        assert_eq!(returned, expected);
    }
}

#[test]
fn get_point_with_result_parameter() {
    let dir = Cartesian3::normalize_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let ray = Ray::new(Some(&Cartesian3::UNIT_X), Some(&dir));
    let mut result = Cartesian3::default();
    for i in -10..=10 {
        let expected = Cartesian3::add_new(
            &Cartesian3::multiply_by_scalar_new(&dir, i as f64),
            &Cartesian3::UNIT_X,
        );
        Ray::get_point(&ray, Some(i as f64), &mut result);
        assert_eq!(result, expected);
    }
}

#[test]
fn get_point_throws_without_t() {
    let dir = Cartesian3::normalize_new(&Cartesian3::new(1.0, 2.0, 3.0));
    let ray = Ray::new(Some(&Cartesian3::UNIT_X), Some(&dir));
    let mut result = Cartesian3::default();
    expect_to_throw_dev_error(|| {
        Ray::get_point(&ray, None, &mut result);
    });
}
