//! CameraEventAggregator specs - ported from CameraEventAggregatorSpec.js
//!
//! Tests event aggregation: button down/up, drag movement, wheel events,
//! frame reset, multi-button state, movement queries.

use cesium_interaction::{
    AggregateMovement, CameraEventAggregator, CameraEventType, MouseButton,
};
use glam::DVec2;

// ─── AggregateMovement Unit Tests ─────────────────────────────────────────

#[test]
fn aggregate_movement_default_state() {
    let m = AggregateMovement::new();
    assert!(!m.is_button_down);
    assert!(!m.is_moving);
    assert_eq!(m.movement, DVec2::ZERO);
    assert_eq!(m.start_position, DVec2::ZERO);
    assert_eq!(m.end_position, DVec2::ZERO);
}

#[test]
fn aggregate_movement_button_down() {
    let mut m = AggregateMovement::new();
    m.button_down(DVec2::new(100.0, 200.0), 1.5);

    assert!(m.is_button_down);
    assert_eq!(m.start_position, DVec2::new(100.0, 200.0));
    assert_eq!(m.end_position, DVec2::new(100.0, 200.0));
    assert!((m.start_time - 1.5).abs() < 1e-10);
    assert!((m.last_time - 1.5).abs() < 1e-10);
}

#[test]
fn aggregate_movement_button_up() {
    let mut m = AggregateMovement::new();
    m.button_down(DVec2::new(50.0, 50.0), 0.0);
    m.button_up(2.0);

    assert!(!m.is_button_down);
    assert!((m.last_time - 2.0).abs() < 1e-10);
}

#[test]
fn aggregate_movement_drag_computes_delta() {
    let mut m = AggregateMovement::new();
    m.button_down(DVec2::new(100.0, 100.0), 0.0);
    m.drag(DVec2::new(130.0, 140.0), 0.5);

    assert!(m.is_moving);
    assert_eq!(m.movement, DVec2::new(30.0, 40.0));
    assert_eq!(m.end_position, DVec2::new(130.0, 140.0));
}

#[test]
fn aggregate_movement_drag_without_button_down_ignored() {
    let mut m = AggregateMovement::new();
    // No button_down first
    m.drag(DVec2::new(200.0, 200.0), 0.5);

    assert!(!m.is_moving);
    assert_eq!(m.movement, DVec2::ZERO);
}

#[test]
fn aggregate_movement_wheel() {
    let mut m = AggregateMovement::new();
    m.wheel(-120.0, 3.0);

    assert!(m.is_moving);
    assert!((m.movement.y - (-120.0)).abs() < 1e-10);
    assert!((m.movement.x - 0.0).abs() < 1e-10);
    assert!((m.last_time - 3.0).abs() < 1e-10);
}

#[test]
fn aggregate_movement_reset_frame() {
    let mut m = AggregateMovement::new();
    m.button_down(DVec2::new(10.0, 10.0), 0.0);
    m.drag(DVec2::new(50.0, 50.0), 0.5);

    m.reset_frame();
    assert!(!m.is_moving);
    assert_eq!(m.movement, DVec2::ZERO);
    // Button state preserved
    assert!(m.is_button_down);
}

// ─── CameraEventAggregator Integration ─────────────────────────────────────

#[test]
fn aggregator_new_no_movements() {
    let agg = CameraEventAggregator::new();
    assert!(agg.get_movement(CameraEventType::LeftDrag).is_none());
    assert!(!agg.is_moving(CameraEventType::LeftDrag));
    assert!(!agg.is_button_down(MouseButton::Left));
}

#[test]
fn aggregator_left_button_lifecycle() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    // Button down
    agg.button_down(MouseButton::Left, DVec2::new(100.0, 100.0));
    assert!(agg.is_button_down(MouseButton::Left));
    assert!(!agg.is_button_down(MouseButton::Right));

    // Drag
    agg.mouse_move(MouseButton::Left, DVec2::new(150.0, 120.0));
    assert!(agg.is_moving(CameraEventType::LeftDrag));

    let delta = agg.get_movement_delta(CameraEventType::LeftDrag);
    assert!((delta.x - 50.0).abs() < 1e-10);
    assert!((delta.y - 20.0).abs() < 1e-10);

    // Button up
    agg.button_up(MouseButton::Left);
    assert!(!agg.is_button_down(MouseButton::Left));
}

#[test]
fn aggregator_right_button_drag() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    agg.button_down(MouseButton::Right, DVec2::new(200.0, 200.0));
    agg.mouse_move(MouseButton::Right, DVec2::new(180.0, 250.0));

    assert!(agg.is_moving(CameraEventType::RightDrag));
    let delta = agg.get_movement_delta(CameraEventType::RightDrag);
    assert!((delta.x - (-20.0)).abs() < 1e-10);
    assert!((delta.y - 50.0).abs() < 1e-10);
}

#[test]
fn aggregator_middle_button() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    agg.button_down(MouseButton::Middle, DVec2::new(300.0, 300.0));
    assert!(agg.is_button_down(MouseButton::Middle));

    agg.mouse_move(MouseButton::Middle, DVec2::new(310.0, 310.0));
    assert!(agg.is_moving(CameraEventType::MiddleDrag));
}

#[test]
fn aggregator_wheel_event() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    agg.wheel(120.0);
    assert!(agg.is_moving(CameraEventType::Wheel));
    let delta = agg.get_movement_delta(CameraEventType::Wheel);
    assert!((delta.y - 120.0).abs() < 1e-10);
}

#[test]
fn aggregator_reset_clears_movement_preserves_button() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    agg.button_down(MouseButton::Left, DVec2::new(100.0, 100.0));
    agg.mouse_move(MouseButton::Left, DVec2::new(200.0, 200.0));
    assert!(agg.is_moving(CameraEventType::LeftDrag));

    // New frame
    agg.reset(1.0 / 60.0);
    assert!(!agg.is_moving(CameraEventType::LeftDrag));
    assert_eq!(agg.get_movement_delta(CameraEventType::LeftDrag), DVec2::ZERO);
    // Button still held
    assert!(agg.is_button_down(MouseButton::Left));
}

#[test]
fn aggregator_multiple_buttons_simultaneous() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    agg.button_down(MouseButton::Left, DVec2::new(0.0, 0.0));
    agg.button_down(MouseButton::Right, DVec2::new(100.0, 100.0));
    agg.button_down(MouseButton::Middle, DVec2::new(200.0, 200.0));

    assert!(agg.is_button_down(MouseButton::Left));
    assert!(agg.is_button_down(MouseButton::Right));
    assert!(agg.is_button_down(MouseButton::Middle));

    // Release one
    agg.button_up(MouseButton::Right);
    assert!(agg.is_button_down(MouseButton::Left));
    assert!(!agg.is_button_down(MouseButton::Right));
    assert!(agg.is_button_down(MouseButton::Middle));
}

#[test]
fn aggregator_movement_delta_no_event_returns_zero() {
    let agg = CameraEventAggregator::new();
    let delta = agg.get_movement_delta(CameraEventType::Wheel);
    assert_eq!(delta, DVec2::ZERO);
}

#[test]
fn aggregator_drag_continues_from_start() {
    let mut agg = CameraEventAggregator::new();
    agg.reset(0.0);

    agg.button_down(MouseButton::Left, DVec2::new(100.0, 100.0));
    agg.mouse_move(MouseButton::Left, DVec2::new(110.0, 110.0));
    agg.mouse_move(MouseButton::Left, DVec2::new(130.0, 130.0));

    // Movement is always from start to current end
    let delta = agg.get_movement_delta(CameraEventType::LeftDrag);
    assert!((delta.x - 30.0).abs() < 1e-10);
    assert!((delta.y - 30.0).abs() < 1e-10);
}
