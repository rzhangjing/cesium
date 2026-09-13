//! Content decoder extended spec tests (i3dm, pnts, cmpt, detect).
//!
//! Maps to CesiumJS:
//! - Scene/I3dmParserSpec.js
//! - Scene/PntsParserSpec.js
//! - Scene/Composite3DTileContentSpec.js
//! - Core/getMagicSpec.js
//!
//! A-class tests: binary parsing, content type detection, error handling.

use cesium_tileset::content_decoder::{
    detect_content_type, parse_cmpt, parse_i3dm, parse_pnts, DecodeError, TileContentType,
};

// === Helper: build binary buffers ===

fn make_i3dm(ft_json: &str, gltf_format: u32, gltf: &[u8]) -> Vec<u8> {
    let ft_bytes = ft_json.as_bytes();
    let ft_json_len = ft_bytes.len() as u32;
    let ft_bin_len = 0u32;
    let bt_json_len = 0u32;
    let bt_bin_len = 0u32;
    let gltf_len = gltf.len() as u32;

    // Header: magic(4) + version(4) + byteLength(4) + ftJsonLen(4) + ftBinLen(4) +
    //         btJsonLen(4) + btBinLen(4) + gltfFormat(4) = 32 bytes
    let total = 32 + ft_json_len + gltf_len;

    let mut buf = Vec::with_capacity(total as usize);
    buf.extend_from_slice(b"i3dm");
    buf.extend_from_slice(&1u32.to_le_bytes()); // version
    buf.extend_from_slice(&total.to_le_bytes());
    buf.extend_from_slice(&ft_json_len.to_le_bytes());
    buf.extend_from_slice(&ft_bin_len.to_le_bytes());
    buf.extend_from_slice(&bt_json_len.to_le_bytes());
    buf.extend_from_slice(&bt_bin_len.to_le_bytes());
    buf.extend_from_slice(&gltf_format.to_le_bytes());
    buf.extend_from_slice(ft_bytes);
    buf.extend_from_slice(gltf);
    buf
}

fn make_pnts(ft_json: &str, ft_binary: &[u8]) -> Vec<u8> {
    let ft_bytes = ft_json.as_bytes();
    let ft_json_len = ft_bytes.len() as u32;
    let ft_bin_len = ft_binary.len() as u32;
    let bt_json_len = 0u32;
    let bt_bin_len = 0u32;

    // Header: magic(4) + version(4) + byteLength(4) + ftJsonLen(4) + ftBinLen(4) +
    //         btJsonLen(4) + btBinLen(4) = 28 bytes
    let total = 28 + ft_json_len + ft_bin_len;

    let mut buf = Vec::with_capacity(total as usize);
    buf.extend_from_slice(b"pnts");
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&total.to_le_bytes());
    buf.extend_from_slice(&ft_json_len.to_le_bytes());
    buf.extend_from_slice(&ft_bin_len.to_le_bytes());
    buf.extend_from_slice(&bt_json_len.to_le_bytes());
    buf.extend_from_slice(&bt_bin_len.to_le_bytes());
    buf.extend_from_slice(ft_bytes);
    buf.extend_from_slice(ft_binary);
    buf
}

fn make_cmpt(inner_tiles: &[&[u8]]) -> Vec<u8> {
    let tiles_length = inner_tiles.len() as u32;
    let inner_total: u32 = inner_tiles.iter().map(|t| t.len() as u32).sum();

    // Header: magic(4) + version(4) + byteLength(4) + tilesLength(4) = 16 bytes
    let total = 16 + inner_total;

    let mut buf = Vec::with_capacity(total as usize);
    buf.extend_from_slice(b"cmpt");
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&total.to_le_bytes());
    buf.extend_from_slice(&tiles_length.to_le_bytes());
    for tile in inner_tiles {
        buf.extend_from_slice(tile);
    }
    buf
}

fn make_glb_simple() -> Vec<u8> {
    // Minimal GLB: magic + version + length + JSON chunk
    let json = b"{}";
    let json_len = json.len() as u32;
    let total = 12 + 8 + json_len;

    let mut buf = Vec::new();
    buf.extend_from_slice(b"glTF");
    buf.extend_from_slice(&2u32.to_le_bytes());
    buf.extend_from_slice(&total.to_le_bytes());
    // JSON chunk
    buf.extend_from_slice(&json_len.to_le_bytes());
    buf.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // JSON
    buf.extend_from_slice(json);
    buf
}

// === detect_content_type ===

#[test]
fn detect_b3dm() {
    assert_eq!(
        detect_content_type(b"b3dm\x01\x00\x00\x00"),
        TileContentType::Batched3DModel
    );
}

#[test]
fn detect_i3dm() {
    assert_eq!(
        detect_content_type(b"i3dm\x01\x00\x00\x00"),
        TileContentType::Instanced3DModel
    );
}

#[test]
fn detect_pnts() {
    assert_eq!(
        detect_content_type(b"pnts\x01\x00\x00\x00"),
        TileContentType::PointCloud
    );
}

#[test]
fn detect_cmpt() {
    assert_eq!(
        detect_content_type(b"cmpt\x01\x00\x00\x00"),
        TileContentType::Composite
    );
}

#[test]
fn detect_glb() {
    assert_eq!(
        detect_content_type(b"glTF\x02\x00\x00\x00"),
        TileContentType::GltfBinary
    );
}

#[test]
fn detect_subtree() {
    assert_eq!(
        detect_content_type(b"subt\x01\x00\x00\x00"),
        TileContentType::ImplicitSubtree
    );
}

#[test]
fn detect_unknown() {
    assert_eq!(detect_content_type(b"xxxx"), TileContentType::Unknown);
    assert_eq!(detect_content_type(b""), TileContentType::Unknown);
    assert_eq!(detect_content_type(b"ab"), TileContentType::Unknown);
}

#[test]
fn content_type_is_binary() {
    assert!(TileContentType::Batched3DModel.is_binary());
    assert!(TileContentType::Instanced3DModel.is_binary());
    assert!(TileContentType::PointCloud.is_binary());
    assert!(TileContentType::Composite.is_binary());
    assert!(TileContentType::GltfBinary.is_binary());
    assert!(!TileContentType::Unknown.is_binary());
}

// === parse_i3dm ===

#[test]
fn i3dm_parse_basic() {
    let glb = make_glb_simple();
    let buf = make_i3dm(r#"{"INSTANCES_LENGTH":10}"#, 1, &glb);
    let result = parse_i3dm(&buf).unwrap();

    assert_eq!(result.gltf_format, 1);
    assert!(!result.gltf.is_empty());
    let ft = result.feature_table_json.unwrap();
    assert_eq!(ft["INSTANCES_LENGTH"], 10);
}

#[test]
fn i3dm_gltf_format_uri() {
    let uri = b"model.gltf";
    let buf = make_i3dm(r#"{"INSTANCES_LENGTH":5}"#, 0, uri);
    let result = parse_i3dm(&buf).unwrap();

    assert_eq!(result.gltf_format, 0);
    assert_eq!(result.gltf, uri.to_vec());
}

#[test]
fn i3dm_invalid_magic() {
    let mut buf = make_i3dm("{}", 1, &[]);
    buf[0] = b'x';
    let err = parse_i3dm(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::InvalidMagic { .. }));
}

#[test]
fn i3dm_buffer_too_short() {
    let buf = b"i3dm\x01";
    let err = parse_i3dm(buf).unwrap_err();
    assert!(matches!(err, DecodeError::BufferTooSmall { .. }));
}

#[test]
fn i3dm_unsupported_version() {
    let mut buf = make_i3dm("{}", 1, &[]);
    // Set version to 2
    buf[4..8].copy_from_slice(&2u32.to_le_bytes());
    let err = parse_i3dm(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::UnsupportedVersion { .. }));
}

// === parse_pnts ===

#[test]
fn pnts_parse_basic() {
    let ft_json = r#"{"POINTS_LENGTH":100,"POSITION":{"byteOffset":0}}"#;
    let ft_binary = vec![0u8; 1200]; // 100 points * 3 floats * 4 bytes
    let buf = make_pnts(ft_json, &ft_binary);
    let result = parse_pnts(&buf).unwrap();

    let ft = result.feature_table_json.unwrap();
    assert_eq!(ft["POINTS_LENGTH"], 100);
    assert_eq!(result.feature_table_binary.len(), 1200);
}

#[test]
fn pnts_empty_feature_table() {
    let buf = make_pnts("", &[]);
    let err = parse_pnts(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::EmptyFeatureTable));
}

#[test]
fn pnts_invalid_magic() {
    let mut buf = make_pnts(r#"{"POINTS_LENGTH":1}"#, &[]);
    buf[0] = b'x';
    let err = parse_pnts(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::InvalidMagic { .. }));
}

#[test]
fn pnts_buffer_too_short() {
    let buf = b"pnts";
    let err = parse_pnts(buf).unwrap_err();
    assert!(matches!(err, DecodeError::BufferTooSmall { .. }));
}

#[test]
fn pnts_no_binary_body() {
    let ft_json = r#"{"POINTS_LENGTH":10}"#;
    let buf = make_pnts(ft_json, &[]);
    let result = parse_pnts(&buf).unwrap();
    assert!(result.feature_table_binary.is_empty());
}

// === parse_cmpt ===

#[test]
fn cmpt_parse_empty() {
    let buf = make_cmpt(&[]);
    let result = parse_cmpt(&buf).unwrap();
    assert_eq!(result.inner_tiles.len(), 0);
}

#[test]
fn cmpt_parse_single_glb() {
    let glb = make_glb_simple();
    let buf = make_cmpt(&[&glb]);
    let result = parse_cmpt(&buf).unwrap();
    assert_eq!(result.inner_tiles.len(), 1);
}

#[test]
fn cmpt_parse_multiple_tiles() {
    let glb1 = make_glb_simple();
    let glb2 = make_glb_simple();
    let buf = make_cmpt(&[&glb1, &glb2]);
    let result = parse_cmpt(&buf).unwrap();
    assert_eq!(result.inner_tiles.len(), 2);
}

#[test]
fn cmpt_invalid_magic() {
    let mut buf = make_cmpt(&[]);
    buf[0] = b'x';
    let err = parse_cmpt(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::InvalidMagic { .. }));
}

#[test]
fn cmpt_buffer_too_short() {
    let buf = b"cmpt\x01";
    let err = parse_cmpt(buf).unwrap_err();
    assert!(matches!(err, DecodeError::BufferTooSmall { .. }));
}

#[test]
fn cmpt_unsupported_version() {
    let mut buf = make_cmpt(&[]);
    buf[4..8].copy_from_slice(&3u32.to_le_bytes());
    let err = parse_cmpt(&buf).unwrap_err();
    assert!(matches!(err, DecodeError::UnsupportedVersion { .. }));
}
