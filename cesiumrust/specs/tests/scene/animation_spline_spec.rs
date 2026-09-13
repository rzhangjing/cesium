//! Scene/ModelAnimation + CameraFlightPath → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/Model/ModelAnimation.js (animation state machine, spline evaluation)
//! - Scene/Camera.js flyTo (flight path interpolation)
//!
//! A-class tests: RuntimeAnimation state machine (play/pause/stop/advance/loop),
//! AnimationSpline evaluation (Step/Linear/CubicSpline/QuaternionSlerp),
//! CameraFlight update interpolation.
//! C-class omitted: WebGL rendering, actual model loading, Scene integration.

use cesium_gltf::animation_runtime::{
    AnimationLoop, AnimationSpline, AnimationState, CubicSpline, LinearSpline,
    QuaternionSpline, RuntimeAnimation, StepSpline,
};
use cesium_gltf::{Animation, AnimationPath, Interpolation};
use cesium_interaction::flight::CameraFlight;
use cesium_camera::Camera;
use glam::DVec3;

// === RuntimeAnimation State Machine ===

fn make_animation(duration: f64) -> RuntimeAnimation {
    let anim = Animation {
        name: Some("test".to_string()),
        channels: vec![],
        samplers: vec![],
    };
    RuntimeAnimation::from_gltf(&anim, duration)
}

#[test]
fn runtime_animation_initial_state() {
    let anim = make_animation(2.0);
    assert_eq!(anim.state, AnimationState::Stopped);
    assert_eq!(anim.local_time, 0.0);
    assert_eq!(anim.duration, 2.0);
    assert_eq!(anim.multiplier, 1.0);
    assert!(!anim.reverse);
}

#[test]
fn runtime_animation_play() {
    let mut anim = make_animation(2.0);
    anim.play();
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_pause() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.pause();
    assert_eq!(anim.state, AnimationState::Paused);
}

#[test]
fn runtime_animation_stop_resets_time() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.advance(1.0);
    anim.stop();
    assert_eq!(anim.state, AnimationState::Stopped);
    assert_eq!(anim.local_time, 0.0);
}

#[test]
fn runtime_animation_advance_playing() {
    let mut anim = make_animation(2.0);
    anim.play();
    let active = anim.advance(0.5);
    assert!(active);
    assert!((anim.local_time - 0.5).abs() < 1e-10);
}

#[test]
fn runtime_animation_advance_stopped_noop() {
    let mut anim = make_animation(2.0);
    // Not playing - advance returns false
    let active = anim.advance(0.5);
    assert!(!active);
    assert_eq!(anim.local_time, 0.0);
}

#[test]
fn runtime_animation_advance_with_multiplier() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.multiplier = 2.0;
    anim.advance(0.5);
    assert!((anim.local_time - 1.0).abs() < 1e-10);
}

#[test]
fn runtime_animation_advance_reverse() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.local_time = 1.0;
    anim.reverse = true;
    anim.advance(0.5);
    assert!((anim.local_time - 0.5).abs() < 1e-10);
}

#[test]
fn runtime_animation_loop_none_stops_at_end() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.loop_mode = AnimationLoop::None;
    let active = anim.advance(3.0);
    assert!(!active);
    assert_eq!(anim.state, AnimationState::Stopped);
    assert!((anim.local_time - 2.0).abs() < 1e-10);
}

#[test]
fn runtime_animation_loop_repeat_wraps() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.loop_mode = AnimationLoop::Repeat;
    anim.advance(3.0);
    // 3.0 % 2.0 = 1.0
    assert!((anim.local_time - 1.0).abs() < 1e-10);
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_loop_mirrored_repeat() {
    let mut anim = make_animation(2.0);
    anim.play();
    anim.loop_mode = AnimationLoop::MirroredRepeat;
    anim.advance(3.0);
    // cycle = 4.0, t = 3.0 % 4.0 = 3.0, > duration(2.0) -> 4.0 - 3.0 = 1.0
    assert!((anim.local_time - 1.0).abs() < 1e-10);
}

#[test]
fn runtime_animation_effective_time_clamped() {
    let mut anim = make_animation(2.0);
    anim.clamp_animations = true;
    anim.local_time = 5.0;
    assert!((anim.effective_time() - 2.0).abs() < 1e-10);
}

#[test]
fn runtime_animation_effective_time_wrapped() {
    let mut anim = make_animation(2.0);
    anim.clamp_animations = false;
    anim.local_time = 5.0;
    // 5.0 % 2.0 = 1.0
    assert!((anim.effective_time() - 1.0).abs() < 1e-10);
}

// === AnimationSpline: Step ===

#[test]
fn step_spline_holds_previous_value() {
    let spline = AnimationSpline::Step(StepSpline {
        times: vec![0.0, 1.0, 2.0],
        values: vec![0.0, 10.0, 20.0],
        components: 1,
    });
    assert_eq!(spline.evaluate(0.0), vec![0.0]);
    assert_eq!(spline.evaluate(0.5), vec![0.0]); // holds first
    assert_eq!(spline.evaluate(1.0), vec![10.0]);
    assert_eq!(spline.evaluate(1.5), vec![10.0]); // holds second
    assert_eq!(spline.evaluate(2.0), vec![20.0]);
}

#[test]
fn step_spline_vec3() {
    let spline = AnimationSpline::Step(StepSpline {
        times: vec![0.0, 1.0],
        values: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        components: 3,
    });
    assert_eq!(spline.evaluate(0.5), vec![1.0, 2.0, 3.0]);
    assert_eq!(spline.evaluate(1.0), vec![4.0, 5.0, 6.0]);
}

// === AnimationSpline: Linear ===

#[test]
fn linear_spline_interpolates() {
    let spline = AnimationSpline::Linear(LinearSpline {
        times: vec![0.0, 1.0, 2.0],
        values: vec![0.0, 10.0, 20.0],
        components: 1,
    });
    assert_eq!(spline.evaluate(0.0), vec![0.0]);
    assert_eq!(spline.evaluate(0.5), vec![5.0]);
    assert_eq!(spline.evaluate(1.0), vec![10.0]);
    assert_eq!(spline.evaluate(1.5), vec![15.0]);
    assert_eq!(spline.evaluate(2.0), vec![20.0]);
}

#[test]
fn linear_spline_vec3() {
    let spline = AnimationSpline::Linear(LinearSpline {
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 10.0, 20.0, 30.0],
        components: 3,
    });
    let result = spline.evaluate(0.5);
    assert!((result[0] - 5.0).abs() < 1e-10);
    assert!((result[1] - 10.0).abs() < 1e-10);
    assert!((result[2] - 15.0).abs() < 1e-10);
}

// === AnimationSpline: QuaternionSlerp ===

#[test]
fn quaternion_slerp_identity_to_90_deg() {
    // Slerp from identity [0,0,0,1] to 90° around Z [0,0,sin(45°),cos(45°)]
    let sin45 = std::f64::consts::FRAC_1_SQRT_2;
    let cos45 = std::f64::consts::FRAC_1_SQRT_2;
    let spline = AnimationSpline::QuaternionSlerp(QuaternionSpline {
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, sin45, cos45],
    });

    // At t=0: identity
    let r0 = spline.evaluate(0.0);
    assert!((r0[3] - 1.0).abs() < 1e-10);

    // At t=1: 90° around Z
    let r1 = spline.evaluate(1.0);
    assert!((r1[2] - sin45).abs() < 1e-10);
    assert!((r1[3] - cos45).abs() < 1e-10);

    // At t=0.5: 45° around Z -> [0, 0, sin(22.5°), cos(22.5°)]
    let r_mid = spline.evaluate(0.5);
    let sin22_5 = (std::f64::consts::FRAC_PI_4 / 2.0).sin();
    let cos22_5 = (std::f64::consts::FRAC_PI_4 / 2.0).cos();
    assert!((r_mid[2] - sin22_5).abs() < 1e-6);
    assert!((r_mid[3] - cos22_5).abs() < 1e-6);
}

// === AnimationSpline: CubicSpline ===

#[test]
fn cubic_spline_evaluates_at_keyframes() {
    // CubicSpline with 2 keyframes, 1 component
    // Data layout per keyframe: [inTangent, value, outTangent]
    let spline = AnimationSpline::CubicSpline(CubicSpline {
        times: vec![0.0, 1.0],
        values: vec![0.0, 10.0], // values at keyframes
        in_tangents: vec![0.0],   // in-tangent at keyframe 1
        out_tangents: vec![0.0],  // out-tangent at keyframe 0
        components: 1,
    });

    // At keyframes, should return exact values
    let r0 = spline.evaluate(0.0);
    assert!((r0[0] - 0.0).abs() < 1e-10);
    let r1 = spline.evaluate(1.0);
    assert!((r1[0] - 10.0).abs() < 1e-10);
}

// === AnimationSpline: from_keyframes ===

#[test]
fn from_keyframes_single_keyframe_constant() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0],
        vec![5.0, 10.0, 15.0],
        Interpolation::Linear,
        AnimationPath::Translation,
        3,
    );
    // Single keyframe -> constant
    assert_eq!(spline.evaluate(0.0), vec![5.0, 10.0, 15.0]);
    assert_eq!(spline.evaluate(100.0), vec![5.0, 10.0, 15.0]);
}

#[test]
fn from_keyframes_step_interpolation() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0, 1.0],
        vec![0.0, 10.0],
        Interpolation::Step,
        AnimationPath::Translation,
        1,
    );
    assert!(matches!(spline, AnimationSpline::Step(_)));
    assert_eq!(spline.evaluate(0.5), vec![0.0]);
}

#[test]
fn from_keyframes_linear_rotation_uses_slerp() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.707, 0.707],
        Interpolation::Linear,
        AnimationPath::Rotation,
        4,
    );
    assert!(matches!(spline, AnimationSpline::QuaternionSlerp(_)));
}

#[test]
fn from_keyframes_linear_translation_uses_linear() {
    let spline = AnimationSpline::from_keyframes(
        vec![0.0, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0],
        Interpolation::Linear,
        AnimationPath::Translation,
        3,
    );
    assert!(matches!(spline, AnimationSpline::Linear(_)));
}

// === AnimationSpline: clamp_time / wrap_time ===

#[test]
fn spline_clamp_time() {
    let spline = AnimationSpline::Linear(LinearSpline {
        times: vec![0.0, 2.0],
        values: vec![0.0, 10.0],
        components: 1,
    });
    assert!((spline.clamp_time(-1.0) - 0.0).abs() < 1e-10);
    assert!((spline.clamp_time(1.0) - 1.0).abs() < 1e-10);
    assert!((spline.clamp_time(5.0) - 2.0).abs() < 1e-10);
}

#[test]
fn spline_wrap_time() {
    let spline = AnimationSpline::Linear(LinearSpline {
        times: vec![0.0, 2.0],
        values: vec![0.0, 10.0],
        components: 1,
    });
    assert!((spline.wrap_time(3.0) - 1.0).abs() < 1e-10);
    assert!((spline.wrap_time(4.0) - 0.0).abs() < 1e-10);
    assert!((spline.wrap_time(1.0) - 1.0).abs() < 1e-10);
}

// === CameraFlight ===

fn make_camera() -> Camera {
    Camera::new(
        DVec3::new(0.0, 0.0, 10000.0),
        DVec3::new(0.0, 0.0, -1.0),
        DVec3::new(0.0, 1.0, 0.0),
    )
}

#[test]
fn camera_flight_initial_progress_zero() {
    let camera = make_camera();
    let flight = CameraFlight::fly_to(
        &camera,
        DVec3::new(0.0, 0.0, 5000.0),
        None,
        None,
        2.0,
    );
    assert!((flight.progress() - 0.0).abs() < 1e-10);
    assert!(!flight.complete);
}

#[test]
fn camera_flight_update_interpolates_position() {
    let camera = make_camera();
    let mut flight = CameraFlight::fly_to(
        &camera,
        DVec3::new(0.0, 0.0, 0.0),
        Some(DVec3::new(0.0, 0.0, -1.0)),
        Some(DVec3::new(0.0, 1.0, 0.0)),
        2.0,
    );

    // After 1 second (half duration), position should be interpolated
    let result = flight.update(1.0);
    assert!(result.is_some());
    let (pos, _dir, _up) = result.unwrap();
    // With sinusoidal easing at t=0.5: eased = 0.5
    // position = lerp(10000, 0, 0.5) = 5000
    assert!((pos.z - 5000.0).abs() < 100.0); // allow easing tolerance
}

#[test]
fn camera_flight_completes_at_duration() {
    let camera = make_camera();
    let mut flight = CameraFlight::fly_to(
        &camera,
        DVec3::new(0.0, 0.0, 5000.0),
        None,
        None,
        1.0,
    );

    flight.update(0.5);
    assert!(!flight.complete);

    flight.update(0.5);
    assert!(flight.complete);
    assert!((flight.progress() - 1.0).abs() < 1e-10);
}

#[test]
fn camera_flight_returns_none_after_complete() {
    let camera = make_camera();
    let mut flight = CameraFlight::fly_to(
        &camera,
        DVec3::new(0.0, 0.0, 5000.0),
        None,
        None,
        1.0,
    );

    flight.update(2.0); // overshoot
    assert!(flight.complete);
    assert!(flight.update(0.1).is_none());
}

#[test]
fn camera_flight_end_position_reached() {
    let camera = make_camera();
    let destination = DVec3::new(1000.0, 2000.0, 3000.0);
    let mut flight = CameraFlight::fly_to(
        &camera,
        destination,
        Some(DVec3::new(0.0, 0.0, -1.0)),
        Some(DVec3::new(0.0, 1.0, 0.0)),
        1.0,
    );

    // Advance to completion
    let result = flight.update(1.0);
    assert!(result.is_some());
    let (pos, _dir, _up) = result.unwrap();
    assert!((pos.x - destination.x).abs() < 1e-6);
    assert!((pos.y - destination.y).abs() < 1e-6);
    assert!((pos.z - destination.z).abs() < 1e-6);
}
