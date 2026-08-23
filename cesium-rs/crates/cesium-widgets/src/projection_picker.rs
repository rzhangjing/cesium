//! Ported from `packages/widgets/Source/ProjectionPicker/ProjectionPicker.js`.

/// The ProjectionPicker widget that provides a projection mode switcher (perspective/orthographic).
pub struct ProjectionPicker {
    is_destroyed: bool,
}

impl ProjectionPicker {
    /// Creates a new ProjectionPicker widget.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ProjectionPicker {
    fn default() -> Self { Self::new() }
}
