//! Port of `Core/NearFarScalarSpec.js`.

use cesium_core::near_far_scalar::NearFarScalar;

#[test]
fn constructs_without_arguments() {
    let s = NearFarScalar::default();
    assert_eq!(s.near, 0.0);
    assert_eq!(s.near_value, 0.0);
    assert_eq!(s.far, 1.0);
    assert_eq!(s.far_value, 0.0);
}

#[test]
fn constructs_with_arguments() {
    let s = NearFarScalar::new(1.0, 1.0, 1.0e6, 0.5);
    assert_eq!(s.near, 1.0);
    assert_eq!(s.near_value, 1.0);
    assert_eq!(s.far, 1.0e6);
    assert_eq!(s.far_value, 0.5);
}

#[test]
fn pack_and_unpack() {
    let s = NearFarScalar::new(1.0, 2.0, 3.0, 4.0);
    let mut array = [0.0; 4];
    NearFarScalar::pack(&s, &mut array, 0);
    assert_eq!(array, [1.0, 2.0, 3.0, 4.0]);

    let unpacked = NearFarScalar::unpack(&array, 0);
    assert_eq!(unpacked, s);
}

#[test]
fn equals_works() {
    let a = NearFarScalar::new(1.0, 2.0, 3.0, 4.0);
    let b = NearFarScalar::new(1.0, 2.0, 3.0, 4.0);
    let c = NearFarScalar::new(5.0, 2.0, 3.0, 4.0);
    assert!(NearFarScalar::equals(&a, &b));
    assert!(!NearFarScalar::equals(&a, &c));
}

#[test]
fn clone_is_equal() {
    let s = NearFarScalar::new(1.0, 2.0, 3.0, 4.0);
    let cloned = s.clone();
    assert_eq!(cloned, s);
}
