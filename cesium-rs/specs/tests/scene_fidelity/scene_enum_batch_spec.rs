//! Scene enum batch spec tests.
//!
//! Mirrors CesiumJS Jasmine specs for Scene-level enum/constant types:
//! - BoundingVolumeSemantics
//! - Cesium3DTileRefine
//! - PostProcessStageSampleMode
//! - MapMode2D
//! - CullFace
//! - HeightReference
//! - HorizontalOrigin
//! - EdgeDisplayMode
//! - SensorVolumePortionToDisplay
//! - LabelStyle
//! - Cesium3DTileColorBlendMode
//! - Cesium3DTileOptimizationHint
//! - QuadtreeTileLoadState
//! - ShadowMode
//! - BlendOption
//! - InstanceAttributeSemantic
//! - TerrainState
//! - BillboardLoadState
//! - JobType
//! - Cesium3DTilePointCloudColorBlendMode
//! - Cesium3DTileOptimizedHint
//! - PropertyAttributeProperty
//! - Multiple3DTileContent

use cesium_scene::billboard_load_state::BillboardLoadState;
use cesium_scene::blend_option::BlendOption;
use cesium_scene::bounding_volume_semantics::BoundingVolumeSemantics;
use cesium_scene::cesium3_d_tile_color_blend_mode::Cesium3DTileColorBlendMode;
use cesium_scene::cesium3_d_tile_optimization_hint::Cesium3DTileOptimizationHint;
use cesium_scene::cesium3_d_tile_optimized_hint::Cesium3DTileOptimizedHint;
use cesium_scene::cesium3_d_tile_point_cloud_color_blend_mode::Cesium3DTilePointCloudColorBlendMode;
use cesium_scene::cesium3_d_tile_refine::Cesium3DTileRefine;
use cesium_scene::cull_face::CullFace;
use cesium_scene::edge_display_mode::EdgeDisplayMode;
use cesium_scene::height_reference::HeightReference;
use cesium_scene::horizontal_origin::HorizontalOrigin;
use cesium_scene::instance_attribute_semantic::InstanceAttributeSemantic;
use cesium_scene::job_type::JobType;
use cesium_scene::label_style::LabelStyle;
use cesium_scene::map_mode2_d::MapMode2D;
use cesium_scene::multiple3_d_tile_content::Multiple3DTileContent;
use cesium_scene::post_process_stage_sample_mode::PostProcessStageSampleMode;
use cesium_scene::property_attribute_property::PropertyAttributeProperty;
use cesium_scene::quadtree_tile_load_state::QuadtreeTileLoadState;
use cesium_scene::sensor_volume_portion_to_display::SensorVolumePortionToDisplay;
use cesium_scene::shadow_mode::ShadowMode;
use cesium_scene::terrain_state::TerrainState;

// ════════════════════════════════════════════════════════════════
// BoundingVolumeSemantics
// ════════════════════════════════════════════════════════════════

#[test]
fn bounding_volume_semantics_roundtrip() {
    assert_eq!(BoundingVolumeSemantics::from_i32(0), Some(BoundingVolumeSemantics::BoundingVolume));
    assert_eq!(BoundingVolumeSemantics::from_i32(1), Some(BoundingVolumeSemantics::ContentBoundingVolume));
    assert_eq!(BoundingVolumeSemantics::from_i32(99), None);
    assert_eq!(BoundingVolumeSemantics::BoundingVolume.as_i32(), 0);
    assert_eq!(BoundingVolumeSemantics::BoundingVolume.as_str(), "BOUNDING_VOLUME");
    assert_eq!(BoundingVolumeSemantics::ContentBoundingVolume.as_str(), "CONTENT_BOUNDING_VOLUME");
    assert_eq!(BoundingVolumeSemantics::default(), BoundingVolumeSemantics::BoundingVolume);
}

// ════════════════════════════════════════════════════════════════
// Cesium3DTileRefine
// ════════════════════════════════════════════════════════════════

#[test]
fn cesium3d_tile_refine_roundtrip() {
    assert_eq!(Cesium3DTileRefine::from_i32(0), Some(Cesium3DTileRefine::Add));
    assert_eq!(Cesium3DTileRefine::from_i32(1), Some(Cesium3DTileRefine::Replace));
    assert_eq!(Cesium3DTileRefine::from_i32(5), None);
    assert_eq!(Cesium3DTileRefine::Add.as_str(), "ADD");
    assert_eq!(Cesium3DTileRefine::Replace.as_str(), "REPLACE");
    assert_eq!(Cesium3DTileRefine::default(), Cesium3DTileRefine::Replace);
}

// ════════════════════════════════════════════════════════════════
// PostProcessStageSampleMode
// ════════════════════════════════════════════════════════════════

#[test]
fn post_process_stage_sample_mode_roundtrip() {
    assert_eq!(PostProcessStageSampleMode::from_i32(0), Some(PostProcessStageSampleMode::Nearest));
    assert_eq!(PostProcessStageSampleMode::from_i32(1), Some(PostProcessStageSampleMode::Linear));
    assert_eq!(PostProcessStageSampleMode::from_i32(2), None);
    assert_eq!(PostProcessStageSampleMode::Nearest.as_str(), "NEAREST");
    assert_eq!(PostProcessStageSampleMode::default(), PostProcessStageSampleMode::Nearest);
}

// ════════════════════════════════════════════════════════════════
// MapMode2D
// ════════════════════════════════════════════════════════════════

#[test]
fn map_mode2d_roundtrip() {
    assert_eq!(MapMode2D::from_i32(0), Some(MapMode2D::Rotate));
    assert_eq!(MapMode2D::from_i32(1), Some(MapMode2D::InfiniteScroll));
    assert_eq!(MapMode2D::from_i32(-1), None);
    assert_eq!(MapMode2D::Rotate.as_str(), "ROTATE");
    assert_eq!(MapMode2D::InfiniteScroll.as_str(), "INFINITE_SCROLL");
    assert_eq!(MapMode2D::default(), MapMode2D::Rotate);
}

// ════════════════════════════════════════════════════════════════
// CullFace
// ════════════════════════════════════════════════════════════════

#[test]
fn cull_face_webgl_values() {
    assert_eq!(CullFace::Front.as_i32(), 0x0404);
    assert_eq!(CullFace::Back.as_i32(), 0x0405);
    assert_eq!(CullFace::from_i32(0x0404), Some(CullFace::Front));
    assert_eq!(CullFace::from_i32(0x0405), Some(CullFace::Back));
    assert_eq!(CullFace::from_i32(0), None);
    assert_eq!(CullFace::Front.as_str(), "FRONT");
    assert_eq!(CullFace::Back.as_str(), "BACK");
    assert_eq!(CullFace::default(), CullFace::Back);
}

// ════════════════════════════════════════════════════════════════
// HeightReference
// ════════════════════════════════════════════════════════════════

#[test]
fn height_reference_roundtrip() {
    assert_eq!(HeightReference::from_i32(0), Some(HeightReference::None));
    assert_eq!(HeightReference::from_i32(1), Some(HeightReference::ClampToGround));
    assert_eq!(HeightReference::from_i32(2), Some(HeightReference::RelativeToGround));
    assert_eq!(HeightReference::from_i32(99), None);
    assert_eq!(HeightReference::None.as_str(), "NONE");
    assert_eq!(HeightReference::ClampToGround.as_str(), "CLAMP_TO_GROUND");
}

#[test]
fn height_reference_predicates() {
    assert!(HeightReference::ClampToGround.is_clamp());
    assert!(!HeightReference::None.is_clamp());
    assert!(HeightReference::RelativeToGround.is_relative());
    assert!(!HeightReference::None.is_relative());
    assert_eq!(HeightReference::default(), HeightReference::None);
}

// ════════════════════════════════════════════════════════════════
// HorizontalOrigin
// ════════════════════════════════════════════════════════════════

#[test]
fn horizontal_origin_roundtrip() {
    assert_eq!(HorizontalOrigin::from_i32(0), Some(HorizontalOrigin::Center));
    assert_eq!(HorizontalOrigin::from_i32(1), Some(HorizontalOrigin::Left));
    assert_eq!(HorizontalOrigin::from_i32(-1), Some(HorizontalOrigin::Right));
    assert_eq!(HorizontalOrigin::from_i32(2), None);
    assert_eq!(HorizontalOrigin::Center.as_str(), "CENTER");
    assert_eq!(HorizontalOrigin::Left.as_str(), "LEFT");
    assert_eq!(HorizontalOrigin::Right.as_str(), "RIGHT");
    assert_eq!(HorizontalOrigin::default(), HorizontalOrigin::Center);
}

// ════════════════════════════════════════════════════════════════
// EdgeDisplayMode
// ════════════════════════════════════════════════════════════════

#[test]
fn edge_display_mode_roundtrip() {
    assert_eq!(EdgeDisplayMode::from_i32(0), Some(EdgeDisplayMode::None));
    assert_eq!(EdgeDisplayMode::from_i32(1), Some(EdgeDisplayMode::Flat));
    assert_eq!(EdgeDisplayMode::from_i32(2), Some(EdgeDisplayMode::Phong));
    assert_eq!(EdgeDisplayMode::from_i32(3), None);
    assert_eq!(EdgeDisplayMode::None.as_str(), "NONE");
    assert_eq!(EdgeDisplayMode::default(), EdgeDisplayMode::None);
}

// ════════════════════════════════════════════════════════════════
// SensorVolumePortionToDisplay
// ════════════════════════════════════════════════════════════════

#[test]
fn sensor_volume_portion_roundtrip() {
    assert_eq!(SensorVolumePortionToDisplay::from_i32(0), Some(SensorVolumePortionToDisplay::Complete));
    assert_eq!(SensorVolumePortionToDisplay::from_i32(1), Some(SensorVolumePortionToDisplay::AboveEllipsoidHorizonOnly));
    assert_eq!(SensorVolumePortionToDisplay::from_i32(2), Some(SensorVolumePortionToDisplay::BelowEllipsoidHorizonOnly));
    assert_eq!(SensorVolumePortionToDisplay::Complete.as_str(), "COMPLETE");
    assert!(SensorVolumePortionToDisplay::validate(0));
    assert!(!SensorVolumePortionToDisplay::validate(99));
    assert_eq!(SensorVolumePortionToDisplay::default(), SensorVolumePortionToDisplay::Complete);
}

// ════════════════════════════════════════════════════════════════
// LabelStyle
// ════════════════════════════════════════════════════════════════

#[test]
fn label_style_roundtrip() {
    assert_eq!(LabelStyle::from_i32(0), Some(LabelStyle::Fill));
    assert_eq!(LabelStyle::from_i32(1), Some(LabelStyle::Outline));
    assert_eq!(LabelStyle::from_i32(2), Some(LabelStyle::FillAndOutline));
    assert_eq!(LabelStyle::from_i32(3), None);
    assert_eq!(LabelStyle::Fill.as_str(), "FILL");
    assert_eq!(LabelStyle::FillAndOutline.as_str(), "FILL_AND_OUTLINE");
    assert_eq!(LabelStyle::default(), LabelStyle::Fill);
}

// ════════════════════════════════════════════════════════════════
// Cesium3DTileColorBlendMode
// ════════════════════════════════════════════════════════════════

#[test]
fn cesium3d_tile_color_blend_mode_roundtrip() {
    assert_eq!(Cesium3DTileColorBlendMode::from_i32(0), Some(Cesium3DTileColorBlendMode::Highlight));
    assert_eq!(Cesium3DTileColorBlendMode::from_i32(1), Some(Cesium3DTileColorBlendMode::Replace));
    assert_eq!(Cesium3DTileColorBlendMode::from_i32(2), Some(Cesium3DTileColorBlendMode::Mix));
    assert_eq!(Cesium3DTileColorBlendMode::from_i32(3), None);
    assert_eq!(Cesium3DTileColorBlendMode::Highlight.as_str(), "HIGHLIGHT");
    assert_eq!(Cesium3DTileColorBlendMode::default(), Cesium3DTileColorBlendMode::Highlight);
}

// ════════════════════════════════════════════════════════════════
// Cesium3DTileOptimizationHint
// ════════════════════════════════════════════════════════════════

#[test]
fn cesium3d_tile_optimization_hint_roundtrip() {
    assert_eq!(Cesium3DTileOptimizationHint::from_i32(-1), Some(Cesium3DTileOptimizationHint::NotComputed));
    assert_eq!(Cesium3DTileOptimizationHint::from_i32(0), Some(Cesium3DTileOptimizationHint::SkipOptimization));
    assert_eq!(Cesium3DTileOptimizationHint::from_i32(1), Some(Cesium3DTileOptimizationHint::UseOptimization));
    assert_eq!(Cesium3DTileOptimizationHint::from_i32(2), None);
    assert_eq!(Cesium3DTileOptimizationHint::NotComputed.as_str(), "NOT_COMPUTED");
    assert_eq!(Cesium3DTileOptimizationHint::default(), Cesium3DTileOptimizationHint::NotComputed);
}

// ════════════════════════════════════════════════════════════════
// QuadtreeTileLoadState
// ════════════════════════════════════════════════════════════════

#[test]
fn quadtree_tile_load_state_roundtrip() {
    assert_eq!(QuadtreeTileLoadState::from_i32(0), Some(QuadtreeTileLoadState::Start));
    assert_eq!(QuadtreeTileLoadState::from_i32(1), Some(QuadtreeTileLoadState::Loading));
    assert_eq!(QuadtreeTileLoadState::from_i32(2), Some(QuadtreeTileLoadState::Done));
    assert_eq!(QuadtreeTileLoadState::from_i32(3), Some(QuadtreeTileLoadState::Failed));
    assert_eq!(QuadtreeTileLoadState::from_i32(4), None);
    assert_eq!(QuadtreeTileLoadState::Done.as_str(), "DONE");
    assert!(QuadtreeTileLoadState::Done.is_ready());
    assert!(!QuadtreeTileLoadState::Loading.is_ready());
    assert_eq!(QuadtreeTileLoadState::default(), QuadtreeTileLoadState::Start);
}

// ════════════════════════════════════════════════════════════════
// ShadowMode
// ════════════════════════════════════════════════════════════════

#[test]
fn shadow_mode_roundtrip() {
    assert_eq!(ShadowMode::from_i32(0), Some(ShadowMode::Disabled));
    assert_eq!(ShadowMode::from_i32(1), Some(ShadowMode::Enabled));
    assert_eq!(ShadowMode::from_i32(2), Some(ShadowMode::CastOnly));
    assert_eq!(ShadowMode::from_i32(3), Some(ShadowMode::ReceiveOnly));
    assert_eq!(ShadowMode::from_i32(4), None);
    assert_eq!(ShadowMode::Enabled.as_str(), "ENABLED");
}

#[test]
fn shadow_mode_cast_receive() {
    assert!(ShadowMode::Enabled.cast_shadows());
    assert!(ShadowMode::CastOnly.cast_shadows());
    assert!(!ShadowMode::ReceiveOnly.cast_shadows());
    assert!(!ShadowMode::Disabled.cast_shadows());

    assert!(ShadowMode::Enabled.receive_shadows());
    assert!(ShadowMode::ReceiveOnly.receive_shadows());
    assert!(!ShadowMode::CastOnly.receive_shadows());
    assert!(!ShadowMode::Disabled.receive_shadows());
}

#[test]
fn shadow_mode_from_cast_receive() {
    assert_eq!(ShadowMode::from_cast_receive(true, true), ShadowMode::Enabled);
    assert_eq!(ShadowMode::from_cast_receive(true, false), ShadowMode::CastOnly);
    assert_eq!(ShadowMode::from_cast_receive(false, true), ShadowMode::ReceiveOnly);
    assert_eq!(ShadowMode::from_cast_receive(false, false), ShadowMode::Disabled);
    assert_eq!(ShadowMode::NUMBER_OF_SHADOW_MODES, 4);
    assert_eq!(ShadowMode::default(), ShadowMode::Disabled);
}

// ════════════════════════════════════════════════════════════════
// BlendOption
// ════════════════════════════════════════════════════════════════

#[test]
fn blend_option_roundtrip() {
    assert_eq!(BlendOption::from_i32(0), Some(BlendOption::Disabled));
    assert_eq!(BlendOption::from_i32(1), Some(BlendOption::AlphaBlend));
    assert_eq!(BlendOption::from_i32(2), Some(BlendOption::Premultiplied));
    assert_eq!(BlendOption::from_i32(3), Some(BlendOption::Additive));
    assert_eq!(BlendOption::from_i32(4), None);
    assert_eq!(BlendOption::Disabled.as_str(), "DISABLED");
    assert!(!BlendOption::Disabled.is_transparent());
    assert!(BlendOption::AlphaBlend.is_transparent());
    assert!(BlendOption::Additive.is_transparent());
    assert_eq!(BlendOption::default(), BlendOption::Disabled);
}

// ════════════════════════════════════════════════════════════════
// InstanceAttributeSemantic
// ════════════════════════════════════════════════════════════════

#[test]
fn instance_attribute_semantic_roundtrip() {
    assert_eq!(InstanceAttributeSemantic::from_i32(0), Some(InstanceAttributeSemantic::Position));
    assert_eq!(InstanceAttributeSemantic::from_i32(3), Some(InstanceAttributeSemantic::Translation));
    assert_eq!(InstanceAttributeSemantic::from_i32(4), None);
    assert_eq!(InstanceAttributeSemantic::Position.as_str(), "POSITION");
    assert_eq!(InstanceAttributeSemantic::Translation.as_str(), "TRANSLATION");
    assert_eq!(InstanceAttributeSemantic::default(), InstanceAttributeSemantic::Position);
}

#[test]
fn instance_attribute_semantic_from_gltf() {
    assert_eq!(
        InstanceAttributeSemantic::from_gltf_semantic("TRANSLATION"),
        Some(InstanceAttributeSemantic::Translation)
    );
    assert_eq!(
        InstanceAttributeSemantic::from_gltf_semantic("ROTATION"),
        Some(InstanceAttributeSemantic::Rotation)
    );
    assert_eq!(
        InstanceAttributeSemantic::from_gltf_semantic("SCALE"),
        Some(InstanceAttributeSemantic::Scale)
    );
    assert_eq!(InstanceAttributeSemantic::from_gltf_semantic("POSITION"), None);
}

// ════════════════════════════════════════════════════════════════
// TerrainState
// ════════════════════════════════════════════════════════════════

#[test]
fn terrain_state_roundtrip() {
    assert_eq!(TerrainState::from_i32(0), Some(TerrainState::Start));
    assert_eq!(TerrainState::from_i32(1), Some(TerrainState::Loading));
    assert_eq!(TerrainState::from_i32(2), Some(TerrainState::Ready));
    assert_eq!(TerrainState::from_i32(3), Some(TerrainState::Failed));
    assert_eq!(TerrainState::from_i32(4), None);
    assert_eq!(TerrainState::Ready.as_str(), "READY");
    assert!(TerrainState::Ready.is_ready());
    assert!(!TerrainState::Loading.is_ready());
    assert!(TerrainState::Failed.is_failed());
    assert!(!TerrainState::Ready.is_failed());
    assert_eq!(TerrainState::default(), TerrainState::Start);
}

// ════════════════════════════════════════════════════════════════
// BillboardLoadState
// ════════════════════════════════════════════════════════════════

#[test]
fn billboard_load_state_roundtrip() {
    assert_eq!(BillboardLoadState::from_i32(0), Some(BillboardLoadState::Unloaded));
    assert_eq!(BillboardLoadState::from_i32(1), Some(BillboardLoadState::Loading));
    assert_eq!(BillboardLoadState::from_i32(2), Some(BillboardLoadState::Ready));
    assert_eq!(BillboardLoadState::from_i32(3), Some(BillboardLoadState::Failed));
    assert_eq!(BillboardLoadState::from_i32(4), None);
    assert_eq!(BillboardLoadState::Unloaded.as_str(), "UNLOADED");
    assert!(BillboardLoadState::Ready.is_ready());
    assert!(!BillboardLoadState::Unloaded.is_ready());
    assert_eq!(BillboardLoadState::default(), BillboardLoadState::Unloaded);
}

// ════════════════════════════════════════════════════════════════
// JobType
// ════════════════════════════════════════════════════════════════

#[test]
fn job_type_roundtrip() {
    assert_eq!(JobType::from_i32(0), Some(JobType::Default));
    assert_eq!(JobType::from_i32(1), Some(JobType::Terrain));
    assert_eq!(JobType::from_i32(2), Some(JobType::Imagery));
    assert_eq!(JobType::from_i32(3), None);
    assert_eq!(JobType::Default.as_str(), "DEFAULT");
    assert_eq!(JobType::Terrain.as_str(), "TERRAIN");
    assert_eq!(JobType::NUMBER_OF_JOB_TYPES, 3);
    assert_eq!(JobType::default(), JobType::Default);
}

// ════════════════════════════════════════════════════════════════
// Cesium3DTilePointCloudColorBlendMode
// ════════════════════════════════════════════════════════════════

#[test]
fn point_cloud_color_blend_mode_roundtrip() {
    assert_eq!(Cesium3DTilePointCloudColorBlendMode::from_i32(0), Some(Cesium3DTilePointCloudColorBlendMode::Replace));
    assert_eq!(Cesium3DTilePointCloudColorBlendMode::from_i32(1), Some(Cesium3DTilePointCloudColorBlendMode::Highlight));
    assert_eq!(Cesium3DTilePointCloudColorBlendMode::from_i32(2), Some(Cesium3DTilePointCloudColorBlendMode::Mix));
    assert_eq!(Cesium3DTilePointCloudColorBlendMode::from_i32(3), None);
    assert_eq!(Cesium3DTilePointCloudColorBlendMode::Replace.as_str(), "REPLACE");
    assert_eq!(Cesium3DTilePointCloudColorBlendMode::default(), Cesium3DTilePointCloudColorBlendMode::Replace);
}

// ════════════════════════════════════════════════════════════════
// Cesium3DTileOptimizedHint
// ════════════════════════════════════════════════════════════════

#[test]
fn cesium3d_tile_optimized_hint_constants() {
    assert_eq!(Cesium3DTileOptimizedHint::NOT_COMPUTED, -1);
    assert_eq!(Cesium3DTileOptimizedHint::SKIP_OPTIMIZATION, 0);
    assert_eq!(Cesium3DTileOptimizedHint::USE_OPTIMIZATION, 1);
    assert!(Cesium3DTileOptimizedHint::should_optimize(1));
    assert!(!Cesium3DTileOptimizedHint::should_optimize(0));
    assert!(!Cesium3DTileOptimizedHint::should_optimize(-1));
    assert!(Cesium3DTileOptimizedHint::is_computed(0));
    assert!(Cesium3DTileOptimizedHint::is_computed(1));
    assert!(!Cesium3DTileOptimizedHint::is_computed(-1));
}

// ════════════════════════════════════════════════════════════════
// PropertyAttributeProperty
// ════════════════════════════════════════════════════════════════

#[test]
fn property_attribute_property_new() {
    let p = PropertyAttributeProperty::new("_FEATURE_ID_0".to_string());
    assert_eq!(p.attribute, "_FEATURE_ID_0");
    assert!(!p.has_value_transform);
    assert!(p.offset.is_none());
    assert!(p.scale.is_none());
    assert!(!p.has_transforms());
}

#[test]
fn property_attribute_property_with_transforms() {
    let mut p = PropertyAttributeProperty::new("height".to_string());
    p.offset = Some(serde_json::json!(10.0));
    p.scale = Some(serde_json::json!(2.0));
    assert!(p.has_transforms());
}

#[test]
fn property_attribute_property_default() {
    let p = PropertyAttributeProperty::default();
    assert!(p.attribute.is_empty());
}

// ════════════════════════════════════════════════════════════════
// Multiple3DTileContent
// ════════════════════════════════════════════════════════════════

#[test]
fn multiple3d_tile_content_new() {
    let m = Multiple3DTileContent::new();
    assert_eq!(m.contents_length, 0);
    assert!(m.content_urls.is_empty());
    assert!(!m.ready);
    assert_eq!(m.requests_in_flight, 0);
    assert_eq!(m.external_tileset_count, 0);
    assert!(!m.is_ready());
    assert_eq!(m.inner_length(), 0);
}

#[test]
fn multiple3d_tile_content_add_urls() {
    let mut m = Multiple3DTileContent::new();
    m.add_content_url("content1.b3dm".to_string());
    m.add_content_url("content2.b3dm".to_string());
    assert_eq!(m.inner_length(), 2);
    assert_eq!(m.content_urls.len(), 2);
    assert_eq!(m.content_urls[0], "content1.b3dm");
}
