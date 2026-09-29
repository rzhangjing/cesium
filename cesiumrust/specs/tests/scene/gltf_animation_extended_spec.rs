//! glTF 动画运行时扩展规范
//!
//! 测试 RuntimeAnimation 状态机：play/pause/stop/advance、
//! 倍率、反向、延迟、循环模式及 effective_time。

use cesium_gltf::animation_runtime::{AnimationLoop, AnimationState, RuntimeAnimation};

const EPSILON7: f64 = 1e-7;

fn make_animation() -> RuntimeAnimation {
    RuntimeAnimation {
        name: Some("test".to_string()),
        state: AnimationState::Stopped,
        loop_mode: AnimationLoop::None,
        multiplier: 1.0,
        reverse: false,
        local_time: 0.0,
        duration: 2.0,
        delay: 0.0,
        remove_on_stop: false,
        clamp_animations: true,
    }
}

// ─── RuntimeAnimation 状态机 ─────────────────────────────────────────

#[test]
fn runtime_animation_initial_state() {
    let anim = make_animation();
    assert_eq!(anim.state, AnimationState::Stopped);
    assert!((anim.local_time - 0.0).abs() < EPSILON7);
    assert!((anim.duration - 2.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_play_pause() {
    let mut anim = make_animation();
    anim.play();
    assert_eq!(anim.state, AnimationState::Playing);
    anim.pause();
    assert_eq!(anim.state, AnimationState::Paused);
}

#[test]
fn runtime_animation_stop_resets_time() {
    let mut anim = make_animation();
    anim.play();
    anim.advance(1.0);
    assert!(anim.local_time > 0.0);
    anim.stop();
    assert_eq!(anim.state, AnimationState::Stopped);
    assert!((anim.local_time - 0.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_advance_increments_time() {
    let mut anim = make_animation();
    anim.play();
    let still_playing = anim.advance(0.5);
    assert!(still_playing, "should still be playing");
    assert!((anim.local_time - 0.5).abs() < EPSILON7);
}

#[test]
fn runtime_animation_advance_completes_at_duration() {
    let mut anim = make_animation();
    anim.play();
    let still_playing = anim.advance(2.0);
    // 动画完成（停止）时 advance 返回 false
    assert!(!still_playing, "should have stopped");
    assert_eq!(anim.state, AnimationState::Stopped);
}

#[test]
fn runtime_animation_advance_past_duration_clamps() {
    let mut anim = make_animation();
    anim.play();
    anim.advance(5.0);
    assert!((anim.local_time - 2.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_advance_when_paused() {
    let mut anim = make_animation();
    anim.state = AnimationState::Paused;
    let result = anim.advance(1.0);
    // 暂停：返回 true（未停止），但 local_time 不变
    assert!(result, "paused should return true");
    assert!((anim.local_time - 0.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_advance_when_stopped() {
    let mut anim = make_animation();
    let result = anim.advance(1.0);
    // 停止：返回 false
    assert!(!result, "stopped should return false");
    assert!((anim.local_time - 0.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_multiplier() {
    let mut anim = make_animation();
    anim.duration = 10.0;
    anim.multiplier = 2.0;
    anim.play();
    anim.advance(1.0);
    assert!((anim.local_time - 2.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_reverse() {
    let mut anim = make_animation();
    anim.duration = 10.0;
    anim.reverse = true;
    anim.local_time = 10.0;
    anim.play();
    anim.advance(1.0);
    assert!((anim.local_time - 9.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_loop_repeat() {
    let mut anim = make_animation();
    anim.duration = 2.0;
    anim.loop_mode = AnimationLoop::Repeat;
    anim.play();
    anim.advance(3.0);
    // 应环绕：3.0 % 2.0 = 1.0
    assert!((anim.local_time - 1.0).abs() < EPSILON7);
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_loop_mirrored() {
    let mut anim = make_animation();
    anim.duration = 2.0;
    anim.loop_mode = AnimationLoop::MirroredRepeat;
    anim.play();
    anim.advance(3.0);
    // 镜像：应位于 [0, duration] 内
    assert!(anim.local_time >= 0.0 && anim.local_time <= 2.0);
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_effective_time() {
    let mut anim = make_animation();
    anim.play();
    anim.advance(1.5);
    assert!((anim.effective_time() - 1.5).abs() < EPSILON7);
}

#[test]
fn runtime_animation_multiple_advance_calls() {
    let mut anim = make_animation();
    anim.play();
    anim.advance(0.3);
    anim.advance(0.3);
    anim.advance(0.3);
    assert!((anim.local_time - 0.9).abs() < EPSILON7);
}

#[test]
fn runtime_animation_zero_duration() {
    let mut anim = make_animation();
    anim.duration = 0.0;
    anim.play();
    let result = anim.advance(0.1);
    // 零时长：跳过循环处理，返回 true（仍在播放）
    assert!(result, "zero duration should return true");
}

#[test]
fn runtime_animation_negative_multiplier() {
    let mut anim = make_animation();
    anim.duration = 10.0;
    anim.multiplier = -1.0;
    anim.local_time = 10.0;
    anim.play();
    anim.advance(1.0);
    // 负倍率反转方向
    assert!((anim.local_time - 9.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_loop_repeat_exact_multiple() {
    let mut anim = make_animation();
    anim.duration = 2.0;
    anim.loop_mode = AnimationLoop::Repeat;
    anim.play();
    anim.advance(4.0);
    // 4.0 % 2.0 = 0.0
    assert!((anim.local_time - 0.0).abs() < EPSILON7);
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_clamp_animations() {
    let mut anim = make_animation();
    anim.clamp_animations = true;
    anim.play();
    anim.advance(100.0);
    assert!((anim.local_time - 2.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_name_preserved() {
    let anim = make_animation();
    assert_eq!(anim.name.as_deref(), Some("test"));
}

#[test]
fn runtime_animation_play_from_stopped() {
    let mut anim = make_animation();
    assert_eq!(anim.state, AnimationState::Stopped);
    anim.play();
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_pause_from_playing() {
    let mut anim = make_animation();
    anim.play();
    anim.pause();
    assert_eq!(anim.state, AnimationState::Paused);
}

#[test]
fn runtime_animation_resume_from_paused() {
    let mut anim = make_animation();
    anim.play();
    anim.pause();
    anim.play();
    assert_eq!(anim.state, AnimationState::Playing);
}

#[test]
fn runtime_animation_stop_from_playing() {
    let mut anim = make_animation();
    anim.play();
    anim.advance(0.5);
    anim.stop();
    assert_eq!(anim.state, AnimationState::Stopped);
    assert!((anim.local_time - 0.0).abs() < EPSILON7);
}

#[test]
fn runtime_animation_loop_none_stops_at_end() {
    let mut anim = make_animation();
    anim.loop_mode = AnimationLoop::None;
    anim.play();
    anim.advance(2.5);
    assert_eq!(anim.state, AnimationState::Stopped);
    assert!((anim.local_time - 2.0).abs() < EPSILON7);
}
