//! Scene/ModelAnimation + CameraFlightPath → Rust 集成测试。
//!
//! 参考 CesiumJS：
//! - Scene/Model/ModelAnimation（动画状态机、样条求值）
//! - Scene/Camera flyTo（飞行路径插值）
//!
//! A 类测试：RuntimeAnimation 状态机（play/pause/stop/advance/loop），
//! AnimationSpline 求值（Step/Linear/CubicSpline/QuaternionSlerp），
//! CameraFlight update 插值。
//! C 类省略：WebGL 渲染、实际模型加载、Scene 集成。

use cesium_gltf::animation_runtime::{
    AnimationLoop, AnimationSpline, AnimationState, CubicSpline, LinearSpline,
    QuaternionSpline, RuntimeAnimation, StepSpline,
};
use cesium_gltf::{Animation, AnimationPath, Interpolation};
use cesium_interaction::flight::CameraFlight;
use cesium_camera::Camera;
use glam::DVec3;

// === RuntimeAnimation 状态机 ===

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
    // 未播放 - advance 返回 false
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
    assert_eq!(spline.evaluate(0.5), vec![0.0]); // 保持第一个
    assert_eq!(spline.evaluate(1.0), vec![10.0]);
    assert_eq!(spline.evaluate(1.5), vec![10.0]); // 保持第二个
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
    // Slerp 从 identity [0,0,0,1] 到 绕 Z 90° [0,0,sin(45°),cos(45°)]
    let sin45 = std::f64::consts::FRAC_1_SQRT_2;
    let cos45 = std::f64::consts::FRAC_1_SQRT_2;
    let spline = AnimationSpline::QuaternionSlerp(QuaternionSpline {
        times: vec![0.0, 1.0],
        values: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, sin45, cos45],
    });

    // 在 t=0：identity
    let r0 = spline.evaluate(0.0);
    assert!((r0[3] - 1.0).abs() < 1e-10);

    // 在 t=1：绕 Z 90°
    let r1 = spline.evaluate(1.0);
    assert!((r1[2] - sin45).abs() < 1e-10);
    assert!((r1[3] - cos45).abs() < 1e-10);

    // 在 t=0.5：绕 Z 45° -> [0, 0, sin(22.5°), cos(22.5°)]
    let r_mid = spline.evaluate(0.5);
    let sin22_5 = (std::f64::consts::FRAC_PI_4 / 2.0).sin();
    let cos22_5 = (std::f64::consts::FRAC_PI_4 / 2.0).cos();
    assert!((r_mid[2] - sin22_5).abs() < 1e-6);
    assert!((r_mid[3] - cos22_5).abs() < 1e-6);
}

// === AnimationSpline: CubicSpline ===

#[test]
fn cubic_spline_evaluates_at_keyframes() {
    // CubicSpline，含 2 个关键帧、1 个分量
    // 每个关键帧的数据布局：[inTangent, value, outTangent]
    let spline = AnimationSpline::CubicSpline(CubicSpline {
        times: vec![0.0, 1.0],
        values: vec![0.0, 10.0], // 关键帧处的值
        in_tangents: vec![0.0],   // 关键帧 1 的 in-tangent
        out_tangents: vec![0.0],  // 关键帧 0 的 out-tangent
        components: 1,
    });

    // 在关键帧处，应返回精确值
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
    // 单个关键帧 -> 常量
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

    // 1 秒后（时长的一半），position 应被插值
    let result = flight.update(1.0);
    assert!(result.is_some());
    let (pos, _dir, _up) = result.unwrap();
    // 使用正弦缓动，在 t=0.5：eased = 0.5
    // position = lerp(10000, 0, 0.5) = 5000
    assert!((pos.z - 5000.0).abs() < 100.0); // 允许缓动容差
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

    flight.update(2.0); // 过冲
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

    // 推进至完成
    let result = flight.update(1.0);
    assert!(result.is_some());
    let (pos, _dir, _up) = result.unwrap();
    assert!((pos.x - destination.x).abs() < 1e-6);
    assert!((pos.y - destination.y).abs() < 1e-6);
    assert!((pos.z - destination.z).abs() < 1e-6);
}
