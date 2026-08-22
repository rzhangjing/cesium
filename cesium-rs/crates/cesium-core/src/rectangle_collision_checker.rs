//! Ported from `packages/engine/Source/Core/RectangleCollisionChecker.js`.
//!
//! Simple rectangle collision checker. In the JS original this uses `rbush` (an R-tree);
//! here we use a simple brute-force linear scan, which is sufficient for the small
//! rectangle counts typical in Cesium's internal usage.

use crate::rectangle::Rectangle;

/// Wrapper around a list of rectangles for collision detection.
pub struct RectangleCollisionChecker {
    rects: Vec<RectangleWithId>,
}

struct RectangleWithId {
    id: String,
    rectangle: Rectangle,
}

impl RectangleCollisionChecker {
    /// Creates a new, empty collision checker.
    pub fn new() -> Self {
        Self { rects: Vec::new() }
    }

    /// Insert a rectangle into the collision checker.
    pub fn insert(&mut self, id: String, rectangle: Rectangle) {
        self.rects.push(RectangleWithId { id, rectangle });
    }

    /// Remove a rectangle from the collision checker by id.
    pub fn remove(&mut self, id: &str) {
        self.rects.retain(|r| r.id != id);
    }

    /// Checks if a given rectangle collides with any of the stored rectangles.
    pub fn collides(&self, rectangle: &Rectangle) -> bool {
        for r in &self.rects {
            if rectangles_overlap(&r.rectangle, rectangle) {
                return true;
            }
        }
        false
    }
}

impl Default for RectangleCollisionChecker {
    fn default() -> Self {
        Self::new()
    }
}

fn rectangles_overlap(a: &Rectangle, b: &Rectangle) -> bool {
    a.west < b.east && a.east > b.west && a.south < b.north && a.north > b.south
}
