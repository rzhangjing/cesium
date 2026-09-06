//! Batch 6 spec tests for substantiated Scene stubs:
//! - VertexAttributeSemantic (enum + methods)
//! - ModelUtility (utility functions)
//! - Cesium3DTilesetTraversal (traversal helpers)
//! - GltfLoaderUtil (get_image_id_from_texture)
//! - ImageryLayerFeatureInfo (data + configure methods)
//! - GetFeatureInfoFormat (constructor mapping + parsers)

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix4::Matrix4;
use cesium_core::primitive_type::PrimitiveType;
use cesium_core::quaternion::Quaternion;
use serde_json::json;

use cesium_scene::attribute_type::AttributeType;
use cesium_scene::axis::Axis;
use cesium_scene::cesium3_d_tile::Cesium3DTile;
use cesium_scene::cesium3_d_tile_optimization_hint::Cesium3DTileOptimizationHint;
use cesium_scene::cesium3_d_tile_refine::Cesium3DTileRefine;
use cesium_scene::cesium3_d_tileset_traversal::{
    Cesium3DTilesetTraversal, TilePriorityRange,
};
use cesium_scene::cull_face::CullFace;
use cesium_scene::get_feature_info_format::{FeatureInfoType, GetFeatureInfoFormat};
use cesium_scene::gltf_loader_util::GltfLoaderUtil;
use cesium_scene::imagery_layer_feature_info::ImageryLayerFeatureInfo;
use cesium_scene::model::model_utility::{ModelAttribute, ModelNode, ModelUtility};
use cesium_scene::supported_image_formats::SupportedImageFormats;
use cesium_scene::vertex_attribute_semantic::VertexAttributeSemantic;

// ── VertexAttributeSemantic ──────────────────────────────────────

#[test]
fn vas_has_set_index_correct_for_all_semantics() {
    assert!(!VertexAttributeSemantic::Position.has_set_index());
    assert!(!VertexAttributeSemantic::Normal.has_set_index());
    assert!(!VertexAttributeSemantic::Tangent.has_set_index());
    assert!(VertexAttributeSemantic::TexCoord.has_set_index());
    assert!(VertexAttributeSemantic::Color.has_set_index());
    assert!(VertexAttributeSemantic::Joints.has_set_index());
    assert!(VertexAttributeSemantic::Weights.has_set_index());
    assert!(VertexAttributeSemantic::FeatureId.has_set_index());
    assert!(VertexAttributeSemantic::Scale.has_set_index());
    assert!(VertexAttributeSemantic::Rotation.has_set_index());
    assert!(!VertexAttributeSemantic::CumulativeDistance.has_set_index());
}

#[test]
fn vas_from_gltf_semantic_maps_standard_semantics() {
    assert_eq!(
        VertexAttributeSemantic::from_gltf_semantic("POSITION"),
        Some(VertexAttributeSemantic::Position)
    );
    assert_eq!(
        VertexAttributeSemantic::from_gltf_semantic("TEXCOORD_0"),
        Some(VertexAttributeSemantic::TexCoord)
    );
    assert_eq!(
        VertexAttributeSemantic::from_gltf_semantic("COLOR_1"),
        Some(VertexAttributeSemantic::Color)
    );
    assert_eq!(
        VertexAttributeSemantic::from_gltf_semantic("_FEATURE_ID_0"),
        Some(VertexAttributeSemantic::FeatureId)
    );
    assert_eq!(
        VertexAttributeSemantic::from_gltf_semantic("NORMAL"),
        Some(VertexAttributeSemantic::Normal)
    );
}

#[test]
fn vas_from_gltf_semantic_returns_none_for_unknown() {
    assert_eq!(VertexAttributeSemantic::from_gltf_semantic("BOGUS"), None);
}

#[test]
fn vas_get_glsl_type_returns_correct_types() {
    assert_eq!(VertexAttributeSemantic::Position.get_glsl_type(), "vec3");
    assert_eq!(VertexAttributeSemantic::TexCoord.get_glsl_type(), "vec2");
    assert_eq!(VertexAttributeSemantic::Color.get_glsl_type(), "vec4");
    assert_eq!(VertexAttributeSemantic::Joints.get_glsl_type(), "ivec4");
    assert_eq!(VertexAttributeSemantic::FeatureId.get_glsl_type(), "int");
    assert_eq!(
        VertexAttributeSemantic::CumulativeDistance.get_glsl_type(),
        "float"
    );
}

#[test]
fn vas_get_variable_name_appends_set_index() {
    assert_eq!(
        VertexAttributeSemantic::Position.get_variable_name(None),
        "positionMC"
    );
    assert_eq!(
        VertexAttributeSemantic::TexCoord.get_variable_name(Some(0)),
        "texCoord_0"
    );
    assert_eq!(
        VertexAttributeSemantic::Color.get_variable_name(Some(2)),
        "color_2"
    );
}

// ── ModelUtility ─────────────────────────────────────────────────

#[test]
fn model_utility_get_error_formats_message() {
    let err = ModelUtility::get_error("model", "/path/to.gltf", Some("timeout"));
    let msg = format!("{err}");
    assert!(msg.contains("Failed to load model: /path/to.gltf"));
    assert!(msg.contains("timeout"));
}

#[test]
fn model_utility_get_error_without_error_detail() {
    let err = ModelUtility::get_error("model", "/path.gltf", None);
    let msg = format!("{err}");
    assert!(msg.contains("Failed to load model: /path.gltf"));
    assert!(!msg.contains("timeout"));
}

#[test]
fn model_utility_get_node_transform_uses_matrix_when_present() {
    let matrix = Matrix4::from_translation_quaternion_rotation_scale_new(
        &Cartesian3::from_elements_new(1.0, 2.0, 3.0),
        &Quaternion::IDENTITY,
        &Cartesian3::ONE,
    );
    let node = ModelNode {
        matrix: Some(matrix),
        ..Default::default()
    };
    let result = ModelUtility::get_node_transform(&node);
    assert_eq!(result, matrix);
}

#[test]
fn model_utility_get_node_transform_composes_trs() {
    let node = ModelNode {
        translation: Some(Cartesian3::from_elements_new(10.0, 0.0, 0.0)),
        rotation: None,
        scale: Some(Cartesian3::from_elements_new(2.0, 2.0, 2.0)),
        ..Default::default()
    };
    let result = ModelUtility::get_node_transform(&node);
    // Translation column (elements 12,13,14) should reflect the translation.
    assert!((result.elements[12] - 10.0).abs() < 1e-10);
}

#[test]
fn model_utility_get_attribute_by_semantic_finds_position() {
    let attrs = vec![
        ModelAttribute {
            semantic: Some(VertexAttributeSemantic::Position),
            name: "POSITION".to_string(),
            ..Default::default()
        },
        ModelAttribute {
            semantic: Some(VertexAttributeSemantic::Normal),
            name: "NORMAL".to_string(),
            ..Default::default()
        },
    ];
    let result = ModelUtility::get_attribute_by_semantic(
        &attrs,
        VertexAttributeSemantic::Position,
        None,
    );
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "POSITION");
}

#[test]
fn model_utility_get_attribute_by_semantic_returns_none_for_missing() {
    let attrs = vec![ModelAttribute {
        semantic: Some(VertexAttributeSemantic::Position),
        ..Default::default()
    }];
    assert!(ModelUtility::get_attribute_by_semantic(
        &attrs,
        VertexAttributeSemantic::TexCoord,
        None,
    )
    .is_none());
}

#[test]
fn model_utility_get_attribute_by_name_finds_custom() {
    let attrs = vec![
        ModelAttribute {
            name: "POSITION".to_string(),
            ..Default::default()
        },
        ModelAttribute {
            name: "_custom".to_string(),
            ..Default::default()
        },
    ];
    let result = ModelUtility::get_attribute_by_name(&attrs, "_custom");
    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "_custom");
}

#[test]
fn model_utility_get_feature_ids_by_label_finds_match() {
    let ids = vec![
        json!({"label": "featureId_0", "positionalLabel": "_FEATURE_ID_0"}),
        json!({"label": "featureId_1", "positionalLabel": "_FEATURE_ID_1"}),
    ];
    let result = ModelUtility::get_feature_ids_by_label(&ids, "featureId_1");
    assert!(result.is_some());
    assert_eq!(result.unwrap()["label"], "featureId_1");
}

#[test]
fn model_utility_get_feature_ids_by_label_returns_none_for_missing() {
    let ids = vec![json!({"label": "featureId_0"})];
    assert!(ModelUtility::get_feature_ids_by_label(&ids, "nonexistent").is_none());
}

#[test]
fn model_utility_has_quantized_attributes_detects_quantization() {
    let attrs_no_quant = vec![ModelAttribute::default()];
    assert!(!ModelUtility::has_quantized_attributes(&attrs_no_quant));

    let attrs_quant = vec![ModelAttribute {
        quantization: Some(cesium_scene::model::model_utility::ModelAttributeQuantization {
            gl_type: "VEC3".to_string(),
        }),
        ..Default::default()
    }];
    assert!(ModelUtility::has_quantized_attributes(&attrs_quant));
}

#[test]
fn model_utility_get_attribute_info_strips_underscore_and_lowercases() {
    let attr = ModelAttribute {
        name: "_CustomAttr".to_string(),
        gl_type: "VEC3".to_string(),
        ..Default::default()
    };
    let info = ModelUtility::get_attribute_info(&attr);
    assert_eq!(info.variable_name, "customattr");
    assert!(!info.has_semantic);
    assert!(!info.is_quantized);
}

#[test]
fn model_utility_get_attribute_info_uses_semantic_variable_name() {
    let attr = ModelAttribute {
        semantic: Some(VertexAttributeSemantic::TexCoord),
        set_index: Some(1),
        gl_type: "VEC2".to_string(),
        ..Default::default()
    };
    let info = ModelUtility::get_attribute_info(&attr);
    assert_eq!(info.variable_name, "texCoord_1");
    assert!(info.has_semantic);
}

#[test]
fn model_utility_get_axis_correction_matrix_y_up() {
    let m = ModelUtility::get_axis_correction_matrix(Axis::Y, Axis::X);
    // Should not be identity (Y-up to Z-up is a rotation).
    assert_ne!(m, Matrix4::IDENTITY);
}

#[test]
fn model_utility_get_axis_correction_matrix_z_up_with_z_forward() {
    let m = ModelUtility::get_axis_correction_matrix(Axis::Z, Axis::Z);
    // Z-up + Z-forward → should apply Z_UP_TO_X_UP.
    assert_ne!(m, Matrix4::IDENTITY);
}

#[test]
fn model_utility_get_cull_face_back_for_triangles_positive_det() {
    let identity = Matrix4::IDENTITY;
    assert_eq!(
        ModelUtility::get_cull_face(&identity, PrimitiveType::Triangles),
        CullFace::Back
    );
}

#[test]
fn model_utility_get_cull_face_front_for_negative_determinant() {
    // A matrix with negative determinant (reflection).
    let mut m = Matrix4::IDENTITY;
    m.elements[0] = -1.0; // Flip x-axis → det < 0.
    assert_eq!(
        ModelUtility::get_cull_face(&m, PrimitiveType::Triangles),
        CullFace::Front
    );
}

#[test]
fn model_utility_get_cull_face_back_for_non_triangle() {
    let m = Matrix4::IDENTITY;
    assert_eq!(
        ModelUtility::get_cull_face(&m, PrimitiveType::Lines),
        CullFace::Back
    );
}

#[test]
fn model_utility_sanitize_glsl_identifier_removes_gl_prefix() {
    assert_eq!(
        ModelUtility::sanitize_glsl_identifier("gl_customProperty"),
        "customProperty"
    );
}

#[test]
fn model_utility_sanitize_glsl_identifier_prefixes_digit() {
    assert_eq!(
        ModelUtility::sanitize_glsl_identifier("1234"),
        "_1234"
    );
}

#[test]
fn model_utility_sanitize_glsl_identifier_replaces_non_alphanumeric() {
    assert_eq!(
        ModelUtility::sanitize_glsl_identifier("foo--bar..baz"),
        "foo_bar_baz"
    );
}

// ── Cesium3DTilesetTraversal ─────────────────────────────────────

#[test]
fn traversal_select_tile_marks_frame_and_returns_newly_selected() {
    let mut tile = Cesium3DTile::new();
    tile.selected_frame = 0;
    assert!(Cesium3DTilesetTraversal::select_tile(&mut tile, 5));
    assert_eq!(tile.selected_frame, 5);
    assert!(tile.was_selected_last_frame);

    // Same frame → not newly selected.
    assert!(!Cesium3DTilesetTraversal::select_tile(&mut tile, 5));
}

#[test]
fn traversal_load_tile_rejects_already_requested() {
    let mut tile = Cesium3DTile::new();
    tile.requested_frame = 10;
    assert!(!Cesium3DTilesetTraversal::load_tile(&tile, 10, true, 1.0, 0.0));
}

#[test]
fn traversal_load_tile_rejects_loaded_content() {
    let mut tile = Cesium3DTile::new();
    tile.content_state = cesium_scene::cesium3_d_tile_content_state::Cesium3DTileContentState::Ready;
    assert!(!Cesium3DTilesetTraversal::load_tile(&tile, 1, true, 1.0, 0.0));
}

#[test]
fn traversal_is_on_screen_long_enough_returns_true_when_culling_disabled() {
    let tile = Cesium3DTile::new();
    assert!(Cesium3DTilesetTraversal::is_on_screen_long_enough(
        &tile, false, 1.0, 1000.0
    ));
}

#[test]
fn traversal_update_tile_flags_returns_correct_defaults() {
    let flags = Cesium3DTilesetTraversal::update_tile_flags();
    assert!(!flags.was_min_priority_child);
    assert!(!flags.should_select);
    assert!(flags.final_resolution);
}

#[test]
fn traversal_update_minimum_maximum_priority_tracks_range() {
    let mut min_p = TilePriorityRange {
        distance: f64::MAX,
        depth: i32::MAX,
        foveated_factor: f64::MAX,
        reverse_screen_space_error: f64::MAX,
    };
    let mut max_p = TilePriorityRange::default();

    Cesium3DTilesetTraversal::update_minimum_maximum_priority(
        &mut min_p, &mut max_p, 100.0, 3, 0.5, 50.0,
    );
    Cesium3DTilesetTraversal::update_minimum_maximum_priority(
        &mut min_p, &mut max_p, 50.0, 5, 0.8, 30.0,
    );

    assert_eq!(max_p.distance, 100.0);
    assert_eq!(min_p.distance, 50.0);
    assert_eq!(max_p.depth, 5);
    assert_eq!(min_p.depth, 3);
}

#[test]
fn traversal_meets_screen_space_error_early_false_without_parent() {
    assert!(!Cesium3DTilesetTraversal::meets_screen_space_error_early(
        false, false, false, Cesium3DTileRefine::Add, 1.0, 10.0
    ));
}

#[test]
fn traversal_meets_screen_space_error_early_true_for_add_parent() {
    assert!(Cesium3DTilesetTraversal::meets_screen_space_error_early(
        true, false, false, Cesium3DTileRefine::Add, 5.0, 10.0
    ));
}

#[test]
fn traversal_should_cull_by_children_union_requires_replace_and_optimization() {
    assert!(Cesium3DTilesetTraversal::should_cull_by_children_union(
        Cesium3DTileRefine::Replace,
        Cesium3DTileOptimizationHint::UseOptimization,
        true,
    ));
    assert!(!Cesium3DTilesetTraversal::should_cull_by_children_union(
        Cesium3DTileRefine::Add,
        Cesium3DTileOptimizationHint::UseOptimization,
        true,
    ));
    assert!(!Cesium3DTilesetTraversal::should_cull_by_children_union(
        Cesium3DTileRefine::Replace,
        Cesium3DTileOptimizationHint::UseOptimization,
        false,
    ));
}

// ── GltfLoaderUtil.get_image_id_from_texture ─────────────────────

#[test]
fn gltf_loader_util_get_image_id_returns_source() {
    let gltf = json!({
        "textures": [{"source": 3}],
    });
    let formats = SupportedImageFormats::new(false, false);
    assert_eq!(GltfLoaderUtil::get_image_id_from_texture(&gltf, 0, &formats), Some(3));
}

#[test]
fn gltf_loader_util_get_image_id_prefers_webp_when_supported() {
    let gltf = json!({
        "textures": [{
            "source": 1,
            "extensions": {
                "EXT_texture_webp": {"source": 5}
            }
        }],
    });
    let formats = SupportedImageFormats::new(true, false);
    assert_eq!(GltfLoaderUtil::get_image_id_from_texture(&gltf, 0, &formats), Some(5));
}

#[test]
fn gltf_loader_util_get_image_id_falls_back_when_webp_not_supported() {
    let gltf = json!({
        "textures": [{
            "source": 1,
            "extensions": {
                "EXT_texture_webp": {"source": 5}
            }
        }],
    });
    let formats = SupportedImageFormats::new(false, false);
    assert_eq!(GltfLoaderUtil::get_image_id_from_texture(&gltf, 0, &formats), Some(1));
}

#[test]
fn gltf_loader_util_get_image_id_returns_none_for_invalid_texture() {
    let gltf = json!({"textures": []});
    let formats = SupportedImageFormats::default();
    assert_eq!(GltfLoaderUtil::get_image_id_from_texture(&gltf, 0, &formats), None);
}

// ── ImageryLayerFeatureInfo ──────────────────────────────────────

#[test]
fn imagery_layer_feature_info_configure_name_finds_name_field() {
    let mut info = ImageryLayerFeatureInfo::new();
    let props = json!({"name": "Test Feature", "other": "value"});
    info.configure_name_from_properties(&props);
    assert_eq!(info.name, Some("Test Feature".to_string()));
}

#[test]
fn imagery_layer_feature_info_configure_name_finds_title_field() {
    let mut info = ImageryLayerFeatureInfo::new();
    let props = json!({"title": "My Title"});
    info.configure_name_from_properties(&props);
    assert_eq!(info.name, Some("My Title".to_string()));
}

#[test]
fn imagery_layer_feature_info_configure_description_builds_table() {
    let mut info = ImageryLayerFeatureInfo::new();
    let props = json!({"key1": "val1"});
    info.configure_description_from_properties(&props);
    assert!(info.description.is_some());
    let desc = info.description.unwrap();
    assert!(desc.contains("<table"));
    assert!(desc.contains("key1"));
    assert!(desc.contains("val1"));
}

// ── GetFeatureInfoFormat ─────────────────────────────────────────

#[test]
fn get_feature_info_format_default_json_format() {
    let fmt = GetFeatureInfoFormat::new(FeatureInfoType::Json, None);
    assert_eq!(fmt.format, "application/json");
}

#[test]
fn get_feature_info_format_xml_default() {
    let fmt = GetFeatureInfoFormat::new(FeatureInfoType::Xml, None);
    assert_eq!(fmt.format, "text/xml");
}

#[test]
fn get_feature_info_format_custom_format() {
    let fmt = GetFeatureInfoFormat::new(FeatureInfoType::Json, Some("application/geo+json"));
    assert_eq!(fmt.format, "application/geo+json");
}

#[test]
fn get_feature_info_format_geo_json_to_feature_info_parses_features() {
    let json = json!({
        "features": [
            {
                "type": "Feature",
                "properties": {"name": "A", "value": 42},
                "geometry": {
                    "type": "Point",
                    "coordinates": [10.0, 20.0]
                }
            }
        ]
    });
    let features = GetFeatureInfoFormat::geo_json_to_feature_info(&json);
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].name, Some("A".to_string()));
    assert!(features[0].position.is_some());
}

#[test]
fn get_feature_info_format_geo_json_handles_no_features() {
    let json = json!({"type": "FeatureCollection"});
    let features = GetFeatureInfoFormat::geo_json_to_feature_info(&json);
    assert!(features.is_empty());
}

#[test]
fn get_feature_info_format_text_to_feature_info_returns_none_for_empty_body() {
    assert!(GetFeatureInfoFormat::text_to_feature_info("<body>  </body>").is_none());
}

#[test]
fn get_feature_info_format_text_to_feature_info_returns_none_for_exception() {
    let text = "<ServiceExceptionReport>error</ServiceExceptionReport>";
    assert!(GetFeatureInfoFormat::text_to_feature_info(text).is_none());
}

#[test]
fn get_feature_info_format_text_to_feature_info_extracts_title() {
    let text = "<html><head><title>My Layer</title></head><body>data</body></html>";
    let result = GetFeatureInfoFormat::text_to_feature_info(text);
    assert!(result.is_some());
    let features = result.unwrap();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].name, Some("My Layer".to_string()));
}
