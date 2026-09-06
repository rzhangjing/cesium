//! Scene Model batch 4 spec tests.
//!
//! Mirrors CesiumJS Jasmine specs for Scene-level types:
//! - ColorBlendMode
//! - ClassificationType
//! - SplitDirection
//! - VerticalOrigin
//! - AlphaMode
//! - MetadataEntity
//! - MetadataSemantic
//! - PropertyAttribute / PropertyAttributeMapping
//! - PropertyTexture
//! - PropertyTextureProperty
//! - BatchTexture
//! - parse_batch_table / partition_properties / transcode_component_type

use serde_json::{json, Value};
use std::collections::HashMap;

use cesium_scene::alpha_mode::AlphaMode;
use cesium_scene::batch_texture::BatchTexture;
use cesium_scene::classification_type::ClassificationType;
use cesium_scene::color_blend_mode::ColorBlendMode;
use cesium_scene::metadata_entity::MetadataEntity;
use cesium_scene::metadata_semantic::MetadataSemantic;
use cesium_scene::parse_batch_table::{
    parse_batch_table, partition_properties, transcode_component_type, ParseBatchTableOptions,
};
use cesium_scene::property_attribute::{PropertyAttribute, PropertyAttributeMapping};
use cesium_scene::property_texture::PropertyTexture;
use cesium_scene::property_texture_property::PropertyTextureProperty;
use cesium_scene::split_direction::SplitDirection;
use cesium_scene::vertical_origin::VerticalOrigin;

// ════════════════════════════════════════════════════════════════
// ColorBlendMode
// ════════════════════════════════════════════════════════════════

#[test]
fn color_blend_mode_from_i32_valid() {
    assert_eq!(ColorBlendMode::from_i32(0), Some(ColorBlendMode::Highlight));
    assert_eq!(ColorBlendMode::from_i32(1), Some(ColorBlendMode::Replace));
    assert_eq!(ColorBlendMode::from_i32(2), Some(ColorBlendMode::Mix));
}

#[test]
fn color_blend_mode_from_i32_invalid() {
    assert_eq!(ColorBlendMode::from_i32(99), None);
    assert_eq!(ColorBlendMode::from_i32(-1), None);
}

#[test]
fn color_blend_mode_as_i32() {
    assert_eq!(ColorBlendMode::Highlight.as_i32(), 0);
    assert_eq!(ColorBlendMode::Replace.as_i32(), 1);
    assert_eq!(ColorBlendMode::Mix.as_i32(), 2);
}

#[test]
fn color_blend_mode_blend_factor() {
    assert_eq!(ColorBlendMode::Highlight.blend_factor(0.5), 0.0);
    assert_eq!(ColorBlendMode::Replace.blend_factor(0.5), 1.0);
    let mix_factor = ColorBlendMode::Mix.blend_factor(0.7);
    assert!((mix_factor - 0.7).abs() < 1e-10);
    // Clamped to EPSILON minimum
    let clamped = ColorBlendMode::Mix.blend_factor(0.0);
    assert!(clamped >= 1e-14);
}

#[test]
fn color_blend_mode_default() {
    assert_eq!(ColorBlendMode::default(), ColorBlendMode::Highlight);
}

// ════════════════════════════════════════════════════════════════
// ClassificationType
// ════════════════════════════════════════════════════════════════

#[test]
fn classification_type_from_i32_valid() {
    assert_eq!(ClassificationType::from_i32(0), Some(ClassificationType::Cesium3DTiles));
    assert_eq!(ClassificationType::from_i32(1), Some(ClassificationType::Terrain));
    assert_eq!(ClassificationType::from_i32(2), Some(ClassificationType::Both));
}

#[test]
fn classification_type_from_i32_invalid() {
    assert_eq!(ClassificationType::from_i32(3), None);
    assert_eq!(ClassificationType::from_i32(-1), None);
}

#[test]
fn classification_type_as_i32_and_str() {
    assert_eq!(ClassificationType::Cesium3DTiles.as_i32(), 0);
    assert_eq!(ClassificationType::Cesium3DTiles.as_str(), "CESIUM_3D_TILE");
    assert_eq!(ClassificationType::Terrain.as_i32(), 1);
    assert_eq!(ClassificationType::Terrain.as_str(), "TERRAIN");
    assert_eq!(ClassificationType::Both.as_i32(), 2);
    assert_eq!(ClassificationType::Both.as_str(), "BOTH");
}

#[test]
fn classification_type_default() {
    assert_eq!(ClassificationType::default(), ClassificationType::Both);
}

#[test]
fn classification_type_count() {
    assert_eq!(ClassificationType::NUMBER_OF_CLASSIFICATION_TYPES, 3);
}

// ════════════════════════════════════════════════════════════════
// SplitDirection
// ════════════════════════════════════════════════════════════════

#[test]
fn split_direction_from_i32_valid() {
    assert_eq!(SplitDirection::from_i32(-1), Some(SplitDirection::Left));
    assert_eq!(SplitDirection::from_i32(0), Some(SplitDirection::Both));
    assert_eq!(SplitDirection::from_i32(1), Some(SplitDirection::Right));
}

#[test]
fn split_direction_from_i32_invalid() {
    assert_eq!(SplitDirection::from_i32(2), None);
    assert_eq!(SplitDirection::from_i32(-2), None);
}

#[test]
fn split_direction_as_i32_and_f64() {
    assert_eq!(SplitDirection::Left.as_i32(), -1);
    assert_eq!(SplitDirection::Left.as_f64(), -1.0);
    assert_eq!(SplitDirection::Both.as_i32(), 0);
    assert_eq!(SplitDirection::Both.as_f64(), 0.0);
    assert_eq!(SplitDirection::Right.as_i32(), 1);
    assert_eq!(SplitDirection::Right.as_f64(), 1.0);
}

#[test]
fn split_direction_default() {
    assert_eq!(SplitDirection::default(), SplitDirection::Both);
}

// ════════════════════════════════════════════════════════════════
// VerticalOrigin
// ════════════════════════════════════════════════════════════════

#[test]
fn vertical_origin_from_i32_valid() {
    assert_eq!(VerticalOrigin::from_i32(0), Some(VerticalOrigin::Center));
    assert_eq!(VerticalOrigin::from_i32(1), Some(VerticalOrigin::Bottom));
    assert_eq!(VerticalOrigin::from_i32(2), Some(VerticalOrigin::Baseline));
    assert_eq!(VerticalOrigin::from_i32(-1), Some(VerticalOrigin::Top));
}

#[test]
fn vertical_origin_from_i32_invalid() {
    assert_eq!(VerticalOrigin::from_i32(3), None);
    assert_eq!(VerticalOrigin::from_i32(-2), None);
}

#[test]
fn vertical_origin_as_i32() {
    assert_eq!(VerticalOrigin::Center.as_i32(), 0);
    assert_eq!(VerticalOrigin::Bottom.as_i32(), 1);
    assert_eq!(VerticalOrigin::Baseline.as_i32(), 2);
    assert_eq!(VerticalOrigin::Top.as_i32(), -1);
}

#[test]
fn vertical_origin_default() {
    assert_eq!(VerticalOrigin::default(), VerticalOrigin::Center);
}

// ════════════════════════════════════════════════════════════════
// AlphaMode
// ════════════════════════════════════════════════════════════════

#[test]
fn alpha_mode_from_str_valid() {
    assert_eq!(AlphaMode::from_str("OPAQUE"), Some(AlphaMode::Opaque));
    assert_eq!(AlphaMode::from_str("MASK"), Some(AlphaMode::Mask));
    assert_eq!(AlphaMode::from_str("BLEND"), Some(AlphaMode::Blend));
    // Case-insensitive
    assert_eq!(AlphaMode::from_str("opaque"), Some(AlphaMode::Opaque));
    assert_eq!(AlphaMode::from_str("Blend"), Some(AlphaMode::Blend));
}

#[test]
fn alpha_mode_from_str_invalid() {
    assert_eq!(AlphaMode::from_str("UNKNOWN"), None);
    assert_eq!(AlphaMode::from_str(""), None);
}

#[test]
fn alpha_mode_as_str() {
    assert_eq!(AlphaMode::Opaque.as_str(), "OPAQUE");
    assert_eq!(AlphaMode::Mask.as_str(), "MASK");
    assert_eq!(AlphaMode::Blend.as_str(), "BLEND");
}

#[test]
fn alpha_mode_from_i32() {
    assert_eq!(AlphaMode::from_i32(0), Some(AlphaMode::Opaque));
    assert_eq!(AlphaMode::from_i32(1), Some(AlphaMode::Mask));
    assert_eq!(AlphaMode::from_i32(2), Some(AlphaMode::Blend));
    assert_eq!(AlphaMode::from_i32(3), None);
}

#[test]
fn alpha_mode_needs_alpha_cutoff() {
    assert!(!AlphaMode::Opaque.needs_alpha_cutoff());
    assert!(AlphaMode::Mask.needs_alpha_cutoff());
    assert!(!AlphaMode::Blend.needs_alpha_cutoff());
}

#[test]
fn alpha_mode_is_transparent() {
    assert!(!AlphaMode::Opaque.is_transparent());
    assert!(!AlphaMode::Mask.is_transparent());
    assert!(AlphaMode::Blend.is_transparent());
}

#[test]
fn alpha_mode_default() {
    assert_eq!(AlphaMode::default(), AlphaMode::Opaque);
}

// ════════════════════════════════════════════════════════════════
// MetadataEntity
// ════════════════════════════════════════════════════════════════

#[test]
fn metadata_entity_new_is_empty() {
    let entity = MetadataEntity::new();
    assert!(entity.properties.is_empty());
    assert!(!entity.has_property("foo"));
    assert_eq!(entity.get_property("foo"), None);
}

#[test]
fn metadata_entity_from_properties_and_accessors() {
    let mut map = HashMap::new();
    map.insert("name".to_string(), json!("building_A"));
    map.insert("height".to_string(), json!(42.0));
    let entity = MetadataEntity::from_properties(map);

    assert!(entity.has_property("name"));
    assert!(entity.has_property("height"));
    assert!(!entity.has_property("missing"));
    assert_eq!(entity.get_property("name"), Some(&json!("building_A")));
    assert_eq!(entity.get_property("height"), Some(&json!(42.0)));
}

#[test]
fn metadata_entity_set_property_and_get_ids() {
    let mut entity = MetadataEntity::new();
    entity.set_property("a", json!(1));
    entity.set_property("b", json!(2));

    assert_eq!(entity.get_property_ids().len(), 2);
    let mut ids = entity.get_property_ids();
    ids.sort();
    assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn metadata_entity_semantic_lookup() {
    let mut map = HashMap::new();
    map.insert(
        "prop1".to_string(),
        json!({"semantic": "NAME", "value": "test"}),
    );
    map.insert("prop2".to_string(), json!(42));
    let entity = MetadataEntity::from_properties(map);

    assert!(entity.has_property_by_semantic("NAME"));
    assert!(!entity.has_property_by_semantic("ID"));
    let found = entity.get_property_by_semantic("NAME");
    assert!(found.is_some());
    assert_eq!(found.unwrap().get("value"), Some(&json!("test")));
}

#[test]
fn metadata_entity_apply_value_transform_no_data() {
    let val = json!(0);
    let no_data = json!(0);
    let default_val = json!(-1);
    let result = MetadataEntity::apply_value_transform(
        &val,
        Some(&no_data),
        Some(&default_val),
        false,
        None,
        None,
    );
    assert_eq!(result, json!(-1));
}

#[test]
fn metadata_entity_apply_value_transform_normalize_offset_scale() {
    let val = json!(128.0);
    let result = MetadataEntity::apply_value_transform(
        &val,
        None,
        None,
        true,
        Some(10.0),
        Some(2.0),
    );
    // normalize: 128/255 ≈ 0.50196, scale: *2 ≈ 1.00392, offset: +10 ≈ 11.00392
    if let Some(f) = result.as_f64() {
        assert!((f - 11.003921568627451).abs() < 0.01);
    } else {
        panic!("Expected numeric result");
    }
}

// ════════════════════════════════════════════════════════════════
// MetadataSemantic
// ════════════════════════════════════════════════════════════════

#[test]
fn metadata_semantic_constants() {
    assert_eq!(MetadataSemantic::ID, "ID");
    assert_eq!(MetadataSemantic::NAME, "NAME");
    assert_eq!(MetadataSemantic::TILE_BOUNDING_BOX, "TILE_BOUNDING_BOX");
    assert_eq!(MetadataSemantic::CONTENT_BOUNDING_SPHERE, "CONTENT_BOUNDING_SPHERE");
}

#[test]
fn metadata_semantic_is_known() {
    assert!(MetadataSemantic::is_known("ID"));
    assert!(MetadataSemantic::is_known("NAME"));
    assert!(MetadataSemantic::is_known("TILE_GEOMETRIC_ERROR"));
    assert!(MetadataSemantic::is_known("CONTENT_MAXIMUM_HEIGHT"));
    assert!(!MetadataSemantic::is_known("UNKNOWN_SEMANTIC"));
}

#[test]
fn metadata_semantic_tile_and_content_classification() {
    assert!(MetadataSemantic::is_tile_semantic("TILE_BOUNDING_BOX"));
    assert!(MetadataSemantic::is_tile_semantic("TILE_GEOMETRIC_ERROR"));
    assert!(!MetadataSemantic::is_tile_semantic("CONTENT_BOUNDING_BOX"));
    assert!(!MetadataSemantic::is_tile_semantic("ID"));

    assert!(MetadataSemantic::is_content_semantic("CONTENT_BOUNDING_BOX"));
    assert!(MetadataSemantic::is_content_semantic("CONTENT_MAXIMUM_HEIGHT"));
    assert!(!MetadataSemantic::is_content_semantic("TILE_BOUNDING_BOX"));
}

// ════════════════════════════════════════════════════════════════
// PropertyAttribute
// ════════════════════════════════════════════════════════════════

#[test]
fn property_attribute_new_is_empty() {
    let pa = PropertyAttribute::new();
    assert!(pa.name.is_none());
    assert!(pa.class_name.is_none());
    assert_eq!(pa.property_count(), 0);
    assert!(pa.property_ids().is_empty());
}

#[test]
fn property_attribute_add_mappings() {
    let mut pa = PropertyAttribute::new();
    pa.name = Some("test_attr".to_string());
    pa.class_name = Some("building".to_string());
    pa.properties.insert(
        "height".to_string(),
        PropertyAttributeMapping {
            attribute: "_FEATURE_ID_0".to_string(),
            property_type: Some("SCALAR".to_string()),
            component_type: Some("FLOAT32".to_string()),
            channels: None,
            offset: None,
            scale: None,
            max: None,
            min: None,
            no_data: None,
            default: None,
        },
    );

    assert_eq!(pa.property_count(), 1);
    let mapping = pa.get_property("height").unwrap();
    assert_eq!(mapping.attribute, "_FEATURE_ID_0");
    assert_eq!(mapping.property_type.as_deref(), Some("SCALAR"));
    assert_eq!(mapping.component_type.as_deref(), Some("FLOAT32"));
}

#[test]
fn property_attribute_property_ids() {
    let mut pa = PropertyAttribute::new();
    pa.properties.insert(
        "a".to_string(),
        PropertyAttributeMapping {
            attribute: "_A".to_string(),
            property_type: None,
            component_type: None,
            channels: None,
            offset: None,
            scale: None,
            max: None,
            min: None,
            no_data: None,
            default: None,
        },
    );
    pa.properties.insert(
        "b".to_string(),
        PropertyAttributeMapping {
            attribute: "_B".to_string(),
            property_type: None,
            component_type: None,
            channels: None,
            offset: None,
            scale: None,
            max: None,
            min: None,
            no_data: None,
            default: None,
        },
    );
    let mut ids = pa.property_ids();
    ids.sort();
    assert_eq!(ids, vec!["a", "b"]);
}

// ════════════════════════════════════════════════════════════════
// PropertyTexture
// ════════════════════════════════════════════════════════════════

#[test]
fn property_texture_new_is_empty() {
    let pt = PropertyTexture::new();
    assert!(pt.name.is_none());
    assert!(pt.class_name.is_none());
    assert_eq!(pt.property_count(), 0);
}

#[test]
fn property_texture_add_and_query() {
    let mut pt = PropertyTexture::new();
    pt.name = Some("facade_texture".to_string());
    pt.class_name = Some("facade".to_string());

    let prop = PropertyTextureProperty {
        texture_index: 0,
        channels: vec![0, 1],
        offset: None,
        scale: None,
        has_value_transform: false,
        min: None,
        max: None,
        no_data: None,
        default: None,
        extras: None,
        extensions: None,
    };
    pt.properties.insert("color".to_string(), prop);

    assert_eq!(pt.property_count(), 1);
    let found = pt.get_property("color").unwrap();
    assert_eq!(found.texture_index, 0);
    assert_eq!(found.channels, vec![0, 1]);
    assert!(pt.get_property("missing").is_none());
}

// ════════════════════════════════════════════════════════════════
// PropertyTextureProperty
// ════════════════════════════════════════════════════════════════

#[test]
fn property_texture_property_defaults() {
    let p = PropertyTextureProperty::new();
    assert_eq!(p.texture_index, 0);
    assert_eq!(p.channels, vec![0]);
    assert!(p.offset.is_none());
    assert!(p.scale.is_none());
    assert!(!p.has_value_transform);
}

#[test]
fn property_texture_property_reformat_channels() {
    let mut p = PropertyTextureProperty::new();
    p.channels = vec![0];
    assert_eq!(p.reformat_channels(), "r");

    p.channels = vec![0, 1];
    assert_eq!(p.reformat_channels(), "rg");

    p.channels = vec![0, 1, 2];
    assert_eq!(p.reformat_channels(), "rgb");

    p.channels = vec![0, 1, 2, 3];
    assert_eq!(p.reformat_channels(), "rgba");
}

#[test]
fn property_texture_property_with_transforms() {
    let p = PropertyTextureProperty {
        texture_index: 1,
        channels: vec![0, 1],
        offset: Some(0.5),
        scale: Some(2.0),
        has_value_transform: true,
        min: Some(json!(0.0)),
        max: Some(json!(1.0)),
        no_data: None,
        default: None,
        extras: None,
        extensions: None,
    };
    assert_eq!(p.texture_index, 1);
    assert!(p.has_value_transform);
    assert_eq!(p.offset, Some(0.5));
    assert_eq!(p.scale, Some(2.0));
}

// ════════════════════════════════════════════════════════════════
// BatchTexture
// ════════════════════════════════════════════════════════════════

#[test]
fn batch_texture_new() {
    let bt = BatchTexture::new(10);
    assert_eq!(bt.features_length, 10);
    assert!(!bt.batch_values_dirty);
    assert_eq!(bt.translucent_features_length, 0);
    // 10 features × 2 bytes (show+alpha) + 10 features × 4 bytes (RGBA) = 60
    assert_eq!(bt.byte_length(), 60);
}

#[test]
fn batch_texture_show_get_set() {
    let mut bt = BatchTexture::new(5);
    // Default: all visible
    assert!(bt.get_show(0));
    assert!(bt.get_show(4));

    bt.set_show(2, false);
    assert!(!bt.get_show(2));
    assert!(bt.get_show(0)); // unchanged
    assert!(bt.batch_values_dirty);
}

#[test]
fn batch_texture_set_all_show() {
    let mut bt = BatchTexture::new(3);
    bt.set_all_show(false);
    assert!(!bt.get_show(0));
    assert!(!bt.get_show(1));
    assert!(!bt.get_show(2));

    bt.set_all_show(true);
    assert!(bt.get_show(0));
    assert!(bt.get_show(1));
    assert!(bt.get_show(2));
}

#[test]
fn batch_texture_color_get_set() {
    let mut bt = BatchTexture::new(4);
    // Default: all white
    assert_eq!(bt.get_color(0), [0xFF, 0xFF, 0xFF, 0xFF]);

    bt.set_color(1, [0xFF, 0x00, 0x00, 0x80]);
    assert_eq!(bt.get_color(1), [0xFF, 0x00, 0x00, 0x80]);
    assert_eq!(bt.get_color(0), [0xFF, 0xFF, 0xFF, 0xFF]); // unchanged
    assert!(bt.batch_values_dirty);
}

#[test]
fn batch_texture_default_zero_features() {
    let bt = BatchTexture::default();
    assert_eq!(bt.features_length, 0);
    assert_eq!(bt.byte_length(), 0);
}

// ════════════════════════════════════════════════════════════════
// parse_batch_table
// ════════════════════════════════════════════════════════════════

#[test]
fn parse_batch_table_basic() {
    let batch_json = json!({
        "name": ["a", "b", "c"],
        "height": [1, 2, 3]
    });
    let options = ParseBatchTableOptions {
        count: 3,
        batch_table: batch_json,
        binary_body: None,
        parse_as_property_attributes: false,
    };
    let result = parse_batch_table(&options);
    assert_eq!(result.features_length, 3);
    assert_eq!(result.json_properties.len(), 2);
    assert!(result.json_properties.contains_key("name"));
    assert!(result.json_properties.contains_key("height"));
    assert!(result.hierarchy.is_none());
}

#[test]
fn parse_batch_table_with_hierarchy() {
    let batch_json = json!({
        "name": ["a"],
        "HIERARCHY": {"classes": []}
    });
    let options = ParseBatchTableOptions {
        count: 1,
        batch_table: batch_json,
        binary_body: None,
        parse_as_property_attributes: false,
    };
    let result = parse_batch_table(&options);
    assert_eq!(result.json_properties.len(), 1); // only "name"
    assert!(result.hierarchy.is_some());
}

#[test]
fn partition_properties_separates_binary_and_json() {
    let batch = json!({
        "name": ["a", "b"],
        "binary_prop": {"byteOffset": 0, "componentType": "FLOAT"},
        "HIERARCHY": {"classes": []},
        "extras": {"custom": true},
        "extensions": {"ext1": {}}
    });
    let result = partition_properties(&batch);
    assert_eq!(result.json.len(), 1); // "name"
    assert!(result.json.contains_key("name"));
    assert_eq!(result.binary.len(), 1); // "binary_prop"
    assert!(result.binary.contains_key("binary_prop"));
    assert!(result.hierarchy.is_some());
    assert!(result.extras.is_some());
    assert!(result.extensions.is_some());
}

#[test]
fn transcode_component_type_mappings() {
    assert_eq!(transcode_component_type("BYTE"), "INT8");
    assert_eq!(transcode_component_type("UNSIGNED_BYTE"), "UINT8");
    assert_eq!(transcode_component_type("SHORT"), "INT16");
    assert_eq!(transcode_component_type("UNSIGNED_SHORT"), "UINT16");
    assert_eq!(transcode_component_type("INT"), "INT32");
    assert_eq!(transcode_component_type("UNSIGNED_INT"), "UINT32");
    assert_eq!(transcode_component_type("FLOAT"), "FLOAT32");
    assert_eq!(transcode_component_type("DOUBLE"), "FLOAT64");
    // Case insensitive
    assert_eq!(transcode_component_type("float"), "FLOAT32");
    // Unknown fallback
    assert_eq!(transcode_component_type("UNKNOWN"), "UINT8");
}
