//! Tests for `cesium_core::HilbertOrder`.

use cesium_core::hilbert_order;

#[test]
fn encode_decode_roundtrip_level_1() {
    for x in 0..2u32 {
        for y in 0..2u32 {
            let idx = hilbert_order::encode_2d(1, x, y);
            let (dx, dy) = hilbert_order::decode_2d(1, idx);
            assert_eq!((dx, dy), (x, y));
        }
    }
}

#[test]
fn encode_decode_roundtrip_level_3() {
    let n = 1u32 << 3;
    for x in 0..n {
        for y in 0..n {
            let idx = hilbert_order::encode_2d(3, x, y);
            let (dx, dy) = hilbert_order::decode_2d(3, idx);
            assert_eq!((dx, dy), (x, y));
        }
    }
}

#[test]
fn all_indices_distinct_at_level_2() {
    let n = 1u32 << 2;
    let mut indices = Vec::new();
    for x in 0..n {
        for y in 0..n {
            indices.push(hilbert_order::encode_2d(2, x, y));
        }
    }
    indices.sort();
    indices.dedup();
    assert_eq!(indices.len(), (n * n) as usize);
}
