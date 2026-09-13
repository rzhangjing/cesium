//! GLB/b3dm binary format + Vector3DTile specs
//! Tests: GlbData parsing, B3dmData parsing, Vector3DTilePoints/Polylines/Polygons, MVT decode

use cesium_gltf::binary_format::{
    B3dmData, BinaryFormatError, GlbData, GLB_CHUNK_BIN, GLB_CHUNK_JSON, GLB_MAGIC,
};
use cesium_vector::{
    decode_mvt_geometry, MvtFeature, MvtGeometryType, MvtLayer, Vector3DTileContent,
    Vector3DTilePoints, Vector3DTilePolygons, Vector3DTilePolylines, Vector3DTileType,
};
use glam::DVec3;

// ═══════════════════════════════════════════════════════════════════════════════
// GLB Parsing
// ═══════════════════════════════════════════════════════════════════════════════

fn make_glb(json: &str, binary: Option<&[u8]>) -> Vec<u8> {
    let json_bytes = json.as_bytes();
    let json_padded: Vec<u8> = {
        let mut v = json_bytes.to_vec();
        while v.len() % 4 != 0 {
            v.push(0x20); // space padding
        }
        v
    };

    let mut total = 12 + 8 + json_padded.len();
    let bin_padded: Vec<u8> = if let Some(bin) = binary {
        let mut v = bin.to_vec();
        while v.len() % 4 != 0 {
            v.push(0x00);
        }
        total += 8 + v.len();
        v
    } else {
        vec![]
    };

    let mut data = Vec::with_capacity(total);
    // Header
    data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&(total as u32).to_le_bytes());
    // JSON chunk
    data.extend_from_slice(&(json_padded.len() as u32).to_le_bytes());
    data.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
    data.extend_from_slice(&json_padded);
    // BIN chunk
    if binary.is_some() {
        data.extend_from_slice(&(bin_padded.len() as u32).to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_BIN.to_le_bytes());
        data.extend_from_slice(&bin_padded);
    }
    data
}

#[test]
fn glb_parse_minimal() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let result = GlbData::from_bytes(&glb).unwrap();
    assert!(!result.has_binary());
}

#[test]
fn glb_parse_with_binary() {
    let bin_data = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, Some(&bin_data));
    let result = GlbData::from_bytes(&glb).unwrap();
    assert!(result.has_binary());
    let chunk = result.binary_chunk.unwrap();
    assert!(chunk.len() >= 8);
    assert_eq!(&chunk[..8], &bin_data);
}

#[test]
fn glb_error_buffer_too_short() {
    let data = [0u8; 8];
    let result = GlbData::from_bytes(&data);
    assert!(result.is_err());
    match result.unwrap_err() {
        BinaryFormatError::BufferTooShort { expected, actual } => {
            assert_eq!(expected, 12);
            assert_eq!(actual, 8);
        }
        _ => panic!("Expected BufferTooShort"),
    }
}

#[test]
fn glb_error_invalid_magic() {
    let mut glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    glb[0] = 0xFF; // corrupt magic
    let result = GlbData::from_bytes(&glb);
    assert!(matches!(result, Err(BinaryFormatError::InvalidMagic { .. })));
}

#[test]
fn glb_error_unsupported_version() {
    let mut glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    // Set version to 1
    glb[4..8].copy_from_slice(&1u32.to_le_bytes());
    let result = GlbData::from_bytes(&glb);
    assert!(matches!(result, Err(BinaryFormatError::UnsupportedVersion(1))));
}

// ═══════════════════════════════════════════════════════════════════════════════
// b3dm Parsing
// ═══════════════════════════════════════════════════════════════════════════════

fn make_b3dm(glb: &[u8], batch_length: u32) -> Vec<u8> {
    let ft_json = format!(r#"{{"BATCH_LENGTH":{}}}"#, batch_length);
    let mut ft_bytes = ft_json.into_bytes();
    while ft_bytes.len() % 8 != 0 {
        ft_bytes.push(0x20);
    }

    let total = 28 + ft_bytes.len() + glb.len();
    let mut data = Vec::with_capacity(total);
    data.extend_from_slice(b"b3dm");
    data.extend_from_slice(&1u32.to_le_bytes()); // version
    data.extend_from_slice(&(total as u32).to_le_bytes());
    data.extend_from_slice(&(ft_bytes.len() as u32).to_le_bytes()); // ft json len
    data.extend_from_slice(&0u32.to_le_bytes()); // ft binary len
    data.extend_from_slice(&0u32.to_le_bytes()); // bt json len
    data.extend_from_slice(&0u32.to_le_bytes()); // bt binary len
    data.extend_from_slice(&ft_bytes);
    data.extend_from_slice(glb);
    data
}

#[test]
fn b3dm_parse_valid() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let b3dm = make_b3dm(&glb, 10);
    let result = B3dmData::from_bytes(&b3dm).unwrap();
    assert_eq!(result.feature_table.batch_length, 10);
}

#[test]
fn b3dm_error_too_short() {
    let data = [0u8; 20];
    let result = B3dmData::from_bytes(&data);
    assert!(matches!(
        result,
        Err(BinaryFormatError::BufferTooShort { expected: 28, .. })
    ));
}

#[test]
fn b3dm_error_invalid_magic() {
    let glb = make_glb(r#"{"asset":{"version":"2.0"}}"#, None);
    let mut b3dm = make_b3dm(&glb, 5);
    b3dm[0] = b'x'; // corrupt magic
    let result = B3dmData::from_bytes(&b3dm);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Vector3DTilePoints
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vector_points_add_and_query() {
    let mut points = Vector3DTilePoints::new();
    assert_eq!(points.points_length(), 0);

    points.add_point(DVec3::new(1.0, 2.0, 3.0), 0);
    points.add_point(DVec3::new(4.0, 5.0, 6.0), 1);
    points.add_point(DVec3::new(7.0, 8.0, 9.0), 2);

    assert_eq!(points.points_length(), 3);
    assert_eq!(points.positions[0], DVec3::new(1.0, 2.0, 3.0));
    assert_eq!(points.batch_ids, vec![0, 1, 2]);
}

#[test]
fn vector_points_byte_length() {
    let mut points = Vector3DTilePoints::new();
    points.add_point(DVec3::ONE, 0);
    // 1 position * 24 bytes + 1 batch_id * 4 bytes = 28
    assert_eq!(points.geometry_byte_length(), 28);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Vector3DTilePolylines
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vector_polylines_add_and_query() {
    let mut polylines = Vector3DTilePolylines::new();
    polylines.add_polyline(
        &[DVec3::ZERO, DVec3::X, DVec3::new(2.0, 0.0, 0.0)],
        0,
        2.0,
    );
    polylines.add_polyline(&[DVec3::ZERO, DVec3::Y], 1, 1.5);

    assert_eq!(polylines.polylines_length(), 2);
    assert_eq!(polylines.get_polyline(0).unwrap().len(), 3);
    assert_eq!(polylines.get_polyline(1).unwrap().len(), 2);
    assert!(polylines.get_polyline(2).is_none());
}

#[test]
fn vector_polylines_triangles_length() {
    let mut polylines = Vector3DTilePolylines::new();
    // 3 vertices = 2 segments = 4 triangles
    polylines.add_polyline(&[DVec3::ZERO, DVec3::X, DVec3::Y], 0, 1.0);
    // 2 vertices = 1 segment = 2 triangles
    polylines.add_polyline(&[DVec3::ZERO, DVec3::Z], 1, 1.0);

    assert_eq!(polylines.triangles_length(), 6);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Vector3DTilePolygons
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vector_polygons_add_and_query() {
    let mut polygons = Vector3DTilePolygons::new();
    polygons.add_polygon(
        &[
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
        ],
        &[0, 1, 2, 0, 2, 3],
        0,
        0.0,
        20.0,
    );

    assert_eq!(polygons.polygons_length(), 1);
    assert_eq!(polygons.triangles_length(), 2);
    assert!(polygons.geometry_byte_length() > 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Vector3DTileContent
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vector_content_types() {
    let mut content = Vector3DTileContent::new();
    assert!(content.content_types().is_empty());

    content.points = Some(Vector3DTilePoints::new());
    content.polylines = Some(Vector3DTilePolylines::new());
    content.polygons = Some(Vector3DTilePolygons::new());

    let types = content.content_types();
    assert_eq!(types.len(), 3);
    assert!(types.contains(&Vector3DTileType::Points));
    assert!(types.contains(&Vector3DTileType::Polylines));
    assert!(types.contains(&Vector3DTileType::Polygons));
}

// ═══════════════════════════════════════════════════════════════════════════════
// MVT Layer/Feature
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn mvt_layer_defaults() {
    let layer = MvtLayer::new("roads");
    assert_eq!(layer.name, "roads");
    assert_eq!(layer.version, 2);
    assert_eq!(layer.extent, 4096);
    assert!(layer.features.is_empty());
}

#[test]
fn mvt_feature_creation() {
    let mut feature = MvtFeature::new(MvtGeometryType::LineString);
    feature.id = Some(123);
    assert_eq!(feature.geometry_type, MvtGeometryType::LineString);
    assert_eq!(feature.id, Some(123));
}

// ═══════════════════════════════════════════════════════════════════════════════
// MVT Geometry Decode
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn mvt_decode_point() {
    // MoveTo count=1, params: zigzag(50)=100, zigzag(25)=50
    let commands = vec![(1 << 3) | 1, 100, 50];
    let rings = decode_mvt_geometry(&commands, 4096);
    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].len(), 1);
    assert!((rings[0][0].x - 50.0 / 4096.0).abs() < 1e-10);
    assert!((rings[0][0].y - 25.0 / 4096.0).abs() < 1e-10);
}

#[test]
fn mvt_decode_linestring() {
    // MoveTo(1) + LineTo(2)
    let commands = vec![
        (1 << 3) | 1, // MoveTo count=1
        4,            // zigzag(2)
        4,            // zigzag(2)
        (2 << 3) | 2, // LineTo count=2
        2,            // zigzag(1)
        0,            // zigzag(0)
        0,            // zigzag(0)
        2,            // zigzag(1)
    ];
    let rings = decode_mvt_geometry(&commands, 4096);
    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].len(), 3);
    // First point: (2/4096, 2/4096)
    assert!((rings[0][0].x - 2.0 / 4096.0).abs() < 1e-10);
    // Second point: (3/4096, 2/4096)
    assert!((rings[0][1].x - 3.0 / 4096.0).abs() < 1e-10);
    // Third point: (3/4096, 3/4096)
    assert!((rings[0][2].y - 3.0 / 4096.0).abs() < 1e-10);
}

#[test]
fn mvt_decode_polygon_closed() {
    // Square: MoveTo + LineTo(2) + ClosePath
    let commands = vec![
        (1 << 3) | 1, // MoveTo count=1
        0,            // x=0
        0,            // y=0
        (3 << 3) | 2, // LineTo count=3
        2,            // dx=1
        0,            // dy=0
        0,            // dx=0
        2,            // dy=1
        3,            // dx=-1 (zigzag(3)=-2? no, zigzag(3)=-2)
        0,            // dy=0
        15,           // ClosePath
    ];
    let rings = decode_mvt_geometry(&commands, 4096);
    assert_eq!(rings.len(), 1);
    // Ring should be closed (first == last)
    assert_eq!(rings[0].first(), rings[0].last());
}

#[test]
fn mvt_decode_multipoint() {
    // MoveTo count=3
    let commands = vec![
        (3 << 3) | 1, // MoveTo count=3
        2, 4,         // point 1: (1, 2)
        2, 0,         // point 2: (2, 2) delta
        0, 2,         // point 3: (2, 3) delta
    ];
    let rings = decode_mvt_geometry(&commands, 4096);
    // Each MoveTo starts a new ring
    assert_eq!(rings.len(), 3);
}

#[test]
fn mvt_decode_empty() {
    let commands: Vec<u32> = vec![];
    let rings = decode_mvt_geometry(&commands, 4096);
    assert!(rings.is_empty());
}
