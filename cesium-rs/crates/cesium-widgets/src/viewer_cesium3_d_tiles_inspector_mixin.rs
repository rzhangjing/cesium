//! Ported from `packages/widgets/Source/Viewer/viewer3dTilesInspectorMixin.js`.
//!
//! A mixin that adds the 3D Tiles Inspector widget.

/// Trait for the 3D Tiles Inspector.
///
/// In CesiumJS, this mixin adds a debugging panel for 3D Tiles that shows:
/// - Tileset statistics
/// - Tile tree visualization
/// - Point cloud shading options
/// - Debug colorization modes
pub trait ViewerCesium3DTilesInspectorMixin {
    /// Returns whether the inspector is visible.
    fn is_inspector_visible(&self) -> bool;

    /// Sets whether the inspector is visible.
    fn set_inspector_visible(&mut self, visible: bool);
}
