//! Ported from `packages/widgets/Source/HomeButton/HomeButton.js`.
//!
//! A button that flies the camera to the default view.

/// A button that flies the camera to the default view.
pub struct HomeButton {
    is_destroyed: bool,
}

impl HomeButton {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for HomeButton {
    fn default() -> Self { Self::new() }
}
