//! Ported from `packages/widgets/Source/SceneModePicker/SceneModePicker.js`.
//!
//! A widget for switching between 2D, 3D, and Columbus View modes.

/// A widget for switching between 2D, 3D, and Columbus View modes.
pub struct SceneModePicker {
    is_destroyed: bool,
}

impl SceneModePicker {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for SceneModePicker {
    fn default() -> Self { Self::new() }
}
