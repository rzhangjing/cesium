use bevy::prelude::*;
use cesium_camera::Frustum;
use cesium_scene_mode::SceneMode;

use crate::camera::components::{ActiveMorph, CesiumCamera};

/// Scene mode switching system: keyboard shortcuts and morph animation.
///
/// Press `2` for 2D, `3` for 3D, `C` for Columbus View.
/// Morphing animates smoothly between modes.
pub fn scene_mode_system(
    mut cameras: Query<&mut CesiumCamera>,
    mut morph: ResMut<ActiveMorph>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs() as f64;

    // --- Check mode switch keys ---
    let target_mode = if keys.just_pressed(KeyCode::Digit2) {
        Some(SceneMode::Scene2D)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(SceneMode::Scene3D)
    } else if keys.just_pressed(KeyCode::KeyC) {
        Some(SceneMode::ColumbusView)
    } else {
        None
    };

    if let Some(target) = target_mode {
        for cesium_cam in cameras.iter() {
            let current = cesium_cam.scene_mode;
            if current != target && current != SceneMode::Morphing {
                morph.state.start_morph(current, target, 2.0);
            }
        }
    }

    // --- Advance morph ---
    let was_active = morph.state.active;
    morph.state.update(dt);
    let just_finished = was_active && !morph.state.active;

    for mut cesium_cam in cameras.iter_mut() {
        if morph.state.active {
            cesium_cam.scene_mode = SceneMode::Morphing;
            cesium_cam.camera.mode = cesium_camera::SceneMode::Morphing;

            let t = morph.state.progress;
            let from_2d_or_cv = matches!(morph.state.from, SceneMode::Scene2D | SceneMode::ColumbusView);
            let to_2d_or_cv = matches!(morph.state.to, SceneMode::Scene2D | SceneMode::ColumbusView);

            if from_2d_or_cv && morph.state.to == SceneMode::Scene3D {
                update_projection_for_morph(&mut cesium_cam, t, true);
            } else if morph.state.from == SceneMode::Scene3D && to_2d_or_cv {
                update_projection_for_morph(&mut cesium_cam, t, false);
            }
        } else if just_finished {
            let target = morph.state.to;
            cesium_cam.scene_mode = target;
            apply_mode_projection(&mut cesium_cam, target);
        }
    }
}

fn apply_mode_projection(cesium_cam: &mut CesiumCamera, mode: SceneMode) {
    let cam = &mut cesium_cam.camera;
    match mode {
        SceneMode::Scene3D => {
            cam.mode = cesium_camera::SceneMode::Scene3D;
            cam.frustum = Frustum::Perspective(cesium_geospatial::PerspectiveFrustum::new(
                std::f64::consts::FRAC_PI_3,
                16.0 / 9.0,
                1.0,
                500_000_000.0,
            ));
        }
        SceneMode::Scene2D => {
            cam.mode = cesium_camera::SceneMode::Scene2D;
            cam.frustum = Frustum::Orthographic(cesium_geospatial::OrthographicFrustum::new(
                6378137.0 * std::f64::consts::TAU,
                16.0 / 9.0,
                1.0,
                500_000_000.0,
            ));
        }
        SceneMode::ColumbusView => {
            cam.mode = cesium_camera::SceneMode::ColumbusView;
            cam.frustum = Frustum::Perspective(cesium_geospatial::PerspectiveFrustum::new(
                std::f64::consts::FRAC_PI_3,
                16.0 / 9.0,
                1.0,
                500_000_000.0,
            ));
        }
        SceneMode::Morphing => {
            cam.mode = cesium_camera::SceneMode::Morphing;
        }
    }
}

fn update_projection_for_morph(cesium_cam: &mut CesiumCamera, t: f64, from_2d_to_3d: bool) {
    let perspective_fov = std::f64::consts::FRAC_PI_3;

    if from_2d_to_3d {
        let fov = perspective_fov * t + (1.0 - t) * 0.01;
        cesium_cam.camera.frustum = Frustum::Perspective(
            cesium_geospatial::PerspectiveFrustum::new(fov, 16.0 / 9.0, 1.0, 500_000_000.0),
        );
    } else {
        let fov = perspective_fov * (1.0 - t) + t * 0.01;
        cesium_cam.camera.frustum = Frustum::Perspective(
            cesium_geospatial::PerspectiveFrustum::new(fov, 16.0 / 9.0, 1.0, 500_000_000.0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_scene_mode::SceneMode;

    #[test]
    fn test_apply_mode_projection_3d() {
        let mut cc = CesiumCamera::default();
        apply_mode_projection(&mut cc, SceneMode::Scene3D);
        assert_eq!(cc.camera.mode, cesium_camera::SceneMode::Scene3D);
        assert!(matches!(cc.camera.frustum, Frustum::Perspective(_)));
    }

    #[test]
    fn test_apply_mode_projection_2d() {
        let mut cc = CesiumCamera::default();
        apply_mode_projection(&mut cc, SceneMode::Scene2D);
        assert_eq!(cc.camera.mode, cesium_camera::SceneMode::Scene2D);
        assert!(matches!(cc.camera.frustum, Frustum::Orthographic(_)));
    }

    #[test]
    fn test_morph_state_default_inactive() {
        let morph = ActiveMorph::default();
        assert!(!morph.state.active);
        assert!((morph.state.progress - 1.0).abs() < 1e-10);
    }
}
