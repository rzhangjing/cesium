//! Port of `Core/EncodedCartesian3Spec.js`.
use cesium_core::cartesian3::Cartesian3;
use cesium_core::encoded_cartesian3::EncodedCartesian3;

#[test]
fn default_construct() {
    let encoded = EncodedCartesian3::default();
    assert_eq!(encoded.high, Cartesian3::ZERO);
    assert_eq!(encoded.low, Cartesian3::ZERO);
}

#[test]
fn encode_negative_value() {
    let result = EncodedCartesian3::encode(-10000000.0);
    assert!((result.high + result.low - (-10000000.0)).abs() < 1e-10);
}

#[test]
fn encode_positive_value() {
    let result = EncodedCartesian3::encode(10000000.0);
    assert!((result.high + result.low - 10000000.0).abs() < 1e-10);
}

#[test]
fn encode_zero() {
    let result = EncodedCartesian3::encode(0.0);
    assert!((result.high + result.low).abs() < 1e-15);
}

#[test]
fn from_cartesian() {
    let c = Cartesian3::new(-10000000.0, 0.0, 10000000.0);
    let encoded = EncodedCartesian3::from_cartesian(&c);
    assert!((encoded.high.x + encoded.low.x - (-10000000.0)).abs() < 1e-10);
    assert!((encoded.high.y + encoded.low.y).abs() < 1e-15);
    assert!((encoded.high.z + encoded.low.z - 10000000.0).abs() < 1e-10);
}

#[test]
fn write_elements() {
    let c = Cartesian3::new(-10000000.0, 0.0, 10000000.0);
    let mut positions = vec![0.0; 6];
    EncodedCartesian3::write_elements(&c, &mut positions, 0);

    let encoded = EncodedCartesian3::from_cartesian(&c);
    assert_eq!(encoded.high.x, positions[0]);
    assert_eq!(encoded.high.y, positions[1]);
    assert_eq!(encoded.high.z, positions[2]);
    assert_eq!(encoded.low.x, positions[3]);
    assert_eq!(encoded.low.y, positions[4]);
    assert_eq!(encoded.low.z, positions[5]);
}
