//! Animation widget extended specs - format_date/format_time/multiplier_string/shuttle ring
//! Ported from Widgets/Animation/AnimationViewModelSpec.js (A-class formatting/logic)

use cesium_widgets::animation::{
    AnimationViewModel, ShuttleRing, DEFAULT_SHUTTLE_RING_TICKS,
    MAX_SHUTTLE_RING_ANGLE, REALTIME_SHUTTLE_RING_ANGLE,
};

// ─── ShuttleRing extended ───────────────────────────────────────────────────

#[test]
fn shuttle_ring_with_ticks_sorts() {
    let ring = ShuttleRing::with_ticks(vec![100.0, -50.0, 10.0, -1000.0, 5.0]);
    assert_eq!(ring.ticks[0], -1000.0);
    assert_eq!(ring.ticks[1], -50.0);
    assert_eq!(ring.ticks[2], 5.0);
    assert_eq!(ring.ticks[3], 10.0);
    assert_eq!(ring.ticks[4], 100.0);
}

#[test]
fn shuttle_ring_get_typical_multiplier_index_exact() {
    let ring = ShuttleRing::default();
    // 1.0 is at index 8 in DEFAULT_SHUTTLE_RING_TICKS
    let idx = ring.get_typical_multiplier_index(1.0);
    assert_eq!(ring.ticks[idx], 1.0);
}

#[test]
fn shuttle_ring_get_typical_multiplier_index_negative() {
    let ring = ShuttleRing::default();
    let idx = ring.get_typical_multiplier_index(-10.0);
    assert_eq!(ring.ticks[idx], -10.0);
}

#[test]
fn shuttle_ring_angle_roundtrip() {
    let ring = ShuttleRing::default();
    // angle → multiplier → angle should be approximately identity
    let angle = 45.0;
    let mult = ring.angle_to_multiplier(angle);
    let back = ring.multiplier_to_angle(mult, false);
    assert!((back - angle).abs() < 0.01, "roundtrip failed: {} → {} → {}", angle, mult, back);
}

#[test]
fn shuttle_ring_max_angle_gives_max_tick() {
    let ring = ShuttleRing::default();
    let mult = ring.angle_to_multiplier(MAX_SHUTTLE_RING_ANGLE);
    let max_tick = *ring.ticks.last().unwrap();
    assert!((mult - max_tick).abs() < 1.0, "max angle should give ~max tick, got {}", mult);
}

// ─── AnimationViewModel formatting ──────────────────────────────────────────

#[test]
fn format_time_at_j2000_epoch() {
    let mut vm = AnimationViewModel::new();
    vm.update_time(0.0); // J2000 epoch = 2000-01-01 12:00:00
    let time_str = vm.format_time();
    // Should contain "12:00:00 UTC"
    assert!(time_str.contains("12:00:00"), "expected 12:00:00, got {}", time_str);
    assert!(time_str.contains("UTC"));
}

#[test]
fn format_time_six_hours_after_epoch() {
    let mut vm = AnimationViewModel::new();
    vm.update_time(6.0 * 3600.0); // 6 hours after J2000
    let time_str = vm.format_time();
    assert!(time_str.contains("18:00:00"), "expected 18:00:00, got {}", time_str);
}

#[test]
fn format_date_at_j2000_epoch() {
    let mut vm = AnimationViewModel::new();
    vm.update_time(0.0);
    let date_str = vm.format_date();
    // Should contain "2000" (approximate date calculation)
    assert!(date_str.contains("2000"), "expected year 2000, got {}", date_str);
}

#[test]
fn multiplier_string_one_x() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(1.0);
    assert_eq!(vm.multiplier_string(), "1x");
}

#[test]
fn multiplier_string_negative_one_x() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(-1.0);
    assert_eq!(vm.multiplier_string(), "-1x");
}

#[test]
fn multiplier_string_large() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(100.0);
    assert_eq!(vm.multiplier_string(), "100x");
}

#[test]
fn multiplier_string_fractional() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(0.5);
    assert_eq!(vm.multiplier_string(), "0.50x");
}

// ─── AnimationViewModel play logic ──────────────────────────────────────────

#[test]
fn play_reverse_negates_positive_multiplier() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(10.0);
    vm.play_reverse();
    assert!(vm.is_playing);
    assert!(vm.multiplier < 0.0);
}

#[test]
fn play_reverse_keeps_negative_multiplier() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(-5.0);
    vm.play_reverse();
    assert!(vm.is_playing);
    assert_eq!(vm.multiplier, -5.0);
}

#[test]
fn play_forward_negates_negative_multiplier() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(-10.0);
    vm.play_forward();
    assert!(vm.is_playing);
    assert!(vm.multiplier > 0.0);
}

#[test]
fn set_shuttle_ring_angle_clamps() {
    let mut vm = AnimationViewModel::new();
    vm.set_shuttle_ring_angle(200.0); // Beyond max
    assert!(vm.shuttle_ring_angle <= MAX_SHUTTLE_RING_ANGLE);
}

#[test]
fn set_system_clock_resets_angle() {
    let mut vm = AnimationViewModel::new();
    vm.set_multiplier(100.0);
    vm.set_system_clock(true);
    assert!((vm.shuttle_ring_angle - REALTIME_SHUTTLE_RING_ANGLE).abs() < 1e-10);
}

#[test]
fn default_ticks_match_cesiumjs() {
    // CesiumJS default ticks: [-1000, -100, -50, -25, -10, -5, -2, -1, 1, 2, 5, 10, 25, 50, 100, 1000]
    assert_eq!(DEFAULT_SHUTTLE_RING_TICKS.len(), 16);
    assert_eq!(DEFAULT_SHUTTLE_RING_TICKS[0], -1000.0);
    assert_eq!(DEFAULT_SHUTTLE_RING_TICKS[7], -1.0);
    assert_eq!(DEFAULT_SHUTTLE_RING_TICKS[8], 1.0);
    assert_eq!(DEFAULT_SHUTTLE_RING_TICKS[15], 1000.0);
}
