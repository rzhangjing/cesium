//! Ported from `packages/widgets/Source/SelectionIndicator/SelectionIndicator.js`.
//!
//! A widget that displays a selection indicator at an entity's position.

/// A widget that displays a selection indicator at an entity's position.
pub struct SelectionIndicator {
    is_destroyed: bool,
}

impl SelectionIndicator {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for SelectionIndicator {
    fn default() -> Self { Self::new() }
}
