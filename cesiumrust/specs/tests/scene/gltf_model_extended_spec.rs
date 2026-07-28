//! Scene/GltfLoaderSpec.js + ModelSpec.js → Rust integration tests (extended).
//!
//! Maps to CesiumJS:
//! - Scene/GltfLoader.js (JSON parsing, accessor data reading, sparse accessors)
//! - Scene/Model/ModelUtility.js (node transforms, triangle/vertex counts)
//!
//! A-class tests: Node.local_transform, Accessor read methods (f32/u16/u32/sparse/stride),
//! GltfModel.triangle_count/vertex_count, PrimitiveMode/ComponentType/Interpolation serde,
//! full model parsing (materials, animations, skins).
//! C-class omitted: WebGL resource creation, shader compilation, texture upload.

use cesium_gltf::{
    Accessor, AccessorSparse, AccessorSparseIndices, AccessorSparseValues,
    AccessorType, AlphaMode, AnimationPath, BufferView, ComponentType,
    GltfModel, Interpolation, Node, PrimitiveMode,
};

// === Helper: build a buffer with f32 values ===
fn f32_buffer(values: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * 4);
    for v in values {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

fn make_buffer_view(buffer: usize, byte_length: usize) -> BufferView {
    BufferView {
        name: None,
        buffer,
        byte_offset: 0,
        byte_length,
        byte_stride: None,
        target: None,
    }
}

fn make_accessor(
    buffer_view: Option<usize>,
    component_type: ComponentType,
    count: usize,
    accessor_type: AccessorType,
) -> Accessor {
    Accessor {
        name: None,
        buffer_view,
        byte_offset: 0,
        component_type,
        normalized: false,
        count,
        accessor_type,
        max: vec![],
        min: vec![],
        sparse: None,
    }
}

// === Node.local_transform ===

#[test]
fn node_local_transform_identity() {
    let node = Node::default();
    let m = node.local_transform();
    assert_eq!(m, glam::DMat4::IDENTITY);
}

#[test]
fn node_local_transform_from_matrix() {
    // When matrix is provided, it takes precedence over TRS
    let cols: [f64; 16] = [
        2.0, 0.0, 0.0, 0.0,
        0.0, 3.0, 0.0, 0.0,
        0.0, 0.0, 4.0, 0.0,
        10.0, 20.0, 30.0, 1.0,
    ];
    let node = Node {
        matrix: Some(cols),
        translation: Some([99.0, 99.0, 99.0]), // should be ignored
        ..Default::default()
    };
    let m = node.local_transform();
    let t = m.w_axis.truncate();
    assert!((t.x - 10.0).abs() < 1e-10);
    assert!((t.y - 20.0).abs() < 1e-10);
    assert!((t.z - 30.0).abs() < 1e-10);
    // Scale from matrix
    assert!((m.x_axis.x - 2.0).abs() < 1e-10);
    assert!((m.y_axis.y - 3.0).abs() < 1e-10);
    assert!((m.z_axis.z - 4.0).abs() < 1e-10);
}

#[test]
fn node_local_transform_translation_only() {
    let node = Node {
        translation: Some([5.0, -3.0, 7.0]),
        ..Default::default()
    };
    let m = node.local_transform();
    let t = m.w_axis.truncate();
    assert!((t.x - 5.0).abs() < 1e-10);
    assert!((t.y - (-3.0)).abs() < 1e-10);
    assert!((t.z - 7.0).abs() < 1e-10);
}

#[test]
fn node_local_transform_scale_only() {
    let node = Node {
        scale: Some([2.0, 3.0, 4.0]),
        ..Default::default()
    };
    let m = node.local_transform();
    assert!((m.x_axis.x - 2.0).abs() < 1e-10);
    assert!((m.y_axis.y - 3.0).abs() < 1e-10);
    assert!((m.z_axis.z - 4.0).abs() < 1e-10);
}

#[test]
fn node_local_transform_rotation_90_z() {
    // Quaternion for 90° around Z: [0, 0, sin(45°), cos(45°)]
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let node = Node {
        rotation: Some([0.0, 0.0, s, s]),
        ..Default::default()
    };
    let m = node.local_transform();
    // After 90° Z rotation: X axis → Y axis
    assert!(m.x_axis.x.abs() < 1e-10);
    assert!((m.x_axis.y - 1.0).abs() < 1e-10);
}

#[test]
fn node_local_transform_trs_combined() {
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let node = Node {
        translation: Some([1.0, 2.0, 3.0]),
        rotation: Some([0.0, 0.0, s, s]),
        scale: Some([2.0, 2.0, 2.0]),
        ..Default::default()
    };
    let m = node.local_transform();
    // Translation preserved
    let t = m.w_axis.truncate();
    assert!((t.x - 1.0).abs() < 1e-10);
    assert!((t.y - 2.0).abs() < 1e-10);
    assert!((t.z - 3.0).abs() < 1e-10);
    // Scale applied to axes
    let x_len = m.x_axis.truncate().length();
    assert!((x_len - 2.0).abs() < 1e-10);
}

// === Accessor methods ===

#[test]
fn accessor_components_per_element() {
    let types = [
        (AccessorType::Scalar, 1),
        (AccessorType::Vec2, 2),
        (AccessorType::Vec3, 3),
        (AccessorType::Vec4, 4),
        (AccessorType::Mat2, 4),
        (AccessorType::Mat3, 9),
        (AccessorType::Mat4, 16),
    ];
    for (at, expected) in types {
        let acc = make_accessor(None, ComponentType::F32, 1, at);
        assert_eq!(acc.components_per_element(), expected);
    }
}

#[test]
fn accessor_component_byte_size() {
    let types = [
        (ComponentType::I8, 1),
        (ComponentType::U8, 1),
        (ComponentType::I16, 2),
        (ComponentType::U16, 2),
        (ComponentType::U32, 4),
        (ComponentType::F32, 4),
    ];
    for (ct, expected) in types {
        let acc = make_accessor(None, ct, 1, AccessorType::Scalar);
        assert_eq!(acc.component_byte_size(), expected);
    }
}

#[test]
fn accessor_element_byte_size() {
    let acc = make_accessor(None, ComponentType::F32, 1, AccessorType::Vec3);
    assert_eq!(acc.element_byte_size(), 12); // 3 * 4

    let acc2 = make_accessor(None, ComponentType::U16, 1, AccessorType::Scalar);
    assert_eq!(acc2.element_byte_size(), 2); // 1 * 2
}

#[test]
fn accessor_read_f32_scalar() {
    let buffer = f32_buffer(&[1.0, 2.0, 3.0, 4.0]);
    let buffers = vec![buffer];
    let bvs = vec![make_buffer_view(0, 16)];
    let acc = make_accessor(Some(0), ComponentType::F32, 4, AccessorType::Scalar);

    let data = acc.read_f32_data(&buffers, &bvs);
    assert_eq!(data.len(), 4);
    assert!((data[0] - 1.0).abs() < 1e-6);
    assert!((data[3] - 4.0).abs() < 1e-6);
}

#[test]
fn accessor_read_f32_vec2() {
    let buffer = f32_buffer(&[1.0, 2.0, 3.0, 4.0]);
    let buffers = vec![buffer];
    let bvs = vec![make_buffer_view(0, 16)];
    let acc = make_accessor(Some(0), ComponentType::F32, 2, AccessorType::Vec2);

    let data = acc.read_f32_data(&buffers, &bvs);
    assert_eq!(data.len(), 4); // 2 elements * 2 components
    assert!((data[0] - 1.0).abs() < 1e-6);
    assert!((data[1] - 2.0).abs() < 1e-6);
    assert!((data[2] - 3.0).abs() < 1e-6);
    assert!((data[3] - 4.0).abs() < 1e-6);
}

#[test]
fn accessor_read_f32_with_byte_offset() {
    // Skip first 4 bytes (one f32)
    let buffer = f32_buffer(&[99.0, 10.0, 20.0]);
    let buffers = vec![buffer];
    let bvs = vec![make_buffer_view(0, 12)];
    let mut acc = make_accessor(Some(0), ComponentType::F32, 2, AccessorType::Scalar);
    acc.byte_offset = 4; // skip first float

    let data = acc.read_f32_data(&buffers, &bvs);
    assert_eq!(data.len(), 2);
    assert!((data[0] - 10.0).abs() < 1e-6);
    assert!((data[1] - 20.0).abs() < 1e-6);
}

#[test]
fn accessor_read_f32_with_stride() {
    // Interleaved: pos(2f) + color(2f) = 16 bytes stride
    let buffer = f32_buffer(&[
        1.0, 2.0, 0.5, 0.5,  // elem 0: pos=(1,2), color=(0.5,0.5)
        3.0, 4.0, 0.8, 0.8,  // elem 1: pos=(3,4), color=(0.8,0.8)
    ]);
    let buffers = vec![buffer];
    let mut bv = make_buffer_view(0, 32);
    bv.byte_stride = Some(16); // 4 floats * 4 bytes
    let bvs = vec![bv];
    let acc = make_accessor(Some(0), ComponentType::F32, 2, AccessorType::Vec2);

    let data = acc.read_f32_data(&buffers, &bvs);
    assert_eq!(data.len(), 4);
    assert!((data[0] - 1.0).abs() < 1e-6);
    assert!((data[1] - 2.0).abs() < 1e-6);
    assert!((data[2] - 3.0).abs() < 1e-6);
    assert!((data[3] - 4.0).abs() < 1e-6);
}

#[test]
fn accessor_read_u16_data() {
    let mut buffer = Vec::new();
    for v in [0u16, 1, 2, 100, 65535] {
        buffer.extend_from_slice(&v.to_le_bytes());
    }
    let buffers = vec![buffer];
    let bvs = vec![make_buffer_view(0, 10)];
    let acc = make_accessor(Some(0), ComponentType::U16, 5, AccessorType::Scalar);

    let data = acc.read_u16_data(&buffers, &bvs);
    assert_eq!(data, vec![0, 1, 2, 100, 65535]);
}

#[test]
fn accessor_read_u32_data() {
    let mut buffer = Vec::new();
    for v in [0u32, 1000, 70000, 4294967295] {
        buffer.extend_from_slice(&v.to_le_bytes());
    }
    let buffers = vec![buffer];
    let bvs = vec![make_buffer_view(0, 16)];
    let acc = make_accessor(Some(0), ComponentType::U32, 4, AccessorType::Scalar);

    let data = acc.read_u32_data(&buffers, &bvs);
    assert_eq!(data, vec![0, 1000, 70000, 4294967295]);
}

#[test]
fn accessor_sparse_override() {
    // Base: [0, 0, 0], sparse: index 1 → 9.0
    let base_buf = f32_buffer(&[0.0, 0.0, 0.0]);
    let mut idx_buf = Vec::new();
    idx_buf.extend_from_slice(&1u16.to_le_bytes());
    let val_buf = f32_buffer(&[9.0]);

    let buffers = vec![base_buf, idx_buf, val_buf];
    let bvs = vec![
        make_buffer_view(0, 12),
        make_buffer_view(1, 2),
        make_buffer_view(2, 4),
    ];

    let mut acc = make_accessor(Some(0), ComponentType::F32, 3, AccessorType::Scalar);
    acc.sparse = Some(AccessorSparse {
        count: 1,
        indices: AccessorSparseIndices {
            buffer_view: 1,
            byte_offset: 0,
            component_type: ComponentType::U16,
        },
        values: AccessorSparseValues {
            buffer_view: 2,
            byte_offset: 0,
        },
    });

    assert!(acc.is_sparse());
    let data = acc.read_f32_data(&buffers, &bvs);
    assert!((data[0] - 0.0).abs() < 1e-6);
    assert!((data[1] - 9.0).abs() < 1e-6);
    assert!((data[2] - 0.0).abs() < 1e-6);
}

#[test]
fn accessor_no_buffer_view_returns_zeros() {
    let acc = make_accessor(None, ComponentType::F32, 3, AccessorType::Scalar);
    let data = acc.read_f32_data(&[], &[]);
    assert_eq!(data, vec![0.0, 0.0, 0.0]);
}

// === GltfModel parsing ===

#[test]
fn gltf_model_triangle_count_multiple_meshes() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "meshes": [
            {"primitives": [{"attributes": {"POSITION": 0}, "indices": 1, "mode": 4}]},
            {"primitives": [{"attributes": {"POSITION": 2}, "indices": 3, "mode": 4}]}
        ],
        "accessors": [
            {"componentType": 5126, "count": 8, "type": "VEC3"},
            {"componentType": 5123, "count": 12, "type": "SCALAR"},
            {"componentType": 5126, "count": 4, "type": "VEC3"},
            {"componentType": 5123, "count": 6, "type": "SCALAR"}
        ]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    // mesh0: 12/3=4 tris, mesh1: 6/3=2 tris → total 6
    assert_eq!(model.triangle_count(), 6);
}

#[test]
fn gltf_model_vertex_count() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "meshes": [
            {"primitives": [{"attributes": {"POSITION": 0}, "mode": 4}]},
            {"primitives": [{"attributes": {"POSITION": 1}, "mode": 4}]}
        ],
        "accessors": [
            {"componentType": 5126, "count": 100, "type": "VEC3"},
            {"componentType": 5126, "count": 50, "type": "VEC3"}
        ]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    assert_eq!(model.vertex_count(), 150);
}

#[test]
fn gltf_model_default_scene_fallback() {
    // No "scene" field → defaults to index 0
    let json = r#"{
        "asset": {"version": "2.0"},
        "scenes": [{"nodes": [0, 1]}, {"nodes": [2]}],
        "nodes": [{}, {}, {}]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let scene = model.default_scene().unwrap();
    assert_eq!(scene.nodes, vec![0, 1]);
}

#[test]
fn gltf_model_parse_materials() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "materials": [{
            "name": "Red",
            "pbrMetallicRoughness": {
                "baseColorFactor": [1.0, 0.0, 0.0, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.9
            },
            "alphaMode": "BLEND",
            "doubleSided": true
        }]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let mat = &model.materials[0];
    assert_eq!(mat.name.as_deref(), Some("Red"));
    assert!(mat.double_sided);
    assert_eq!(mat.alpha_mode, Some(AlphaMode::Blend));
    let pbr = mat.pbr_metallic_roughness.as_ref().unwrap();
    assert_eq!(pbr.base_color_factor, Some([1.0, 0.0, 0.0, 1.0]));
    assert_eq!(pbr.metallic_factor, Some(0.0));
}

#[test]
fn gltf_model_parse_animation() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "animations": [{
            "name": "Walk",
            "channels": [{
                "sampler": 0,
                "target": {"node": 1, "path": "rotation"}
            }],
            "samplers": [{
                "input": 0,
                "output": 1,
                "interpolation": "CUBICSPLINE"
            }]
        }]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let anim = &model.animations[0];
    assert_eq!(anim.name.as_deref(), Some("Walk"));
    assert_eq!(anim.channels.len(), 1);
    assert_eq!(anim.channels[0].target.node, 1);
    assert_eq!(anim.channels[0].target.path, AnimationPath::Rotation);
    assert_eq!(anim.samplers[0].interpolation, Interpolation::CubicSpline);
}

#[test]
fn gltf_model_parse_skin() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "skins": [{
            "name": "Armature",
            "inverseBindMatrices": 5,
            "skeleton": 0,
            "joints": [0, 1, 2, 3]
        }]
    }"#;
    let model = GltfModel::from_json(json).unwrap();
    let skin = &model.skins[0];
    assert_eq!(skin.name.as_deref(), Some("Armature"));
    assert_eq!(skin.inverse_bind_matrices, Some(5));
    assert_eq!(skin.skeleton, Some(0));
    assert_eq!(skin.joints, vec![0, 1, 2, 3]);
}

// === PrimitiveMode / Interpolation serde ===

#[test]
fn primitive_mode_serde_roundtrip() {
    let modes = [
        (PrimitiveMode::Points, "0"),
        (PrimitiveMode::Lines, "1"),
        (PrimitiveMode::LineLoop, "2"),
        (PrimitiveMode::LineStrip, "3"),
        (PrimitiveMode::Triangles, "4"),
        (PrimitiveMode::TriangleStrip, "5"),
        (PrimitiveMode::TriangleFan, "6"),
    ];
    for (mode, expected_json) in modes {
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, expected_json);
        let parsed: PrimitiveMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mode);
    }
}

#[test]
fn interpolation_serde() {
    let json = serde_json::to_string(&Interpolation::Step).unwrap();
    assert_eq!(json, "\"STEP\"");
    let parsed: Interpolation = serde_json::from_str("\"LINEAR\"").unwrap();
    assert_eq!(parsed, Interpolation::Linear);
}

#[test]
fn component_type_all_values() {
    let pairs = [
        (5120u32, ComponentType::I8),
        (5121, ComponentType::U8),
        (5122, ComponentType::I16),
        (5123, ComponentType::U16),
        (5125, ComponentType::U32),
        (5126, ComponentType::F32),
    ];
    for (num, ct) in pairs {
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, num.to_string());
        let parsed: ComponentType = serde_json::from_str(&num.to_string()).unwrap();
        assert_eq!(parsed, ct);
    }
}
