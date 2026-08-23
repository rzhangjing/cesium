//! Ported from `packages/widgets/Source/VoxelInspector/VoxelInspectorViewModel.js`.

/// The view model for the VoxelInspector widget.
pub struct VoxelInspectorViewModel {
    /// Whether the inspector is visible.
    pub is_visible: bool,
}

impl VoxelInspectorViewModel {
    /// Creates a new voxel inspector view model.
    pub fn new() -> Self {
        Self { is_visible: false }
    }
}

impl Default for VoxelInspectorViewModel {
    fn default() -> Self { Self::new() }
}
