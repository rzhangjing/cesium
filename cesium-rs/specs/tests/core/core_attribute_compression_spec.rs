use cesium_core::attribute_compression::AttributeCompression;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;

#[test]
fn oct_encode_and_decode_unit_vectors() {
    let vectors = vec![
        Cartesian3::UNIT_X,
        Cartesian3::UNIT_Y,
        Cartesian3::UNIT_Z,
        Cartesian3::new(-1.0, 0.0, 0.0),
        Cartesian3::new(0.0, -1.0, 0.0),
        Cartesian3::new(0.0, 0.0, -1.0),
        Cartesian3::new(1.0, 1.0, 1.0),
    ];

    for v in &vectors {
        // Normalize
        let mag = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        let normalized = Cartesian3::new(v.x / mag, v.y / mag, v.z / mag);

        let mut encoded = Cartesian2::ZERO;
        AttributeCompression::oct_encode(&normalized, &mut encoded);

        let mut decoded = Cartesian3::ZERO;
        AttributeCompression::oct_decode(encoded.x, encoded.y, &mut decoded);

        assert!(
            (decoded.x - normalized.x).abs() < CesiumMath::EPSILON2,
            "x: {} vs {}",
            decoded.x,
            normalized.x
        );
        assert!(
            (decoded.y - normalized.y).abs() < CesiumMath::EPSILON2,
            "y: {} vs {}",
            decoded.y,
            normalized.y
        );
        assert!(
            (decoded.z - normalized.z).abs() < CesiumMath::EPSILON2,
            "z: {} vs {}",
            decoded.z,
            normalized.z
        );
    }
}

#[test]
fn oct_encode_negative_hemisphere() {
    let v = Cartesian3::new(-1.0, -1.0, -1.0);
    let mag = (3.0_f64).sqrt();
    let normalized = Cartesian3::new(-1.0 / mag, -1.0 / mag, -1.0 / mag);

    let mut encoded = Cartesian2::ZERO;
    AttributeCompression::oct_encode(&normalized, &mut encoded);

    let mut decoded = Cartesian3::ZERO;
    AttributeCompression::oct_decode(encoded.x, encoded.y, &mut decoded);

    // Negative hemisphere encoding has lower precision
    assert!((decoded.x - normalized.x).abs() < CesiumMath::EPSILON1);
    assert!((decoded.y - normalized.y).abs() < CesiumMath::EPSILON1);
    assert!((decoded.z - normalized.z).abs() < CesiumMath::EPSILON1);
}

#[test]
fn decode_rgb565_matches_js_bit_exactly() {
    // Phase 2 diff golden (ac.decodeRGB565.a0): CesiumJS computes
    // `component * normalize` in f64 and rounds exactly once into the
    // Float32Array. Expected values are the f32 bit patterns produced by
    // the Node golden generator (finding D2 regression guard).
    let input: [u16; 7] = [0x0000, 0xffff, 0xf800, 0x07e0, 0x001f, 0x1234, 0xabcd];
    let expected_bits: [[u32; 3]; 7] = [
        [0x00000000, 0x00000000, 0x00000000],
        [0x3f800000, 0x3f800000, 0x3f800000],
        [0x3f800000, 0x00000000, 0x00000000],
        [0x00000000, 0x3f800000, 0x00000000],
        [0x00000000, 0x00000000, 0x3f800000],
        [0x3d842108, 0x3e8a28a3, 0x3f25294a],
        [0x3f2d6b5b, 0x3ef3cf3d, 0x3ed6b5ad],
    ];
    let decoded = AttributeCompression::decode_rgb565(&input);
    assert_eq!(decoded.len(), 21);
    for (i, bits) in expected_bits.iter().enumerate() {
        for c in 0..3 {
            assert_eq!(
                decoded[i * 3 + c].to_bits(),
                bits[c],
                "pixel {i} channel {c}: got {:08x}, expected {:08x}",
                decoded[i * 3 + c].to_bits(),
                bits[c]
            );
        }
    }
}

#[test]
fn compress_and_decompress_texture_coordinates() {
    let coords = vec![
        Cartesian2::new(0.0, 0.0),
        Cartesian2::new(1.0, 1.0),
        Cartesian2::new(0.5, 0.5),
        Cartesian2::new(0.25, 0.75),
    ];

    for tc in &coords {
        let compressed = AttributeCompression::compress_texture_coordinates(tc);
        let mut decompressed = Cartesian2::ZERO;
        AttributeCompression::decompress_texture_coordinates(compressed, &mut decompressed);

        assert!(
            (decompressed.x - tc.x).abs() < CesiumMath::EPSILON2,
            "x: {} vs {}",
            decompressed.x,
            tc.x
        );
        assert!(
            (decompressed.y - tc.y).abs() < CesiumMath::EPSILON2,
            "y: {} vs {}",
            decompressed.y,
            tc.y
        );
    }
}
