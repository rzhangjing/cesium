//! Mirrors packages/engine/Specs/Core/BoundingRectangleSpec.js
//!
//! `createPackableSpecs` is inlined.

use cesium_core::bounding_rectangle::BoundingRectangle;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::intersect::Intersect;
use cesium_core::rectangle::Rectangle;

// --- constructor ---

#[test]
fn default_constructor_sets_expected_values() {
    let r = BoundingRectangle::default();
    assert_eq!(r.x, 0.0);
    assert_eq!(r.y, 0.0);
    assert_eq!(r.width, 0.0);
    assert_eq!(r.height, 0.0);
}

#[test]
fn constructor_sets_expected_parameters() {
    let r = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(r.x, 1.0);
    assert_eq!(r.y, 2.0);
    assert_eq!(r.width, 3.0);
    assert_eq!(r.height, 4.0);
}

// --- clone ---

#[test]
fn clone_without_result_parameter() {
    let r = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    let result = BoundingRectangle::clone_new(&r);
    assert_eq!(result, r);
}

#[test]
fn clone_with_result_parameter() {
    let r = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    let mut result = BoundingRectangle::new(6.0, 7.0, 8.0, 9.0);
    BoundingRectangle::clone(&r, &mut result);
    assert_eq!(result, r);
}

#[test]
fn clone_works_with_self_result_parameter() {
    let mut r = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    // In Rust, we can't borrow r as both & and &mut simultaneously.
    // Instead, copy the values first, then clone into r.
    let saved = r;
    BoundingRectangle::clone(&saved, &mut r);
    assert_eq!(r.x, 1.0);
    assert_eq!(r.y, 2.0);
    assert_eq!(r.width, 3.0);
    assert_eq!(r.height, 4.0);
}

// --- equals ---

#[test]
fn equals_works() {
    let r = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    assert!(r == BoundingRectangle::new(1.0, 2.0, 3.0, 4.0));
    assert!(r != BoundingRectangle::new(5.0, 2.0, 3.0, 4.0));
    assert!(r != BoundingRectangle::new(1.0, 6.0, 3.0, 4.0));
    assert!(r != BoundingRectangle::new(1.0, 2.0, 7.0, 4.0));
    assert!(r != BoundingRectangle::new(1.0, 2.0, 3.0, 8.0));
}

// --- fromPoints ---

fn sample_positions() -> Vec<Cartesian2> {
    vec![
        Cartesian2::new(3.0, -1.0),
        Cartesian2::new(2.0, -2.0),
        Cartesian2::new(1.0, -3.0),
        Cartesian2::new(0.0, 0.0),
        Cartesian2::new(-1.0, 1.0),
        Cartesian2::new(-2.0, 2.0),
        Cartesian2::new(-3.0, 3.0),
    ]
}

#[test]
fn create_axis_aligned_bounding_rectangle() {
    let positions = sample_positions();
    let r = BoundingRectangle::from_points_new(&positions);
    assert_eq!(r.x, -3.0);
    assert_eq!(r.y, -3.0);
    assert_eq!(r.width, 6.0);
    assert_eq!(r.height, 6.0);
}

#[test]
fn from_points_works_with_result_parameter() {
    let positions = sample_positions();
    let mut result = BoundingRectangle::default();
    BoundingRectangle::from_points(&positions, &mut result);
    assert_eq!(result.x, -3.0);
    assert_eq!(result.y, -3.0);
    assert_eq!(result.width, 6.0);
    assert_eq!(result.height, 6.0);
}

#[test]
fn from_points_creates_empty_rectangle_with_no_positions() {
    let r = BoundingRectangle::from_points_new(&[]);
    assert_eq!(r.x, 0.0);
    assert_eq!(r.y, 0.0);
    assert_eq!(r.width, 0.0);
    assert_eq!(r.height, 0.0);
}

// --- fromRectangle ---

#[test]
fn from_rectangle_creates_empty_with_no_rectangle() {
    let r = BoundingRectangle::from_rectangle(None, None);
    assert_eq!(r.x, 0.0);
    assert_eq!(r.y, 0.0);
    assert_eq!(r.width, 0.0);
    assert_eq!(r.height, 0.0);
}

#[test]
fn from_rectangle_creates_bounding_rectangle() {
    let rectangle = Rectangle::MAX_VALUE;
    let projection = GeographicProjection::new(Some(Ellipsoid::UNIT_SPHERE));
    let expected = BoundingRectangle::new(
        rectangle.west,
        rectangle.south,
        rectangle.east - rectangle.west,
        rectangle.north - rectangle.south,
    );
    assert_eq!(
        BoundingRectangle::from_rectangle(Some(&rectangle), Some(&projection)),
        expected
    );
}

#[test]
fn from_rectangle_works_with_a_result_parameter() {
    let rectangle = Rectangle::MAX_VALUE;
    let expected = BoundingRectangle::new(
        rectangle.west,
        rectangle.south,
        rectangle.east - rectangle.west,
        rectangle.north - rectangle.south,
    );
    let projection = GeographicProjection::new(Some(Ellipsoid::UNIT_SPHERE));

    let mut result = BoundingRectangle::default();
    BoundingRectangle::from_rectangle_into(Some(&rectangle), Some(&projection), &mut result);
    assert_eq!(result, expected);
}

// --- intersect ---

#[test]
fn intersect_works() {
    let r1 = BoundingRectangle::new(0.0, 0.0, 4.0, 4.0);
    let r2 = BoundingRectangle::new(2.0, 2.0, 4.0, 4.0);
    let r3 = BoundingRectangle::new(-6.0, 2.0, 4.0, 4.0);
    let r4 = BoundingRectangle::new(8.0, 2.0, 4.0, 4.0);
    let r5 = BoundingRectangle::new(2.0, -6.0, 4.0, 4.0);
    let r6 = BoundingRectangle::new(2.0, 8.0, 4.0, 4.0);

    assert_eq!(BoundingRectangle::intersect(&r1, &r2), Intersect::Intersecting);
    assert_eq!(BoundingRectangle::intersect(&r1, &r3), Intersect::Outside);
    assert_eq!(BoundingRectangle::intersect(&r1, &r4), Intersect::Outside);
    assert_eq!(BoundingRectangle::intersect(&r1, &r5), Intersect::Outside);
    assert_eq!(BoundingRectangle::intersect(&r1, &r6), Intersect::Outside);
}

// --- union ---

#[test]
fn union_works_without_result_parameter() {
    let r1 = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let r2 = BoundingRectangle::new(-2.0, 0.0, 1.0, 2.0);
    let expected = BoundingRectangle::new(-2.0, 0.0, 5.0, 2.0);
    let result = BoundingRectangle::union_new(&r1, &r2);
    assert_eq!(result, expected);
}

#[test]
fn union_works_with_result_parameter() {
    let r1 = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let r2 = BoundingRectangle::new(-2.0, 0.0, 1.0, 2.0);
    let expected = BoundingRectangle::new(-2.0, 0.0, 5.0, 2.0);
    let mut result = BoundingRectangle::new(-1.0, -1.0, 10.0, 10.0);
    BoundingRectangle::union(&r1, &r2, &mut result);
    assert_eq!(result, expected);
}

// --- expand ---

#[test]
fn expand_works_if_rectangle_needs_to_grow_right() {
    let r = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let point = Cartesian2::new(4.0, 0.0);
    let expected = BoundingRectangle::new(2.0, 0.0, 2.0, 1.0);
    let result = BoundingRectangle::expand_new(&r, &point);
    assert_eq!(result, expected);
}

#[test]
fn expand_works_if_rectangle_needs_x_to_grow_left() {
    let r = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let point = Cartesian2::new(0.0, 0.0);
    let expected = BoundingRectangle::new(0.0, 0.0, 3.0, 1.0);
    let result = BoundingRectangle::expand_new(&r, &point);
    assert_eq!(result, expected);
}

#[test]
fn expand_works_if_rectangle_needs_to_grow_up() {
    let r = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let point = Cartesian2::new(2.0, 2.0);
    let expected = BoundingRectangle::new(2.0, 0.0, 1.0, 2.0);
    let result = BoundingRectangle::expand_new(&r, &point);
    assert_eq!(result, expected);
}

#[test]
fn expand_works_if_rectangle_needs_x_to_grow_down() {
    let r = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let point = Cartesian2::new(2.0, -1.0);
    let expected = BoundingRectangle::new(2.0, -1.0, 1.0, 2.0);
    let result = BoundingRectangle::expand_new(&r, &point);
    assert_eq!(result, expected);
}

#[test]
fn expand_works_if_rectangle_does_not_need_to_grow() {
    let r = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let point = Cartesian2::new(2.5, 0.6);
    let expected = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let result = BoundingRectangle::expand_new(&r, &point);
    assert_eq!(result, expected);
}

#[test]
fn expand_works_with_result_parameter() {
    let r = BoundingRectangle::new(2.0, 0.0, 1.0, 1.0);
    let point = Cartesian2::new(2.0, -1.0);
    let expected = BoundingRectangle::new(2.0, -1.0, 1.0, 2.0);
    let mut result = BoundingRectangle::default();
    BoundingRectangle::expand(&r, &point, &mut result);
    assert_eq!(result, expected);
}

// --- packable ---

#[test]
fn pack_works() {
    let value = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    let mut array = [0.0; 4];
    BoundingRectangle::pack(&value, &mut array, 0);
    assert_eq!(array, [1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn unpack_works() {
    let array = [1.0, 2.0, 3.0, 4.0];
    let result = BoundingRectangle::unpack_new(&array, 0);
    assert_eq!(result, BoundingRectangle::new(1.0, 2.0, 3.0, 4.0));
}

#[test]
fn pack_then_unpack_roundtrip() {
    let original = BoundingRectangle::new(1.0, 2.0, 3.0, 4.0);
    let mut array = [0.0; 4];
    BoundingRectangle::pack(&original, &mut array, 0);
    let unpacked = BoundingRectangle::unpack_new(&array, 0);
    assert_eq!(unpacked, original);
}
