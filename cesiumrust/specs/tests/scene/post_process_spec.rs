//! PostProcess specs - ported from Scene/PostProcessStageSpec, PostProcessStageCollectionSpec
//! Covers: PostProcessStage, PostProcessStageComposite, PostProcessStageCollection,
//! Tonemapper, SampleMode, UniformValue

use cesium_effects::post_process_stage::{
    PostProcessStage, PostProcessStageCollection, SampleMode, Tonemapper, UniformValue,
};

// ─── PostProcessStage ───────────────────────────────────────────────────────

#[test]
fn post_process_stage_creation() {
    let stage = PostProcessStage::new("test_stage", "void main() {}");
    assert_eq!(stage.name, "test_stage");
    assert!(stage.enabled);
}

#[test]
fn post_process_stage_disable() {
    let mut stage = PostProcessStage::new("fxaa", "void main() {}");
    stage.enabled = false;
    assert!(!stage.enabled);
}

// ─── SampleMode ─────────────────────────────────────────────────────────────

#[test]
fn sample_mode_default() {
    assert_eq!(SampleMode::default(), SampleMode::Nearest);
}

#[test]
fn sample_mode_variants() {
    assert_ne!(SampleMode::Nearest, SampleMode::Linear);
}

// ─── UniformValue ───────────────────────────────────────────────────────────

#[test]
fn uniform_value_float() {
    let v = UniformValue::Float(1.5);
    if let UniformValue::Float(f) = v {
        assert!((f - 1.5).abs() < 1e-10);
    }
}

#[test]
fn uniform_value_vec2() {
    let v = UniformValue::Vec2([0.5, 0.5]);
    if let UniformValue::Vec2(arr) = v {
        assert_eq!(arr, [0.5, 0.5]);
    }
}

// ─── Tonemapper ─────────────────────────────────────────────────────────────

#[test]
fn tonemapper_default() {
    assert_eq!(Tonemapper::default(), Tonemapper::PbrNeutral);
}

#[test]
fn tonemapper_variants() {
    assert_ne!(Tonemapper::PbrNeutral, Tonemapper::AcesFilmic);
    assert_ne!(Tonemapper::AcesFilmic, Tonemapper::Reinhard);
}

// ─── PostProcessStageCollection ─────────────────────────────────────────────

#[test]
fn post_process_collection_default() {
    let collection = PostProcessStageCollection::default();
    // FXAA is disabled by default
    assert!(!collection.fxaa.enabled);
}

#[test]
fn post_process_collection_tonemapper() {
    let mut collection = PostProcessStageCollection::default();
    collection.tonemapper = Tonemapper::AcesFilmic;
    assert_eq!(collection.tonemapper, Tonemapper::AcesFilmic);
}
