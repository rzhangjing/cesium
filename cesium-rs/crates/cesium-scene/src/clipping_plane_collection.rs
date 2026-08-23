//! Ported from `packages/engine/Source/Scene/ClippingPlaneCollection.js`.
//!
//! A collection of clipping planes.

use crate::clipping_plane::ClippingPlane;
use crate::frame_state::FrameState;

/// A collection of clipping planes used to clip models and terrain.
///
/// Mirrors CesiumJS `ClippingPlaneCollection` (558 lines).
pub struct ClippingPlaneCollection {
    /// Whether this collection is enabled.
    pub enabled: bool,
    /// Whether to apply edge styling.
    pub edge_color: cesium_core::color::Color,
    /// The edge width.
    pub edge_width: f64,
    /// The union flag (if true, clipping uses union of planes).
    pub union_clipping_regions: bool,
    /// The clipping planes.
    planes: Vec<ClippingPlane>,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
}

impl ClippingPlaneCollection {
    /// Creates a new ClippingPlaneCollection.
    pub fn new() -> Self {
        Self {
            enabled: true,
            edge_color: cesium_core::color::Color::new(1.0, 1.0, 1.0, 0.5),
            edge_width: 0.0,
            union_clipping_regions: false,
            planes: Vec::new(),
            is_destroyed: false,
        }
    }

    /// Adds a clipping plane.
    pub fn add(&mut self, plane: ClippingPlane) {
        self.planes.push(plane);
    }

    /// Removes a clipping plane by index.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.planes.len() {
            self.planes.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes all clipping planes.
    pub fn remove_all(&mut self) {
        self.planes.clear();
    }

    /// Gets a clipping plane by index.
    pub fn get(&self, index: usize) -> Option<&ClippingPlane> {
        self.planes.get(index)
    }

    /// Returns the number of clipping planes.
    pub fn len(&self) -> usize {
        self.planes.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.planes.is_empty()
    }

    /// Updates the collection for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires shader uniform updates
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this collection.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for ClippingPlaneCollection {
    fn default() -> Self { Self::new() }
}
