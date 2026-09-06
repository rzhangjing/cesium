//! Scene fidelity specs for the third batch of substantiated Scene types:
//! - MetadataEnum
//! - UniformType / VaryingType
//! - CustomShaderMode / CustomShaderTranslucencyMode
//! - StyleCommandsNeeded
//! - BlendingState
//! - SupportedImageFormats
//! - ImageryFlags / ModelAlphaOptions / ModelLightingOptions / ImageryConfiguration
//! - get_metadata_class_property / get_metadata_property
//! - I3dmParser

use cesium_scene::blending_state::{BlendingState, BlendingStateConfig};
use cesium_scene::blend_equation::BlendEquation;
use cesium_scene::blend_function::BlendFunction;
use cesium_scene::get_metadata_class_property::get_metadata_class_property;
use cesium_scene::get_metadata_property::get_metadata_property;
use cesium_scene::i3dm_parser::I3dmParser;
use cesium_scene::metadata_component_type::MetadataComponentType;
use cesium_scene::metadata_enum::MetadataEnum;
use cesium_scene::metadata_enum_value::MetadataEnumValue;
use cesium_scene::model::custom_shader_mode::CustomShaderMode;
use cesium_scene::model::custom_shader_translucency_mode::CustomShaderTranslucencyMode;
use cesium_scene::model::imagery_configuration::ImageryConfiguration;
use cesium_scene::model::imagery_flags::ImageryFlags;
use cesium_scene::model::lighting_model::LightingModel;
use cesium_scene::model::model_alpha_options::ModelAlphaOptions;
use cesium_scene::model::model_lighting_options::ModelLightingOptions;
use cesium_scene::model::style_commands_needed::StyleCommandsNeeded;
use cesium_scene::model::uniform_type::UniformType;
use cesium_scene::model::varying_type::VaryingType;
use cesium_scene::supported_image_formats::SupportedImageFormats;

// ─── MetadataEnum ──────────────────────────────────────────────

#[test]
fn metadata_enum_new_builds_names_by_value_and_values_by_name() {
    let values = vec![
        MetadataEnumValue::new(0, "CONCRETE".to_string(), None, None, None),
        MetadataEnumValue::new(1, "WOOD".to_string(), None, None, None),
        MetadataEnumValue::new(2, "METAL".to_string(), None, None, None),
    ];
    let e = MetadataEnum::new("materialEnum".to_string(), values, None, None, None, None, None);

    assert_eq!(e.id(), "materialEnum");
    assert_eq!(e.values().len(), 3);
    assert_eq!(e.names_by_value().get(&0), Some(&"CONCRETE".to_string()));
    assert_eq!(e.names_by_value().get(&2), Some(&"METAL".to_string()));
    assert_eq!(e.values_by_name().get("WOOD"), Some(&1));
    assert_eq!(e.value_type(), MetadataComponentType::Uint16); // default
}

#[test]
fn metadata_enum_from_json_parses_values_and_value_type() {
    let json = serde_json::json!({
        "values": [
            {"value": 10, "name": "LOW"},
            {"value": 20, "name": "HIGH"}
        ],
        "valueType": "UINT32",
        "name": "QualityLevel",
        "description": "Quality levels"
    });

    let e = MetadataEnum::from_json("quality", &json).unwrap();
    assert_eq!(e.id(), "quality");
    assert_eq!(e.name(), Some("QualityLevel"));
    assert_eq!(e.description(), Some("Quality levels"));
    assert_eq!(e.value_type(), MetadataComponentType::Uint32);
    assert_eq!(e.values().len(), 2);
    assert_eq!(e.names_by_value().get(&10), Some(&"LOW".to_string()));
    assert_eq!(e.values_by_name().get("HIGH"), Some(&20));
}

#[test]
fn metadata_enum_from_json_returns_none_for_invalid_input() {
    assert!(MetadataEnum::from_json("x", &serde_json::json!("not an object")).is_none());
    assert!(MetadataEnum::from_json("x", &serde_json::json!({"values": "not array"})).is_none());
}

// ─── UniformType ───────────────────────────────────────────────

#[test]
fn uniform_type_as_str_round_trips() {
    assert_eq!(UniformType::Float.as_str(), "float");
    assert_eq!(UniformType::Vec3.as_str(), "vec3");
    assert_eq!(UniformType::Mat4.as_str(), "mat4");
    assert_eq!(UniformType::Sampler2D.as_str(), "sampler2D");
    assert_eq!(UniformType::SamplerCube.as_str(), "samplerCube");
    assert_eq!(UniformType::BoolVec4.as_str(), "bvec4");
}

#[test]
fn uniform_type_from_str_parses_all_variants() {
    assert_eq!(UniformType::from_str("float"), Some(UniformType::Float));
    assert_eq!(UniformType::from_str("ivec3"), Some(UniformType::IntVec3));
    assert_eq!(UniformType::from_str("mat2"), Some(UniformType::Mat2));
    assert_eq!(UniformType::from_str("sampler2D"), Some(UniformType::Sampler2D));
    assert_eq!(UniformType::from_str("unknown"), None);
}

#[test]
fn uniform_type_classification_methods() {
    assert!(UniformType::Mat2.is_matrix_type());
    assert!(UniformType::Mat3.is_matrix_type());
    assert!(UniformType::Mat4.is_matrix_type());
    assert!(!UniformType::Float.is_matrix_type());

    assert!(UniformType::Vec2.is_vector_type());
    assert!(UniformType::IntVec3.is_vector_type());
    assert!(UniformType::BoolVec4.is_vector_type());
    assert!(!UniformType::Float.is_vector_type());

    assert!(UniformType::Sampler2D.is_sampler_type());
    assert!(UniformType::SamplerCube.is_sampler_type());
    assert!(!UniformType::Float.is_sampler_type());
}

// ─── VaryingType ───────────────────────────────────────────────

#[test]
fn varying_type_as_str_and_from_str() {
    assert_eq!(VaryingType::Float.as_str(), "float");
    assert_eq!(VaryingType::Vec4.as_str(), "vec4");
    assert_eq!(VaryingType::Mat4.as_str(), "mat4");

    assert_eq!(VaryingType::from_str("vec2"), Some(VaryingType::Vec2));
    assert_eq!(VaryingType::from_str("mat3"), Some(VaryingType::Mat3));
    assert_eq!(VaryingType::from_str("sampler2D"), None);
}

#[test]
fn varying_type_classification() {
    assert!(VaryingType::Mat2.is_matrix_type());
    assert!(!VaryingType::Float.is_matrix_type());
    assert!(VaryingType::Vec3.is_vector_type());
    assert!(!VaryingType::Mat4.is_vector_type());
}

// ─── CustomShaderMode ──────────────────────────────────────────

#[test]
fn custom_shader_mode_as_str_and_from_str() {
    assert_eq!(CustomShaderMode::ModifyMaterial.as_str(), "MODIFY_MATERIAL");
    assert_eq!(CustomShaderMode::ReplaceMaterial.as_str(), "REPLACE_MATERIAL");

    assert_eq!(
        CustomShaderMode::from_str("MODIFY_MATERIAL"),
        Some(CustomShaderMode::ModifyMaterial)
    );
    assert_eq!(CustomShaderMode::from_str("INVALID"), None);
}

#[test]
fn custom_shader_mode_get_define_name() {
    assert_eq!(
        CustomShaderMode::ModifyMaterial.get_define_name(),
        "CUSTOM_SHADER_MODIFY_MATERIAL"
    );
    assert_eq!(
        CustomShaderMode::ReplaceMaterial.get_define_name(),
        "CUSTOM_SHADER_REPLACE_MATERIAL"
    );
}

// ─── CustomShaderTranslucencyMode ──────────────────────────────

#[test]
fn custom_shader_translucency_mode_values() {
    assert_eq!(CustomShaderTranslucencyMode::Inherit.as_i32(), 0);
    assert_eq!(CustomShaderTranslucencyMode::Opaque.as_i32(), 1);
    assert_eq!(CustomShaderTranslucencyMode::Translucent.as_i32(), 2);
}

#[test]
fn custom_shader_translucency_mode_round_trips() {
    assert_eq!(
        CustomShaderTranslucencyMode::from_i32(0),
        Some(CustomShaderTranslucencyMode::Inherit)
    );
    assert_eq!(
        CustomShaderTranslucencyMode::from_str("OPAQUE"),
        Some(CustomShaderTranslucencyMode::Opaque)
    );
    assert_eq!(CustomShaderTranslucencyMode::from_i32(99), None);
}

// ─── StyleCommandsNeeded ───────────────────────────────────────

#[test]
fn style_commands_needed_from_feature_counts() {
    assert_eq!(
        StyleCommandsNeeded::from_feature_counts(10, 0),
        StyleCommandsNeeded::AllOpaque
    );
    assert_eq!(
        StyleCommandsNeeded::from_feature_counts(10, 10),
        StyleCommandsNeeded::AllTranslucent
    );
    assert_eq!(
        StyleCommandsNeeded::from_feature_counts(10, 5),
        StyleCommandsNeeded::OpaqueAndTranslucent
    );
}

#[test]
fn style_commands_needed_integer_values() {
    assert_eq!(StyleCommandsNeeded::AllOpaque.as_i32(), 0);
    assert_eq!(StyleCommandsNeeded::AllTranslucent.as_i32(), 1);
    assert_eq!(StyleCommandsNeeded::OpaqueAndTranslucent.as_i32(), 2);
    assert_eq!(StyleCommandsNeeded::from_i32(2), Some(StyleCommandsNeeded::OpaqueAndTranslucent));
}

// ─── BlendingState ─────────────────────────────────────────────

#[test]
fn blending_state_disabled_is_not_enabled() {
    let config: BlendingStateConfig = BlendingState::DISABLED;
    assert!(!config.enabled);
}

#[test]
fn blending_state_alpha_blend_has_correct_functions() {
    let config = BlendingState::ALPHA_BLEND;
    assert!(config.enabled);
    assert_eq!(config.equation_rgb, BlendEquation::Add);
    assert_eq!(config.equation_alpha, BlendEquation::Add);
    assert_eq!(config.function_source_rgb, BlendFunction::SourceAlpha);
    assert_eq!(config.function_source_alpha, BlendFunction::One);
    assert_eq!(config.function_destination_rgb, BlendFunction::OneMinusSourceAlpha);
    assert_eq!(config.function_destination_alpha, BlendFunction::OneMinusSourceAlpha);
}

#[test]
fn blending_state_pre_multiplied_has_one_for_source() {
    let config = BlendingState::PRE_MULTIPLIED_ALPHA_BLEND;
    assert!(config.enabled);
    assert_eq!(config.function_source_rgb, BlendFunction::One);
    assert_eq!(config.function_source_alpha, BlendFunction::One);
    assert_eq!(config.function_destination_rgb, BlendFunction::OneMinusSourceAlpha);
}

#[test]
fn blending_state_additive_has_one_for_destination() {
    let config = BlendingState::ADDITIVE_BLEND;
    assert!(config.enabled);
    assert_eq!(config.function_source_rgb, BlendFunction::SourceAlpha);
    assert_eq!(config.function_destination_rgb, BlendFunction::One);
    assert_eq!(config.function_destination_alpha, BlendFunction::One);
}

// ─── SupportedImageFormats ─────────────────────────────────────

#[test]
fn supported_image_formats_default_is_all_false() {
    let f = SupportedImageFormats::default();
    assert!(!f.webp);
    assert!(!f.basis);
}

#[test]
fn supported_image_formats_new_sets_values() {
    let f = SupportedImageFormats::new(true, true);
    assert!(f.webp);
    assert!(f.basis);
}

// ─── ImageryFlags ──────────────────────────────────────────────

#[test]
fn imagery_flags_default_all_false() {
    let f = ImageryFlags::default();
    assert!(!f.alpha);
    assert!(!f.brightness);
    assert!(!f.contrast);
    assert!(!f.hue);
    assert!(!f.saturation);
    assert!(!f.gamma);
    assert!(!f.color_to_alpha);
}

// ─── ModelAlphaOptions ─────────────────────────────────────────

#[test]
fn model_alpha_options_default_is_none() {
    let o = ModelAlphaOptions::default();
    assert!(o.pass.is_none());
    assert!(o.alpha_cutoff.is_none());
}

// ─── ModelLightingOptions ──────────────────────────────────────

#[test]
fn model_lighting_options_default_is_unlit() {
    let o = ModelLightingOptions::default();
    assert_eq!(o.lighting_model, LightingModel::Unlit);
}

// ─── ImageryConfiguration ──────────────────────────────────────

#[test]
fn imagery_configuration_stores_values() {
    let c = ImageryConfiguration {
        show: true,
        alpha: 0.5,
        brightness: 1.1,
        contrast: 0.9,
        hue: 0.1,
        saturation: 1.2,
        gamma: 2.0,
        color_to_alpha: None,
    };
    assert!(c.show);
    assert!((c.alpha - 0.5).abs() < f64::EPSILON);
    assert!(c.color_to_alpha.is_none());
}

// ─── get_metadata_class_property ───────────────────────────────

#[test]
fn get_metadata_class_property_returns_property_when_found() {
    let schema = serde_json::json!({
        "id": "mySchema",
        "classes": {
            "Building": {
                "properties": {
                    "height": {"type": "SCALAR", "componentType": "FLOAT32"}
                }
            }
        }
    });

    let result = get_metadata_class_property(
        Some(&schema),
        Some("mySchema"),
        "Building",
        "height",
    );
    assert!(result.is_some());
    let prop = result.unwrap();
    assert_eq!(prop.get("type").unwrap().as_str(), Some("SCALAR"));
}

#[test]
fn get_metadata_class_property_returns_none_for_missing_schema() {
    assert!(get_metadata_class_property(None, Some("x"), "C", "P").is_none());
}

#[test]
fn get_metadata_class_property_returns_none_for_mismatched_schema_id() {
    let schema = serde_json::json!({"id": "A", "classes": {}});
    assert!(get_metadata_class_property(Some(&schema), Some("B"), "C", "P").is_none());
}

#[test]
fn get_metadata_class_property_returns_none_for_missing_class() {
    let schema = serde_json::json!({"id": "A", "classes": {}});
    assert!(get_metadata_class_property(Some(&schema), None, "Missing", "P").is_none());
}

#[test]
fn get_metadata_class_property_returns_none_for_missing_property() {
    let schema = serde_json::json!({"id": "A", "classes": {"C": {"properties": {}}}});
    assert!(get_metadata_class_property(Some(&schema), None, "C", "Missing").is_none());
}

// ─── get_metadata_property ─────────────────────────────────────

#[test]
fn get_metadata_property_finds_matching_property_texture() {
    let sm = serde_json::json!({
        "propertyTextures": [
            {
                "class": {"id": "Wall"},
                "properties": {"color": "texColor", "height": "texHeight"}
            },
            {
                "class": {"id": "Roof"},
                "properties": {"material": "texMaterial"}
            }
        ]
    });

    let result = get_metadata_property(Some(&sm), "Wall", "color");
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_str(), Some("texColor"));

    let result2 = get_metadata_property(Some(&sm), "Roof", "material");
    assert!(result2.is_some());
    assert_eq!(result2.unwrap().as_str(), Some("texMaterial"));
}

#[test]
fn get_metadata_property_returns_none_for_no_match() {
    let sm = serde_json::json!({
        "propertyTextures": [
            {"class": {"id": "Wall"}, "properties": {"color": "val"}}
        ]
    });
    assert!(get_metadata_property(Some(&sm), "Floor", "color").is_none());
    assert!(get_metadata_property(Some(&sm), "Wall", "missing").is_none());
}

#[test]
fn get_metadata_property_returns_none_for_none_input() {
    assert!(get_metadata_property(None, "C", "P").is_none());
}

// ─── I3dmParser ────────────────────────────────────────────────

fn build_i3dm_buffer(feature_table_json: &str, feature_table_binary: &[u8], batch_table_json: Option<&str>, batch_table_binary: Option<&[u8]>, gltf: &[u8]) -> Vec<u8> {
    let ft_json_padded = pad_to_4byte(feature_table_json.as_bytes());
    let ft_bin = feature_table_binary;
    let bt_json_padded = batch_table_json.map(|s| pad_to_4byte(s.as_bytes()));
    let bt_bin = batch_table_binary.unwrap_or(&[]);
    let bt_json_bytes = bt_json_padded.as_deref().unwrap_or(&[]);

    let header_size = 32; // 8 * uint32
    let total = header_size + ft_json_padded.len() + ft_bin.len() + bt_json_bytes.len() + bt_bin.len() + gltf.len();

    let mut buf = Vec::with_capacity(total);

    // magic "i3dm"
    buf.extend_from_slice(b"i3dm");
    // version
    buf.extend_from_slice(&1u32.to_le_bytes());
    // byteLength (placeholder, fill later)
    let byte_length_pos = buf.len();
    buf.extend_from_slice(&0u32.to_le_bytes());
    // featureTableJsonByteLength
    buf.extend_from_slice(&(ft_json_padded.len() as u32).to_le_bytes());
    // featureTableBinaryByteLength
    buf.extend_from_slice(&(ft_bin.len() as u32).to_le_bytes());
    // batchTableJsonByteLength
    buf.extend_from_slice(&(bt_json_bytes.len() as u32).to_le_bytes());
    // batchTableBinaryByteLength
    buf.extend_from_slice(&(bt_bin.len() as u32).to_le_bytes());
    // gltfFormat = 1 (embedded)
    buf.extend_from_slice(&1u32.to_le_bytes());

    buf.extend_from_slice(&ft_json_padded);
    buf.extend_from_slice(ft_bin);
    buf.extend_from_slice(bt_json_bytes);
    buf.extend_from_slice(bt_bin);
    buf.extend_from_slice(gltf);

    // Fix byteLength
    let total_len = buf.len() as u32;
    buf[byte_length_pos..byte_length_pos + 4].copy_from_slice(&total_len.to_le_bytes());

    buf
}

fn pad_to_4byte(data: &[u8]) -> Vec<u8> {
    let mut padded = data.to_vec();
    let remainder = padded.len() % 4;
    if remainder != 0 {
        padded.extend(std::iter::repeat(0x20u8).take(4 - remainder));
    }
    padded
}

#[test]
fn i3dm_parser_parse_minimal_valid_buffer() {
    let gltf = b"glTF"; // minimal glTF placeholder
    let buf = build_i3dm_buffer(
        r#"{"INSTANCES_LENGTH": 1}"#,
        &[0xAA, 0xBB],
        None,
        None,
        gltf,
    );

    let result = I3dmParser::parse(&buf, None).unwrap();
    assert_eq!(result.gltf_format, 1);
    assert_eq!(
        result.feature_table_json.get("INSTANCES_LENGTH").unwrap().as_i64(),
        Some(1)
    );
    assert_eq!(result.feature_table_binary, vec![0xAA, 0xBB]);
    assert!(result.batch_table_json.is_none());
    assert!(result.batch_table_binary.is_none());
    assert_eq!(result.gltf, gltf);
}

#[test]
fn i3dm_parser_parse_with_batch_table() {
    let gltf = b"glbData";
    let buf = build_i3dm_buffer(
        r#"{"INSTANCES_LENGTH": 2}"#,
        &[],
        Some(r#"{"id": [1, 2]}"#),
        Some(&[0xCC]),
        gltf,
    );

    let result = I3dmParser::parse(&buf, None).unwrap();
    assert!(result.batch_table_json.is_some());
    assert_eq!(result.batch_table_binary, Some(vec![0xCC]));
}

#[test]
fn i3dm_parser_rejects_wrong_version() {
    let mut buf = build_i3dm_buffer(r#"{"X":1}"#, &[], None, None, b"glb");
    // Overwrite version to 2
    buf[4..8].copy_from_slice(&2u32.to_le_bytes());
    let err = I3dmParser::parse(&buf, None).unwrap_err();
    assert!(err.message.contains("version 1"));
}

#[test]
fn i3dm_parser_rejects_zero_feature_table_json_length() {
    let mut buf = build_i3dm_buffer(r#"{"X":1}"#, &[], None, None, b"glb");
    // Overwrite featureTableJsonByteLength to 0 (at offset 12)
    buf[12..16].copy_from_slice(&0u32.to_le_bytes());
    let err = I3dmParser::parse(&buf, None).unwrap_err();
    assert!(err.message.contains("feature table must be defined"));
}
