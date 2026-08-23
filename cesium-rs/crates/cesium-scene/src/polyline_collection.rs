//! Ported from `packages/engine/Source/Scene/PolylineCollection.js`.
//!
//! A collection of polylines.

use crate::frame_state::FrameState;
use crate::polyline::Polyline;

/// A collection of polylines for efficient rendering of many lines.
///
/// Mirrors CesiumJS `PolylineCollection` (763 lines).
pub struct PolylineCollection {
    /// Whether this collection is shown.
    pub show: bool,
    /// The polylines in this collection.
    polylines: Vec<Polyline>,
    /// Whether this collection has been destroyed.
    is_destroyed: bool,
}

impl PolylineCollection {
    /// Creates a new PolylineCollection.
    pub fn new() -> Self {
        Self {
            show: true,
            polylines: Vec::new(),
            is_destroyed: false,
        }
    }

    /// Adds a polyline to the collection.
    pub fn add(&mut self, polyline: Polyline) -> usize {
        let index = self.polylines.len();
        self.polylines.push(polyline);
        index
    }

    /// Removes a polyline from the collection by index.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.polylines.len() {
            self.polylines.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes all polylines from the collection.
    pub fn remove_all(&mut self) {
        self.polylines.clear();
    }

    /// Gets a polyline by index.
    pub fn get(&self, index: usize) -> Option<&Polyline> {
        self.polylines.get(index)
    }

    /// Returns the number of polylines.
    pub fn len(&self) -> usize {
        self.polylines.len()
    }

    /// Returns whether the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.polylines.is_empty()
    }

    /// Updates the collection for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires GPU buffer management
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

impl Default for PolylineCollection {
    fn default() -> Self { Self::new() }
}
