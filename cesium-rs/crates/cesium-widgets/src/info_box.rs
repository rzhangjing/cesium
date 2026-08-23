//! Ported from `packages/widgets/Source/InfoBox/InfoBox.js`.

/// The InfoBox widget that provides an information panel for selected entities.
pub struct InfoBox {
    is_destroyed: bool,
}

impl InfoBox {
    /// Creates a new InfoBox widget.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for InfoBox {
    fn default() -> Self { Self::new() }
}
