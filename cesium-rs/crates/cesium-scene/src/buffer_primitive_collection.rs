//! Ported from `packages/engine/Source/Scene/BufferPrimitiveCollection.js`.

/// A collection of buffer primitives.
///
/// Manages a batch of geometry primitives for efficient rendering.
pub struct BufferPrimitiveCollection {
    /// Number of primitives in the collection.
    pub length: u32,
    /// Whether the collection is visible.
    pub show: bool,
    /// Whether the collection needs update.
    pub needs_update: bool,
}

impl BufferPrimitiveCollection {
    /// Creates a new BufferPrimitiveCollection.
    pub fn new() -> Self {
        Self { length: 0, show: true, needs_update: false }
    }

    /// Returns the number of primitives.
    pub fn len(&self) -> u32 { self.length }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool { self.length == 0 }
}

impl Default for BufferPrimitiveCollection {
    fn default() -> Self { Self::new() }
}
