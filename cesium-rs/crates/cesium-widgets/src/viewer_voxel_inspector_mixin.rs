//! Ported from `packages/widgets/Source/Viewer/viewerVoxelInspectorMixin.js`.
//!
//! A mixin that adds the Voxel Inspector widget.

/// Trait for the Voxel Inspector.
///
/// In CesiumJS, this mixin adds a debugging panel for VoxelPrimitive
/// that shows voxel shader properties and visualization options.
pub trait ViewerVoxelInspectorMixin {
    /// Returns whether the inspector is visible.
    fn is_inspector_visible(&self) -> bool;

    /// Sets whether the inspector is visible.
    fn set_inspector_visible(&mut self, visible: bool);
}
