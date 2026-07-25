//! Scene mode morphing (transitions between 2D/3D/Columbus View).
//!
//! Maps to CesiumJS `Scene/SceneMode.js` morphing behavior
//! and `Scene/Scene.js` morph transitions.

use cesium_camera::{Camera, SceneMode};
use cesium_geospatial::Ellipsoid;
use glam::DVec3;

/// The state of a morph transition.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MorphState {
    /// Not morphing - stable in a scene mode.
    #[default]
    Idle,
    /// Morphing from one mode to another.
    Morphing {
        /// Source mode.
        from: SceneMode,
        /// Target mode.
        to: SceneMode,
        /// Progress (0.0 to 1.0).
        progress: f64,
    },
}

/// Manages scene mode morphing transitions.
/// Maps to CesiumJS morph behavior in `Scene.js`
#[derive(Debug, Clone)]
pub struct SceneMorph {
    /// Current morph state.
    pub state: MorphState,
    /// Duration of the morph transition in seconds.
    pub duration: f64,
    /// Elapsed time in the current morph.
    pub elapsed: f64,
    /// Start camera position.
    pub start_position: DVec3,
    /// End camera position.
    pub end_position: DVec3,
    /// Start camera direction.
    pub start_direction: DVec3,
    /// End camera direction.
    pub end_direction: DVec3,
    /// Start camera up vector.
    pub start_up: DVec3,
    /// End camera up vector.
    pub end_up: DVec3,
}

impl Default for SceneMorph {
    fn default() -> Self {
        Self {
            state: MorphState::Idle,
            duration: 2.0,
            elapsed: 0.0,
            start_position: DVec3::ZERO,
            end_position: DVec3::ZERO,
            start_direction: -DVec3::Z,
            end_direction: -DVec3::Z,
            start_up: DVec3::Y,
            end_up: DVec3::Y,
        }
    }
}

impl SceneMorph {
    /// Creates a new scene morph manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether a morph is currently in progress.
    pub fn is_morphing(&self) -> bool {
        matches!(self.state, MorphState::Morphing { .. })
    }

    /// Returns the current progress (0.0 to 1.0), or 0 if not morphing.
    pub fn progress(&self) -> f64 {
        match self.state {
            MorphState::Morphing { progress, .. } => progress,
            MorphState::Idle => 0.0,
        }
    }

    /// Starts a morph transition between scene modes.
    ///
    /// # Arguments
    /// * `camera` - Current camera state
    /// * `from` - Source scene mode
    /// * `to` - Target scene mode
    /// * `ellipsoid` - The ellipsoid for coordinate conversions
    /// * `duration` - Transition duration in seconds
    pub fn start_morph(
        &mut self,
        camera: &Camera,
        from: SceneMode,
        to: SceneMode,
        ellipsoid: &Ellipsoid,
        duration: f64,
    ) {
        if from == to {
            return;
        }

        self.state = MorphState::Morphing {
            from,
            to,
            progress: 0.0,
        };
        self.duration = duration.max(0.001);
        self.elapsed = 0.0;

        // Save start state
        self.start_position = camera.position;
        self.start_direction = camera.direction;
        self.start_up = camera.up;

        // Compute end state based on target mode
        let (end_pos, end_dir, end_up) =
            compute_morph_target(camera, from, to, ellipsoid);
        self.end_position = end_pos;
        self.end_direction = end_dir;
        self.end_up = end_up;
    }

    /// Updates the morph transition.
    ///
    /// # Arguments
    /// * `dt` - Time delta in seconds
    /// * `camera` - Camera to update
    ///
    /// # Returns
    /// `true` if morphing is still in progress, `false` if complete.
    pub fn update(&mut self, dt: f64, camera: &mut Camera) -> bool {
        let (from, to) = match self.state {
            MorphState::Morphing { from, to, .. } => (from, to),
            MorphState::Idle => return false,
        };

        self.elapsed += dt;
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);

        // Smooth step easing
        let t_smooth = t * t * (3.0 - 2.0 * t);

        // Interpolate camera state
        camera.position = self.start_position.lerp(self.end_position, t_smooth);
        camera.direction = self.start_direction.lerp(self.end_direction, t_smooth).normalize();
        camera.up = self.start_up.lerp(self.end_up, t_smooth).normalize();
        camera.right = camera.direction.cross(camera.up).normalize();
        camera.up = camera.right.cross(camera.direction).normalize();

        if t >= 1.0 {
            self.state = MorphState::Idle;
            camera.mode = to;
            false
        } else {
            self.state = MorphState::Morphing {
                from,
                to,
                progress: t,
            };
            camera.mode = SceneMode::Morphing;
            true
        }
    }

    /// Immediately completes the morph transition.
    pub fn complete_morph(&mut self, camera: &mut Camera) {
        if let MorphState::Morphing { to, .. } = self.state {
            camera.position = self.end_position;
            camera.direction = self.end_direction;
            camera.up = self.end_up;
            camera.right = camera.direction.cross(camera.up).normalize();
            camera.up = camera.right.cross(camera.direction).normalize();
            camera.mode = to;
        }
        self.state = MorphState::Idle;
    }

    /// Cancels the morph and returns to the source mode.
    pub fn cancel_morph(&mut self, camera: &mut Camera) {
        if let MorphState::Morphing { from, .. } = self.state {
            camera.position = self.start_position;
            camera.direction = self.start_direction;
            camera.up = self.start_up;
            camera.right = camera.direction.cross(camera.up).normalize();
            camera.up = camera.right.cross(camera.direction).normalize();
            camera.mode = from;
        }
        self.state = MorphState::Idle;
    }
}

/// Computes the target camera state for a morph transition.
fn compute_morph_target(
    camera: &Camera,
    from: SceneMode,
    to: SceneMode,
    ellipsoid: &Ellipsoid,
) -> (DVec3, DVec3, DVec3) {
    match (from, to) {
        // 3D → 2D: Move camera to top-down view
        (SceneMode::Scene3D, SceneMode::Scene2D) => {
            let height = camera.position.length() - ellipsoid.maximum_radius();
            let carto = ellipsoid.cartesian_to_cartographic(camera.position);
            if let Some(carto) = carto {
                let pos = ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::Cartographic::from_radians(
                        carto.longitude,
                        carto.latitude,
                        height.max(ellipsoid.maximum_radius()),
                    ),
                );
                let dir = -pos.normalize();
                let up = DVec3::Z.cross(dir).normalize();
                let up = if up.length_squared() < 1e-10 { DVec3::Y } else { up };
                (pos, dir, up)
            } else {
                (camera.position, camera.direction, camera.up)
            }
        }
        // 2D → 3D: Move camera to angled view
        (SceneMode::Scene2D, SceneMode::Scene3D) => {
            let pos = camera.position;
            let normal = pos.normalize();
            // Tilt to look at horizon
            let dir = (-normal + DVec3::new(0.0, 0.0, 0.3)).normalize();
            let right = dir.cross(DVec3::Z).normalize();
            let up = right.cross(dir).normalize();
            (pos, dir, up)
        }
        // 3D → Columbus View: Flatten to 2.5D
        (SceneMode::Scene3D, SceneMode::ColumbusView) => {
            let carto = ellipsoid.cartesian_to_cartographic(camera.position);
            if let Some(carto) = carto {
                let height = carto.height.max(1000.0);
                // In CV, position is in a flat coordinate system
                let pos = DVec3::new(
                    carto.longitude * ellipsoid.maximum_radius(),
                    carto.latitude * ellipsoid.maximum_radius(),
                    height,
                );
                let dir = -DVec3::Z;
                let up = DVec3::Y;
                (pos, dir, up)
            } else {
                (camera.position, camera.direction, camera.up)
            }
        }
        // Columbus View → 3D
        (SceneMode::ColumbusView, SceneMode::Scene3D) => {
            // Convert flat CV coordinates back to 3D
            let lon = camera.position.x / ellipsoid.maximum_radius();
            let lat = camera.position.y / ellipsoid.maximum_radius();
            let height = camera.position.z;
            let carto = cesium_geospatial::Cartographic::from_radians(lon, lat, height);
            let pos = ellipsoid.cartographic_to_cartesian(&carto);
            let dir = -pos.normalize();
            let up = DVec3::Z.cross(dir).normalize();
            let up = if up.length_squared() < 1e-10 { DVec3::Y } else { up };
            (pos, dir, up)
        }
        // Default: keep current state
        _ => (camera.position, camera.direction, camera.up),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_camera() -> Camera {
        Camera::new(
            DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    }

    #[test]
    fn test_morph_state_default() {
        let morph = SceneMorph::new();
        assert_eq!(morph.state, MorphState::Idle);
        assert!(!morph.is_morphing());
        assert!((morph.progress()).abs() < 1e-10);
    }

    #[test]
    fn test_start_morph() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::Scene2D,
            &Ellipsoid::WGS84,
            2.0,
        );

        assert!(morph.is_morphing());
        assert!((morph.progress()).abs() < 1e-10);
    }

    #[test]
    fn test_morph_same_mode_noop() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::Scene3D,
            &Ellipsoid::WGS84,
            2.0,
        );

        assert!(!morph.is_morphing());
    }

    #[test]
    fn test_morph_update() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();
        let mut camera = camera;

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::Scene2D,
            &Ellipsoid::WGS84,
            2.0,
        );

        // Halfway through
        let still_morphing = morph.update(1.0, &mut camera);
        assert!(still_morphing);
        assert!((morph.progress() - 0.5).abs() < 0.01);
        assert_eq!(camera.mode, SceneMode::Morphing);

        // Complete
        let still_morphing = morph.update(1.0, &mut camera);
        assert!(!still_morphing);
        assert!(!morph.is_morphing());
        assert_eq!(camera.mode, SceneMode::Scene2D);
    }

    #[test]
    fn test_complete_morph() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();
        let mut camera = camera;

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::Scene2D,
            &Ellipsoid::WGS84,
            2.0,
        );

        morph.update(0.5, &mut camera);
        morph.complete_morph(&mut camera);

        assert!(!morph.is_morphing());
        assert_eq!(camera.mode, SceneMode::Scene2D);
        assert!(camera.position.abs_diff_eq(morph.end_position, 1e-6));
    }

    #[test]
    fn test_cancel_morph() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();
        let mut camera = camera;
        let original_pos = camera.position;

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::Scene2D,
            &Ellipsoid::WGS84,
            2.0,
        );

        morph.update(0.5, &mut camera);
        morph.cancel_morph(&mut camera);

        assert!(!morph.is_morphing());
        assert_eq!(camera.mode, SceneMode::Scene3D);
        assert!(camera.position.abs_diff_eq(original_pos, 1e-6));
    }

    #[test]
    fn test_morph_3d_to_columbus_view() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();
        let mut camera = camera;

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::ColumbusView,
            &Ellipsoid::WGS84,
            1.0,
        );

        assert!(morph.is_morphing());

        // Complete the morph
        morph.update(1.0, &mut camera);
        assert_eq!(camera.mode, SceneMode::ColumbusView);
    }

    #[test]
    fn test_morph_duration_clamped() {
        let mut morph = SceneMorph::new();
        let camera = create_test_camera();

        morph.start_morph(
            &camera,
            SceneMode::Scene3D,
            SceneMode::Scene2D,
            &Ellipsoid::WGS84,
            0.0, // Should be clamped to 0.001
        );

        assert!(morph.duration >= 0.001);
    }
}
