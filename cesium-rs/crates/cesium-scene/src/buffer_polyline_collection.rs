//! Ported from `packages/engine/Source/Scene/BufferPolylineCollection.js`.

/// A collection of buffer polylines.
///
/// Manages a batch of polyline primitives for efficient rendering.
pub struct BufferPolylineCollection {
    /// Number of polylines in the collection.
    pub length: u32,
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl BufferPolylineCollection {
    /// Creates a new BufferPolylineCollection.
    pub fn new() -> Self {
        Self { length: 0, show: true, needs_update: false }
    }

    /// Returns the number of polylines.
    pub fn len(&self) -> u32 { self.length }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool { self.length == 0 }
}

impl Default for BufferPolylineCollection {
    fn default() -> Self { Self::new() }
}
