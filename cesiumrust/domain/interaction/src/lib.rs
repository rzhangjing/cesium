//! cesium-interaction: Camera controllers, flight animations, picking, and events.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/ScreenSpaceCameraController.js` → camera_controller
//! - `Scene/ScreenSpaceCameraController.js` (inertia) → inertia
//! - `Scene/Camera.js` (flyTo/lookAt) → flight
//! - `Scene/Scene.js` (pick) → picking
//! - `Scene/CameraEventAggregator.js` → event_aggregator
//! - `Scene/SceneMode.js` morphing → morphing

pub mod camera_controller;
pub mod flight;
pub mod inertia;
pub mod picking;
pub mod event_aggregator;
pub mod morphing;

pub use camera_controller::{CameraController, CameraControllerConfig};
pub use flight::{
    compute_flight_duration, compute_look_at, compute_set_view, select_flight_easing, CameraFlight,
    FlightOptions,
};
pub use inertia::{
    decay, InertiaController, InertiaMovementState, InertiaSample, InertiaState,
    INERTIA_MAX_CLICK_TIME_THRESHOLD, INERTIA_STOP_DISTANCE,
};
pub use picking::{Viewport, get_pick_ray, pick_ellipsoid, world_to_screen, window_center};
pub use event_aggregator::{
    AggregateMovement, CameraEventAggregator, CameraEventType, MouseButton, PinchMovement, StartEnd,
};
pub use morphing::{SceneMorph, MorphState};
