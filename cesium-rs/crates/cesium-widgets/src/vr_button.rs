//! Ported from `packages/widgets/Source/VRButton/VRButton.js`.
//!
//! A button that enables VR mode.

/// A button that enables VR mode.
pub struct VrButton {
    is_destroyed: bool,
}

impl VrButton {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for VrButton {
    fn default() -> Self { Self::new() }
}
