//! Ported from `packages/widgets/Source/CesiumInspector/CesiumInspector.js`.

/// The CesiumInspector widget.
pub struct CesiumInspector {
    is_destroyed: bool,
}

impl CesiumInspector {
    /// Creates a new CesiumInspector.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for CesiumInspector {
    fn default() -> Self { Self::new() }
}
