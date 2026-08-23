//! Ported from `packages/widgets/Source/I3SBuildingSceneLayerExplorer/I3SBuildingSceneLayerExplorerViewModel.js`.

/// The view model for the I3S Building Scene Layer Explorer widget.
pub struct I3sBuildingSceneLayerExplorerViewModel {
    /// Whether the explorer is visible.
    pub is_visible: bool,
}

impl I3sBuildingSceneLayerExplorerViewModel {
    /// Creates a new I3S building scene layer explorer view model.
    pub fn new() -> Self {
        Self { is_visible: false }
    }
}

impl Default for I3sBuildingSceneLayerExplorerViewModel {
    fn default() -> Self { Self::new() }
}
