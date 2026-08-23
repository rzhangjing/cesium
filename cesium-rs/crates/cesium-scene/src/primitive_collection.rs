//! Ported from `packages/engine/Source/Scene/PrimitiveCollection.js`.

/// A collection of primitives.
///
/// Primitives are the basic rendering units in the scene.
pub struct PrimitiveCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The primitives in this collection.
    primitives: Vec<usize>,
    is_destroyed: bool,
}

impl PrimitiveCollection {
    /// Creates a new primitive collection.
    pub fn new() -> Self {
        Self { show: true, primitives: Vec::new(), is_destroyed: false }
    }

    /// Adds a primitive to the collection.
    pub fn add(&mut self, _index: usize) {
        self.primitives.push(_index);
    }

    /// Removes a primitive from the collection.
    pub fn remove(&mut self, index: usize) -> bool {
        if let Some(pos) = self.primitives.iter().position(|&x| x == index) {
            self.primitives.remove(pos);
            true
        } else {
            false
        }
    }

    /// Removes all primitives from the collection.
    pub fn remove_all(&mut self) {
        self.primitives.clear();
    }

    /// Returns the number of primitives.
    pub fn len(&self) -> usize { self.primitives.len() }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool { self.primitives.is_empty() }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this collection.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for PrimitiveCollection {
    fn default() -> Self { Self::new() }
}
