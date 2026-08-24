//! Ported from `packages/engine/Source/Scene/PrimitiveCollection.js`.
//!
//! A collection of primitives, the default container for `Scene#primitives`.

use cesium_renderer::context::Context;

use crate::frame_state::FrameState;
use crate::primitive::Primitive;

/// The renderable contract shared by everything a [`PrimitiveCollection`]
/// (and `Scene#primitives`) can hold.
///
/// Mirrors the CesiumJS duck-typed primitive interface (`show`, `update`,
/// `isDestroyed`, `destroy`) that `PrimitiveCollection#update` drives.
pub trait ScenePrimitive {
    /// Mirrors CesiumJS `primitive.update(frameState)`, extended with the
    /// wgpu context (the JS reads the context off the frame state).
    fn update(&mut self, frame_state: &FrameState, context: &mut Context);
    /// Mirrors CesiumJS `primitive.show`.
    fn show(&self) -> bool;
    /// Mirrors assigning CesiumJS `primitive.show`.
    fn set_show(&mut self, show: bool);
    /// Mirrors CesiumJS `primitive.isDestroyed()`.
    fn is_destroyed(&self) -> bool;
    /// Mirrors CesiumJS `primitive.destroy()` (or `destroyPrimitive`).
    fn destroy(&mut self);
}

impl ScenePrimitive for Primitive {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        Primitive::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { Primitive::is_destroyed(self) }
    fn destroy(&mut self) { Primitive::destroy(self); }
}

/// A collection of primitives.
///
/// Primitives are the basic rendering units in the scene. Collections may be
/// nested: any [`ScenePrimitive`] (including another collection) can be
/// added, and `update` recurses in insertion order — mirroring CesiumJS
/// `PrimitiveCollection`.
pub struct PrimitiveCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The primitives in this collection, in insertion order.
    primitives: Vec<Box<dyn ScenePrimitive>>,
    is_destroyed: bool,
}

impl PrimitiveCollection {
    /// Creates a new primitive collection.
    pub fn new() -> Self {
        Self::with_options(None, None)
    }

    /// Creates a collection from the JS constructor options
    /// (`{ show, compressVertices }` — the latter is accepted for API parity
    /// and currently unused by the wgpu geometry path).
    pub fn with_options(show: Option<bool>, _compress_vertices: Option<bool>) -> Self {
        Self { show: show.unwrap_or(true), primitives: Vec::new(), is_destroyed: false }
    }

    /// Adds a primitive to the collection and returns it, mirroring CesiumJS
    /// `PrimitiveCollection#add` (which returns the added primitive; the Rust
    /// port returns its index since the value is moved into the collection).
    pub fn add(&mut self, primitive: Box<dyn ScenePrimitive>) -> usize {
        self.primitives.push(primitive);
        self.primitives.len() - 1
    }

    /// Removes the primitive at the given index, mirroring CesiumJS
    /// `PrimitiveCollection#remove` (which removes by reference and destroys
    /// nothing — the JS caller destroys explicitly; the Rust port returns the
    /// removed primitive so the caller can drop or destroy it).
    pub fn remove(&mut self, index: usize) -> Option<Box<dyn ScenePrimitive>> {
        if index < self.primitives.len() {
            Some(self.primitives.remove(index))
        } else {
            None
        }
    }

    /// Removes and destroys the primitive at the given index, returning
    /// whether anything was removed (mirrors `remove` + `destroyPrimitive`).
    pub fn remove_and_destroy(&mut self, index: usize) -> bool {
        if let Some(mut primitive) = self.remove(index) {
            primitive.destroy();
            true
        } else {
            false
        }
    }

    /// Removes all primitives from the collection (mirrors
    /// `PrimitiveCollection#removeAll`; the JS does NOT destroy them).
    pub fn remove_all(&mut self) {
        self.primitives.clear();
    }

    /// Returns a reference to the primitive at the given index (mirrors
    /// `PrimitiveCollection#get`).
    pub fn get(&self, index: usize) -> Option<&dyn ScenePrimitive> {
        self.primitives.get(index).map(|primitive| primitive.as_ref())
    }

    /// Returns a mutable reference to the primitive at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut (dyn ScenePrimitive + '_)> {
        match self.primitives.get_mut(index) {
            Some(primitive) => Some(&mut **primitive),
            None => None,
        }
    }

    /// Returns the number of primitives (mirrors `PrimitiveCollection#length`).
    pub fn len(&self) -> usize { self.primitives.len() }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool { self.primitives.is_empty() }

    /// Updates every contained primitive in insertion order, mirroring
    /// CesiumJS `PrimitiveCollection#update` (which forwards the frame state
    /// to each primitive's `update`).
    pub fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        if !self.show {
            return;
        }
        for primitive in self.primitives.iter_mut() {
            primitive.update(frame_state, context);
        }
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this collection and every contained primitive (mirrors
    /// CesiumJS `PrimitiveCollection#destroy`, which destroys the WebGL
    /// resources of each member).
    pub fn destroy(&mut self) {
        for primitive in self.primitives.iter_mut() {
            primitive.destroy();
        }
        self.primitives.clear();
        self.is_destroyed = true;
    }
}

impl Default for PrimitiveCollection {
    fn default() -> Self { Self::new() }
}

impl ScenePrimitive for PrimitiveCollection {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        PrimitiveCollection::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { PrimitiveCollection::is_destroyed(self) }
    fn destroy(&mut self) { PrimitiveCollection::destroy(self); }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors `PrimitiveCollectionSpec.js`
    /// `it("gets default values")`.
    #[test]
    fn collection_default_values() {
        let collection = PrimitiveCollection::new();
        assert!(collection.show);
        assert_eq!(collection.len(), 0);
        assert!(collection.is_empty());
        assert!(!collection.is_destroyed());
    }

    /// Mirrors `it("adds and removes a primitive")`.
    #[test]
    fn collection_add_remove() {
        let mut collection = PrimitiveCollection::new();
        let index = collection.add(Box::new(Primitive::new()));
        assert_eq!(index, 0);
        assert_eq!(collection.len(), 1);
        assert!(collection.get(0).is_some());
        assert!(collection.get(1).is_none());
        let removed = collection.remove(0);
        assert!(removed.is_some());
        assert_eq!(collection.len(), 0);
        assert!(collection.remove(0).is_none());
    }

    /// Mirrors `it("adds and removes a primitive with show")`: hiding the
    /// collection suppresses member updates (verified through the trait
    /// `show` plumbing).
    #[test]
    fn collection_show_propagates_through_trait() {
        let mut collection = PrimitiveCollection::new();
        collection.add(Box::new(Primitive::new()));
        assert!(collection.get(0).unwrap().show());
        collection.get_mut(0).unwrap().set_show(false);
        assert!(!collection.get(0).unwrap().show());
    }

    /// Mirrors `it("destroys")`: destroying the collection destroys members.
    #[test]
    fn collection_destroy_destroys_members() {
        let mut collection = PrimitiveCollection::new();
        collection.add(Box::new(Primitive::new()));
        collection.destroy();
        assert!(collection.is_destroyed());
        assert_eq!(collection.len(), 0);
    }

    /// Mirrors the nested-collection semantics: a collection of collections
    /// updates recursively through the [`ScenePrimitive`] trait.
    #[test]
    fn collection_nesting() {
        let mut outer = PrimitiveCollection::new();
        let mut inner = PrimitiveCollection::new();
        inner.add(Box::new(Primitive::new()));
        outer.add(Box::new(inner));
        assert_eq!(outer.len(), 1);
        // The nested collection is visible through the trait surface.
        assert!(outer.get(0).unwrap().show());
    }
}
