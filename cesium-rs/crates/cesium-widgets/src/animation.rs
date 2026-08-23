//! Ported from `packages/widgets/Source/Animation/Animation.js`.
//!
//! The animation controller widget.

/// The animation controller widget.
///
/// Provides play/pause/rewind/fast-forward controls and a timeline.
pub struct Animation {
    is_destroyed: bool,
}

impl Animation {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn resize(&mut self) {}
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Animation {
    fn default() -> Self { Self::new() }
}
