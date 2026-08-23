//! Ported from `packages/widgets/Source/Cesium3DTilesInspector/Cesium3DTilesInspectorViewModel.js`.

/// The view model for the Cesium3DTilesInspector widget.
pub struct Cesium3DTilesInspectorViewModel {
    /// Whether the inspector is visible.
    pub is_visible: bool,
    /// The maximum screen space error.
    pub maximum_screen_space_error: f64,
    /// Whether to show the bounding volumes.
    pub show_bounding_volumes: bool,
    /// Whether to show the rendering statistics.
    pub show_rendering_statistics: bool,
}

impl Cesium3DTilesInspectorViewModel {
    /// Creates a new 3D Tiles inspector view model.
    pub fn new() -> Self {
        Self {
            is_visible: false,
            maximum_screen_space_error: 16.0,
            show_bounding_volumes: false,
            show_rendering_statistics: false,
        }
    }
}

impl Default for Cesium3DTilesInspectorViewModel {
    fn default() -> Self { Self::new() }
}
