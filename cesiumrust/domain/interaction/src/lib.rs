//! cesium-interaction: Camera controllers, flight animations, and picking.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/ScreenSpaceCameraController.js` → camera_controller
//! - `Scene/Camera.js` (flyTo/lookAt) → flight
//! - `Scene/Scene.js` (pick) → picking

pub mod camera_controller;
pub mod flight;
pub mod picking;

pub use camera_controller::{CameraController, CameraControllerConfig};
pub use flight::{CameraFlight, compute_look_at, compute_set_view};
pub use picking::{Viewport, get_pick_ray, pick_ellipsoid, world_to_screen, window_center};
