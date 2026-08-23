//! Ported from `packages/widgets/Source/Viewer/viewerCesiumInspectorMixin.js`.
//!
//! A mixin that adds the Cesium Inspector widget.

/// Trait for the Cesium Inspector.
///
/// In CesiumJS, this mixin adds a debugging panel that shows:
/// - Scene graph information
/// - Shader details
/// - Render statistics
/// - Primitive list
pub trait ViewerCesiumInspectorMixin {
    /// Returns whether the inspector is visible.
    fn is_inspector_visible(&self) -> bool;

    /// Sets whether the inspector is visible.
    fn set_inspector_visible(&mut self, visible: bool);
}
