//! Widgets/Animation/AnimationViewModel + Timeline → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Widgets/Animation/AnimationViewModel.js
//! - Widgets/Timeline/Timeline.js
//!
//! A-class tests: AnimationController play/pause/reverse/stop/tick/loop/seek,
//! shuttle ring, progress, TimelineConfig, SpeedPreset.
//! C-class omitted: DOM elements, SVG rendering, drag events.

use cesium_animation::timeline::{AnimationController, SpeedPreset, TimelineConfig};
use cesium_time::clock::Clock;
use cesium_time::julian_date::JulianDate;

fn make_clock() -> Clock {
    let start = JulianDate::from_date_components(2024, 6, 1, 0, 0, 0, 0.0);
    let stop = start.add_seconds(3600.0); // 1 hour duration
    Clock::new(start, stop, start)
}

fn make_controller() -> AnimationController {
    AnimationController::new(make_clock())
}

// === TimelineConfig ===

#[test]
fn timeline_config_duration() {
    let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
    let end = start.add_seconds(7200.0);
    let config = TimelineConfig::new(start, end);
    assert!((config.duration_seconds() - 7200.0).abs() < 1e-10);
    assert!(config.visible);
}

#[test]
fn timeline_config_seconds_per_pixel() {
    let start = JulianDate::from_date_components(2024, 1, 1, 0, 0, 0, 0.0);
    let end = start.add_seconds(1000.0);
    let config = TimelineConfig::new(start, end);
    // Default: duration / 1000 pixels
    assert!((config.seconds_per_pixel - 1.0).abs() < 1e-10);
}

// === AnimationController creation ===

#[test]
fn controller_default_state() {
    let controller = make_controller();
    assert!(controller.paused);
    assert!((controller.speed_multiplier - 1.0).abs() < 1e-10);
    assert!(controller.looping);
    assert!((controller.shuttle_ring_angle - 0.0).abs() < 1e-10);
}

// === Play / Pause / Reverse ===

#[test]
fn controller_play() {
    let mut controller = make_controller();
    controller.play();
    assert!(!controller.paused);
    assert!(controller.is_playing_forward());
    assert!(!controller.is_playing_reverse());
}

#[test]
fn controller_pause() {
    let mut controller = make_controller();
    controller.play();
    controller.pause();
    assert!(controller.paused);
    assert!(!controller.is_playing_forward());
}

#[test]
fn controller_play_reverse() {
    let mut controller = make_controller();
    controller.play_reverse();
    assert!(!controller.paused);
    assert!(controller.is_playing_reverse());
    assert!(!controller.is_playing_forward());
    assert!(controller.speed_multiplier < 0.0);
}

#[test]
fn controller_stop_resets_to_start() {
    let mut controller = make_controller();
    let start = controller.clock.start_time;
    controller.play();
    controller.tick(100.0);
    controller.stop();
    assert!(controller.paused);
    assert_eq!(controller.clock.current_time, start);
}

// === Tick ===

#[test]
fn controller_tick_advances_time() {
    let mut controller = make_controller();
    let start = controller.clock.current_time;
    controller.play();
    let new_time = controller.tick(1.0);
    let elapsed = new_time.seconds_difference(&start);
    assert!((elapsed - 1.0).abs() < 1e-10);
}

#[test]
fn controller_tick_with_speed_multiplier() {
    let mut controller = make_controller();
    let start = controller.clock.current_time;
    controller.set_speed(60.0);
    controller.play();
    let new_time = controller.tick(1.0); // 1 real second at 60x
    let elapsed = new_time.seconds_difference(&start);
    assert!((elapsed - 60.0).abs() < 1e-10);
}

#[test]
fn controller_tick_paused_no_change() {
    let mut controller = make_controller();
    let start = controller.clock.current_time;
    // Default is paused
    let result = controller.tick(10.0);
    assert_eq!(result, start);
}

#[test]
fn controller_tick_reverse() {
    let mut controller = make_controller();
    // Move to middle first
    controller.seek_fraction(0.5);
    let mid = controller.clock.current_time;
    controller.play_reverse();
    let new_time = controller.tick(1.0);
    // Should have moved backwards
    assert!(new_time.less_than(&mid));
}

// === Looping ===

#[test]
fn controller_loop_wraps_around() {
    let mut controller = make_controller();
    let start = controller.clock.start_time;
    controller.looping = true;
    controller.play();
    // Advance past end (3600 + 100 = 3700 seconds)
    let new_time = controller.tick(3700.0);
    let elapsed = new_time.seconds_difference(&start);
    assert!((elapsed - 100.0).abs() < 1e-10);
}

#[test]
fn controller_no_loop_clamps_at_end() {
    let mut controller = make_controller();
    let stop = controller.clock.stop_time;
    controller.looping = false;
    controller.play();
    let new_time = controller.tick(5000.0);
    assert_eq!(new_time, stop);
    assert!(controller.paused); // Auto-paused
}

#[test]
fn controller_no_loop_clamps_at_start_reverse() {
    let mut controller = make_controller();
    let start = controller.clock.start_time;
    controller.looping = false;
    controller.play_reverse();
    let new_time = controller.tick(100.0); // Reverse past start
    assert_eq!(new_time, start);
    assert!(controller.paused);
}

// === Seek ===

#[test]
fn controller_seek() {
    let mut controller = make_controller();
    let start = controller.clock.start_time;
    let target = start.add_seconds(1800.0);
    controller.seek(target);
    assert_eq!(controller.clock.current_time, target);
}

#[test]
fn controller_seek_fraction() {
    let mut controller = make_controller();
    let start = controller.clock.start_time;
    controller.seek_fraction(0.5);
    let elapsed = controller.clock.current_time.seconds_difference(&start);
    assert!((elapsed - 1800.0).abs() < 1e-10);
}

#[test]
fn controller_seek_fraction_clamped() {
    let mut controller = make_controller();
    let stop = controller.clock.stop_time;
    controller.seek_fraction(2.0); // > 1.0 clamped
    assert_eq!(controller.clock.current_time, stop);
}

// === Progress ===

#[test]
fn controller_progress() {
    let mut controller = make_controller();
    controller.seek_fraction(0.25);
    assert!((controller.progress() - 0.25).abs() < 1e-10);
}

#[test]
fn controller_progress_at_start() {
    let controller = make_controller();
    assert!((controller.progress() - 0.0).abs() < 1e-10);
}

// === Shuttle Ring ===

#[test]
fn controller_shuttle_ring_positive() {
    let mut controller = make_controller();
    controller.set_shuttle_ring(0.5);
    assert!(!controller.paused);
    assert!(controller.speed_multiplier > 0.0);
    assert!((controller.shuttle_ring_angle - 0.5).abs() < 1e-10);
}

#[test]
fn controller_shuttle_ring_zero_pauses() {
    let mut controller = make_controller();
    controller.set_shuttle_ring(0.5);
    controller.set_shuttle_ring(0.0);
    assert!(controller.paused);
}

#[test]
fn controller_shuttle_ring_clamped() {
    let mut controller = make_controller();
    controller.set_shuttle_ring(5.0);
    assert!((controller.shuttle_ring_angle - 1.0).abs() < 1e-10);
    controller.set_shuttle_ring(-5.0);
    assert!((controller.shuttle_ring_angle - (-1.0)).abs() < 1e-10);
}

// === SpeedPreset ===

#[test]
fn speed_preset_multipliers() {
    assert!((SpeedPreset::RealTime.multiplier() - 1.0).abs() < 1e-10);
    assert!((SpeedPreset::Fast2x.multiplier() - 2.0).abs() < 1e-10);
    assert!((SpeedPreset::Fast5x.multiplier() - 5.0).abs() < 1e-10);
    assert!((SpeedPreset::Fast10x.multiplier() - 10.0).abs() < 1e-10);
    assert!((SpeedPreset::Fast60x.multiplier() - 60.0).abs() < 1e-10);
    assert!((SpeedPreset::Fast3600x.multiplier() - 3600.0).abs() < 1e-10);
    assert!((SpeedPreset::Fast86400x.multiplier() - 86400.0).abs() < 1e-10);
}

// === set_speed ===

#[test]
fn controller_set_speed_while_playing() {
    let mut controller = make_controller();
    controller.play();
    controller.set_speed(10.0);
    assert!((controller.speed_multiplier - 10.0).abs() < 1e-10);
    assert!((controller.clock.multiplier - 10.0).abs() < 1e-10);
}

#[test]
fn controller_set_speed_while_paused() {
    let mut controller = make_controller();
    // paused by default
    controller.set_speed(100.0);
    assert!((controller.speed_multiplier - 100.0).abs() < 1e-10);
    // Clock multiplier should NOT change while paused
    assert!((controller.clock.multiplier - 1.0).abs() < 1e-10);
}
