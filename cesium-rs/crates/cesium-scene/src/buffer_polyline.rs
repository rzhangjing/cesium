//! Ported from `packages/engine/Source/Scene/BufferPolyline.js`.

/// A polyline in a buffer polyline collection.
///
/// Represents a single polyline with positions and styling.
pub struct BufferPolyline {
    /// The polyline positions as (longitude, latitude) pairs.
    pub positions: Vec<(f64, f64)>,
    /// Line width in pixels.
    pub width: f32,
    /// Whether this polyline is visible.
    pub show: bool,
}

impl BufferPolyline {
    /// Creates a new BufferPolyline.
    pub fn new() -> Self {
        Self { positions: Vec::new(), width: 1.0, show: true }
    }
}

impl Default for BufferPolyline {
    fn default() -> Self { Self::new() }
}
