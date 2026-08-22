//! Mirrors packages/engine/Specs/Core/IntersectSpec.js
//!
//! Intersect is a simple enum; the JS spec only checks frozen-ness and
//! numeric values.

use cesium_core::intersect::Intersect;

#[test]
fn outside_is_negative_one() {
    assert_eq!(Intersect::Outside as i32, -1);
}

#[test]
fn intersecting_is_zero() {
    assert_eq!(Intersect::Intersecting as i32, 0);
}

#[test]
fn inside_is_one() {
    assert_eq!(Intersect::Inside as i32, 1);
}
