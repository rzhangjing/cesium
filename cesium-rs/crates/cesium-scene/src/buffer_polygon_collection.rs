//! Ported from `packages/engine/Source/Scene/BufferPolygonCollection.js`.

/// A collection of buffer polygons.
///
/// Manages a batch of polygon primitives for efficient rendering.
pub struct BufferPolygonCollection {
    /// Number of polygons in the collection.
    pub length: u32,
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl BufferPolygonCollection {
    /// Creates a new BufferPolygonCollection.
    pub fn new() -> Self {
        Self { length: 0, show: true, needs_update: false }
    }

    /// Returns the number of polygons.
    pub fn len(&self) -> u32 { self.length }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool { self.length == 0 }
}

impl Default for BufferPolygonCollection {
    fn default() -> Self { Self::new() }
}
