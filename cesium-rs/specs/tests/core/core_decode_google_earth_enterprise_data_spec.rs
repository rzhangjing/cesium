//! Tests for `cesium_core::decode_google_earth_enterprise_data`.

use cesium_core::decode_google_earth_enterprise_data::decode_google_earth_enterprise_data;

#[test]
fn xor_decode_roundtrip() {
    let key = vec![0xAA, 0xBB, 0xCC, 0xDD];
    let original = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let mut data = original.clone();

    // Encode
    decode_google_earth_enterprise_data(&key, &mut data);
    // Data should be different after XOR
    assert_ne!(data, original);

    // Decode (XOR again with same key)
    decode_google_earth_enterprise_data(&key, &mut data);
    assert_eq!(data, original);
}

#[test]
fn empty_key_does_nothing() {
    let key = vec![];
    let mut data = vec![1, 2, 3, 4];
    let original = data.clone();
    decode_google_earth_enterprise_data(&key, &mut data);
    assert_eq!(data, original);
}

#[test]
fn compressed_magic_skips_decoding() {
    let key = vec![0x01, 0x02, 0x03, 0x04];
    let mut data = vec![0xad, 0xde, 0x68, 0x74, 0x00, 0x00, 0x00, 0x00];
    let original = data.clone();
    decode_google_earth_enterprise_data(&key, &mut data);
    // Should not be modified because magic matches COMPRESSED_MAGIC_SWAP
    assert_eq!(data, original);
}
