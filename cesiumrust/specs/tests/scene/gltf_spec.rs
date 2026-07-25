//! Scene/GltfLoaderSpec.js, ModelSpec.js → Rust integration tests

use cesium_gltf::{GltfModel, AccessorType, ComponentType, AlphaMode, PrimitiveMode};

// === AccessorType serde ===

#[test]
fn test_accessor_type_deserialize_scalar() {
    let t: AccessorType = serde_json::from_str("\"SCALAR\"").unwrap();
    assert_eq!(t, AccessorType::Scalar);
}

#[test]
fn test_accessor_type_deserialize_vec3() {
    let t: AccessorType = serde_json::from_str("\"VEC3\"").unwrap();
    assert_eq!(t, AccessorType::Vec3);
}

#[test]
fn test_accessor_type_deserialize_mat4() {
    let t: AccessorType = serde_json::from_str("\"MAT4\"").unwrap();
    assert_eq!(t, AccessorType::Mat4);
}

#[test]
fn test_accessor_type_serialize_roundtrip() {
    let t = AccessorType::Vec2;
    let json = serde_json::to_string(&t).unwrap();
    assert_eq!(json, "\"VEC2\"");
}

// === ComponentType serde ===

#[test]
fn test_component_type_deserialize() {
    let t: ComponentType = serde_json::from_str("5126").unwrap();
    assert_eq!(t, ComponentType::F32);
}

#[test]
fn test_component_type_u8() {
    let t: ComponentType = serde_json::from_str("5121").unwrap();
    assert_eq!(t, ComponentType::U8);
}

#[test]
fn test_component_type_serialize_roundtrip() {
    let t = ComponentType::U16;
    let json = serde_json::to_string(&t).unwrap();
    assert_eq!(json, "5123");
}

// === AlphaMode ===

#[test]
fn test_alpha_mode_default() {
    let mode = AlphaMode::default();
    assert_eq!(mode, AlphaMode::Opaque);
}

// === PrimitiveMode ===

#[test]
fn test_primitive_mode_default() {
    let mode = PrimitiveMode::default();
    assert_eq!(mode, PrimitiveMode::Triangles);
}

// === GltfModel parsing ===

#[test]
fn test_gltf_model_minimal() {
    let json = r#"{
        "asset": {"version": "2.0"},
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "TestNode"}]
    }"#;
    let model: GltfModel = serde_json::from_str(json).unwrap();
    assert_eq!(model.asset.version, "2.0");
    assert_eq!(model.nodes.len(), 1);
}
