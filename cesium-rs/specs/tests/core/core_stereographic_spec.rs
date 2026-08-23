//! Tests for `cesium_core::Stereographic`.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::stereographic::{Stereographic, HALF_UNIT_SPHERE_RADII, NORTH_POLE, SOUTH_POLE};

#[test]
fn default_has_zero_position() {
    let s = Stereographic::default();
    assert_eq!(s.x(), 0.0);
    assert_eq!(s.y(), 0.0);
}

#[test]
fn new_with_position() {
    let pos = Cartesian2::new(1.0, 2.0);
    let s = Stereographic::new(Some(pos));
    assert_eq!(s.x(), 1.0);
    assert_eq!(s.y(), 2.0);
}

#[test]
fn constants_are_correct() {
    assert_eq!(HALF_UNIT_SPHERE_RADII.x, 0.5);
    assert_eq!(HALF_UNIT_SPHERE_RADII.y, 0.5);
    assert_eq!(HALF_UNIT_SPHERE_RADII.z, 0.5);
    assert_eq!(NORTH_POLE.z, 0.5);
    assert_eq!(SOUTH_POLE.z, -0.5);
}
