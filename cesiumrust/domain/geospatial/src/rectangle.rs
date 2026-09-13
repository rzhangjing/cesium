//! Rectangle - a two-dimensional region defined by west, south, east, north.
//! Maps to CesiumJS `Core/Rectangle.js`
//!
//! Faithful port of the original CesiumJS `Rectangle`, including the
//! anti-meridian (IDL) crossing logic in `intersection`/`union`/`center`/
//! `contains`/`subsection` and the `fromCartographicArray`/`fromCartesianArray`
//! "smallest enclosing rectangle" logic.

use crate::bounding::BoundingSphere;
use crate::cartographic::Cartographic;
use crate::ellipsoid::{self, Ellipsoid};
use crate::math_utils::{self, EPSILON14, PI_OVER_TWO, TWO_PI};
use crate::transforms;
use glam::DVec3;
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
    /// The largest possible rectangle. Maps to `Rectangle.MAX_VALUE`.
    pub const MAX_VALUE: Self = Self {
        west: -PI,
        south: -PI_OVER_TWO,
        east: PI,
        north: PI_OVER_TWO,
    };

    /// An empty (all-zero) rectangle, equivalent to CesiumJS `new Rectangle()`.
    pub const EMPTY: Self = Self {
        west: 0.0,
        south: 0.0,
        east: 0.0,
        north: 0.0,
    };

    /// The number of elements used to pack the object into an array.
    /// Maps to `Rectangle.packedLength`.
    pub const PACKED_LENGTH: usize = 4;

    /// Creates a new Rectangle from radians.
    /// Maps to the CesiumJS `Rectangle` constructor.
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// Creates a rectangle given the boundary longitude and latitude in degrees.
    /// Maps to `Rectangle.fromDegrees`
    pub fn from_degrees(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west: math_utils::to_radians(west),
            south: math_utils::to_radians(south),
            east: math_utils::to_radians(east),
            north: math_utils::to_radians(north),
        }
    }

    /// Creates a rectangle given the boundary longitude and latitude in radians.
    /// Maps to `Rectangle.fromRadians`
    pub fn from_radians(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self::new(west, south, east, north)
    }

    /// Stores the provided instance into the provided array.
    /// Maps to `Rectangle.pack`
    pub fn pack_into(&self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = self.west;
        array[starting_index + 1] = self.south;
        array[starting_index + 2] = self.east;
        array[starting_index + 3] = self.north;
    }

    /// Packs this rectangle into a new `[f64; 4]` (`[west, south, east, north]`).
    pub fn pack(&self) -> [f64; 4] {
        [self.west, self.south, self.east, self.north]
    }

    /// Retrieves an instance from a packed array.
    /// Maps to `Rectangle.unpack`
    pub fn unpack(array: &[f64], starting_index: usize) -> Self {
        Self {
            west: array[starting_index],
            south: array[starting_index + 1],
            east: array[starting_index + 2],
            north: array[starting_index + 3],
        }
    }

    /// Computes the width of the rectangle in radians.
    /// Maps to `Rectangle.computeWidth`
    pub fn width(&self) -> f64 {
        let mut east = self.east;
        let west = self.west;
        if east < west {
            east += TWO_PI;
        }
        east - west
    }

    /// Computes the height of the rectangle in radians.
    /// Maps to `Rectangle.computeHeight`
    pub fn height(&self) -> f64 {
        self.north - self.south
    }

    /// Creates the smallest possible Rectangle that encloses all positions in
    /// the provided array.
    /// Maps to `Rectangle.fromCartographicArray`
    pub fn from_cartographic_array(cartographics: &[Cartographic]) -> Self {
        let mut west = f64::MAX;
        let mut east = f64::MIN;
        let mut west_over_idl = f64::MAX;
        let mut east_over_idl = f64::MIN;
        let mut south = f64::MAX;
        let mut north = f64::MIN;

        for position in cartographics {
            west = west.min(position.longitude);
            east = east.max(position.longitude);
            south = south.min(position.latitude);
            north = north.max(position.latitude);

            let lon_adjusted = if position.longitude >= 0.0 {
                position.longitude
            } else {
                position.longitude + TWO_PI
            };
            west_over_idl = west_over_idl.min(lon_adjusted);
            east_over_idl = east_over_idl.max(lon_adjusted);
        }

        if east - west > east_over_idl - west_over_idl {
            west = west_over_idl;
            east = east_over_idl;

            if east > PI {
                east -= TWO_PI;
            }
            if west > PI {
                west -= TWO_PI;
            }
        }

        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// Creates the smallest possible Rectangle that encloses all positions in
    /// the provided array of Cartesian positions.
    /// Maps to `Rectangle.fromCartesianArray`
    pub fn from_cartesian_array(cartesians: &[DVec3], ellipsoid: &Ellipsoid) -> Self {
        let mut west = f64::MAX;
        let mut east = f64::MIN;
        let mut west_over_idl = f64::MAX;
        let mut east_over_idl = f64::MIN;
        let mut south = f64::MAX;
        let mut north = f64::MIN;

        for cartesian in cartesians {
            let position = ellipsoid
                .cartesian_to_cartographic(*cartesian)
                .expect("cartesian must not be at the center of the ellipsoid");
            west = west.min(position.longitude);
            east = east.max(position.longitude);
            south = south.min(position.latitude);
            north = north.max(position.latitude);

            let lon_adjusted = if position.longitude >= 0.0 {
                position.longitude
            } else {
                position.longitude + TWO_PI
            };
            west_over_idl = west_over_idl.min(lon_adjusted);
            east_over_idl = east_over_idl.max(lon_adjusted);
        }

        if east - west > east_over_idl - west_over_idl {
            west = west_over_idl;
            east = east_over_idl;

            if east > PI {
                east -= TWO_PI;
            }
            if west > PI {
                west -= TWO_PI;
            }
        }

        Self {
            west,
            south,
            east,
            north,
        }
    }

    /// Creates a rectangle from a bounding sphere, ignoring height.
    /// Maps to `Rectangle.fromBoundingSphere`
    pub fn from_bounding_sphere(bounding_sphere: &BoundingSphere, ellipsoid: &Ellipsoid) -> Self {
        let center = bounding_sphere.center;
        let radius = bounding_sphere.radius;

        if center == DVec3::ZERO {
            return Self::MAX_VALUE;
        }

        let from_enu = transforms::east_north_up_to_fixed_frame(center, ellipsoid);
        // Matrix4.multiplyByPointAsVector: apply only the linear (rotation) part.
        let east = ellipsoid::normalize_cartesian3(from_enu.transform_vector3(DVec3::X));
        let north = ellipsoid::normalize_cartesian3(from_enu.transform_vector3(DVec3::Y));

        let north = north * radius;
        let east = east * radius;
        let south = -north;
        let west = -east;

        let positions = [
            center + north,
            center + west,
            center + south,
            center + east,
            center,
        ];
        Self::from_cartesian_array(&positions, ellipsoid)
    }

    /// Checks the rectangle's properties and returns an error if they are not
    /// in valid ranges.
    /// Maps to `Rectangle._validate` (CesiumJS throws `DeveloperError`; Rust
    /// returns `Err` so the check is testable without panicking).
    pub fn validate(&self) -> Result<(), String> {
        let north = self.north;
        if !(north >= -PI_OVER_TWO) || !(north <= PI_OVER_TWO) {
            return Err("north must be in the interval [-Pi/2, Pi/2].".to_string());
        }

        let south = self.south;
        if !(south >= -PI_OVER_TWO) || !(south <= PI_OVER_TWO) {
            return Err("south must be in the interval [-Pi/2, Pi/2].".to_string());
        }

        let west = self.west;
        if !(west >= -PI) || !(west <= PI) {
            return Err("west must be in the interval [-Pi, Pi].".to_string());
        }

        let east = self.east;
        if !(east >= -PI) || !(east <= PI) {
            return Err("east must be in the interval [-Pi, Pi].".to_string());
        }

        Ok(())
    }

    /// Computes the southwest corner of the rectangle.
    /// Maps to `Rectangle.southwest`
    pub fn southwest(&self) -> Cartographic {
        Cartographic::from_radians(self.west, self.south, 0.0)
    }

    /// Computes the northwest corner of the rectangle.
    /// Maps to `Rectangle.northwest`
    pub fn northwest(&self) -> Cartographic {
        Cartographic::from_radians(self.west, self.north, 0.0)
    }

    /// Computes the northeast corner of the rectangle.
    /// Maps to `Rectangle.northeast`
    pub fn northeast(&self) -> Cartographic {
        Cartographic::from_radians(self.east, self.north, 0.0)
    }

    /// Computes the southeast corner of the rectangle.
    /// Maps to `Rectangle.southeast`
    pub fn southeast(&self) -> Cartographic {
        Cartographic::from_radians(self.east, self.south, 0.0)
    }

    /// Computes the center of the rectangle.
    /// Maps to `Rectangle.center`
    pub fn center(&self) -> Cartographic {
        let mut east = self.east;
        let west = self.west;

        if east < west {
            east += TWO_PI;
        }

        let longitude = math_utils::negative_pi_to_pi((west + east) * 0.5);
        let latitude = (self.south + self.north) * 0.5;

        Cartographic::from_radians(longitude, latitude, 0.0)
    }

    /// Computes the intersection of two rectangles, taking into account the
    /// wrapping of longitude at the anti-meridian.
    /// Maps to `Rectangle.intersection`
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let mut rectangle_east = self.east;
        let mut rectangle_west = self.west;

        let mut other_rectangle_east = other.east;
        let mut other_rectangle_west = other.west;

        if rectangle_east < rectangle_west && other_rectangle_east > 0.0 {
            rectangle_east += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_east > 0.0 {
            other_rectangle_east += TWO_PI;
        }

        if rectangle_east < rectangle_west && other_rectangle_west < 0.0 {
            other_rectangle_west += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_west < 0.0 {
            rectangle_west += TWO_PI;
        }

        let west = math_utils::negative_pi_to_pi(rectangle_west.max(other_rectangle_west));
        let east = math_utils::negative_pi_to_pi(rectangle_east.min(other_rectangle_east));

        if (self.west < self.east || other.west < other.east) && east <= west {
            return None;
        }

        let south = self.south.max(other.south);
        let north = self.north.min(other.north);

        if south >= north {
            return None;
        }

        Some(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// Computes a simple intersection of two rectangles, ignoring the
    /// anti-meridian (usable with projected coordinates).
    /// Maps to `Rectangle.simpleIntersection`
    pub fn simple_intersection(&self, other: &Self) -> Option<Self> {
        let west = self.west.max(other.west);
        let south = self.south.max(other.south);
        let east = self.east.min(other.east);
        let north = self.north.min(other.north);

        if south >= north || west >= east {
            return None;
        }

        Some(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// Computes a rectangle that is the union of two rectangles, taking into
    /// account the wrapping of longitude at the anti-meridian.
    /// Maps to `Rectangle.union`
    pub fn union(&self, other: &Self) -> Self {
        let mut rectangle_east = self.east;
        let mut rectangle_west = self.west;

        let mut other_rectangle_east = other.east;
        let mut other_rectangle_west = other.west;

        if rectangle_east < rectangle_west && other_rectangle_east > 0.0 {
            rectangle_east += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_east > 0.0 {
            other_rectangle_east += TWO_PI;
        }

        if rectangle_east < rectangle_west && other_rectangle_west < 0.0 {
            other_rectangle_west += TWO_PI;
        } else if other_rectangle_east < other_rectangle_west && rectangle_west < 0.0 {
            rectangle_west += TWO_PI;
        }

        let west = math_utils::negative_pi_to_pi(rectangle_west.min(other_rectangle_west));
        let east = math_utils::negative_pi_to_pi(rectangle_east.max(other_rectangle_east));

        Self {
            west,
            south: self.south.min(other.south),
            east,
            north: self.north.max(other.north),
        }
    }

    /// Computes a rectangle by enlarging this rectangle until it contains the
    /// provided cartographic.
    /// Maps to `Rectangle.expand` (CesiumJS expands to enclose a point; the
    /// point's height is ignored).
    pub fn expand(&self, cartographic: &Cartographic) -> Self {
        Self {
            west: self.west.min(cartographic.longitude),
            south: self.south.min(cartographic.latitude),
            east: self.east.max(cartographic.longitude),
            north: self.north.max(cartographic.latitude),
        }
    }

    /// Returns true if the cartographic position (longitude/latitude, in
    /// radians) is on or inside the rectangle, false otherwise.
    /// Maps to `Rectangle.contains`
    pub fn contains(&self, longitude: f64, latitude: f64) -> bool {
        let mut longitude = longitude;

        let west = self.west;
        let mut east = self.east;

        if east < west {
            east += TWO_PI;
            if longitude < 0.0 {
                longitude += TWO_PI;
            }
        }
        (longitude > west
            || math_utils::equals_epsilon(longitude, west, EPSILON14, EPSILON14))
            && (longitude < east
                || math_utils::equals_epsilon(longitude, east, EPSILON14, EPSILON14))
            && latitude >= self.south
            && latitude <= self.north
    }

    /// Samples the rectangle so that it includes a list of Cartesian points
    /// suitable for passing to `BoundingSphere.fromPoints`. Sampling is
    /// necessary to account for rectangles that cover the poles or cross the
    /// equator.
    /// Maps to `Rectangle.subsample`
    pub fn subsample(&self, ellipsoid: &Ellipsoid, surface_height: f64) -> Vec<DVec3> {
        let mut result = Vec::new();

        let north = self.north;
        let south = self.south;
        let east = self.east;
        let west = self.west;

        let mut lla = Cartographic::from_radians(west, north, surface_height);
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        lla.longitude = east;
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        lla.latitude = south;
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        lla.longitude = west;
        result.push(ellipsoid.cartographic_to_cartesian(&lla));

        if north < 0.0 {
            lla.latitude = north;
        } else if south > 0.0 {
            lla.latitude = south;
        } else {
            lla.latitude = 0.0;
        }

        for i in 1..8 {
            lla.longitude = -PI + i as f64 * math_utils::PI_OVER_TWO;
            if self.contains(lla.longitude, lla.latitude) {
                result.push(ellipsoid.cartographic_to_cartesian(&lla));
            }
        }

        if lla.latitude == 0.0 {
            lla.longitude = west;
            result.push(ellipsoid.cartographic_to_cartesian(&lla));
            lla.longitude = east;
            result.push(ellipsoid.cartographic_to_cartesian(&lla));
        }
        result
    }

    /// Computes a subsection of the rectangle from normalized coordinates in
    /// the range [0.0, 1.0].
    /// Maps to `Rectangle.subsection` (CesiumJS throws `DeveloperError` for
    /// out-of-range lerps; Rust returns `Err`).
    pub fn subsection(
        &self,
        west_lerp: f64,
        south_lerp: f64,
        east_lerp: f64,
        north_lerp: f64,
    ) -> Result<Self, String> {
        if !(west_lerp >= 0.0) || !(west_lerp <= 1.0) {
            return Err("westLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(south_lerp >= 0.0) || !(south_lerp <= 1.0) {
            return Err("southLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(east_lerp >= 0.0) || !(east_lerp <= 1.0) {
            return Err("eastLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(north_lerp >= 0.0) || !(north_lerp <= 1.0) {
            return Err("northLerp must be in the range [0.0, 1.0].".to_string());
        }
        if !(west_lerp <= east_lerp) {
            return Err("westLerp must be less than or equal to eastLerp.".to_string());
        }
        if !(south_lerp <= north_lerp) {
            return Err("southLerp must be less than or equal to northLerp.".to_string());
        }

        // This function doesn't use lerp because it has floating point precision
        // problems when the start and end values are the same but t changes.
        let (mut west, mut east) = if self.west <= self.east {
            let width = self.east - self.west;
            (self.west + west_lerp * width, self.west + east_lerp * width)
        } else {
            let width = TWO_PI + self.east - self.west;
            (
                math_utils::negative_pi_to_pi(self.west + west_lerp * width),
                math_utils::negative_pi_to_pi(self.west + east_lerp * width),
            )
        };
        let height = self.north - self.south;
        let mut south = self.south + south_lerp * height;
        let mut north = self.south + north_lerp * height;

        // Fix floating point precision problems when t = 1
        if west_lerp == 1.0 {
            west = self.east;
        }
        if east_lerp == 1.0 {
            east = self.east;
        }
        if south_lerp == 1.0 {
            south = self.north;
        }
        if north_lerp == 1.0 {
            north = self.north;
        }

        Ok(Self {
            west,
            south,
            east,
            north,
        })
    }

    /// Subdivides the rectangle into a grid of smaller rectangles.
    /// (Rust-side extension; no direct CesiumJS `Rectangle` counterpart.)
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
    /// Maps to `Rectangle.equalsEpsilon`
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
        let c = r.center();
        assert!(c.longitude.abs() < 1e-10);
        assert!(c.latitude.abs() < 1e-10);
    }
}
