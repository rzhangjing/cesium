//! VertexFormat spec tests.
//!
//! Maps to CesiumJS:
//! - Core/VertexFormatSpec.js
//!
//! A-class tests: clone, pack/unpack, constants.

use cesium_geospatial::VertexFormat;

#[test]
fn vertex_format_clone() {
    let vf = VertexFormat {
        position: true,
        normal: true,
        st: false,
        tangent: false,
        bitangent: false,
    };
    let cloned = vf;
    assert_eq!(cloned, vf);
}

#[test]
fn vertex_format_pack() {
    let vf = VertexFormat::POSITION_AND_NORMAL;
    let mut array = vec![0.0; 5];
    vf.pack(&mut array, 0);
    assert_eq!(array, vec![1.0, 1.0, 0.0, 0.0, 0.0]);
}

#[test]
fn vertex_format_unpack() {
    let array = vec![1.0, 1.0, 0.0, 0.0, 0.0];
    let vf = VertexFormat::unpack(&array, 0);
    assert_eq!(vf, VertexFormat::POSITION_AND_NORMAL);
}

#[test]
fn vertex_format_pack_array() {
    let vf = VertexFormat::ALL;
    let array = vf.pack_array();
    assert_eq!(array, vec![1.0, 1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn vertex_format_unpack_array() {
    let array = vec![1.0, 0.0, 1.0, 0.0, 0.0];
    let vf = VertexFormat::unpack_array(&array);
    assert_eq!(vf, VertexFormat::POSITION_AND_ST);
}

#[test]
fn vertex_format_roundtrip() {
    let original = VertexFormat {
        position: true,
        normal: false,
        st: true,
        tangent: true,
        bitangent: false,
    };
    let packed = original.pack_array();
    let unpacked = VertexFormat::unpack_array(&packed);
    assert_eq!(unpacked, original);
}

#[test]
fn vertex_format_packed_length() {
    assert_eq!(VertexFormat::PACKED_LENGTH, 5);
}

#[test]
fn vertex_format_constants() {
    // ALL
    assert!(VertexFormat::ALL.position);
    assert!(VertexFormat::ALL.normal);
    assert!(VertexFormat::ALL.st);
    assert!(VertexFormat::ALL.tangent);
    assert!(VertexFormat::ALL.bitangent);

    // POSITION_ONLY
    assert!(VertexFormat::POSITION_ONLY.position);
    assert!(!VertexFormat::POSITION_ONLY.normal);
    assert!(!VertexFormat::POSITION_ONLY.st);
    assert!(!VertexFormat::POSITION_ONLY.tangent);
    assert!(!VertexFormat::POSITION_ONLY.bitangent);

    // POSITION_AND_NORMAL
    assert!(VertexFormat::POSITION_AND_NORMAL.position);
    assert!(VertexFormat::POSITION_AND_NORMAL.normal);
    assert!(!VertexFormat::POSITION_AND_NORMAL.st);

    // POSITION_AND_ST
    assert!(VertexFormat::POSITION_AND_ST.position);
    assert!(!VertexFormat::POSITION_AND_ST.normal);
    assert!(VertexFormat::POSITION_AND_ST.st);
}

#[test]
fn vertex_format_default() {
    let vf = VertexFormat::default();
    assert_eq!(vf, VertexFormat::ALL);
}
