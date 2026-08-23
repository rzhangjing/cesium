//! Ported from `packages/widgets/Source/Timeline/Timeline.js`.
//!
//! A timeline widget for visualizing time-based data.

/// A timeline widget for visualizing time-based data.
pub struct Timeline {
    is_destroyed: bool,
}

impl Timeline {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn resize(&mut self) {}
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Timeline {
    fn default() -> Self { Self::new() }
}
