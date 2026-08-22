//! Ported from packages/engine/Source/Core/BoundingRectangle.js
//!
//! A bounding rectangle given by a corner, width and height.

use crate::cartesian2::Cartesian2;
use crate::intersect::Intersect;

/// A bounding rectangle given by a corner, width and height.
///
/// Port of `BoundingRectangle`.
#[derive(Clone, Copy, Debug)]
pub struct BoundingRectangle {
    /// The x coordinate of the rectangle.
    pub x: f64,
    /// The y coordinate of the rectangle.
    pub y: f64,
    /// The width of the rectangle.
    pub width: f64,
    /// The height of the rectangle.
    pub height: f64,
}

impl Default for BoundingRectangle {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

impl BoundingRectangle {
    /// The number of elements used to pack the object into an array.
    ///
    /// Port of `BoundingRectangle.packedLength`.
    pub const PACKED_LENGTH: usize = 4;

    /// Creates a new `BoundingRectangle`.
    ///
    /// Port of the `BoundingRectangle(x, y, width, height)` constructor.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Port of `BoundingRectangle.pack`.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.x;
        array[starting_index + 1] = value.y;
        array[starting_index + 2] = value.width;
        array[starting_index + 3] = value.height;
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Port of `BoundingRectangle.unpack`.
    pub fn unpack(array: &[f64], starting_index: usize, result: &mut Self) {
        result.x = array[starting_index];
        result.y = array[starting_index + 1];
        result.width = array[starting_index + 2];
        result.height = array[starting_index + 3];
    }

    /// Allocating variant of [`BoundingRectangle::unpack`].
    pub fn unpack_new(array: &[f64], starting_index: usize) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    /// Computes a bounding rectangle enclosing the list of 2D points.
    ///
    /// Port of `BoundingRectangle.fromPoints`.
    pub fn from_points(positions: &[Cartesian2], result: &mut Self) {
        if positions.is_empty() {
            result.x = 0.0;
            result.y = 0.0;
            result.width = 0.0;
            result.height = 0.0;
            return;
        }

        let mut min_x = positions[0].x;
        let mut min_y = positions[0].y;
        let mut max_x = positions[0].x;
        let mut max_y = positions[0].y;

        for p in &positions[1..] {
            if p.x < min_x {
                min_x = p.x;
            }
            if p.x > max_x {
                max_x = p.x;
            }
            if p.y < min_y {
                min_y = p.y;
            }
            if p.y > max_y {
                max_y = p.y;
            }
        }

        result.x = min_x;
        result.y = min_y;
        result.width = max_x - min_x;
        result.height = max_y - min_y;
    }

    /// Allocating variant of [`BoundingRectangle::from_points`].
    pub fn from_points_new(positions: &[Cartesian2]) -> Self {
        let mut result = Self::default();
        Self::from_points(positions, &mut result);
        result
    }

    /// Computes a bounding rectangle from a rectangle.
    ///
    /// Port of `BoundingRectangle.fromRectangle`.
    ///
    /// DEVIATION (deferred): requires `Rectangle`, `Ellipsoid`,
    /// `GeographicProjection`; will be enabled once those are ported.
    // pub fn from_rectangle(...) { ... }

    /// Duplicates a `BoundingRectangle` instance.
    ///
    /// Port of `BoundingRectangle.clone`.
    pub fn clone(rectangle: &Self, result: &mut Self) {
        result.x = rectangle.x;
        result.y = rectangle.y;
        result.width = rectangle.width;
        result.height = rectangle.height;
    }

    /// Allocating variant of [`BoundingRectangle::clone`].
    pub fn clone_new(rectangle: &Self) -> Self {
        Self {
            x: rectangle.x,
            y: rectangle.y,
            width: rectangle.width,
            height: rectangle.height,
        }
    }

    /// Computes a bounding rectangle that is the union of the left and right
    /// bounding rectangles.
    ///
    /// Port of `BoundingRectangle.union`.
    pub fn union(left: &Self, right: &Self, result: &mut Self) {
        let lower_left_x = left.x.min(right.x);
        let lower_left_y = left.y.min(right.y);
        let upper_right_x = (left.x + left.width).max(right.x + right.width);
        let upper_right_y = (left.y + left.height).max(right.y + right.height);

        result.x = lower_left_x;
        result.y = lower_left_y;
        result.width = upper_right_x - lower_left_x;
        result.height = upper_right_y - lower_left_y;
    }

    /// Allocating variant of [`BoundingRectangle::union`].
    pub fn union_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::union(left, right, &mut result);
        result
    }

    /// Computes a bounding rectangle by enlarging the provided rectangle until
    /// it contains the provided point.
    ///
    /// Port of `BoundingRectangle.expand`.
    pub fn expand(rectangle: &Self, point: &Cartesian2, result: &mut Self) {
        Self::clone(rectangle, result);

        let width = point.x - result.x;
        let height = point.y - result.y;

        if width > result.width {
            result.width = width;
        } else if width < 0.0 {
            result.width -= width;
            result.x = point.x;
        }

        if height > result.height {
            result.height = height;
        } else if height < 0.0 {
            result.height -= height;
            result.y = point.y;
        }
    }

    /// Allocating variant of [`BoundingRectangle::expand`].
    pub fn expand_new(rectangle: &Self, point: &Cartesian2) -> Self {
        let mut result = Self::default();
        Self::expand(rectangle, point, &mut result);
        result
    }

    /// Determines if two rectangles intersect.
    ///
    /// Port of `BoundingRectangle.intersect`.
    ///
    /// Returns `Intersect::Intersecting` or `Intersect::Outside`.
    pub fn intersect(left: &Self, right: &Self) -> Intersect {
        let left_x = left.x;
        let left_y = left.y;
        let right_x = right.x;
        let right_y = right.y;

        if !(left_x > right_x + right.width
            || left_x + left.width < right_x
            || left_y + left.height < right_y
            || left_y > right_y + right.height)
        {
            Intersect::Intersecting
        } else {
            Intersect::Outside
        }
    }

    /// Compares two bounding rectangles componentwise.
    ///
    /// Port of `BoundingRectangle.equals`.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.x == right.x
            && left.y == right.y
            && left.width == right.width
            && left.height == right.height
    }
}

impl PartialEq for BoundingRectangle {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}
