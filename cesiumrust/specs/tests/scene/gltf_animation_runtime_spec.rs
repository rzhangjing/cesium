//! Tests ported from CesiumJS ModelAnimationSpec.js + GltfLoaderSpec.js (A-class)
//! RuntimeAnimation state machine, AnimationSpline evaluation, loop modes

use cesium_gltf::animation_runtime::{
    AnimationLoop, AnimationSpline, AnimationState, RuntimeAnimation,
};
use cesium_gltf::gltf_model::{Animation, AnimationPath, Interpolation};

fn make_animation(name: &str) -> Animation {
    Animation {
        name: Some(name.to_string()),
        channels: vec![],
        samplers: vec![],
    }
}

// ===== RuntimeAnimation State Machine =====

#[test]
fn test_runtime_animation_from_gltf() {
    let anim = make_animation("walk");
    let rt = RuntimeAnimation::from_gltf(&anim, 2.5);
    assert_eq!(rt.name, Some("walk".to_string()));
    assert_eq!(rt.state, AnimationState::Stopped);
    assert_eq!(rt.loop_mode, AnimationLoop::None);
    assert!((rt.duration - 2.5).abs() < 1e-10);
    assert!((rt.multiplier - 1.0).abs() < 1e-10);
    assert!(!rt.reverse);
    assert!((rt.local_time - 0.0).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_play_pause_stop() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 1.0);

    rt.play();
    assert_eq!(rt.state, AnimationState::Playing);

    rt.pause();
    assert_eq!(rt.state, AnimationState::Paused);

    // Pause when not playing does nothing
    rt.pause();
    assert_eq!(rt.state, AnimationState::Paused);

    rt.stop();
    assert_eq!(rt.state, AnimationState::Stopped);
    assert!((rt.local_time - 0.0).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_advance_no_loop() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
    rt.play();

    assert!(rt.advance(0.5));
    assert!((rt.local_time - 0.5).abs() < 1e-10);

    assert!(rt.advance(1.0));
    assert!((rt.local_time - 1.5).abs() < 1e-10);

    // Exceeds duration → stops
    assert!(!rt.advance(1.0));
    assert_eq!(rt.state, AnimationState::Stopped);
    assert!((rt.local_time - 2.0).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_advance_repeat() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
    rt.play();
    rt.loop_mode = AnimationLoop::Repeat;

    assert!(rt.advance(2.5));
    // 2.5 % 2.0 = 0.5
    assert!((rt.local_time - 0.5).abs() < 1e-10);
    assert_eq!(rt.state, AnimationState::Playing);
}

#[test]
fn test_runtime_animation_advance_mirrored_repeat() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
    rt.play();
    rt.loop_mode = AnimationLoop::MirroredRepeat;

    // t=3.0: cycle=4.0, t%4=3.0, 3.0>2.0 → 4.0-3.0=1.0
    assert!(rt.advance(3.0));
    assert!((rt.local_time - 1.0).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_reverse() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
    rt.play();
    rt.reverse = true;
    rt.local_time = 1.0;

    assert!(rt.advance(0.5));
    assert!((rt.local_time - 0.5).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_multiplier() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 10.0);
    rt.play();
    rt.multiplier = 3.0;

    assert!(rt.advance(1.0));
    assert!((rt.local_time - 3.0).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_effective_time_clamped() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
    rt.clamp_animations = true;
    rt.local_time = 5.0;
    assert!((rt.effective_time() - 2.0).abs() < 1e-10);

    rt.local_time = -1.0;
    assert!((rt.effective_time() - 0.0).abs() < 1e-10);
}

#[test]
fn test_runtime_animation_effective_time_wrapped() {
    let anim = make_animation("test");
    let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
    rt.clamp_animations = false;
    rt.local_time = 5.0;
    // 5.0 % 2.0 = 1.0
    assert!((rt.effective_time() - 1.0).abs() < 1e-10);
}

// ===== AnimationSpline =====

#[test]
fn test_spline_constant_single_keyframe() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0],
        vec![1.0, 2.0, 3.0],
        Interpolation::Linear,
        AnimationPath::Translation,
        3,
    );
    let result = spline.evaluate(0.5);
    assert_eq!(result, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_spline_step_interpolation() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0, 1.0, 2.0],
        vec![0.0, 10.0, 20.0],
        Interpolation::Step,
        AnimationPath::Translation,
        1,
    );
    // Step holds value until next keyframe
    let result = spline.evaluate(0.5);
    assert!((result[0] - 0.0).abs() < 1e-10);

    let result = spline.evaluate(1.5);
    assert!((result[0] - 10.0).abs() < 1e-10);
}

#[test]
fn test_spline_linear_interpolation() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0, 1.0],
        vec![0.0, 0.0, 0.0, 10.0, 20.0, 30.0],
        Interpolation::Linear,
        AnimationPath::Translation,
        3,
    );
    let result = spline.evaluate(0.5);
    assert!((result[0] - 5.0).abs() < 1e-10);
    assert!((result[1] - 10.0).abs() < 1e-10);
    assert!((result[2] - 15.0).abs() < 1e-10);
}

#[test]
fn test_spline_clamp_time() {
    let spline = AnimationSpline::from_keyframes(
        vec![1.0, 3.0],
        vec![0.0, 10.0],
        Interpolation::Step,
        AnimationPath::Translation,
        1,
    );
    assert!((spline.clamp_time(0.0) - 1.0).abs() < 1e-10);
    assert!((spline.clamp_time(2.0) - 2.0).abs() < 1e-10);
    assert!((spline.clamp_time(5.0) - 3.0).abs() < 1e-10);
}

#[test]
fn test_spline_wrap_time() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0, 2.0],
        vec![0.0, 10.0],
        Interpolation::Step,
        AnimationPath::Translation,
        1,
    );
    // 3.0 wraps to 1.0 (duration=2.0)
    assert!((spline.wrap_time(3.0) - 1.0).abs() < 1e-10);
    // -0.5 wraps to 1.5
    assert!((spline.wrap_time(-0.5) - 1.5).abs() < 1e-10);
}
