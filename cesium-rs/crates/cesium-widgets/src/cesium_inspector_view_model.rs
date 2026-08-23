//! Ported from `packages/widgets/Source/CesiumInspector/CesiumInspectorViewModel.js`.

/// The view model for the CesiumInspector widget.
pub struct CesiumInspectorViewModel {
    /// Whether the inspector is visible.
    pub is_visible: bool,
    /// Whether to show the frustum lines.
    pub show_frustum: bool,
    /// Whether to show the reference frame.
    pub show_reference_frame: bool,
    /// Whether to show performance statistics.
    pub show_performance: bool,
}

impl CesiumInspectorViewModel {
    /// Creates a new Cesium inspector view model.
    pub fn new() -> Self {
        Self {
            is_visible: false,
            show_frustum: false,
            show_reference_frame: false,
            show_performance: false,
        }
    }
}

impl Default for CesiumInspectorViewModel {
    fn default() -> Self { Self::new() }
}
