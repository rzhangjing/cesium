//! Ported from `packages/engine/Source/Scene/Model/ImageryCoverage.js`.
//!
//! Describes the coverage of a single imagery tile over a model surface,
//! defined by tile coordinates and a texture-coordinate rectangle.

use super::cartesian_rectangle::CartesianRectangle;

/// Coverage information for a single imagery tile draped on a model.
///
/// Stores the tile coordinates (x, y, level) and the texture-coordinate
/// rectangle that maps the imagery onto the model surface.
/// Mirrors CesiumJS `ImageryCoverage` (~200 lines).
pub struct ImageryCoverage {
    /// The imagery tile X coordinate.
    pub x: u32,
    /// The imagery tile Y coordinate.
    pub y: u32,
    /// The imagery tile level.
    pub level: u32,
    /// The texture-coordinate rectangle (min_u, min_v, max_u, max_v)
    /// that maps this imagery tile onto the model.
    pub texture_coordinate_rectangle: CartesianRectangle,
    /// Whether this coverage has been applied to the model.
    pub applied: bool,
}

impl ImageryCoverage {
    /// Creates a new `ImageryCoverage`.
    pub fn new(
        x: u32,
        y: u32,
        level: u32,
        texture_coordinate_rectangle: CartesianRectangle,
    ) -> Self {
        Self {
            x,
            y,
            level,
            texture_coordinate_rectangle,
            applied: false,
        }
    }

    /// Returns whether the given UV coordinates fall within this coverage.
    pub fn contains_uv(&self, u: f64, v: f64) -> bool {
        self.texture_coordinate_rectangle.contains(u, v)
    }

    /// Returns the width of the texture-coordinate rectangle.
    pub fn width(&self) -> f64 {
        self.texture_coordinate_rectangle.max_x - self.texture_coordinate_rectangle.min_x
    }

    /// Returns the height of the texture-coordinate rectangle.
    pub fn height(&self) -> f64 {
        self.texture_coordinate_rectangle.max_y - self.texture_coordinate_rectangle.min_y
    }
}

impl Default for ImageryCoverage {
    fn default() -> Self {
        Self::new(0, 0, 0, CartesianRectangle::default())
    }
}
