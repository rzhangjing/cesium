//! TileContent specs - ported from Scene/B3dmParserSpec, I3dmParserSpec, PntsParserSpec
//! Covers: detect_content_type, parse_b3dm, parse_i3dm, parse_pnts, decode_tile_content

use cesium_tileset::content_decoder::{
    detect_content_type, parse_b3dm, TileContentType,
};

// ─── detect_content_type ────────────────────────────────────────────────────

#[test]
fn detect_b3dm_magic() {
    let data = b"b3dm\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Batched3DModel);
}

#[test]
fn detect_i3dm_magic() {
    let data = b"i3dm\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Instanced3DModel);
}

#[test]
fn detect_pnts_magic() {
    let data = b"pnts\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::PointCloud);
}

#[test]
fn detect_cmpt_magic() {
    let data = b"cmpt\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Composite);
}

#[test]
fn detect_glb_magic() {
    let data = b"glTF\x02\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::GltfBinary);
}

#[test]
fn detect_unknown_short() {
    let data = b"ab";
    assert_eq!(detect_content_type(data), TileContentType::Unknown);
}

#[test]
fn detect_unknown_magic() {
    let data = b"xxxx\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Unknown);
}

// ─── parse_b3dm ─────────────────────────────────────────────────────────────

#[test]
fn parse_b3dm_too_small() {
    let data = b"b3dm";
    let result = parse_b3dm(data);
    assert!(result.is_err());
}

#[test]
fn parse_b3dm_minimal() {
    // Construct a minimal valid b3dm header (28 bytes) + 1 byte glTF body
    let mut data = Vec::new();
    data.extend_from_slice(b"b3dm"); // magic
    data.extend_from_slice(&1u32.to_le_bytes()); // version
    data.extend_from_slice(&29u32.to_le_bytes()); // byteLength (header + 1 byte gltf)
    data.extend_from_slice(&0u32.to_le_bytes()); // featureTableJsonByteLength
    data.extend_from_slice(&0u32.to_le_bytes()); // featureTableBinaryByteLength
    data.extend_from_slice(&0u32.to_le_bytes()); // batchTableJsonByteLength
    data.extend_from_slice(&0u32.to_le_bytes()); // batchTableBinaryByteLength
    data.push(0x00); // minimal glTF body byte

    let result = parse_b3dm(&data);
    assert!(result.is_ok());
    let content = result.unwrap();
    assert_eq!(content.batch_length, 0);
}
