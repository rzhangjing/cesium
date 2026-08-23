//! Ported from `packages/widgets/Source/Cesium3DTilesInspector/Cesium3DTilesInspector.js`.

/// The Cesium3DTilesInspector widget.
pub struct Cesium3DTilesInspector {
    is_destroyed: bool,
}

impl Cesium3DTilesInspector {
    /// Creates a new Cesium3DTilesInspector.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Cesium3DTilesInspector {
    fn default() -> Self { Self::new() }
}
