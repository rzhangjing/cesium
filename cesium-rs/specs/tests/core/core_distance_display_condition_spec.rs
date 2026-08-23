//! Port of `Core/DistanceDisplayConditionSpec.js`.
use cesium_core::distance_display_condition::DistanceDisplayCondition;

#[test]
fn default_constructs() {
    let dc = DistanceDisplayCondition::default();
    assert_eq!(dc.near, 0.0);
    assert_eq!(dc.far, f64::MAX);
}

#[test]
fn constructs_with_parameters() {
    let dc = DistanceDisplayCondition::new(10.0, 100.0);
    assert_eq!(dc.near, 10.0);
    assert_eq!(dc.far, 100.0);
}

#[test]
fn gets_and_sets_properties() {
    let mut dc = DistanceDisplayCondition::default();
    dc.near = 10.0;
    dc.far = 100.0;
    assert_eq!(dc.near, 10.0);
    assert_eq!(dc.far, 100.0);
}

#[test]
fn equals_static() {
    let dc = DistanceDisplayCondition::new(10.0, 100.0);
    assert!(DistanceDisplayCondition::equals(&dc, &DistanceDisplayCondition::new(10.0, 100.0)));
    assert!(!DistanceDisplayCondition::equals(&dc, &DistanceDisplayCondition::new(11.0, 100.0)));
    assert!(!DistanceDisplayCondition::equals(&dc, &DistanceDisplayCondition::new(10.0, 101.0)));
}

#[test]
fn equals_via_partial_eq() {
    let dc = DistanceDisplayCondition::new(10.0, 100.0);
    assert_eq!(dc, DistanceDisplayCondition::new(10.0, 100.0));
    assert_ne!(dc, DistanceDisplayCondition::new(11.0, 100.0));
}

#[test]
fn clone_works() {
    let dc = DistanceDisplayCondition::new(10.0, 100.0);
    let cloned = dc.clone();
    assert_eq!(dc, cloned);
}

#[test]
fn pack_and_unpack() {
    let dc = DistanceDisplayCondition::new(1.0, 2.0);
    let mut array = vec![0.0; DistanceDisplayCondition::PACKED_LENGTH];
    DistanceDisplayCondition::pack(&dc, &mut array, 0);
    assert_eq!(array[0], 1.0);
    assert_eq!(array[1], 2.0);

    let unpacked = DistanceDisplayCondition::unpack(&array, 0);
    assert_eq!(unpacked.near, 1.0);
    assert_eq!(unpacked.far, 2.0);
}
