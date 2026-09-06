//! Ported from `packages/engine/Source/Scene/BufferPointCollection.js`.

/// A collection of buffer points.
///
/// Manages a batch of point primitives for efficient rendering.
pub struct BufferPointCollection {
    /// Number of points in the collection.
    pub length: u32,
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl BufferPointCollection {
    /// Creates a new BufferPointCollection.
    pub fn new() -> Self {
        Self { length: 0, show: true, needs_update: false }
    }

    /// Returns the number of points.
    pub fn len(&self) -> u32 { self.length }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool { self.length == 0 }
}

impl Default for BufferPointCollection {
    fn default() -> Self { Self::new() }
}
