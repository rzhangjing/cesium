//! Ported from `packages/widgets/Source/Viewer/viewerDragDropMixin.js`.
//!
//! A mixin that adds drag-and-drop support to the Viewer.

/// Trait for Viewer drag-and-drop functionality.
///
/// In CesiumJS, this mixin adds the ability to drag-and-drop CZML, GeoJSON,
/// KML, and other files onto the Viewer canvas to load them as data sources.
pub trait ViewerDragDropMixin {
    /// Returns whether drag-and-drop is enabled.
    fn is_drag_drop_enabled(&self) -> bool;

    /// Enables or disables drag-and-drop.
    fn set_drag_drop_enabled(&mut self, enabled: bool);

    /// Returns the drop error message, if any.
    fn drop_error(&self) -> Option<&str>;
}

/// Configuration for the drag-drop mixin.
pub struct DragDropMixinOptions {
    /// Whether drag-and-drop is enabled by default.
    pub enabled: bool,
    /// Whether to clear existing data sources on drop.
    pub clear_on_drop: bool,
    /// Whether to clamp loaded GeoJSON/KML to ground.
    pub clamp_to_ground: bool,
}

impl Default for DragDropMixinOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            clear_on_drop: true,
            clamp_to_ground: false,
        }
    }
}
