//! Tests for `cesium_core::decode_google_earth_enterprise_data`.

use cesium_core::decode_google_earth_enterprise_data::decode_google_earth_enterprise_data;

#[test]
fn xor_decode_roundtrip() {
    // Note: the JS algorithm reads 8-byte (two u32) chunks from the key, so
    // realistic keys are long (dbRoot keys are 1024 bytes); a 4-byte key
    // would make the JS DataView reads throw RangeError. The bytes are also
    // chosen so that neither the plaintext nor the XOR'd form starts with
    // the compressed magic (which would make the decoder return early,
    // exactly like the JS implementation).
    let key: Vec<u8> = (0u8..24).collect();
    // 20 bytes: two full 8-byte blocks + a 4-byte tail.
    let original = vec![0x01u8; 20];
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
