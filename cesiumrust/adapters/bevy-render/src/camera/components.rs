use bevy::prelude::*;
use cesium_camera::Camera;
use cesium_scene_mode::SceneMode;

/// Main camera component wrapping the domain Camera.
#[derive(Component)]
pub struct CesiumCamera {
    pub camera: Camera,
    pub scene_mode: SceneMode,
    pub enable_collision_detection: bool,
    pub minimum_zoom_distance: f64,
    pub maximum_zoom_distance: f64,
}

impl Default for CesiumCamera {
    fn default() -> Self {
        Self {
            camera: Camera::default_camera(),
            scene_mode: SceneMode::Scene3D,
            enable_collision_detection: true,
            minimum_zoom_distance: 100.0,
            maximum_zoom_distance: 20_000_000.0,
        }
    }
}

impl CesiumCamera {
    pub fn new(camera: Camera, scene_mode: SceneMode) -> Self {
        Self {
            camera,
            scene_mode,
            ..Default::default()
        }
    }
}

/// Request to fly the camera to a cartographic destination.
#[derive(Event)]
pub struct FlyToRequest {
    pub destination: cesium_geospatial::Cartographic,
    pub duration_secs: f64,
}

/// Emitted when a camera flight completes.
#[derive(Event)]
pub struct FlightComplete;

/// Mouse and touch input state for camera control.
#[derive(Resource, Default)]
pub struct CameraInputState {
    pub left_mouse_down: bool,
    pub right_mouse_down: bool,
    pub middle_mouse_down: bool,
    pub last_mouse_pos: Option<Vec2>,
    pub orbit_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub touch_active: bool,
    pub last_touch_distance: Option<f32>,
    pub last_touch_center: Option<Vec2>,
}

/// Active flight animation state (stored as a resource).
#[derive(Resource, Default)]
pub struct ActiveFlight {
    pub flight: Option<cesium_interaction::CameraFlight>,
}

/// Active scene mode morph state.
#[derive(Resource)]
pub struct ActiveMorph {
    pub state: cesium_scene_mode::MorphState,
}

impl Default for ActiveMorph {
    fn default() -> Self {
        Self {
            state: cesium_scene_mode::MorphState::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cesium_camera_default() {
        let cc = CesiumCamera::default();
        assert_eq!(cc.scene_mode, SceneMode::Scene3D);
        assert!(cc.enable_collision_detection);
        assert!((cc.minimum_zoom_distance - 100.0).abs() < 1e-10);
        assert!((cc.maximum_zoom_distance - 20_000_000.0).abs() < 1e-10);
    }

    #[test]
    fn test_camera_input_state_default() {
        let state = CameraInputState::default();
        assert!(!state.left_mouse_down);
        assert!(!state.right_mouse_down);
        assert!(!state.middle_mouse_down);
        assert!(state.last_mouse_pos.is_none());
    }

    #[test]
    fn test_cesium_camera_new() {
        let cam = Camera::default_camera();
        let cc = CesiumCamera::new(cam.clone(), SceneMode::Scene2D);
        assert_eq!(cc.scene_mode, SceneMode::Scene2D);
        assert_eq!(cc.camera.position, cam.position);
    }
}
