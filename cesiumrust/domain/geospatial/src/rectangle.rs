//! Rectangle - a two-dimensional region defined by west, south, east, north.
//! Maps to CesiumJS `Core/Rectangle.js`

use crate::math_utils;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// A two-dimensional region defined by longitude/latitude bounds (in radians).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    /// The westernmost longitude in radians [-PI, PI].
    pub west: f64,
    /// The southernmost latitude in radians [-PI/2, PI/2].
    pub south: f64,
    /// The easternmost longitude in radians [-PI, PI].
    pub east: f64,
    /// The northernmost latitude in radians [-PI/2, PI/2].
    pub north: f64,
}

impl Rectangle {
    /// The maximum rectangle: covers the entire globe.
    pub const MAX_VALUE: Self = Self {
        west: -PI,
        south: -PI / 2.0,
        east: PI,
        north: PI / 2.0,
    };

    /// An empty rectangle.
    pub const EMPTY: Self = Self {
        west: 0.0,
        south: 0.0,
        east: 0.0,
        north: 0.0,
    };

    /// Creates a new Rectangle from radians.
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// Creates a Rectangle from degrees.
    /// Maps to `Rectangle.fromDegrees`
    pub fn from_degrees(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west: math_utils::to_radians(west),
            south: math_utils::to_radians(south),
            east: math_utils::to_radians(east),
            north: math_utils::to_radians(north),
        }
    }

    /// Computes the width of the rectangle in radians.
    /// Maps to `Rectangle.computeWidth`
    pub fn width(&self) -> f64 {
        let mut east = self.east;
        if east < self.west {
            east += math_utils::TWO_PI;
        }
        east - self.west
    }

    /// Computes the height of the rectangle in radians.
    /// Maps to `Rectangle.computeHeight`
    pub fn height(&self) -> f64 {
        self.north - self.south
    }

    /// Determines if this rectangle contains a given cartographic position.
    pub fn contains(&self, longitude: f64, latitude: f64) -> bool {
        let mut east = self.east;
        if east < self.west {
            east += math_utils::TWO_PI;
        }
        let mut lon = longitude;
        if lon < self.west {
            lon += math_utils::TWO_PI;
        }
        lon >= self.west && lon <= east && latitude >= self.south && latitude <= self.north
    }

    /// Computes the intersection of two rectangles.
    /// Maps to `Rectangle.intersection`
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let west = self.west.max(other.west);
        let south = self.south.max(other.south);
        let east = self.east.min(other.east);
        let north = self.north.min(other.north);

        if west >= east || south >= north {
            return None;
        }

        Some(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// Computes the union of two rectangles.
    /// Maps to `Rectangle.union`
    pub fn union(&self, other: &Self) -> Self {
        Self {
            west: self.west.min(other.west),
            south: self.south.min(other.south),
            east: self.east.max(other.east),
            north: self.north.max(other.north),
        }
    }

    /// Expands this rectangle by the given amount in radians.
    pub fn expand(&self, amount: f64) -> Self {
        Self {
            west: self.west - amount,
            south: (self.south - amount).max(-PI / 2.0),
            east: self.east + amount,
            north: (self.north + amount).min(PI / 2.0),
        }
    }

    /// Computes the center of the rectangle.
    /// Maps to `Rectangle.center`
    pub fn center(&self) -> (f64, f64) {
        let mut east = self.east;
        if east < self.west {
            east += math_utils::TWO_PI;
        }
        let longitude = math_utils::negative_pi_to_pi((self.west + east) * 0.5);
        let latitude = (self.south + self.north) * 0.5;
        (longitude, latitude)
    }

    /// Subdivides the rectangle into a grid of smaller rectangles.
    pub fn subdivide(&self, x_segments: u32, y_segments: u32) -> Vec<Self> {
        let mut result = Vec::with_capacity((x_segments * y_segments) as usize);
        let width = self.width();
        let height = self.height();
        let dx = width / x_segments as f64;
        let dy = height / y_segments as f64;

        for j in 0..y_segments {
            for i in 0..x_segments {
                let west = self.west + dx * i as f64;
                let south = self.south + dy * j as f64;
                result.push(Self {
                    west,
                    south,
                    east: west + dx,
                    north: south + dy,
                });
            }
        }
        result
    }

    /// Determines if this rectangle equals another within an epsilon.
    pub fn equals_epsilon(&self, other: &Self, epsilon: f64) -> bool {
        (self.west - other.west).abs() <= epsilon
            && (self.south - other.south).abs() <= epsilon
            && (self.east - other.east).abs() <= epsilon
            && (self.north - other.north).abs() <= epsilon
    }
}

impl Default for Rectangle {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_degrees() {
        let r = Rectangle::from_degrees(-180.0, -90.0, 180.0, 90.0);
        assert!(r.equals_epsilon(&Rectangle::MAX_VALUE, 1e-10));
    }

    #[test]
    fn test_width_height() {
        let r = Rectangle::from_degrees(-90.0, -45.0, 90.0, 45.0);
        assert!((r.width() - PI).abs() < 1e-10);
        assert!((r.height() - PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_contains() {
        let r = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        assert!(r.contains(0.0, 0.0));
        assert!(!r.contains(math_utils::to_radians(20.0), 0.0));
    }

    #[test]
    fn test_intersection() {
        let a = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let b = Rectangle::from_degrees(0.0, 0.0, 20.0, 20.0);
        let inter = a.intersection(&b).unwrap();
        assert!((inter.west - 0.0).abs() < 1e-10);
        assert!((inter.south - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_union() {
        let a = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let b = Rectangle::from_degrees(0.0, 0.0, 20.0, 20.0);
        let u = a.union(&b);
        assert!((u.west - math_utils::to_radians(-10.0)).abs() < 1e-10);
        assert!((u.east - math_utils::to_radians(20.0)).abs() < 1e-10);
    }

    #[test]
    fn test_center() {
        let r = Rectangle::from_degrees(-90.0, -45.0, 90.0, 45.0);
        let (lon, lat) = r.center();
        assert!(lon.abs() < 1e-10);
        assert!(lat.abs() < 1e-10);
    }
}
