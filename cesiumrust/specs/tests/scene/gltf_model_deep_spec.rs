//! GltfModel deep specs - ported from GltfLoaderSpec.js, ModelReaderSpec.js
//!
//! Tests GltfModel parsing, triangle_count, vertex_count, Node::local_transform,
//! Accessor component/element sizes, and binary data reading (f32/u16/u32).

use cesium_gltf::{
    Accessor, AccessorType, BufferView, ComponentType, GltfMesh, GltfModel, Node, Primitive,
    PrimitiveMode,
};
use std::collections::HashMap;

// ─── GltfModel parsing ─────────────────────────────────────────────────────

#[test]
fn parse_minimal_gltf() {
    let json = r#"{"asset":{"version":"2.0"}}"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.asset.version, "2.0");
    assert!(model.scenes.is_empty());
    assert!(model.nodes.is_empty());
    assert!(model.meshes.is_empty());
}

#[test]
fn parse_with_scenes_and_nodes() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0, 1]}],
        "nodes": [
            {"name": "Root", "children": [1]},
            {"name": "Child", "translation": [1.0, 2.0, 3.0]}
        ]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.scene, Some(0));
    assert_eq!(model.scenes.len(), 1);
    assert_eq!(model.nodes.len(), 2);
    assert_eq!(model.nodes[0].name.as_deref(), Some("Root"));
    assert_eq!(model.nodes[1].translation, Some([1.0, 2.0, 3.0]));
}

#[test]
fn default_scene_selection() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "scene": 1,
        "scenes": [{"name": "A"}, {"name": "B"}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let scene = model.default_scene().unwrap();
    assert_eq!(scene.name.as_deref(), Some("B"));
}

#[test]
fn default_scene_fallback_to_first() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "scenes": [{"name": "First"}, {"name": "Second"}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let scene = model.default_scene().unwrap();
    assert_eq!(scene.name.as_deref(), Some("First"));
}

// ─── triangle_count / vertex_count ─────────────────────────────────────────

#[test]
fn triangle_count_single_mesh() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "accessors": [{"componentType": 5123, "count": 36, "type": "SCALAR"}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 0}]}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.triangle_count(), 12); // 36 indices / 3
}

#[test]
fn triangle_count_multiple_meshes() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "accessors": [
            {"componentType": 5123, "count": 6, "type": "SCALAR"},
            {"componentType": 5123, "count": 12, "type": "SCALAR"}
        ],
        "meshes": [
            {"primitives": [{"attributes": {"POSITION": 0}, "indices": 0}]},
            {"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}
        ]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.triangle_count(), 6); // (6+12)/3
}

#[test]
fn triangle_count_ignores_non_triangle_mode() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "accessors": [{"componentType": 5123, "count": 9, "type": "SCALAR"}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 0, "mode": 0}]}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.triangle_count(), 0); // mode=0 is Points
}

#[test]
fn vertex_count_from_position_accessor() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "accessors": [
            {"componentType": 5126, "count": 24, "type": "VEC3"},
            {"componentType": 5123, "count": 36, "type": "SCALAR"}
        ],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.vertex_count(), 24);
}

// ─── Node::local_transform ─────────────────────────────────────────────────

#[test]
fn node_transform_from_matrix() {
    let node = Node {
        name: None,
        children: vec![],
        mesh: None,
        skin: None,
        matrix: Some([
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            5.0, 6.0, 7.0, 1.0,
        ]),
        translation: None,
        rotation: None,
        scale: None,
        extensions: None,
    };
    let mat = node.local_transform();
    let translation = mat.w_axis;
    assert!((translation.x - 5.0).abs() < 1e-10);
    assert!((translation.y - 6.0).abs() < 1e-10);
    assert!((translation.z - 7.0).abs() < 1e-10);
}

#[test]
fn node_transform_from_trs() {
    let node = Node {
        name: None,
        children: vec![],
        mesh: None,
        skin: None,
        matrix: None,
        translation: Some([10.0, 20.0, 30.0]),
        rotation: Some([0.0, 0.0, 0.0, 1.0]), // identity
        scale: Some([2.0, 2.0, 2.0]),
        extensions: None,
    };
    let mat = node.local_transform();
    let translation = mat.w_axis;
    assert!((translation.x - 10.0).abs() < 1e-10);
    assert!((translation.y - 20.0).abs() < 1e-10);
    assert!((translation.z - 30.0).abs() < 1e-10);
    // Scale on diagonal
    assert!((mat.x_axis.x - 2.0).abs() < 1e-10);
    assert!((mat.y_axis.y - 2.0).abs() < 1e-10);
    assert!((mat.z_axis.z - 2.0).abs() < 1e-10);
}

#[test]
fn node_transform_identity_default() {
    let node = Node {
        name: None,
        children: vec![],
        mesh: None,
        skin: None,
        matrix: None,
        translation: None,
        rotation: None,
        scale: None,
        extensions: None,
    };
    let mat = node.local_transform();
    assert!((mat.x_axis.x - 1.0).abs() < 1e-10);
    assert!((mat.y_axis.y - 1.0).abs() < 1e-10);
    assert!((mat.z_axis.z - 1.0).abs() < 1e-10);
    assert!((mat.w_axis.w - 1.0).abs() < 1e-10);
}

// ─── Accessor component/element sizes ──────────────────────────────────────

#[test]
fn accessor_components_per_element() {
    let make = |t: AccessorType| Accessor {
        name: None,
        buffer_view: None,
        byte_offset: 0,
        component_type: ComponentType::F32,
        normalized: false,
        count: 1,
        accessor_type: t,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    assert_eq!(make(AccessorType::Scalar).components_per_element(), 1);
    assert_eq!(make(AccessorType::Vec2).components_per_element(), 2);
    assert_eq!(make(AccessorType::Vec3).components_per_element(), 3);
    assert_eq!(make(AccessorType::Vec4).components_per_element(), 4);
    assert_eq!(make(AccessorType::Mat2).components_per_element(), 4);
    assert_eq!(make(AccessorType::Mat3).components_per_element(), 9);
    assert_eq!(make(AccessorType::Mat4).components_per_element(), 16);
}

#[test]
fn accessor_component_byte_size() {
    let make = |ct: ComponentType| Accessor {
        name: None,
        buffer_view: None,
        byte_offset: 0,
        component_type: ct,
        normalized: false,
        count: 1,
        accessor_type: AccessorType::Scalar,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    assert_eq!(make(ComponentType::I8).component_byte_size(), 1);
    assert_eq!(make(ComponentType::U8).component_byte_size(), 1);
    assert_eq!(make(ComponentType::I16).component_byte_size(), 2);
    assert_eq!(make(ComponentType::U16).component_byte_size(), 2);
    assert_eq!(make(ComponentType::U32).component_byte_size(), 4);
    assert_eq!(make(ComponentType::F32).component_byte_size(), 4);
}

#[test]
fn accessor_element_byte_size() {
    let acc = Accessor {
        name: None,
        buffer_view: None,
        byte_offset: 0,
        component_type: ComponentType::F32,
        normalized: false,
        count: 10,
        accessor_type: AccessorType::Vec3,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    assert_eq!(acc.element_byte_size(), 12); // 3 components * 4 bytes
}

#[test]
fn accessor_is_sparse() {
    let mut acc = Accessor {
        name: None,
        buffer_view: None,
        byte_offset: 0,
        component_type: ComponentType::F32,
        normalized: false,
        count: 1,
        accessor_type: AccessorType::Scalar,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    assert!(!acc.is_sparse());
    acc.sparse = Some(cesium_gltf::AccessorSparse {
        count: 1,
        indices: cesium_gltf::AccessorSparseIndices {
            buffer_view: 0,
            byte_offset: 0,
            component_type: ComponentType::U16,
        },
        values: cesium_gltf::AccessorSparseValues {
            buffer_view: 1,
            byte_offset: 0,
        },
    });
    assert!(acc.is_sparse());
}

// ─── Accessor binary data reading ──────────────────────────────────────────

#[test]
fn read_f32_data_basic() {
    // 3 VEC3 vertices: (1,2,3), (4,5,6), (7,8,9)
    let mut buffer: Vec<u8> = Vec::new();
    for v in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0] {
        buffer.extend_from_slice(&v.to_le_bytes());
    }
    let buffers = vec![buffer];
    let buffer_views = vec![BufferView {
        name: None,
        buffer: 0,
        byte_offset: 0,
        byte_length: 36,
        byte_stride: None,
        target: None,
    }];
    let acc = Accessor {
        name: None,
        buffer_view: Some(0),
        byte_offset: 0,
        component_type: ComponentType::F32,
        normalized: false,
        count: 3,
        accessor_type: AccessorType::Vec3,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    let data = acc.read_f32_data(&buffers, &buffer_views);
    assert_eq!(data.len(), 9);
    assert!((data[0] - 1.0).abs() < 1e-6);
    assert!((data[4] - 5.0).abs() < 1e-6);
    assert!((data[8] - 9.0).abs() < 1e-6);
}

#[test]
fn read_f32_data_with_offset() {
    // Buffer with 4 bytes padding before data
    let mut buffer: Vec<u8> = vec![0, 0, 0, 0]; // padding
    for v in [10.0f32, 20.0] {
        buffer.extend_from_slice(&v.to_le_bytes());
    }
    let buffers = vec![buffer];
    let buffer_views = vec![BufferView {
        name: None,
        buffer: 0,
        byte_offset: 4, // skip padding
        byte_length: 8,
        byte_stride: None,
        target: None,
    }];
    let acc = Accessor {
        name: None,
        buffer_view: Some(0),
        byte_offset: 0,
        component_type: ComponentType::F32,
        normalized: false,
        count: 2,
        accessor_type: AccessorType::Scalar,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    let data = acc.read_f32_data(&buffers, &buffer_views);
    assert_eq!(data.len(), 2);
    assert!((data[0] - 10.0).abs() < 1e-6);
    assert!((data[1] - 20.0).abs() < 1e-6);
}

#[test]
fn read_u16_data_basic() {
    // 6 u16 indices: 0, 1, 2, 2, 1, 3
    let mut buffer: Vec<u8> = Vec::new();
    for v in [0u16, 1, 2, 2, 1, 3] {
        buffer.extend_from_slice(&v.to_le_bytes());
    }
    let buffers = vec![buffer];
    let buffer_views = vec![BufferView {
        name: None,
        buffer: 0,
        byte_offset: 0,
        byte_length: 12,
        byte_stride: None,
        target: None,
    }];
    let acc = Accessor {
        name: None,
        buffer_view: Some(0),
        byte_offset: 0,
        component_type: ComponentType::U16,
        normalized: false,
        count: 6,
        accessor_type: AccessorType::Scalar,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    let data = acc.read_u16_data(&buffers, &buffer_views);
    assert_eq!(data, vec![0, 1, 2, 2, 1, 3]);
}

#[test]
fn read_u32_data_basic() {
    let mut buffer: Vec<u8> = Vec::new();
    for v in [100u32, 200, 300] {
        buffer.extend_from_slice(&v.to_le_bytes());
    }
    let buffers = vec![buffer];
    let buffer_views = vec![BufferView {
        name: None,
        buffer: 0,
        byte_offset: 0,
        byte_length: 12,
        byte_stride: None,
        target: None,
    }];
    let acc = Accessor {
        name: None,
        buffer_view: Some(0),
        byte_offset: 0,
        component_type: ComponentType::U32,
        normalized: false,
        count: 3,
        accessor_type: AccessorType::Scalar,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    let data = acc.read_u32_data(&buffers, &buffer_views);
    assert_eq!(data, vec![100, 200, 300]);
}

#[test]
fn read_f32_no_buffer_view_returns_zeros() {
    let acc = Accessor {
        name: None,
        buffer_view: None,
        byte_offset: 0,
        component_type: ComponentType::F32,
        normalized: false,
        count: 3,
        accessor_type: AccessorType::Vec3,
        max: vec![],
        min: vec![],
        sparse: None,
    };
    let data = acc.read_f32_data(&[], &[]);
    assert_eq!(data.len(), 9);
    assert!(data.iter().all(|&v| v == 0.0));
}

// ─── PrimitiveMode ─────────────────────────────────────────────────────────

#[test]
fn primitive_mode_default_is_triangles() {
    assert_eq!(PrimitiveMode::default(), PrimitiveMode::Triangles);
}

#[test]
fn primitive_mode_deserialize() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "meshes": [{"primitives": [
            {"attributes": {"POSITION": 0}, "mode": 0},
            {"attributes": {"POSITION": 0}, "mode": 4},
            {"attributes": {"POSITION": 0}, "mode": 5}
        ]}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let prims = &model.meshes[0].primitives;
    assert_eq!(prims[0].mode, PrimitiveMode::Points);
    assert_eq!(prims[1].mode, PrimitiveMode::Triangles);
    assert_eq!(prims[2].mode, PrimitiveMode::TriangleStrip);
}
