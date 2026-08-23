//! Ported from `packages/widgets/Source/FullscreenButton/FullscreenButton.js`.
//!
//! A button that toggles fullscreen mode.

/// A button that toggles fullscreen mode.
pub struct FullscreenButton {
    is_destroyed: bool,
}

impl FullscreenButton {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for FullscreenButton {
    fn default() -> Self { Self::new() }
}
