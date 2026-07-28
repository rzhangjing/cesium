//! Scene/B3dmParser + PntsParser + I3dmParser + CmptParser → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/B3dmParser.js
//! - Scene/PntsParser.js
//! - Scene/I3dmParser.js
//! - Scene/Composite3DTileContent.js
//! - Core/getMagic.js
//!
//! A-class tests: detect_content_type, parse_b3dm/pnts/i3dm/cmpt header parsing,
//! error handling (invalid magic, version, buffer too small), DecodedTile enum.
//! C-class omitted: glTF model loading, Draco decoding, WebGL buffer upload.

use cesium_tileset::content_decoder::{
    detect_content_type, decode_tile_content, parse_b3dm, parse_pnts, parse_i3dm, parse_cmpt,
    DecodeError, DecodedTile, TileContentType,
};

// === detect_content_type ===

#[test]
fn detect_b3dm() {
    let data = b"b3dm\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Batched3DModel);
}

#[test]
fn detect_pnts() {
    let data = b"pnts\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::PointCloud);
}

#[test]
fn detect_i3dm() {
    let data = b"i3dm\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Instanced3DModel);
}

#[test]
fn detect_cmpt() {
    let data = b"cmpt\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Composite);
}

#[test]
fn detect_glb() {
    let data = b"glTF\x02\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::GltfBinary);
}

#[test]
fn detect_subtree() {
    let data = b"subt\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::ImplicitSubtree);
}

#[test]
fn detect_unknown() {
    let data = b"xxxx\x01\x00\x00\x00";
    assert_eq!(detect_content_type(data), TileContentType::Unknown);
}

#[test]
fn detect_too_short() {
    let data = b"b3";
    assert_eq!(detect_content_type(data), TileContentType::Unknown);
}

#[test]
fn content_type_is_binary() {
    assert!(TileContentType::Batched3DModel.is_binary());
    assert!(TileContentType::PointCloud.is_binary());
    assert!(TileContentType::GltfBinary.is_binary());
    assert!(!TileContentType::Unknown.is_binary());
}

// === parse_b3dm ===

fn make_b3dm_buffer() -> Vec<u8> {
    // Construct a minimal valid b3dm:
    // Header (28 bytes) + feature table JSON + glTF body
    let ft_json = br#"{"BATCH_LENGTH":2}"#;
    let ft_json_padded = pad_to_8(ft_json);
    let gltf_body = b"glTF_FAKE_BODY";

    let total_len = 28 + ft_json_padded.len() + gltf_body.len();
    let mut buf = Vec::with_capacity(total_len);

    // Magic
    buf.extend_from_slice(b"b3dm");
    // Version = 1
    buf.extend_from_slice(&1u32.to_le_bytes());
    // byteLength
    buf.extend_from_slice(&(total_len as u32).to_le_bytes());
    // featureTableJsonByteLength
    buf.extend_from_slice(&(ft_json_padded.len() as u32).to_le_bytes());
    // featureTableBinaryByteLength = 0
    buf.extend_from_slice(&0u32.to_le_bytes());
    // batchTableJsonByteLength = 0
    buf.extend_from_slice(&0u32.to_le_bytes());
    // batchTableBinaryByteLength = 0
    buf.extend_from_slice(&0u32.to_le_bytes());

    // Feature table JSON
    buf.extend_from_slice(&ft_json_padded);
    // glTF body
    buf.extend_from_slice(gltf_body);

    buf
}

fn pad_to_8(data: &[u8]) -> Vec<u8> {
    let mut padded = data.to_vec();
    while padded.len() % 8 != 0 {
        padded.push(b' ');
    }
    padded
}

#[test]
fn parse_b3dm_valid() {
    let buf = make_b3dm_buffer();
    let result = parse_b3dm(&buf).unwrap();
    assert_eq!(result.batch_length, 2);
    assert!(result.feature_table_json.is_some());
    let ft = result.feature_table_json.unwrap();
    assert_eq!(ft["BATCH_LENGTH"], 2);
    assert!(!result.gltf.is_empty());
}

#[test]
fn parse_b3dm_invalid_magic() {
    let mut buf = make_b3dm_buffer();
    buf[0] = b'x'; // Corrupt magic
    let err = parse_b3dm(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::InvalidMagic { .. }));
}

#[test]
fn parse_b3dm_invalid_version() {
    let mut buf = make_b3dm_buffer();
    // Set version to 2
    buf[4..8].copy_from_slice(&2u32.to_le_bytes());
    let err = parse_b3dm(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::UnsupportedVersion { .. }));
}

#[test]
fn parse_b3dm_buffer_too_small() {
    let buf = vec![0u8; 10]; // Less than 28 byte header
    let err = parse_b3dm(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::BufferTooSmall { .. }));
}

// === parse_pnts ===

fn make_pnts_buffer() -> Vec<u8> {
    let ft_json = br#"{"POINTS_LENGTH":3,"POSITION":{"byteOffset":0}}"#;
    let ft_json_padded = pad_to_8(ft_json);
    // 3 points * 3 floats * 4 bytes = 36 bytes binary
    let ft_bin_len = 36usize;

    let total_len = 28 + ft_json_padded.len() + ft_bin_len;
    let mut buf = Vec::with_capacity(total_len);

    buf.extend_from_slice(b"pnts");
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&(total_len as u32).to_le_bytes());
    buf.extend_from_slice(&(ft_json_padded.len() as u32).to_le_bytes());
    buf.extend_from_slice(&(ft_bin_len as u32).to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // bt json len
    buf.extend_from_slice(&0u32.to_le_bytes()); // bt bin len

    buf.extend_from_slice(&ft_json_padded);
    // Binary positions (3 points)
    for i in 0..9u32 {
        buf.extend_from_slice(&(i as f32).to_le_bytes());
    }

    buf
}

#[test]
fn parse_pnts_valid() {
    let buf = make_pnts_buffer();
    let result = parse_pnts(&buf).unwrap();
    assert!(result.feature_table_json.is_some());
    let ft = result.feature_table_json.unwrap();
    assert_eq!(ft["POINTS_LENGTH"], 3);
    assert!(!result.feature_table_binary.is_empty());
}

#[test]
fn parse_pnts_invalid_magic() {
    let mut buf = make_pnts_buffer();
    buf[0..4].copy_from_slice(b"xxxx");
    let err = parse_pnts(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::InvalidMagic { .. }));
}

// === decode_tile_content (dispatch) ===

#[test]
fn decode_tile_content_b3dm() {
    let buf = make_b3dm_buffer();
    let decoded = decode_tile_content(&buf).unwrap();
    assert_eq!(decoded.content_type(), TileContentType::Batched3DModel);
}

#[test]
fn decode_tile_content_pnts() {
    let buf = make_pnts_buffer();
    let decoded = decode_tile_content(&buf).unwrap();
    assert_eq!(decoded.content_type(), TileContentType::PointCloud);
}

#[test]
fn decode_tile_content_glb() {
    let buf = b"glTF\x02\x00\x00\x00some_glb_data";
    let decoded = decode_tile_content(buf).unwrap();
    assert_eq!(decoded.content_type(), TileContentType::GltfBinary);
    if let DecodedTile::Glb(data) = decoded {
        assert!(!data.is_empty());
    } else {
        panic!("Expected Glb variant");
    }
}

#[test]
fn decode_tile_content_unknown() {
    let buf = b"unknown_format_data";
    let result = decode_tile_content(buf);
    // Unknown magic should return error or Unknown type
    assert!(result.is_err() || result.unwrap().content_type() == TileContentType::Unknown);
}
