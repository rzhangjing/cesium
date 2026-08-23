//! Ported from `packages/widgets/Source/I3SBuildingSceneLayerExplorer/I3SBuildingSceneLayerExplorer.js`.

/// The I3SBuildingSceneLayerExplorer widget.
pub struct I3SBuildingSceneLayerExplorer {
    is_destroyed: bool,
}

impl I3SBuildingSceneLayerExplorer {
    /// Creates a new I3SBuildingSceneLayerExplorer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this widget.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for I3SBuildingSceneLayerExplorer {
    fn default() -> Self { Self::new() }
}
