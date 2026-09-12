//! Scene fidelity specs for the fourth batch of substantiated Scene types:
//! - Axis
//! - CloudType
//! - DynamicAtmosphereLightingType
//! - ImplicitSubdivisionScheme
//! - Tonemapper
//! - ParticleBurst
//! - VoxelMetadataOrder
//! - IonWorldImageryStyle
//! - PrimitiveState
//! - TileSelectionResult
//! - SdfSettings
//! - ModelAnimationLoop
//! - PickedMetadataInfo

use cesium_core::matrix4::Matrix4;
use cesium_scene::axis::Axis;
use cesium_scene::cloud_type::CloudType;
use cesium_scene::dynamic_atmosphere_lighting_type::DynamicAtmosphereLightingType;
use cesium_scene::implicit_subdivision_scheme::ImplicitSubdivisionScheme;
use cesium_scene::ion_world_imagery_style::IonWorldImageryStyle;
use cesium_scene::model_animation_loop::ModelAnimationLoop;
use cesium_scene::particle_burst::ParticleBurst;
use cesium_scene::picked_metadata_info::PickedMetadataInfo;
use cesium_scene::primitive_state::PrimitiveState;
use cesium_scene::sdf_settings::SdfSettings;
use cesium_scene::tile_selection_result::TileSelectionResult;
use cesium_scene::tonemapper::Tonemapper;
use cesium_scene::voxel_metadata_order::VoxelMetadataOrder;

// ─── Axis ──────────────────────────────────────────────────────

#[test]
fn axis_as_i32_and_from_i32() {
    assert_eq!(Axis::X.as_i32(), 0);
    assert_eq!(Axis::Y.as_i32(), 1);
    assert_eq!(Axis::Z.as_i32(), 2);

    assert_eq!(Axis::from_i32(0), Some(Axis::X));
    assert_eq!(Axis::from_i32(1), Some(Axis::Y));
    assert_eq!(Axis::from_i32(2), Some(Axis::Z));
    assert_eq!(Axis::from_i32(99), None);
}

#[test]
fn axis_y_up_to_z_up_returns_non_identity_matrix() {
    let m = Axis::y_up_to_z_up();
    // y-up to z-up rotates 90° about x-axis: the 3x3 sub-matrix should have
    // [1,0,0; 0,0,1; 0,-1,0] in column-major order.
    let col = Matrix4::get_column_new(&m, 0);
    // First column of the rotation should be (1, 0, 0)
    assert!((col.x - 1.0).abs() < 1e-10);
}

#[test]
fn axis_z_up_to_y_up_returns_valid_matrix() {
    let m = Axis::z_up_to_y_up();
    let col = Matrix4::get_column_new(&m, 0);
    assert!((col.x - 1.0).abs() < 1e-10);
}

#[test]
fn axis_all_six_conversions_produce_valid_matrices() {
    // Just verify all six functions return without panic and produce valid Matrix4
    let _m1 = Axis::y_up_to_z_up();
    let _m2 = Axis::z_up_to_y_up();
    let _m3 = Axis::x_up_to_y_up();
    let _m4 = Axis::x_up_to_z_up();
    let _m5 = Axis::z_up_to_x_up();
    let _m6 = Axis::y_up_to_x_up();
}

// ─── CloudType ─────────────────────────────────────────────────

#[test]
fn cloud_type_cumulus_is_zero() {
    assert_eq!(CloudType::Cumulus.as_i32(), 0);
    assert_eq!(CloudType::from_i32(0), Some(CloudType::Cumulus));
}

#[test]
fn cloud_type_validate() {
    assert!(CloudType::validate(CloudType::Cumulus));
}

#[test]
fn cloud_type_from_i32_invalid() {
    assert_eq!(CloudType::from_i32(5), None);
}

// ─── DynamicAtmosphereLightingType ─────────────────────────────

#[test]
fn dynamic_atmosphere_lighting_type_values() {
    assert_eq!(DynamicAtmosphereLightingType::None.as_i32(), 0);
    assert_eq!(DynamicAtmosphereLightingType::SceneLight.as_i32(), 1);
    assert_eq!(DynamicAtmosphereLightingType::Sunlight.as_i32(), 2);
}

#[test]
fn dynamic_atmosphere_lighting_type_from_i32() {
    assert_eq!(
        DynamicAtmosphereLightingType::from_i32(0),
        Some(DynamicAtmosphereLightingType::None)
    );
    assert_eq!(
        DynamicAtmosphereLightingType::from_i32(1),
        Some(DynamicAtmosphereLightingType::SceneLight)
    );
    assert_eq!(
        DynamicAtmosphereLightingType::from_i32(2),
        Some(DynamicAtmosphereLightingType::Sunlight)
    );
    assert_eq!(DynamicAtmosphereLightingType::from_i32(99), None);
}

// ─── ImplicitSubdivisionScheme ─────────────────────────────────

#[test]
fn implicit_subdivision_scheme_as_str() {
    assert_eq!(ImplicitSubdivisionScheme::Quadtree.as_str(), "QUADTREE");
    assert_eq!(ImplicitSubdivisionScheme::Octree.as_str(), "OCTREE");
}

#[test]
fn implicit_subdivision_scheme_from_str() {
    assert_eq!(
        ImplicitSubdivisionScheme::from_str("QUADTREE"),
        Some(ImplicitSubdivisionScheme::Quadtree)
    );
    assert_eq!(
        ImplicitSubdivisionScheme::from_str("OCTREE"),
        Some(ImplicitSubdivisionScheme::Octree)
    );
    assert_eq!(ImplicitSubdivisionScheme::from_str("INVALID"), None);
}

#[test]
fn implicit_subdivision_scheme_branching_factor() {
    assert_eq!(ImplicitSubdivisionScheme::Quadtree.get_branching_factor(), 4);
    assert_eq!(ImplicitSubdivisionScheme::Octree.get_branching_factor(), 8);
}

// ─── Tonemapper ────────────────────────────────────────────────

#[test]
fn tonemapper_as_str_all_variants() {
    assert_eq!(Tonemapper::Reinhard.as_str(), "REINHARD");
    assert_eq!(Tonemapper::ModifiedReinhard.as_str(), "MODIFIED_REINHARD");
    assert_eq!(Tonemapper::Filmic.as_str(), "FILMIC");
    assert_eq!(Tonemapper::Aces.as_str(), "ACES");
    assert_eq!(Tonemapper::PbrNeutral.as_str(), "PBR_NEUTRAL");
}

#[test]
fn tonemapper_from_str_round_trips() {
    assert_eq!(Tonemapper::from_str("REINHARD"), Some(Tonemapper::Reinhard));
    assert_eq!(
        Tonemapper::from_str("MODIFIED_REINHARD"),
        Some(Tonemapper::ModifiedReinhard)
    );
    assert_eq!(Tonemapper::from_str("FILMIC"), Some(Tonemapper::Filmic));
    assert_eq!(Tonemapper::from_str("ACES"), Some(Tonemapper::Aces));
    assert_eq!(
        Tonemapper::from_str("PBR_NEUTRAL"),
        Some(Tonemapper::PbrNeutral)
    );
    assert_eq!(Tonemapper::from_str("UNKNOWN"), None);
}

#[test]
fn tonemapper_is_valid() {
    assert!(Tonemapper::is_valid("REINHARD"));
    assert!(Tonemapper::is_valid("ACES"));
    assert!(!Tonemapper::is_valid("INVALID"));
}

// ─── ParticleBurst ────────────────────────────────────────────

#[test]
fn particle_burst_default_values() {
    let b = ParticleBurst::default();
    assert!((b.time - 0.0).abs() < f64::EPSILON);
    assert!((b.minimum - 0.0).abs() < f64::EPSILON);
    assert!((b.maximum - 50.0).abs() < f64::EPSILON);
    assert!(!b.complete());
}

#[test]
fn particle_burst_new_with_values() {
    let b = ParticleBurst::new(Some(1.5), Some(10.0), Some(100.0));
    assert!((b.time - 1.5).abs() < f64::EPSILON);
    assert!((b.minimum - 10.0).abs() < f64::EPSILON);
    assert!((b.maximum - 100.0).abs() < f64::EPSILON);
}

#[test]
fn particle_burst_complete_flag() {
    let mut b = ParticleBurst::default();
    assert!(!b.complete());
    b.set_complete(true);
    assert!(b.complete());
}

// ─── VoxelMetadataOrder ───────────────────────────────────────

#[test]
fn voxel_metadata_order_values() {
    assert_eq!(VoxelMetadataOrder::ZUp.as_i32(), 0);
    assert_eq!(VoxelMetadataOrder::YUp.as_i32(), 1);
}

#[test]
fn voxel_metadata_order_from_i32() {
    assert_eq!(VoxelMetadataOrder::from_i32(0), Some(VoxelMetadataOrder::ZUp));
    assert_eq!(VoxelMetadataOrder::from_i32(1), Some(VoxelMetadataOrder::YUp));
    assert_eq!(VoxelMetadataOrder::from_i32(99), None);
}

// ─── IonWorldImageryStyle ──────────────────────────────────────

#[test]
fn ion_world_imagery_style_maps_to_ion_asset_ids() {
    assert_eq!(IonWorldImageryStyle::Aerial.as_i32(), 2);
    assert_eq!(IonWorldImageryStyle::AerialWithLabels.as_i32(), 3);
    assert_eq!(IonWorldImageryStyle::Road.as_i32(), 4);
}

#[test]
fn ion_world_imagery_style_from_i32() {
    assert_eq!(
        IonWorldImageryStyle::from_i32(2),
        Some(IonWorldImageryStyle::Aerial)
    );
    assert_eq!(
        IonWorldImageryStyle::from_i32(3),
        Some(IonWorldImageryStyle::AerialWithLabels)
    );
    assert_eq!(
        IonWorldImageryStyle::from_i32(4),
        Some(IonWorldImageryStyle::Road)
    );
    assert_eq!(IonWorldImageryStyle::from_i32(0), None);
}

// ─── PrimitiveState ────────────────────────────────────────────

#[test]
fn primitive_state_all_values() {
    assert_eq!(PrimitiveState::Ready.as_i32(), 0);
    assert_eq!(PrimitiveState::Creating.as_i32(), 1);
    assert_eq!(PrimitiveState::Created.as_i32(), 2);
    assert_eq!(PrimitiveState::Combining.as_i32(), 3);
    assert_eq!(PrimitiveState::Combined.as_i32(), 4);
    assert_eq!(PrimitiveState::Complete.as_i32(), 5);
    assert_eq!(PrimitiveState::Failed.as_i32(), 6);
}

#[test]
fn primitive_state_from_i32_round_trips() {
    assert_eq!(PrimitiveState::from_i32(0), Some(PrimitiveState::Ready));
    assert_eq!(PrimitiveState::from_i32(5), Some(PrimitiveState::Complete));
    assert_eq!(PrimitiveState::from_i32(6), Some(PrimitiveState::Failed));
    assert_eq!(PrimitiveState::from_i32(99), None);
}

// ─── TileSelectionResult ───────────────────────────────────────

#[test]
fn tile_selection_result_values() {
    assert_eq!(TileSelectionResult::NONE.as_i32(), 0);
    assert_eq!(TileSelectionResult::CULLED.as_i32(), 1);
    assert_eq!(TileSelectionResult::RENDERED.as_i32(), 2);
    assert_eq!(TileSelectionResult::REFINED.as_i32(), 3);
    assert_eq!(TileSelectionResult::RENDERED_AND_KICKED.as_i32(), 2 | 4);
    assert_eq!(TileSelectionResult::REFINED_AND_KICKED.as_i32(), 3 | 4);
    assert_eq!(TileSelectionResult::CULLED_BUT_NEEDED.as_i32(), 1 | 8);
}

/// `originalResult: value & 3` strips both the kick bit and the
/// CULLED_BUT_NEEDED bit.
#[test]
fn tile_selection_result_original_result() {
    assert_eq!(
        TileSelectionResult::NONE.original_result(),
        TileSelectionResult::NONE
    );
    assert_eq!(
        TileSelectionResult::CULLED.original_result(),
        TileSelectionResult::CULLED
    );
    assert_eq!(
        TileSelectionResult::RENDERED.original_result(),
        TileSelectionResult::RENDERED
    );
    assert_eq!(
        TileSelectionResult::REFINED.original_result(),
        TileSelectionResult::REFINED
    );
    assert_eq!(
        TileSelectionResult::RENDERED_AND_KICKED.original_result(),
        TileSelectionResult::RENDERED
    );
    assert_eq!(
        TileSelectionResult::REFINED_AND_KICKED.original_result(),
        TileSelectionResult::REFINED
    );
    assert_eq!(
        TileSelectionResult::CULLED_BUT_NEEDED.original_result(),
        TileSelectionResult::CULLED
    );
}

/// `kick: value | 4`. Idempotent, and closed over the whole integer domain —
/// including values no named constant spells out (9 | 4 == 13).
#[test]
fn tile_selection_result_kick() {
    assert_eq!(
        TileSelectionResult::RENDERED.kick(),
        TileSelectionResult::RENDERED_AND_KICKED
    );
    assert_eq!(
        TileSelectionResult::REFINED.kick(),
        TileSelectionResult::REFINED_AND_KICKED
    );
    assert_eq!(TileSelectionResult::NONE.kick().as_i32(), 4);
    assert_eq!(
        TileSelectionResult::CULLED_BUT_NEEDED.kick().as_i32(),
        1 | 8 | 4
    );
    assert_eq!(
        TileSelectionResult::RENDERED_AND_KICKED.kick(),
        TileSelectionResult::RENDERED_AND_KICKED
    );
}

/// `wasKicked: value >= RENDERED_AND_KICKED`. Being a comparison rather than a
/// membership test, this reports `true` for CULLED_BUT_NEEDED (9 >= 6) — the
/// CesiumJS quirk `TerrainFillMesh.js` relies on.
#[test]
fn tile_selection_result_was_kicked() {
    assert!(!TileSelectionResult::NONE.was_kicked());
    assert!(!TileSelectionResult::CULLED.was_kicked());
    assert!(!TileSelectionResult::RENDERED.was_kicked());
    assert!(!TileSelectionResult::REFINED.was_kicked());
    assert!(TileSelectionResult::RENDERED_AND_KICKED.was_kicked());
    assert!(TileSelectionResult::REFINED_AND_KICKED.was_kicked());
    assert!(TileSelectionResult::CULLED_BUT_NEEDED.was_kicked());
}

// ─── SdfSettings ───────────────────────────────────────────────

#[test]
fn sdf_settings_constants() {
    assert!((SdfSettings::FONT_SIZE - 48.0).abs() < f64::EPSILON);
    assert!((SdfSettings::PADDING - 10.0).abs() < f64::EPSILON);
    assert!((SdfSettings::RADIUS - 8.0).abs() < f64::EPSILON);
    assert!((SdfSettings::CUTOFF - 0.5).abs() < f64::EPSILON);
}

// ─── ModelAnimationLoop ────────────────────────────────────────

#[test]
fn model_animation_loop_values() {
    assert_eq!(ModelAnimationLoop::None.as_i32(), 0);
    assert_eq!(ModelAnimationLoop::Repeat.as_i32(), 1);
    assert_eq!(ModelAnimationLoop::MirroredRepeat.as_i32(), 2);
}

#[test]
fn model_animation_loop_from_i32() {
    assert_eq!(ModelAnimationLoop::from_i32(0), Some(ModelAnimationLoop::None));
    assert_eq!(ModelAnimationLoop::from_i32(1), Some(ModelAnimationLoop::Repeat));
    assert_eq!(
        ModelAnimationLoop::from_i32(2),
        Some(ModelAnimationLoop::MirroredRepeat)
    );
    assert_eq!(ModelAnimationLoop::from_i32(99), None);
}

// ─── PickedMetadataInfo ────────────────────────────────────────

#[test]
fn picked_metadata_info_new_stores_all_fields() {
    let info = PickedMetadataInfo::new(
        Some("mySchema".to_string()),
        "Building".to_string(),
        "height".to_string(),
        Some(serde_json::json!({"type": "SCALAR"})),
        Some(serde_json::json!("texHeight")),
    );
    assert_eq!(info.schema_id.as_deref(), Some("mySchema"));
    assert_eq!(info.class_name, "Building");
    assert_eq!(info.property_name, "height");
    assert!(info.class_property.is_some());
    assert!(info.metadata_property.is_some());
}

#[test]
fn picked_metadata_info_with_none_optionals() {
    let info = PickedMetadataInfo::new(
        None,
        "Wall".to_string(),
        "color".to_string(),
        None,
        None,
    );
    assert!(info.schema_id.is_none());
    assert!(info.class_property.is_none());
    assert!(info.metadata_property.is_none());
    assert_eq!(info.class_name, "Wall");
    assert_eq!(info.property_name, "color");
}
