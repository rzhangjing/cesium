//! Scene modes and morphing between them.
//!
//! Maps to CesiumJS `Scene/SceneMode.js`:
//! - 3D (globe)
//! - 2D (flat map)
//! - Columbus View (2.5D)
//! - Morphing transitions

use glam::DVec3;
use std::f64::consts::PI;

/// Scene rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneMode {
    /// 3D globe view.
    #[default]
    Scene3D,
    /// 2D flat map (Web Mercator).
    Scene2D,
    /// Columbus View (2.5D perspective on flat map).
    ColumbusView,
    /// Morphing between modes.
    Morphing,
}

impl SceneMode {
    /// Returns true if this is a 3D mode.
    pub fn is_3d(&self) -> bool {
        matches!(self, Self::Scene3D)
    }

    /// Returns true if this is a 2D mode.
    pub fn is_2d(&self) -> bool {
        matches!(self, Self::Scene2D)
    }
}

/// Morphing state between scene modes.
#[derive(Debug, Clone)]
pub struct MorphState {
    /// Source mode.
    pub from: SceneMode,
    /// Target mode.
    pub to: SceneMode,
    /// Morph progress (0.0 = from, 1.0 = to).
    pub progress: f64,
    /// Whether morphing is active.
    pub active: bool,
    /// Duration of the morph in seconds.
    pub duration: f64,
    /// Elapsed time in seconds.
    pub elapsed: f64,
}

impl Default for MorphState {
    fn default() -> Self {
        Self {
            from: SceneMode::Scene3D,
            to: SceneMode::Scene3D,
            progress: 1.0,
            active: false,
            duration: 2.0,
            elapsed: 0.0,
        }
    }
}

impl MorphState {
    /// Starts a morph transition.
    pub fn start_morph(&mut self, from: SceneMode, to: SceneMode, duration_secs: f64) {
        self.from = from;
        self.to = to;
        self.progress = 0.0;
        self.active = true;
        self.duration = duration_secs;
        self.elapsed = 0.0;
    }

    /// Updates the morph progress.
    pub fn update(&mut self, delta_secs: f64) {
        if !self.active {
            return;
        }
        self.elapsed += delta_secs;
        self.progress = (self.elapsed / self.duration).clamp(0.0, 1.0);
        if self.progress >= 1.0 {
            self.active = false;
        }
    }

    /// Returns the current effective mode.
    pub fn current_mode(&self) -> SceneMode {
        if self.active {
            SceneMode::Morphing
        } else {
            self.to
        }
    }
}

/// Projects a 3D position to 2D map coordinates.
///
/// # Arguments
/// * `position` - 3D ECEF position
/// * `ellipsoid_radius` - Ellipsoid semi-major axis
///
/// # Returns
/// 2D position (x = longitude * radius, y = latitude * radius)
pub fn project_to_2d(position: DVec3, ellipsoid_radius: f64) -> DVec3 {
    let lon = position.y.atan2(position.x);
    let lat = (position.z / position.length()).asin();

    DVec3::new(
        lon * ellipsoid_radius,
        lat * ellipsoid_radius,
        position.length() - ellipsoid_radius,
    )
}

/// Unprojects 2D map coordinates to 3D ECEF position.
pub fn unproject_from_2d(position_2d: DVec3, ellipsoid_radius: f64) -> DVec3 {
    let lon = position_2d.x / ellipsoid_radius;
    let lat = position_2d.y / ellipsoid_radius;
    let height = position_2d.z;
    let r = ellipsoid_radius + height;

    DVec3::new(
        r * lat.cos() * lon.cos(),
        r * lat.cos() * lon.sin(),
        r * lat.sin(),
    )
}

/// Projects a 3D position to Columbus View coordinates.
///
/// Columbus View is a 2.5D projection where the map is flat
/// but viewed in perspective.
pub fn project_to_columbus_view(position: DVec3, ellipsoid_radius: f64) -> DVec3 {
    let lon = position.y.atan2(position.x);
    let lat = (position.z / position.length()).asin();
    let height = position.length() - ellipsoid_radius;

    // Columbus View: x = lon, y = lat, z = height (but in a plane)
    DVec3::new(
        lon * ellipsoid_radius,
        lat * ellipsoid_radius,
        height,
    )
}

/// Interpolates between 3D and 2D positions for morphing.
pub fn morph_position(
    position_3d: DVec3,
    position_2d: DVec3,
    progress: f64,
) -> DVec3 {
    // Smooth step for nicer transition
    let t = smoothstep(progress);
    position_3d.lerp(position_2d, t)
}

/// Smooth step function for easing.
pub fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Computes the camera position for a given scene mode.
pub fn compute_camera_for_mode(
    mode: SceneMode,
    center_lon: f64,
    center_lat: f64,
    height: f64,
    ellipsoid_radius: f64,
) -> DVec3 {
    match mode {
        SceneMode::Scene3D => {
            let r = ellipsoid_radius + height;
            DVec3::new(
                r * center_lat.cos() * center_lon.cos(),
                r * center_lat.cos() * center_lon.sin(),
                r * center_lat.sin(),
            )
        }
        SceneMode::Scene2D => {
            DVec3::new(
                center_lon * ellipsoid_radius,
                center_lat * ellipsoid_radius,
                height,
            )
        }
        SceneMode::ColumbusView => {
            DVec3::new(
                center_lon * ellipsoid_radius,
                center_lat * ellipsoid_radius,
                height,
            )
        }
        SceneMode::Morphing => {
            // Default to 3D during morphing
            compute_camera_for_mode(SceneMode::Scene3D, center_lon, center_lat, height, ellipsoid_radius)
        }
    }
}

/// Map projection for 2D mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapProjection2D {
    /// Geographic (equirectangular).
    #[default]
    Geographic,
    /// Web Mercator.
    WebMercator,
}

impl MapProjection2D {
    /// Projects geographic coordinates to 2D.
    pub fn project(&self, lon: f64, lat: f64, radius: f64) -> DVec3 {
        match self {
            Self::Geographic => DVec3::new(lon * radius, lat * radius, 0.0),
            Self::WebMercator => {
                let x = lon * radius;
                let y = (PI / 4.0 + lat / 2.0).tan().ln() * radius;
                DVec3::new(x, y, 0.0)
            }
        }
    }

    /// Unprojects 2D coordinates to geographic.
    pub fn unproject(&self, x: f64, y: f64, radius: f64) -> (f64, f64) {
        match self {
            Self::Geographic => (x / radius, y / radius),
            Self::WebMercator => {
                let lon = x / radius;
                let lat = 2.0 * (y / radius).exp().atan() - PI / 2.0;
                (lon, lat)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARTH_RADIUS: f64 = 6378137.0;

    #[test]
    fn test_scene_mode_default() {
        assert_eq!(SceneMode::default(), SceneMode::Scene3D);
    }

    #[test]
    fn test_scene_mode_is_3d() {
        assert!(SceneMode::Scene3D.is_3d());
        assert!(!SceneMode::Scene2D.is_3d());
        assert!(!SceneMode::ColumbusView.is_3d());
    }

    #[test]
    fn test_scene_mode_is_2d() {
        assert!(SceneMode::Scene2D.is_2d());
        assert!(!SceneMode::Scene3D.is_2d());
    }

    #[test]
    fn test_morph_state_default() {
        let state = MorphState::default();
        assert!(!state.active);
        assert_eq!(state.progress, 1.0);
    }

    #[test]
    fn test_morph_start() {
        let mut state = MorphState::default();
        state.start_morph(SceneMode::Scene3D, SceneMode::Scene2D, 2.0);
        assert!(state.active);
        assert_eq!(state.progress, 0.0);
        assert_eq!(state.from, SceneMode::Scene3D);
        assert_eq!(state.to, SceneMode::Scene2D);
    }

    #[test]
    fn test_morph_update() {
        let mut state = MorphState::default();
        state.start_morph(SceneMode::Scene3D, SceneMode::Scene2D, 2.0);

        state.update(1.0);
        assert!((state.progress - 0.5).abs() < 1e-10);
        assert!(state.active);

        state.update(1.0);
        assert!((state.progress - 1.0).abs() < 1e-10);
        assert!(!state.active);
    }

    #[test]
    fn test_morph_current_mode() {
        let mut state = MorphState::default();
        state.start_morph(SceneMode::Scene3D, SceneMode::Scene2D, 2.0);

        assert_eq!(state.current_mode(), SceneMode::Morphing);

        state.update(3.0); // Complete
        assert_eq!(state.current_mode(), SceneMode::Scene2D);
    }

    #[test]
    fn test_project_to_2d() {
        // Point on equator at prime meridian
        let pos = DVec3::new(EARTH_RADIUS, 0.0, 0.0);
        let pos_2d = project_to_2d(pos, EARTH_RADIUS);

        assert!(pos_2d.x.abs() < 1e-6); // lon = 0
        assert!(pos_2d.y.abs() < 1e-6); // lat = 0
        assert!(pos_2d.z.abs() < 1e-6); // height = 0
    }

    #[test]
    fn test_unproject_from_2d() {
        let pos_2d = DVec3::new(0.0, 0.0, 1000.0);
        let pos_3d = unproject_from_2d(pos_2d, EARTH_RADIUS);

        // Should be on equator at prime meridian, 1000m up
        let expected_r = EARTH_RADIUS + 1000.0;
        assert!((pos_3d.x - expected_r).abs() < 1.0);
        assert!(pos_3d.y.abs() < 1.0);
        assert!(pos_3d.z.abs() < 1.0);
    }

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(0.0) - 0.0).abs() < 1e-10);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-10);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_morph_position() {
        let pos_3d = DVec3::new(100.0, 0.0, 0.0);
        let pos_2d = DVec3::new(0.0, 100.0, 0.0);

        let mid = morph_position(pos_3d, pos_2d, 0.5);
        assert!((mid.x - 50.0).abs() < 1e-10);
        assert!((mid.y - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_camera_3d() {
        let cam = compute_camera_for_mode(SceneMode::Scene3D, 0.0, 0.0, 1000000.0, EARTH_RADIUS);
        let expected_r = EARTH_RADIUS + 1000000.0;
        assert!((cam.x - expected_r).abs() < 1.0);
    }

    #[test]
    fn test_compute_camera_2d() {
        let cam = compute_camera_for_mode(SceneMode::Scene2D, 0.5, 0.3, 1000000.0, EARTH_RADIUS);
        assert!((cam.x - 0.5 * EARTH_RADIUS).abs() < 1.0);
        assert!((cam.y - 0.3 * EARTH_RADIUS).abs() < 1.0);
        assert!((cam.z - 1000000.0).abs() < 1.0);
    }

    #[test]
    fn test_map_projection_geographic() {
        let proj = MapProjection2D::Geographic;
        let pos = proj.project(0.5, 0.3, EARTH_RADIUS);
        assert!((pos.x - 0.5 * EARTH_RADIUS).abs() < 1.0);
        assert!((pos.y - 0.3 * EARTH_RADIUS).abs() < 1.0);

        let (lon, lat) = proj.unproject(pos.x, pos.y, EARTH_RADIUS);
        assert!((lon - 0.5).abs() < 1e-10);
        assert!((lat - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_map_projection_web_mercator() {
        let proj = MapProjection2D::WebMercator;
        let pos = proj.project(0.0, 0.0, EARTH_RADIUS);
        assert!(pos.x.abs() < 1e-6);
        assert!(pos.y.abs() < 1e-6);

        let (lon, lat) = proj.unproject(0.0, 0.0, EARTH_RADIUS);
        assert!(lon.abs() < 1e-10);
        assert!(lat.abs() < 1e-10);
    }
}
