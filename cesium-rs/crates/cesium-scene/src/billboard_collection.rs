//! Ported from `packages/engine/Source/Scene/BillboardCollection.js`.
//!
//! A renderable collection of 2D billboards that always face the camera.

use cesium_core::cartesian3::Cartesian3;
use crate::billboard::Billboard;
use crate::frame_state::FrameState;

/// A renderable collection of 2D billboards that always face the camera.
///
/// Billboards are screen-aligned images positioned at 3D world coordinates.
/// The collection manages GPU resources efficiently for batch rendering.
pub struct BillboardCollection {
    /// The billboards in this collection.
    billboards: Vec<Billboard>,
    /// Whether this collection is shown.
    pub show: bool,
    /// Whether the collection has been modified since the last render.
    dirty: bool,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
}

impl BillboardCollection {
    /// Creates a new BillboardCollection.
    pub fn new() -> Self {
        Self {
            billboards: Vec::new(),
            show: true,
            dirty: true,
            is_destroyed: false,
        }
    }

    /// Adds a billboard to the collection and returns its index.
    pub fn add(&mut self, billboard: Billboard) -> usize {
        self.dirty = true;
        self.billboards.push(billboard);
        self.billboards.len() - 1
    }

    /// Returns the number of billboards.
    pub fn len(&self) -> usize { self.billboards.len() }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool { self.billboards.is_empty() }

    /// Returns the billboard at the given index.
    pub fn get(&self, index: usize) -> Option<&Billboard> { self.billboards.get(index) }

    /// Returns a mutable reference to the billboard at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Billboard> { self.billboards.get_mut(index) }

    /// Removes the billboard at the given index.
    pub fn remove(&mut self, index: usize) -> Option<Billboard> {
        if index < self.billboards.len() {
            self.dirty = true;
            Some(self.billboards.remove(index))
        } else {
            None
        }
    }

    /// Removes all billboards.
    pub fn remove_all(&mut self) {
        self.billboards.clear();
        self.dirty = true;
    }

    /// Updates the collection for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show { return; }
        // In full port: upload billboard data to GPU, generate draw commands
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.billboards.clear();
        self.is_destroyed = true;
    }
}

impl Default for BillboardCollection {
    fn default() -> Self { Self::new() }
}
