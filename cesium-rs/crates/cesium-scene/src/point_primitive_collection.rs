//! Ported from `packages/engine/Source/Scene/PointPrimitiveCollection.js`.
//!
//! A collection of point primitives.

use crate::frame_state::FrameState;
use crate::point_primitive::PointPrimitive;

/// A collection of point primitives for efficient rendering of many points.
///
/// Mirrors CesiumJS `PointPrimitiveCollection` (813 lines).
pub struct PointPrimitiveCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The point primitives in this collection.
    points: Vec<PointPrimitive>,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
}

impl PointPrimitiveCollection {
    /// Creates a new PointPrimitiveCollection.
    pub fn new() -> Self {
        Self {
            show: true,
            points: Vec::new(),
            is_destroyed: false,
        }
    }

    /// Adds a point to the collection.
    pub fn add(&mut self, point: PointPrimitive) -> usize {
        let index = self.points.len();
        self.points.push(point);
        index
    }

    /// Removes a point from the collection by index.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.points.len() {
            self.points.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes all points from the collection.
    pub fn remove_all(&mut self) {
        self.points.clear();
    }

    /// Gets a point by index.
    pub fn get(&self, index: usize) -> Option<&PointPrimitive> {
        self.points.get(index)
    }

    /// Gets a mutable reference to a point by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PointPrimitive> {
        self.points.get_mut(index)
    }

    /// Returns the number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Updates the collection for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires GPU buffer management and draw command generation
    }

    /// Returns whether this collection has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this collection.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for PointPrimitiveCollection {
    fn default() -> Self { Self::new() }
}
