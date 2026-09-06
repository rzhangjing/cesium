//! Scene Model batch 3 spec tests.
//!
//! Mirrors CesiumJS Jasmine specs for Scene-level types:
//! - LightingModel
//! - ModelFeatureTable
//! - TextureUniform / TextureWrap / TextureFilter
//! - ModelDrawCommand / ModelDerivedCommand
//! - NodeRenderResources / NodeVertexAttribute
//! - Model3DTileContent
//! - TilesetMetadata
//! - Tileset3DTileContent
//! - ImageryCoverage
//! - ModelImagery / ImageryLayerSnapshot
//! - ModelImageryMapping (compute_tex_coords / compute_bounding_rectangle)
//! - PrimitiveOutlineGenerator

use serde_json::json;

use cesium_scene::model::lighting_model::LightingModel;
use cesium_scene::model::model_feature_table::ModelFeatureTable;
use cesium_scene::model::texture_uniform::{TextureFilter, TextureUniform, TextureWrap};
use cesium_scene::model::model_draw_command::{ModelDerivedCommand, ModelDrawCommand};
use cesium_scene::model::node_render_resources::{NodeRenderResources, NodeVertexAttribute};
use cesium_scene::model::model3_d_tile_content::Model3DTileContent;
use cesium_scene::tileset_metadata::TilesetMetadata;
use cesium_scene::tileset3_d_tile_content::Tileset3DTileContent;
use cesium_scene::model::imagery_coverage::ImageryCoverage;
use cesium_scene::model::model_imagery::{ImageryLayerSnapshot, ModelImagery};
use cesium_scene::model::model_imagery_mapping::{
    compute_bounding_rectangle, compute_tex_coords,
};
use cesium_scene::model::primitive_outline_generator::PrimitiveOutlineGenerator;
use cesium_scene::model::cartesian_rectangle::CartesianRectangle;
use cesium_scene::property_table::PropertyTable;
use cesium_scene::cull_face::CullFace;

// ── LightingModel ──────────────────────────────────────────────

#[test]
fn lighting_model_from_str_unlit() {
    assert_eq!(LightingModel::from_str("UNLIT"), Some(LightingModel::Unlit));
}

#[test]
fn lighting_model_from_str_pbr() {
    assert_eq!(LightingModel::from_str("PBR"), Some(LightingModel::Pbr));
}

#[test]
fn lighting_model_from_str_case_insensitive() {
    assert_eq!(LightingModel::from_str("unlit"), Some(LightingModel::Unlit));
    assert_eq!(LightingModel::from_str("pbr"), Some(LightingModel::Pbr));
}

#[test]
fn lighting_model_from_str_unknown() {
    assert_eq!(LightingModel::from_str("unknown"), None);
}

#[test]
fn lighting_model_as_str() {
    assert_eq!(LightingModel::Unlit.as_str(), "UNLIT");
    assert_eq!(LightingModel::Pbr.as_str(), "PBR");
}

#[test]
fn lighting_model_is_pbr() {
    assert!(!LightingModel::Unlit.is_pbr());
    assert!(LightingModel::Pbr.is_pbr());
}

#[test]
fn lighting_model_from_i32() {
    assert_eq!(LightingModel::from_i32(0), Some(LightingModel::Unlit));
    assert_eq!(LightingModel::from_i32(1), Some(LightingModel::Pbr));
    assert_eq!(LightingModel::from_i32(99), None);
}

#[test]
fn lighting_model_default_is_pbr() {
    assert_eq!(LightingModel::default(), LightingModel::Pbr);
}

// ── ModelFeatureTable ──────────────────────────────────────────

#[test]
fn model_feature_table_new() {
    let pt = PropertyTable::new(5);
    let ft = ModelFeatureTable::new(pt, 5);
    assert_eq!(ft.features_length, 5);
    assert!(!ft.style_commands_needed_dirty);
}

#[test]
fn model_feature_table_show_default_true() {
    let ft = ModelFeatureTable::new(PropertyTable::new(3), 3);
    assert!(ft.get_show(0));
    assert!(ft.get_show(1));
    assert!(ft.get_show(2));
}

#[test]
fn model_feature_table_set_show() {
    let mut ft = ModelFeatureTable::new(PropertyTable::new(3), 3);
    ft.set_show(1, false);
    assert!(ft.get_show(0));
    assert!(!ft.get_show(1));
    assert!(ft.style_commands_needed_dirty);
}

#[test]
fn model_feature_table_set_all_show() {
    let mut ft = ModelFeatureTable::new(PropertyTable::new(3), 3);
    ft.set_all_show(false);
    assert!(!ft.get_show(0));
    assert!(!ft.get_show(1));
    assert!(!ft.get_show(2));
}

#[test]
fn model_feature_table_color_override() {
    let mut ft = ModelFeatureTable::new(PropertyTable::new(2), 2);
    assert!(ft.get_color(0).is_none());
    ft.set_color(0, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(ft.get_color(0), Some([1.0, 0.0, 0.0, 1.0]));
    assert!(ft.get_color(1).is_none());
}

// ── TextureUniform ─────────────────────────────────────────────

#[test]
fn texture_uniform_default() {
    let tu = TextureUniform::new();
    assert!(tu.typed_array.is_none());
    assert!(tu.resource_url.is_none());
    assert_eq!(tu.wrap_s, TextureWrap::ClampToEdge);
    assert_eq!(tu.min_filter, TextureFilter::Linear);
    assert!(tu.dirty);
}

#[test]
fn texture_uniform_from_typed_array() {
    let tu = TextureUniform::from_typed_array(vec![255, 0, 0, 255], 1, 1);
    assert!(tu.has_typed_array());
    assert!(!tu.has_resource());
    assert_eq!(tu.width, Some(1));
    assert_eq!(tu.height, Some(1));
}

#[test]
fn texture_uniform_from_url() {
    let tu = TextureUniform::from_url("https://example.com/texture.png");
    assert!(!tu.has_typed_array());
    assert!(tu.has_resource());
    assert_eq!(tu.resource_url.as_deref(), Some("https://example.com/texture.png"));
}

// ── ModelDrawCommand ───────────────────────────────────────────

#[test]
fn model_draw_command_defaults() {
    let cmd = ModelDrawCommand::new();
    assert!(cmd.back_face_culling);
    assert_eq!(cmd.cull_face, CullFace::Back);
    assert!(!cmd.debug_show_bounding_volume);
    assert!(cmd.model_matrix_2d_dirty);
    assert!(!cmd.has_2d_commands);
    assert_eq!(cmd.derived_command_count(), 1); // original_command
}

#[test]
fn model_draw_command_mark_2d_dirty() {
    let mut cmd = ModelDrawCommand::new();
    cmd.has_2d_commands = true;
    cmd.model_matrix_2d_dirty = false;
    cmd.mark_2d_dirty();
    assert!(cmd.model_matrix_2d_dirty);
    assert!(!cmd.has_2d_commands);
}

#[test]
fn model_derived_command_defaults() {
    let dc = ModelDerivedCommand::new();
    assert!(dc.update_shadows);
    assert!(dc.update_back_face_culling);
    assert!(!dc.is_2d);
    assert!(dc.derived_command_2d.is_none());
}

// ── NodeRenderResources ────────────────────────────────────────

#[test]
fn node_render_resources_new() {
    let nrr = NodeRenderResources::new(5);
    assert_eq!(nrr.runtime_node_index, 5);
    assert_eq!(nrr.attribute_index, 1);
    assert_eq!(nrr.instance_count, 0);
    assert!(nrr.attributes.is_empty());
}

#[test]
fn node_render_resources_add_attribute() {
    let mut nrr = NodeRenderResources::new(0);
    let attr = NodeVertexAttribute {
        name: "NORMAL".to_string(),
        index: 0,
        components_per_attribute: 3,
        component_datatype: 5126,
        normalized: false,
    };
    let idx = nrr.add_attribute(attr);
    assert_eq!(idx, 1);
    assert_eq!(nrr.attribute_count(), 1);
    assert_eq!(nrr.attribute_index, 2);
}

// ── Model3DTileContent ─────────────────────────────────────────

#[test]
fn model3d_tile_content_new() {
    let content = Model3DTileContent::new("https://example.com/tile.b3dm");
    assert_eq!(content.url, "https://example.com/tile.b3dm");
    assert!(!content.ready);
    assert_eq!(content.features_length, 0);
    assert_eq!(content.total_byte_length(), 0);
    assert!(!content.has_metadata());
}

#[test]
fn model3d_tile_content_mark_ready() {
    let mut content = Model3DTileContent::new("tile.glb");
    content.mark_ready();
    assert!(content.ready);
}

#[test]
fn model3d_tile_content_total_byte_length() {
    let mut content = Model3DTileContent::new("tile.b3dm");
    content.geometry_byte_length = 1000;
    content.textures_byte_length = 500;
    content.batch_table_byte_length = 200;
    assert_eq!(content.total_byte_length(), 1700);
}

// ── TilesetMetadata ────────────────────────────────────────────

#[test]
fn tileset_metadata_new() {
    let meta = TilesetMetadata::new();
    assert!(meta.class_name.is_none());
    assert!(meta.properties.is_empty());
}

#[test]
fn tileset_metadata_property_operations() {
    let mut meta = TilesetMetadata::new();
    assert!(!meta.has_property("name"));
    meta.set_property("name", json!("test tileset"));
    assert!(meta.has_property("name"));
    assert_eq!(meta.get_property("name"), Some(&json!("test tileset")));
    assert_eq!(meta.get_property_ids().len(), 1);
}

#[test]
fn tileset_metadata_semantic_lookup() {
    let mut meta = TilesetMetadata::new();
    meta.set_property("height", json!({"semantic": "HEIGHT", "value": 100}));
    assert!(meta.has_property_by_semantic("HEIGHT"));
    assert!(!meta.has_property_by_semantic("WIDTH"));
    let val = meta.get_property_by_semantic("HEIGHT").unwrap();
    assert_eq!(val.get("value").unwrap().as_i64(), Some(100));
}

// ── Tileset3DTileContent ───────────────────────────────────────

#[test]
fn tileset3d_tile_content_zero_lengths() {
    let content = Tileset3DTileContent::new("https://example.com/tileset.json");
    assert_eq!(content.features_length(), 0);
    assert_eq!(content.points_length(), 0);
    assert_eq!(content.triangles_length(), 0);
    assert_eq!(content.geometry_byte_length(), 0);
    assert_eq!(content.textures_byte_length(), 0);
    assert!(!content.ready);
}

#[test]
fn tileset3d_tile_content_mark_ready() {
    let mut content = Tileset3DTileContent::new("tileset.json");
    content.mark_ready();
    assert!(content.ready);
}

// ── ImageryCoverage ────────────────────────────────────────────

#[test]
fn imagery_coverage_new() {
    let rect = CartesianRectangle::new(0.0, 0.0, 1.0, 1.0);
    let cov = ImageryCoverage::new(5, 10, 3, rect);
    assert_eq!(cov.x, 5);
    assert_eq!(cov.y, 10);
    assert_eq!(cov.level, 3);
    assert!(!cov.applied);
    assert_eq!(cov.width(), 1.0);
    assert_eq!(cov.height(), 1.0);
}

#[test]
fn imagery_coverage_contains_uv() {
    let rect = CartesianRectangle::new(0.2, 0.3, 0.8, 0.9);
    let cov = ImageryCoverage::new(0, 0, 0, rect);
    assert!(cov.contains_uv(0.5, 0.5));
    assert!(!cov.contains_uv(0.1, 0.5)); // below min_x
    assert!(!cov.contains_uv(0.5, 0.95)); // above max_y
}

// ── ModelImagery ───────────────────────────────────────────────

#[test]
fn model_imagery_new() {
    let mi = ModelImagery::new();
    assert_eq!(mi.primitive_imagery_count, 0);
    assert!(!mi.ready);
    assert!(mi.all_primitives_ready()); // vacuously true
}

#[test]
fn model_imagery_add_and_set_primitive() {
    let mut mi = ModelImagery::new();
    mi.add_primitive();
    mi.add_primitive();
    assert_eq!(mi.primitive_imagery_count, 2);
    assert!(!mi.all_primitives_ready());
    mi.set_primitive_ready(0, true);
    mi.set_primitive_ready(1, true);
    assert!(mi.all_primitives_ready());
}

#[test]
fn model_imagery_update_configurations() {
    let mut mi = ModelImagery::new();
    let configs = vec![ImageryLayerSnapshot {
        layer_index: 0,
        enabled: true,
        alpha: 1.0,
    }];
    let changed = mi.update_configurations(configs.clone());
    assert!(changed);
    let changed2 = mi.update_configurations(configs);
    assert!(!changed2); // same configs → no change
}

// ── ModelImageryMapping ────────────────────────────────────────

#[test]
fn compute_bounding_rectangle_basic() {
    let positions = vec![(1.0, 2.0), (3.0, 4.0), (0.5, 1.5)];
    let rect = compute_bounding_rectangle(&positions);
    assert_eq!(rect.min_x, 0.5);
    assert_eq!(rect.min_y, 1.5);
    assert_eq!(rect.max_x, 3.0);
    assert_eq!(rect.max_y, 4.0);
}

#[test]
fn compute_bounding_rectangle_empty() {
    let rect = compute_bounding_rectangle(&[]);
    assert_eq!(rect.min_x, 0.0);
    assert_eq!(rect.max_x, 0.0);
}

#[test]
fn compute_tex_coords_within_bounds() {
    let rect = CartesianRectangle::new(0.0, 0.0, 10.0, 10.0);
    let positions = vec![(5.0, 5.0), (0.0, 0.0), (10.0, 10.0)];
    let uvs = compute_tex_coords(&positions, &rect);
    assert_eq!(uvs.len(), 3);
    assert!((uvs[0].x - 0.5).abs() < 1e-10);
    assert!((uvs[0].y - 0.5).abs() < 1e-10);
    assert!((uvs[1].x - 0.0).abs() < 1e-10);
    assert!((uvs[2].x - 1.0).abs() < 1e-10);
}

#[test]
fn compute_tex_coords_clamped() {
    let rect = CartesianRectangle::new(0.0, 0.0, 1.0, 1.0);
    let positions = vec![(-1.0, 2.0)]; // outside bounds
    let uvs = compute_tex_coords(&positions, &rect);
    assert!((uvs[0].x - 0.0).abs() < 1e-10); // clamped to 0
    assert!((uvs[0].y - 1.0).abs() < 1e-10); // clamped to 1
}

// ── PrimitiveOutlineGenerator ──────────────────────────────────

#[test]
fn primitive_outline_generator_new() {
    let gen = PrimitiveOutlineGenerator::new();
    assert_eq!(gen.atlas_width, 256);
    assert_eq!(gen.atlas_height, 256);
    assert_eq!(gen.active_count(), 0);
}

#[test]
fn primitive_outline_generator_assign_and_query() {
    let mut gen = PrimitiveOutlineGenerator::new();
    assert_eq!(gen.assign_outline(0), Some(0));
    assert_eq!(gen.assign_outline(1), Some(1));
    assert_eq!(gen.assign_outline(0), Some(0)); // idempotent
    assert!(gen.has_outline(0));
    assert!(!gen.has_outline(5));
    assert_eq!(gen.active_count(), 2);
}

#[test]
fn primitive_outline_generator_remove() {
    let mut gen = PrimitiveOutlineGenerator::new();
    gen.assign_outline(0);
    gen.assign_outline(1);
    gen.remove_outline(0);
    assert!(!gen.has_outline(0));
    assert!(gen.has_outline(1));
    assert_eq!(gen.active_count(), 1);
}
