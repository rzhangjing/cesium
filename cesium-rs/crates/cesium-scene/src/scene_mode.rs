//! Ported from `packages/engine/Source/Scene/SceneMode.js`.
//!
//! Indicates if the scene is viewed in 3D, 2D, or 2.5D Columbus view.

/// Indicates the current scene viewing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SceneMode {
    /// Morphing between modes (e.g., 3D to 2D).
    Morphing = 0,
    /// Columbus View mode 鈥?a 2.5D perspective where the map is flat.
    ColumbusView = 1,
    /// 2D mode 鈥?top-down orthographic projection.
    Scene2D = 2,
    /// 3D mode 鈥?traditional 3D perspective of the globe.
    Scene3D = 3,
}

impl SceneMode {
    /// Returns the morph time for the given scene mode.
    pub fn get_morph_time(mode: SceneMode) -> Option<f64> {
        match mode {
            SceneMode::Scene3D => Some(1.0),
            SceneMode::Morphing => None,
            _ => Some(0.0),
        }
    }

    /// Returns whether the mode is a 3D mode.
    pub fn is_3d(mode: SceneMode) -> bool {
        mode == SceneMode::Scene3D
    }

    /// Returns whether the mode is a 2D or Columbus View mode.
    pub fn is_2d(mode: SceneMode) -> bool {
        mode == SceneMode::Scene2D || mode == SceneMode::ColumbusView
    }
}
