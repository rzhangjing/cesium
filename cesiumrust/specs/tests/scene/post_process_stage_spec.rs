//! PostProcessStage/Composite/Collection specs
//! Ported from CesiumJS Scene/PostProcessStageSpec.js + PostProcessStageCollectionSpec.js

use cesium_effects::{
    create_ambient_occlusion_composite, create_auto_exposure_stage, create_bloom_composite,
    create_fxaa_stage, PixelFormat, PostProcessStage, PostProcessStageCollection,
    PostProcessStageComposite, SampleMode, StageRef, Tonemapper, UniformValue,
};

// ==================== PostProcessStage ====================

#[test]
fn stage_new_defaults() {
    let stage = PostProcessStage::new("test_stage", "void main() { out_FragColor = vec4(1.0); }");
    assert_eq!(stage.name, "test_stage");
    assert!(stage.enabled);
    assert!(!stage.ready);
    assert!((stage.texture_scale - 1.0).abs() < 1e-10);
    assert!(!stage.force_power_of_two);
    assert_eq!(stage.sample_mode, SampleMode::Nearest);
    assert_eq!(stage.pixel_format, PixelFormat::Rgba8);
    assert_eq!(stage.clear_color, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn stage_set_get_uniform() {
    let mut stage = PostProcessStage::new("test", "");
    stage.set_uniform("intensity", UniformValue::Float(2.5));
    stage.set_uniform("color", UniformValue::Vec4([1.0, 0.0, 0.0, 1.0]));
    stage.set_uniform("count", UniformValue::Int(16));
    stage.set_uniform("enabled", UniformValue::Bool(true));
    stage.set_uniform("texture", UniformValue::Texture("noise.png".to_string()));

    assert_eq!(stage.get_uniform("intensity"), Some(&UniformValue::Float(2.5)));
    assert_eq!(
        stage.get_uniform("color"),
        Some(&UniformValue::Vec4([1.0, 0.0, 0.0, 1.0]))
    );
    assert_eq!(stage.get_uniform("count"), Some(&UniformValue::Int(16)));
    assert_eq!(stage.get_uniform("enabled"), Some(&UniformValue::Bool(true)));
    assert_eq!(
        stage.get_uniform("texture"),
        Some(&UniformValue::Texture("noise.png".to_string()))
    );
    assert_eq!(stage.get_uniform("nonexistent"), None);
}

#[test]
fn stage_uniform_overwrite() {
    let mut stage = PostProcessStage::new("test", "");
    stage.set_uniform("value", UniformValue::Float(1.0));
    stage.set_uniform("value", UniformValue::Float(2.0));
    assert_eq!(stage.get_uniform("value"), Some(&UniformValue::Float(2.0)));
}

#[test]
fn stage_output_dimensions_scale() {
    let stage = PostProcessStage {
        texture_scale: 0.5,
        ..PostProcessStage::new("test", "")
    };
    let (w, h) = stage.output_dimensions(1920, 1080);
    assert_eq!(w, 960);
    assert_eq!(h, 540);
}

#[test]
fn stage_output_dimensions_full_scale() {
    let stage = PostProcessStage::new("test", "");
    let (w, h) = stage.output_dimensions(800, 600);
    assert_eq!(w, 800);
    assert_eq!(h, 600);
}

#[test]
fn stage_output_dimensions_power_of_two() {
    let stage = PostProcessStage {
        force_power_of_two: true,
        ..PostProcessStage::new("test", "")
    };
    let (w, h) = stage.output_dimensions(1920, 1080);
    // min(1920, 1080) = 1080, next_power_of_two(1080) = 2048
    assert_eq!(w, 2048);
    assert_eq!(h, 2048);
}

#[test]
fn stage_output_dimensions_minimum_one() {
    let stage = PostProcessStage {
        texture_scale: 0.001,
        ..PostProcessStage::new("test", "")
    };
    let (w, h) = stage.output_dimensions(100, 100);
    assert!(w >= 1);
    assert!(h >= 1);
}

// ==================== PostProcessStageComposite ====================

#[test]
fn composite_new_defaults() {
    let composite = PostProcessStageComposite::new("my_composite");
    assert_eq!(composite.name, "my_composite");
    assert!(composite.enabled);
    assert!(composite.is_empty());
    assert_eq!(composite.len(), 0);
    assert!(!composite.parallel);
}

#[test]
fn composite_add_stages() {
    let mut composite = PostProcessStageComposite::new("test");
    composite.add_stage(PostProcessStage::new("s1", "shader1"));
    composite.add_stage(PostProcessStage::new("s2", "shader2"));
    composite.add_stage(PostProcessStage::new("s3", "shader3"));

    assert_eq!(composite.len(), 3);
    assert!(!composite.is_empty());
    assert_eq!(composite.stages[0].name, "s1");
    assert_eq!(composite.stages[2].name, "s3");
}

#[test]
fn composite_is_ready() {
    let mut composite = PostProcessStageComposite::new("test");
    let mut s1 = PostProcessStage::new("s1", "");
    s1.ready = true;
    let s2 = PostProcessStage::new("s2", "");

    composite.add_stage(s1);
    composite.add_stage(s2);

    assert!(!composite.is_ready()); // s2 not ready

    composite.stages[1].ready = true;
    assert!(composite.is_ready());
}

#[test]
fn composite_empty_is_ready() {
    let composite = PostProcessStageComposite::new("empty");
    // Empty composite: all() on empty iterator returns true
    assert!(composite.is_ready());
}

// ==================== Built-in Stage Factories ====================

#[test]
fn fxaa_stage_defaults() {
    let fxaa = create_fxaa_stage();
    assert_eq!(fxaa.name, "czm_fxaa");
    assert!(!fxaa.enabled); // Disabled by default
    assert_eq!(fxaa.sample_mode, SampleMode::Linear);
    assert!(fxaa.fragment_shader.contains("fxaa"));
}

#[test]
fn bloom_composite_structure() {
    let bloom = create_bloom_composite();
    assert_eq!(bloom.name, "czm_bloom");
    assert!(!bloom.enabled);
    assert_eq!(bloom.len(), 2); // brightness + blur

    // Bright pass uniforms
    let bright = &bloom.stages[0];
    assert_eq!(bright.name, "czm_bloom_brightness");
    assert_eq!(bright.get_uniform("contrast"), Some(&UniformValue::Float(128.0)));
    assert_eq!(bright.get_uniform("brightness"), Some(&UniformValue::Float(-0.3)));
    assert_eq!(bright.get_uniform("glowOnly"), Some(&UniformValue::Bool(false)));

    // Blur pass uniforms
    let blur = &bloom.stages[1];
    assert_eq!(blur.name, "czm_bloom_blur");
    assert_eq!(blur.get_uniform("sigma"), Some(&UniformValue::Float(3.8)));
}

#[test]
fn ambient_occlusion_composite_structure() {
    let ao = create_ambient_occlusion_composite();
    assert_eq!(ao.name, "czm_ambient_occlusion");
    assert!(!ao.enabled);
    assert_eq!(ao.len(), 2); // generate + blur

    let gen = &ao.stages[0];
    assert_eq!(gen.name, "czm_ambient_occlusion_generate");
    assert_eq!(gen.get_uniform("intensity"), Some(&UniformValue::Float(3.0)));
    assert_eq!(gen.get_uniform("bias"), Some(&UniformValue::Float(0.1)));
    assert_eq!(gen.get_uniform("lengthCap"), Some(&UniformValue::Float(0.26)));
    assert_eq!(gen.get_uniform("directionCount"), Some(&UniformValue::Int(8)));
    assert_eq!(gen.get_uniform("stepCount"), Some(&UniformValue::Int(32)));
}

#[test]
fn auto_exposure_stage_defaults() {
    let stage = create_auto_exposure_stage();
    assert_eq!(stage.name, "czm_auto_exposure");
    assert!(!stage.enabled);
}

// ==================== Tonemapper ====================

#[test]
fn tonemapper_shader_functions() {
    assert_eq!(Tonemapper::PbrNeutral.shader_function(), "czm_pbrNeutralTonemap");
    assert_eq!(Tonemapper::AcesFilmic.shader_function(), "czm_acesFilmicTonemap");
    assert_eq!(Tonemapper::Reinhard.shader_function(), "czm_reinhardTonemap");
    assert_eq!(Tonemapper::None.shader_function(), "czm_noTonemap");
}

#[test]
fn tonemapper_default_is_pbr_neutral() {
    assert_eq!(Tonemapper::default(), Tonemapper::PbrNeutral);
}

// ==================== PostProcessStageCollection ====================

#[test]
fn collection_new_defaults() {
    let collection = PostProcessStageCollection::new();
    assert!(!collection.fxaa.enabled);
    assert!(!collection.ambient_occlusion.enabled);
    assert!(!collection.bloom.enabled);
    assert!(!collection.auto_exposure_enabled);
    assert!(!collection.tonemapping_enabled);
    assert!((collection.exposure - 1.0).abs() < 1e-10);
    assert_eq!(collection.tonemapper, Tonemapper::PbrNeutral);
    assert!(collection.is_empty());
}

#[test]
fn collection_add_and_get() {
    let mut collection = PostProcessStageCollection::new();
    let stage = PostProcessStage::new("my_effect", "void main() {}");
    let idx = collection.add(stage);

    assert_eq!(idx, 0);
    assert_eq!(collection.len(), 1);
    assert!(!collection.is_empty());

    let retrieved = collection.get_by_name("my_effect");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "my_effect");
    assert!(collection.get_by_name("nonexistent").is_none());
}

#[test]
fn collection_remove() {
    let mut collection = PostProcessStageCollection::new();
    collection.add(PostProcessStage::new("s1", ""));
    collection.add(PostProcessStage::new("s2", ""));
    collection.add(PostProcessStage::new("s3", ""));

    let removed = collection.remove("s2");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().name, "s2");
    assert_eq!(collection.len(), 2);
    assert!(collection.get_by_name("s2").is_none());
    // s1 and s3 still accessible
    assert!(collection.get_by_name("s1").is_some());
    assert!(collection.get_by_name("s3").is_some());
}

#[test]
fn collection_remove_nonexistent() {
    let mut collection = PostProcessStageCollection::new();
    collection.add(PostProcessStage::new("s1", ""));
    let removed = collection.remove("does_not_exist");
    assert!(removed.is_none());
    assert_eq!(collection.len(), 1);
}

#[test]
fn collection_execution_order_empty() {
    let collection = PostProcessStageCollection::new();
    let order = collection.execution_order();
    assert!(order.is_empty());
}

#[test]
fn collection_execution_order_full_pipeline() {
    let mut collection = PostProcessStageCollection::new();
    collection.ambient_occlusion.enabled = true;
    collection.bloom.enabled = true;
    collection.tonemapping_enabled = true;
    collection.fxaa.enabled = true;

    let user_stage = PostProcessStage::new("user_effect", "");
    collection.add(user_stage);

    let order = collection.execution_order();
    assert_eq!(order.len(), 5);
    assert_eq!(order[0], StageRef::AmbientOcclusion);
    assert_eq!(order[1], StageRef::Bloom);
    assert_eq!(order[2], StageRef::User(0));
    assert_eq!(order[3], StageRef::Tonemapping);
    assert_eq!(order[4], StageRef::Fxaa);
}

#[test]
fn collection_execution_order_skips_disabled_user() {
    let mut collection = PostProcessStageCollection::new();
    collection.tonemapping_enabled = true;

    let mut disabled = PostProcessStage::new("disabled_effect", "");
    disabled.enabled = false;
    collection.add(disabled);

    let enabled = PostProcessStage::new("enabled_effect", "");
    collection.add(enabled);

    let order = collection.execution_order();
    // Only enabled user stage + tonemapping
    assert_eq!(order.len(), 2);
    assert_eq!(order[0], StageRef::User(1)); // index 1 is the enabled one
    assert_eq!(order[1], StageRef::Tonemapping);
}

#[test]
fn collection_is_ready_tonemapping() {
    let mut collection = PostProcessStageCollection::new();
    assert!(!collection.is_ready());

    collection.tonemapping_enabled = true;
    assert!(collection.is_ready());
}

#[test]
fn collection_is_ready_user_stage() {
    let mut collection = PostProcessStageCollection::new();
    let mut stage = PostProcessStage::new("ready_stage", "");
    stage.ready = true;
    stage.enabled = true;
    collection.add(stage);

    assert!(collection.is_ready());
}

#[test]
fn collection_get_by_name_mut() {
    let mut collection = PostProcessStageCollection::new();
    collection.add(PostProcessStage::new("editable", ""));

    if let Some(stage) = collection.get_by_name_mut("editable") {
        stage.enabled = false;
        stage.set_uniform("new_uniform", UniformValue::Float(42.0));
    }

    let stage = collection.get_by_name("editable").unwrap();
    assert!(!stage.enabled);
    assert_eq!(stage.get_uniform("new_uniform"), Some(&UniformValue::Float(42.0)));
}
