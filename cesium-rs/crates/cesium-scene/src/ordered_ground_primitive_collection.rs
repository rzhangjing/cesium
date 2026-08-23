//! Ported from `packages/engine/Source/Scene/OrderedGroundPrimitiveCollection.js`.

/// A collection of ground primitives ordered by render priority.
pub struct OrderedGroundPrimitiveCollection {
    is_destroyed: bool,
}

impl OrderedGroundPrimitiveCollection {
    /// Creates a new ordered ground primitive collection.
    pub fn new() -> Self { Self { is_destroyed: false } }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this collection.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for OrderedGroundPrimitiveCollection {
    fn default() -> Self { Self::new() }
}
