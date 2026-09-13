//! Binary format specs - ported from GltfLoaderSpec.js (GLB parsing),
//! Batched3DModel3DTileContentSpec.js (B3DM parsing)
//!
//! Tests GLB header validation, chunk parsing, B3DM header/feature table parsing.

use cesium_gltf::{
    B3dmData, BinaryFormatError, GlbData, B3DM_MAGIC, GLB_CHUNK_BIN, GLB_CHUNK_JSON, GLB_MAGIC,
};

/// Helper: build a minimal valid GLB with JSON chunk only.
fn make_glb(json: &str, binary: Option<&[u8]>) -> Vec<u8> {
    let json_bytes = json.as_bytes();
    let json_len = json_bytes.len();
    // Pad JSON to 4-byte alignment with spaces
    let json_padding = (4 - (json_len % 4)) % 4;
    let json_chunk_len = json_len + json_padding;

    let bin_chunk_len = binary.map(|b| {
        let bin_padding = (4 - (b.len() % 4)) % 4;
        b.len() + bin_padding
    });

    let total_len = 12 + 8 + json_chunk_len + bin_chunk_len.map(|l| 8 + l).unwrap_or(0);

    let mut data = Vec::with_capacity(total_len);
    // Header
    data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes()); // version
    data.extend_from_slice(&(total_len as u32).to_le_bytes());

    // JSON chunk
    data.extend_from_slice(&(json_chunk_len as u32).to_le_bytes());
    data.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
    data.extend_from_slice(json_bytes);
    for _ in 0..json_padding {
        data.push(b' ');
    }

    // BIN chunk (optional)
    if let Some(bin) = binary {
        let bin_padding = (4 - (bin.len() % 4)) % 4;
        let padded_len = bin.len() + bin_padding;
        data.extend_from_slice(&(padded_len as u32).to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_BIN.to_le_bytes());
        data.extend_from_slice(bin);
        for _ in 0..bin_padding {
            data.push(0);
        }
    }

    data
}

/// Helper: build a minimal B3DM with given feature table JSON and GLB.
fn make_b3dm(ft_json: &str, glb: &[u8]) -> Vec<u8> {
    let ft_bytes = ft_json.as_bytes();
    let ft_len = ft_bytes.len();
    let ft_padding = (8 - (ft_len % 8)) % 8;
    let ft_padded = ft_len + ft_padding;

    let total = 28 + ft_padded + glb.len();
    let mut data = Vec::with_capacity(total);
    data.extend_from_slice(B3DM_MAGIC);
    data.extend_from_slice(&1u32.to_le_bytes()); // version
    data.extend_from_slice(&(total as u32).to_le_bytes());
    data.extend_from_slice(&(ft_padded as u32).to_le_bytes()); // FT JSON length
    data.extend_from_slice(&0u32.to_le_bytes()); // FT binary length
    data.extend_from_slice(&0u32.to_le_bytes()); // BT JSON length
    data.extend_from_slice(&0u32.to_le_bytes()); // BT binary length
    data.extend_from_slice(ft_bytes);
    for _ in 0..ft_padding {
        data.push(b' ');
    }
    data.extend_from_slice(glb);
    data
}

// ─── GLB parsing ───────────────────────────────────────────────────────────

#[test]
fn glb_parse_json_only() {
    let json = r#"{"asset":{"version":"2.0"}}"#;
    let glb = make_glb(json, None);
    let result = GlbData::from_bytes(&glb).unwrap();
    assert_eq!(result.model.asset.version, "2.0");
    assert!(!result.has_binary());
}

#[test]
fn glb_parse_with_binary_chunk() {
    let json = r#"{"asset":{"version":"2.0"}}"#;
    let bin = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let glb = make_glb(json, Some(&bin));
    let result = GlbData::from_bytes(&glb).unwrap();
    assert!(result.has_binary());
    let binary = result.binary_chunk.unwrap();
    assert_eq!(&binary[..8], &bin);
}

#[test]
fn glb_invalid_magic() {
    let mut glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    glb[0] = 0xFF; // corrupt magic
    let err = GlbData::from_bytes(&glb).unwrap_err();
    assert!(matches!(err, BinaryFormatError::InvalidMagic { .. }));
}

#[test]
fn glb_unsupported_version() {
    let mut glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    // Set version to 1
    glb[4..8].copy_from_slice(&1u32.to_le_bytes());
    let err = GlbData::from_bytes(&glb).unwrap_err();
    assert!(matches!(err, BinaryFormatError::UnsupportedVersion(1)));
}

#[test]
fn glb_buffer_too_short() {
    let data = [0u8; 8]; // less than 12 bytes
    let err = GlbData::from_bytes(&data).unwrap_err();
    assert!(matches!(err, BinaryFormatError::BufferTooShort { expected: 12, actual: 8 }));
}

#[test]
fn glb_parse_with_meshes() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
        "accessors": [{"componentType": 5126, "count": 3, "type": "VEC3"}]
    }"#;
    let glb = make_glb(json, None);
    let result = GlbData::from_bytes(&glb).unwrap();
    assert_eq!(result.model.meshes.len(), 1);
    assert_eq!(result.model.accessors.len(), 1);
    assert_eq!(result.model.vertex_count(), 3);
}

// ─── B3DM parsing ──────────────────────────────────────────────────────────

#[test]
fn b3dm_parse_basic() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let ft_json = r#"{"BATCH_LENGTH":10}"#;
    let b3dm = make_b3dm(ft_json, &glb);
    let result = B3dmData::from_bytes(&b3dm).unwrap();
    assert_eq!(result.batch_length(), 10);
}

#[test]
fn b3dm_rtc_center() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let ft_json = r#"{"BATCH_LENGTH":5,"RTC_CENTER":[1.0,2.0,3.0]}"#;
    let b3dm = make_b3dm(ft_json, &glb);
    let result = B3dmData::from_bytes(&b3dm).unwrap();
    assert_eq!(result.batch_length(), 5);
    let rtc = result.rtc_center().unwrap();
    assert!((rtc[0] - 1.0).abs() < 1e-10);
    assert!((rtc[1] - 2.0).abs() < 1e-10);
    assert!((rtc[2] - 3.0).abs() < 1e-10);
}

#[test]
fn b3dm_no_rtc_center() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let ft_json = r#"{"BATCH_LENGTH":0}"#;
    let b3dm = make_b3dm(ft_json, &glb);
    let result = B3dmData::from_bytes(&b3dm).unwrap();
    assert!(result.rtc_center().is_none());
}

#[test]
fn b3dm_invalid_magic() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let mut b3dm = make_b3dm(r#"{"BATCH_LENGTH":0}"#, &glb);
    b3dm[0] = b'x'; // corrupt magic
    let err = B3dmData::from_bytes(&b3dm).unwrap_err();
    assert!(matches!(err, BinaryFormatError::InvalidMagic { .. }));
}

#[test]
fn b3dm_buffer_too_short() {
    let data = [0u8; 20]; // less than 28 bytes
    let err = B3dmData::from_bytes(&data).unwrap_err();
    assert!(matches!(err, BinaryFormatError::BufferTooShort { .. }));
}

#[test]
fn b3dm_glb_accessible() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"},"nodes":[{"name":"TestNode"}]}"#, None);
    let ft_json = r#"{"BATCH_LENGTH":1}"#;
    let b3dm = make_b3dm(ft_json, &glb);
    let result = B3dmData::from_bytes(&b3dm).unwrap();
    assert_eq!(result.glb.model.nodes.len(), 1);
    assert_eq!(result.glb.model.nodes[0].name.as_deref(), Some("TestNode"));
}

// ─── Constants ─────────────────────────────────────────────────────────────

#[test]
fn glb_magic_value() {
    // "glTF" in ASCII = 0x67 0x6C 0x54 0x46, little-endian u32
    assert_eq!(GLB_MAGIC, 0x46546C67);
}

#[test]
fn glb_chunk_types() {
    // JSON = 0x4E4F534A ("JSON" reversed)
    assert_eq!(GLB_CHUNK_JSON, 0x4E4F534A);
    // BIN = 0x004E4942 ("BIN\0" reversed)
    assert_eq!(GLB_CHUNK_BIN, 0x004E4942);
}

#[test]
fn b3dm_magic_bytes() {
    assert_eq!(B3DM_MAGIC, b"b3dm");
}
