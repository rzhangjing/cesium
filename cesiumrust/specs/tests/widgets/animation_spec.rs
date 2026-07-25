//! Widgets/AnimationViewModelSpec.js → Rust integration tests

use cesium_widgets::{AnimationViewModel, ShuttleRing};

// === AnimationViewModel ===

#[test]
fn test_animation_view_model_default() {
    let vm = AnimationViewModel::default();
    assert!(!vm.is_playing);
    assert_eq!(vm.multiplier, 1.0);
    assert!(!vm.is_system_clock);
}

#[test]
fn test_animation_toggle_play() {
    let mut vm = AnimationViewModel::new();
    assert!(!vm.is_playing);
    vm.toggle_play();
    assert!(vm.is_playing);
    vm.toggle_play();
    assert!(!vm.is_playing);
}

#[test]
fn test_animation_play_pause() {
    let mut vm = AnimationViewModel::new();
    vm.play();
    assert!(vm.is_playing);
    vm.pause();
    assert!(!vm.is_playing);
}

#[test]
fn test_animation_play_reverse() {
    let mut vm = AnimationViewModel::new();
    vm.play_reverse();
    assert!(vm.is_playing);
    assert!(vm.multiplier < 0.0);
}

#[test]
fn test_animation_play_forward() {
    let mut vm = AnimationViewModel::new();
    vm.multiplier = -2.0;
    vm.play_forward();
    assert!(vm.is_playing);
    assert!(vm.multiplier > 0.0);
}

#[test]
fn test_animation_set_multiplier() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(10.0);
    assert_eq!(vm.multiplier, 10.0);
}

#[test]
fn test_animation_set_system_clock() {
    let mut vm = AnimationViewModel::new();
    vm.set_system_clock(true);
    assert!(vm.is_system_clock);
}

#[test]
fn test_animation_update_time() {
    let mut vm = AnimationViewModel::new();
    vm.update_time(12345.0);
    assert_eq!(vm.current_time, 12345.0);
}

// === ShuttleRing ===

#[test]
fn test_shuttle_ring_default() {
    let ring = ShuttleRing::default();
    assert!(!ring.ticks.is_empty());
}

#[test]
fn test_shuttle_ring_angle_to_multiplier_zero() {
    let ring = ShuttleRing::default();
    let m = ring.angle_to_multiplier(0.0);
    assert!((m - 0.0).abs() < 1e-10);
}

#[test]
fn test_shuttle_ring_angle_to_multiplier_realtime() {
    let ring = ShuttleRing::default();
    // At realtime angle (15 degrees), multiplier should be 1.0
    let m = ring.angle_to_multiplier(15.0);
    assert!((m - 1.0).abs() < 1e-10);
}

#[test]
fn test_shuttle_ring_multiplier_to_angle() {
    let ring = ShuttleRing::default();
    let angle = ring.multiplier_to_angle(1.0, false);
    assert!((angle - 15.0).abs() < 1e-10);
}

#[test]
fn test_shuttle_ring_multiplier_to_angle_system_clock() {
    let ring = ShuttleRing::default();
    let angle = ring.multiplier_to_angle(5.0, true);
    assert!((angle - 15.0).abs() < 1e-10); // Always realtime in system clock mode
}

#[test]
fn test_shuttle_ring_with_ticks() {
    let ring = ShuttleRing::with_ticks(vec![10.0, 1.0, 5.0]);
    // Should be sorted
    assert_eq!(ring.ticks, vec![1.0, 5.0, 10.0]);
}
