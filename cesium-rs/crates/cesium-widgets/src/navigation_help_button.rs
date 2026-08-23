//! Ported from `packages/widgets/Source/NavigationHelpButton/NavigationHelpButton.js`.
//!
//! A button that displays navigation help.

/// A button that displays navigation help.
pub struct NavigationHelpButton {
    is_destroyed: bool,
}

impl NavigationHelpButton {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for NavigationHelpButton {
    fn default() -> Self { Self::new() }
}
