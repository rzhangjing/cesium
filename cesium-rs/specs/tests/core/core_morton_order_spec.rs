//! Tests for `cesium_core::MortonOrder`.

use cesium_core::morton_order::MortonOrder;

#[test]
fn encode_2d_origin() {
    assert_eq!(MortonOrder::encode_2d(0, 0), 0);
}

#[test]
fn decode_2d_zero() {
    assert_eq!(MortonOrder::decode_2d(0), (0, 0));
}

#[test]
fn encode_decode_2d_roundtrip() {
    for x in [0u16, 1, 7, 255, 1000] {
        for y in [0u16, 1, 7, 255, 1000] {
            let m = MortonOrder::encode_2d(x, y);
            assert_eq!(MortonOrder::decode_2d(m), (x, y));
        }
    }
}

#[test]
fn encode_decode_3d_roundtrip() {
    for x in [0u16, 1, 7, 100] {
        for y in [0u16, 1, 7, 100] {
            for z in [0u16, 1, 7, 100] {
                let m = MortonOrder::encode_3d(x, y, z);
                assert_eq!(MortonOrder::decode_3d(m), (x, y, z));
            }
        }
    }
}

#[test]
fn known_2d_values() {
    assert_eq!(MortonOrder::encode_2d(1, 0), 1);
    assert_eq!(MortonOrder::encode_2d(0, 1), 2);
    assert_eq!(MortonOrder::encode_2d(1, 1), 3);
}
