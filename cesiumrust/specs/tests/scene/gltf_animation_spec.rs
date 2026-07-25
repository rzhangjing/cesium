//! GltfAnimation specs - ported from Scene/ModelAnimationSpec, GltfLoaderSpec
//! Covers: AnimationState, AnimationLoop, MorphTargetBlender, CustomShader

use cesium_gltf::animation_runtime::{AnimationLoop, AnimationState, MorphTargetBlender};
use cesium_gltf::custom_shader::{CustomShader, CustomShaderMode, UniformType};

// ─── AnimationState ─────────────────────────────────────────────────────────

#[test]
fn animation_state_default() {
    let state = AnimationState::default();
    assert_eq!(state, AnimationState::Stopped);
}

#[test]
fn animation_state_variants() {
    assert_ne!(AnimationState::Playing, AnimationState::Paused);
    assert_ne!(AnimationState::Paused, AnimationState::Stopped);
}

// ─── AnimationLoop ──────────────────────────────────────────────────────────

#[test]
fn animation_loop_default() {
    let loop_mode = AnimationLoop::default();
    assert_eq!(loop_mode, AnimationLoop::None);
}

#[test]
fn animation_loop_variants() {
    assert_ne!(AnimationLoop::None, AnimationLoop::Repeat);
    assert_ne!(AnimationLoop::Repeat, AnimationLoop::MirroredRepeat);
}

// ─── MorphTargetBlender ─────────────────────────────────────────────────────

#[test]
fn morph_target_blender_default() {
    let blender = MorphTargetBlender::default();
    assert!(blender.weights.is_empty());
}

#[test]
fn morph_target_blender_set_weights() {
    let mut blender = MorphTargetBlender::default();
    blender.weights = vec![0.5, 0.3, 0.0];
    assert_eq!(blender.weights.len(), 3);
    assert!((blender.weights[0] - 0.5).abs() < 1e-10);
}

// ─── CustomShader ───────────────────────────────────────────────────────────

#[test]
fn custom_shader_default() {
    let shader = CustomShader::default();
    assert_eq!(shader.mode, CustomShaderMode::ModifyMaterial);
}

#[test]
fn custom_shader_mode_variants() {
    assert_ne!(CustomShaderMode::ModifyMaterial, CustomShaderMode::ReplaceMaterial);
}

#[test]
fn uniform_type_variants() {
    assert_ne!(UniformType::Float, UniformType::Vec2);
    assert_ne!(UniformType::Vec2, UniformType::Vec3);
    assert_ne!(UniformType::Vec3, UniformType::Vec4);
    assert_ne!(UniformType::Vec4, UniformType::Mat4);
}
