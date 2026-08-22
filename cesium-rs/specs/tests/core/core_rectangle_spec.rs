//! Mirrors packages/engine/Specs/Core/RectangleSpec.js

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::rectangle::Rectangle;

const WEST: f64 = -0.9;
const SOUTH: f64 = 0.5;
const EAST: f64 = 1.4;
const NORTH: f64 = 1.0;

// --- constructor ---

#[test]
fn default_constructor_sets_zero() {
    let r = Rectangle::default();
    assert_eq!(r.west, 0.0);
    assert_eq!(r.south, 0.0);
    assert_eq!(r.east, 0.0);
    assert_eq!(r.north, 0.0);
}

#[test]
fn constructor_sets_values() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    assert_eq!(r.west, WEST);
    assert_eq!(r.south, SOUTH);
    assert_eq!(r.east, EAST);
    assert_eq!(r.north, NORTH);
}

// --- width / height ---

#[test]
fn compute_width_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let expected = EAST - WEST;
    assert_eq!(r.width(), expected);
    assert_eq!(Rectangle::compute_width(&r), expected);
}

#[test]
fn compute_width_crosses_idl() {
    let r = Rectangle::new(2.0, -1.0, -2.0, 1.0);
    let expected = r.east - r.west + CesiumMath::TWO_PI;
    assert_eq!(r.width(), expected);
}

#[test]
fn compute_height_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let expected = NORTH - SOUTH;
    assert_eq!(r.height(), expected);
    assert_eq!(Rectangle::compute_height(&r), expected);
}

// --- fromDegrees ---

#[test]
fn from_degrees_produces_expected() {
    let r = Rectangle::from_degrees(-10.0, -20.0, 10.0, 20.0);
    assert_eq!(r.west, CesiumMath::to_radians(-10.0));
    assert_eq!(r.south, CesiumMath::to_radians(-20.0));
    assert_eq!(r.east, CesiumMath::to_radians(10.0));
    assert_eq!(r.north, CesiumMath::to_radians(20.0));
}

// --- fromRadians ---

#[test]
fn from_radians_produces_expected() {
    let r = Rectangle::from_radians(-1.0, -2.0, 1.0, 2.0);
    assert_eq!(r.west, -1.0);
    assert_eq!(r.south, -2.0);
    assert_eq!(r.east, 1.0);
    assert_eq!(r.north, 2.0);
}

// --- fromCartographicArray ---

#[test]
fn from_cartographic_array_produces_expected() {
    let min_lon = Cartographic::new(-0.1, 0.3, 0.0);
    let min_lat = Cartographic::new(0.0, -0.2, 0.0);
    let max_lon = Cartographic::new(0.3, -0.1, 0.0);
    let max_lat = Cartographic::new(0.2, 0.4, 0.0);

    let r = Rectangle::from_cartographic_array(&[min_lat, min_lon, max_lat, max_lon]);
    assert_eq!(r.west, min_lon.longitude);
    assert_eq!(r.south, min_lat.latitude);
    assert_eq!(r.east, max_lon.longitude);
    assert_eq!(r.north, max_lat.latitude);
}

#[test]
fn from_cartographic_array_crosses_idl() {
    let min_lon = Cartographic::from_degrees_new(-178.0, 3.0, Some(0.0));
    let min_lat = Cartographic::from_degrees_new(-179.0, -4.0, Some(0.0));
    let max_lon = Cartographic::from_degrees_new(178.0, 3.0, Some(0.0));
    let max_lat = Cartographic::from_degrees_new(179.0, 4.0, Some(0.0));

    let r = Rectangle::from_cartographic_array(&[min_lat, min_lon, max_lat, max_lon]);
    assert_eq!(r.east, min_lon.longitude);
    assert_eq!(r.south, min_lat.latitude);
    assert_eq!(r.west, max_lon.longitude);
    assert_eq!(r.north, max_lat.latitude);
}

// --- equals ---

#[test]
fn equals_works() {
    let r = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    assert!(r.equals_to(&Rectangle::new(0.1, 0.2, 0.3, 0.4)));
    assert!(!r.equals_to(&Rectangle::new(0.5, 0.2, 0.3, 0.4)));
    assert!(!r.equals_to(&Rectangle::new(0.1, 0.5, 0.3, 0.4)));
    assert!(!r.equals_to(&Rectangle::new(0.1, 0.2, 0.5, 0.4)));
    assert!(!r.equals_to(&Rectangle::new(0.1, 0.2, 0.3, 0.5)));
}

#[test]
fn equals_epsilon_works() {
    let r = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    assert!(r.equals_epsilon_to(&Rectangle::new(0.1, 0.2, 0.3, 0.4), Some(0.0)));
    assert!(!r.equals_epsilon_to(&Rectangle::new(0.5, 0.2, 0.3, 0.4), Some(0.0)));
    assert!(r.equals_epsilon_to(&Rectangle::new(0.5, 0.2, 0.3, 0.4), Some(0.4)));
    assert!(r.equals_epsilon_to(&Rectangle::new(0.1, 0.5, 0.3, 0.4), Some(0.3)));
    assert!(r.equals_epsilon_to(&Rectangle::new(0.1, 0.2, 0.5, 0.4), Some(0.2)));
    assert!(r.equals_epsilon_to(&Rectangle::new(0.1, 0.2, 0.3, 0.5), Some(0.1)));
}

// --- corners ---

#[test]
fn southwest_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let sw = Rectangle::southwest(&r);
    assert_eq!(sw.longitude, WEST);
    assert_eq!(sw.latitude, SOUTH);
}

#[test]
fn northwest_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let nw = Rectangle::northwest(&r);
    assert_eq!(nw.longitude, WEST);
    assert_eq!(nw.latitude, NORTH);
}

#[test]
fn northeast_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let ne = Rectangle::northeast(&r);
    assert_eq!(ne.longitude, EAST);
    assert_eq!(ne.latitude, NORTH);
}

#[test]
fn southeast_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let se = Rectangle::southeast(&r);
    assert_eq!(se.longitude, EAST);
    assert_eq!(se.latitude, SOUTH);
}

// --- center ---

#[test]
fn center_works() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let c = Rectangle::center(&r);
    let expected = Cartographic::new((WEST + EAST) / 2.0, (SOUTH + NORTH) / 2.0, 0.0);
    assert!((c.longitude - expected.longitude).abs() < CesiumMath::EPSILON11);
    assert!((c.latitude - expected.latitude).abs() < CesiumMath::EPSILON11);
}

#[test]
fn center_across_idl() {
    let r = Rectangle::from_degrees(170.0, 0.0, -170.0, 0.0);
    let c = Rectangle::center(&r);
    let expected = Cartographic::from_degrees_new(180.0, 0.0, Some(0.0));
    assert!((c.longitude - expected.longitude).abs() < CesiumMath::EPSILON11);
}

// --- intersection ---

#[test]
fn intersection_works() {
    let r1 = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let r2 = Rectangle::new(-0.5, -0.5, 0.5, 0.5);
    let result = Rectangle::intersection(&r1, &r2).unwrap();
    assert_eq!(result.west, -0.5);
    assert_eq!(result.south, -0.5);
    assert_eq!(result.east, 0.5);
    assert_eq!(result.north, 0.5);
}

#[test]
fn intersection_returns_none_when_no_overlap() {
    let r1 = Rectangle::new(-1.0, -1.0, -0.5, -0.5);
    let r2 = Rectangle::new(0.5, 0.5, 1.0, 1.0);
    assert!(Rectangle::intersection(&r1, &r2).is_none());
}

// --- simpleIntersection ---

#[test]
fn simple_intersection_works() {
    let r1 = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let r2 = Rectangle::new(-0.5, -0.5, 0.5, 0.5);
    let result = Rectangle::simple_intersection(&r1, &r2).unwrap();
    assert_eq!(result.west, -0.5);
    assert_eq!(result.south, -0.5);
    assert_eq!(result.east, 0.5);
    assert_eq!(result.north, 0.5);
}

// --- union ---

#[test]
fn union_works() {
    let r1 = Rectangle::new(-1.0, -1.0, 0.0, 0.0);
    let r2 = Rectangle::new(-0.5, -0.5, 1.0, 1.0);
    let result = Rectangle::union(&r1, &r2);
    assert_eq!(result.west, -1.0);
    assert_eq!(result.south, -1.0);
    assert_eq!(result.east, 1.0);
    assert_eq!(result.north, 1.0);
}

// --- expand ---

#[test]
fn expand_works() {
    let r = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    let c = Cartographic::new(2.0, 2.0, 0.0);
    let result = Rectangle::expand(&r, &c);
    assert_eq!(result.west, -1.0);
    assert_eq!(result.south, -1.0);
    assert_eq!(result.east, 2.0);
    assert_eq!(result.north, 2.0);
}

// --- contains ---

#[test]
fn contains_works() {
    let r = Rectangle::new(-1.0, -1.0, 1.0, 1.0);
    assert!(Rectangle::contains(&r, &Cartographic::new(0.0, 0.0, 0.0)));
    assert!(!Rectangle::contains(&r, &Cartographic::new(2.0, 0.0, 0.0)));
    assert!(Rectangle::contains(&r, &Cartographic::new(-1.0, -1.0, 0.0))); // boundary
}

#[test]
fn contains_crosses_idl() {
    let r = Rectangle::from_degrees(170.0, -10.0, -170.0, 10.0);
    assert!(Rectangle::contains(&r, &Cartographic::from_degrees_new(180.0, 0.0, Some(0.0))));
    assert!(Rectangle::contains(&r, &Cartographic::from_degrees_new(175.0, 0.0, Some(0.0))));
    assert!(!Rectangle::contains(&r, &Cartographic::from_degrees_new(0.0, 0.0, Some(0.0))));
}

// --- pack/unpack ---

#[test]
fn pack_unpack_roundtrip() {
    let r = Rectangle::new(WEST, SOUTH, EAST, NORTH);
    let mut array = [0.0; 4];
    Rectangle::pack(&r, &mut array, None);
    assert_eq!(array[0], WEST);
    assert_eq!(array[1], SOUTH);
    assert_eq!(array[2], EAST);
    assert_eq!(array[3], NORTH);

    let unpacked = Rectangle::unpack(&array, None);
    assert_eq!(unpacked, r);
}

// --- subsection ---

#[test]
fn subsection_works() {
    let r = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    let sub = Rectangle::subsection(&r, 0.0, 0.0, 1.0, 1.0);
    assert_eq!(sub.west, 0.0);
    assert_eq!(sub.east, 1.0);
    assert_eq!(sub.south, 0.0);
    assert_eq!(sub.north, 1.0);
}

#[test]
fn subsection_half() {
    let r = Rectangle::new(0.0, 0.0, 1.0, 1.0);
    let sub = Rectangle::subsection(&r, 0.0, 0.0, 0.5, 0.5);
    assert_eq!(sub.west, 0.0);
    assert!((sub.east - 0.5).abs() < CesiumMath::EPSILON15);
    assert_eq!(sub.south, 0.0);
    assert!((sub.north - 0.5).abs() < CesiumMath::EPSILON15);
}

// --- subsample ---

#[test]
fn subsample_returns_points() {
    let r = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
    let points = Rectangle::subsample(&r, Some(&Ellipsoid::WGS84), None);
    assert!(points.len() >= 4); // at least the 4 corners
}

// --- MAX_VALUE ---

#[test]
fn max_value_is_correct() {
    assert_eq!(Rectangle::MAX_VALUE.west, -std::f64::consts::PI);
    assert_eq!(Rectangle::MAX_VALUE.south, -CesiumMath::PI_OVER_TWO);
    assert_eq!(Rectangle::MAX_VALUE.east, std::f64::consts::PI);
    assert_eq!(Rectangle::MAX_VALUE.north, CesiumMath::PI_OVER_TWO);
}

// --- fromCartesianArray ---

#[test]
fn from_cartesian_array_produces_expected() {
    let min_lon = Cartographic::new(-0.1, 0.3, 0.0);
    let min_lat = Cartographic::new(0.0, -0.2, 0.0);
    let max_lon = Cartographic::new(0.3, -0.1, 0.0);
    let max_lat = Cartographic::new(0.2, 0.4, 0.0);

    let wgs84 = &Ellipsoid::WGS84;
    let mut cartesians = Vec::new();
    for c in &[min_lat, min_lon, max_lat, max_lon] {
        let mut cart = Cartesian3::default();
        wgs84.cartographic_to_cartesian(c, &mut cart);
        cartesians.push(cart);
    }

    let r = Rectangle::from_cartesian_array(&cartesians, Some(wgs84));
    assert!((r.west - min_lon.longitude).abs() < CesiumMath::EPSILON15);
    assert!((r.south - min_lat.latitude).abs() < CesiumMath::EPSILON15);
    assert!((r.east - max_lon.longitude).abs() < CesiumMath::EPSILON15);
    assert!((r.north - max_lat.latitude).abs() < CesiumMath::EPSILON15);
}
