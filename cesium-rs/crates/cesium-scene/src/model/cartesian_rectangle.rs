//! Ported from `packages/engine/Source/Scene/Model/CartesianRectangle.js`.

/// A rectangle in Cartesian coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CartesianRectangle {
    /// The minimum x-coordinate.
    pub min_x: f64,
    /// The minimum y-coordinate.
    pub min_y: f64,
    /// The maximum x-coordinate.
    pub max_x: f64,
    /// The maximum y-coordinate.
    pub max_y: f64,
}

impl CartesianRectangle {
    /// Creates a new `CartesianRectangle`.
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Returns whether this rectangle contains the given coordinates,
    /// using the default containment check (includes min, excludes max).
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x < self.max_x && y >= self.min_y && y < self.max_y
    }

    /// Returns whether this rectangle contains the given coordinates,
    /// excluding the border.
    pub fn contains_exclusive(&self, x: f64, y: f64) -> bool {
        x > self.min_x && x < self.max_x && y > self.min_y && y < self.max_y
    }

    /// Returns whether this rectangle contains the given coordinates,
    /// including the border.
    pub fn contains_inclusive(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

impl Default for CartesianRectangle {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}
