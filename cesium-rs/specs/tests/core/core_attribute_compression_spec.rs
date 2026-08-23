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
