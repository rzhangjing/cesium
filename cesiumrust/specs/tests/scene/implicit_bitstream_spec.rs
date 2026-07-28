//! Tests ported from CesiumJS ImplicitAvailabilityBitstreamSpec.js (5 A-class tests)

use cesium_scene::implicit_availability_bitstream::{
    ImplicitAvailabilityBitstream, ImplicitAvailabilityBitstreamOptions,
};

#[test]
fn test_reads_bits_from_constant() {
    let length = 21;
    let bitstream = ImplicitAvailabilityBitstream::new(ImplicitAvailabilityBitstreamOptions {
        length_bits: length,
        constant: Some(true),
        bitstream: None,
        available_count: None,
        compute_available_count_enabled: false,
    });

    for i in 0..length {
        assert!(bitstream.get_bit(i));
    }
}

#[test]
fn test_reads_bits_from_bitstream() {
    // Packed representation of 0b0101 1111  1xxx xxxx
    let bitstream_u8 = vec![0xfa, 0x01];
    let expected = [false, true, false, true, true, true, true, true, true];
    let bitstream = ImplicitAvailabilityBitstream::new(ImplicitAvailabilityBitstreamOptions {
        length_bits: expected.len(),
        constant: None,
        bitstream: Some(bitstream_u8),
        available_count: None,
        compute_available_count_enabled: false,
    });

    for i in 0..expected.len() {
        assert_eq!(bitstream.get_bit(i), expected[i], "bit {}", i);
    }
}

#[test]
fn test_stores_available_count() {
    let bitstream = ImplicitAvailabilityBitstream::new(ImplicitAvailabilityBitstreamOptions {
        length_bits: 10,
        constant: None,
        bitstream: Some(vec![0x07, 0x00]),
        available_count: Some(3),
        compute_available_count_enabled: false,
    });
    assert_eq!(bitstream.available_count(), Some(3));
}

#[test]
fn test_computes_available_count_if_enabled() {
    let bitstream = ImplicitAvailabilityBitstream::new(ImplicitAvailabilityBitstreamOptions {
        length_bits: 10,
        constant: None,
        bitstream: Some(vec![0xff, 0x02]),
        available_count: None,
        compute_available_count_enabled: true,
    });
    assert_eq!(bitstream.available_count(), Some(9));
}

#[test]
fn test_does_not_compute_available_count_if_disabled() {
    let bitstream = ImplicitAvailabilityBitstream::new(ImplicitAvailabilityBitstreamOptions {
        length_bits: 10,
        constant: None,
        bitstream: Some(vec![0xff, 0x02]),
        available_count: None,
        compute_available_count_enabled: false,
    });
    assert_eq!(bitstream.available_count(), None);
}
