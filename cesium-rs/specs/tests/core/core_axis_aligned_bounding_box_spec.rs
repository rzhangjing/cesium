use cesium_core::axis_aligned_bounding_box::AxisAlignedBoundingBox;
use cesium_core::cartesian3::Cartesian3;

#[test]
fn default_constructor() {
    let box_a = AxisAlignedBoundingBox::default();
    assert_eq!(box_a.minimum, Cartesian3::ZERO);
    assert_eq!(box_a.maximum, Cartesian3::ZERO);
    assert_eq!(box_a.center, Cartesian3::ZERO);
}

#[test]
fn constructor_with_parameters() {
    let min = Cartesian3::new(1.0, 2.0, 3.0);
    let max = Cartesian3::new(4.0, 5.0, 6.0);
    let center = Cartesian3::new(2.5, 3.5, 4.5);
    let box_a = AxisAlignedBoundingBox::new(min, max, Some(center));
    assert_eq!(box_a.minimum, min);
    assert_eq!(box_a.maximum, max);
    assert_eq!(box_a.center, center);
}

#[test]
fn constructor_computes_center_if_not_supplied() {
    let min = Cartesian3::new(1.0, 2.0, 3.0);
    let max = Cartesian3::new(4.0, 5.0, 6.0);
    let expected_center = Cartesian3::new(2.5, 3.5, 4.5);
    let box_a = AxisAlignedBoundingBox::new(min, max, None);
    assert_eq!(box_a.minimum, min);
    assert_eq!(box_a.maximum, max);
    assert_eq!(box_a.center, expected_center);
}

#[test]
fn from_corners() {
    let min = Cartesian3::new(0.0, 0.0, 0.0);
    let max = Cartesian3::new(1.0, 1.0, 1.0);
    let expected_center = Cartesian3::new(0.5, 0.5, 0.5);
    let box_a = AxisAlignedBoundingBox::from_corners(&min, &max);
    assert_eq!(box_a.minimum, min);
    assert_eq!(box_a.maximum, max);
    assert_eq!(box_a.center, expected_center);
}

#[test]
fn half_diagonal() {
    let min = Cartesian3::new(-1.0, -2.0, -3.0);
    let max = Cartesian3::new(1.0, 2.0, 3.0);
    let box_a = AxisAlignedBoundingBox::new(min, max, None);
    let hd = box_a.half_diagonal();
    assert_eq!(hd, Cartesian3::new(1.0, 2.0, 3.0));
}

#[test]
fn equals_works() {
    let box_a = AxisAlignedBoundingBox::new(
        Cartesian3::UNIT_X,
        Cartesian3::UNIT_Y,
        Some(Cartesian3::UNIT_Z),
    );
    let box_b = AxisAlignedBoundingBox::new(
        Cartesian3::UNIT_X,
        Cartesian3::UNIT_Y,
        Some(Cartesian3::UNIT_Z),
    );
    assert!(AxisAlignedBoundingBox::equals(&box_a, &box_b));

    let box_c = AxisAlignedBoundingBox::new(
        Cartesian3::new(2.0, 3.0, 4.0),
        Cartesian3::UNIT_Y,
        Some(Cartesian3::UNIT_Y),
    );
    assert!(!AxisAlignedBoundingBox::equals(&box_a, &box_c));
}
