//! Ported from `packages/engine/Source/Scene/ImageryLayerCollection.js`.
//!
//! An ordered collection of imagery layers.

use crate::imagery_layer::ImageryLayer;

/// An ordered collection of imagery layers.
///
/// Layers are rendered bottom-to-top (index 0 is the bottom layer).
pub struct ImageryLayerCollection {
    layers: Vec<ImageryLayer>,
    is_destroyed: bool,
}

impl ImageryLayerCollection {
    /// Creates a new empty imagery layer collection.
    pub fn new() -> Self {
        Self { layers: Vec::new(), is_destroyed: false }
    }

    /// Returns the number of layers.
    pub fn length(&self) -> usize { self.layers.len() }

    /// Returns the layer at the given index.
    pub fn get(&self, index: usize) -> Option<&ImageryLayer> { self.layers.get(index) }

    /// Returns a mutable reference to the layer at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ImageryLayer> { self.layers.get_mut(index) }

    /// Adds a layer to the collection.
    pub fn add(&mut self, layer: ImageryLayer) {
        self.layers.push(layer);
    }

    /// Adds a layer at the given index.
    pub fn add_at(&mut self, index: usize, layer: ImageryLayer) {
        if index >= self.layers.len() {
            self.layers.push(layer);
        } else {
            self.layers.insert(index, layer);
        }
    }

    /// Removes a layer from the collection.
    pub fn remove(&mut self, index: usize) -> Option<ImageryLayer> {
        if index < self.layers.len() {
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Removes all layers from the collection.
    pub fn remove_all(&mut self) {
        self.layers.clear();
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this collection.
    pub fn destroy(&mut self) {
        self.layers.clear();
        self.is_destroyed = true;
    }
}

impl Default for ImageryLayerCollection {
    fn default() -> Self { Self::new() }
}
