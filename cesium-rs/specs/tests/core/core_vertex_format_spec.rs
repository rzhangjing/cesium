//! Specs for `VertexFormat` — mirrors `Specs/Core/VertexFormatSpec.js`.

use cesium_core::vertex_format::VertexFormat;

#[test]
fn default_has_all_false() {
    let vf = VertexFormat::default();
    assert!(!vf.position);
    assert!(!vf.normal);
    assert!(!vf.st);
    assert!(!vf.tangent);
    assert!(!vf.bitangent);
    assert!(!vf.color);
}

#[test]
fn position_only() {
    let vf = VertexFormat::position_only();
    assert!(vf.position);
    assert!(!vf.normal);
    assert!(!vf.st);
}

#[test]
fn position_and_normal() {
    let vf = VertexFormat::position_and_normal();
    assert!(vf.position);
    assert!(vf.normal);
    assert!(!vf.st);
}

#[test]
fn position_normal_and_st() {
    let vf = VertexFormat::position_normal_and_st();
    assert!(vf.position);
    assert!(vf.normal);
    assert!(vf.st);
    assert!(!vf.tangent);
}

#[test]
fn position_and_st() {
    let vf = VertexFormat::position_and_st();
    assert!(vf.position);
    assert!(!vf.normal);
    assert!(vf.st);
}

#[test]
fn position_and_color() {
    let vf = VertexFormat::position_and_color();
    assert!(vf.position);
    assert!(vf.color);
    assert!(!vf.normal);
}

#[test]
fn all_format() {
    let vf = VertexFormat::all();
    assert!(vf.position);
    assert!(vf.normal);
    assert!(vf.st);
    assert!(vf.tangent);
    assert!(vf.bitangent);
    assert!(!vf.color);
}

#[test]
fn pack_and_unpack() {
    let vf = VertexFormat::position_normal_and_st();
    let mut array = [0.0f64; 6];
    vf.pack(&mut array, 0);
    assert_eq!(array[0], 1.0); // position
    assert_eq!(array[1], 1.0); // normal
    assert_eq!(array[2], 1.0); // st
    assert_eq!(array[3], 0.0); // tangent
    assert_eq!(array[4], 0.0); // bitangent
    assert_eq!(array[5], 0.0); // color

    let unpacked = VertexFormat::unpack(&array, 0, None);
    assert_eq!(unpacked, vf);
}

#[test]
fn pack_with_offset() {
    let vf = VertexFormat::position_and_color();
    let mut array = [0.0f64; 8];
    vf.pack(&mut array, 2);
    assert_eq!(array[2], 1.0); // position
    assert_eq!(array[3], 0.0); // normal
    assert_eq!(array[4], 0.0); // st
    assert_eq!(array[5], 0.0); // tangent
    assert_eq!(array[6], 0.0); // bitangent
    assert_eq!(array[7], 1.0); // color
}

#[test]
fn clone_into_creates_copy() {
    let vf = VertexFormat::all();
    let cloned = vf.clone_into(None);
    assert_eq!(cloned, vf);
}
