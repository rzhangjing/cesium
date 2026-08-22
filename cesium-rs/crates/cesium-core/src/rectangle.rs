//! Ported from packages/engine/Source/Core/Rectangle.js
//!
//! A two dimensional region specified as longitude and latitude coordinates.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;

/// A two dimensional region specified as longitude and latitude coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    /// The westernmost longitude in radians, in the range [-Pi, Pi].
    pub west: f64,
    /// The southernmost latitude in radians, in the range [-Pi/2, Pi/2].
    pub south: f64,
    /// The easternmost longitude in radians, in the range [-Pi, Pi].
    pub east: f64,
    /// The northernmost latitude in radians, in the range [-Pi/2, Pi/2].
    pub north: f64,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            west: 0.0,
            south: 0.0,
            east: 0.0,
            north: 0.0,
        }
    }
}

impl PartialEq for Rectangle {
    fn eq(&self, other: &Self) -> bool {
        self.west == other.west
            && self.south == other.south
            && self.east == other.east
            && self.north == other.north
    }
}

impl Rectangle {
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self { west, south, east, north }
    }

    /// The largest possible rectangle.
    pub const MAX_VALUE: Self = Self {
        west: -std::f64::consts::PI,
        south: -CesiumMath::PI_OVER_TWO,
        east: std::f64::consts::PI,
        north: CesiumMath::PI_OVER_TWO,
    };

    /// Port of `Rectangle.width` getter.
    pub fn width(&self) -> f64 {
        Self::compute_width(self)
    }

    /// Port of `Rectangle.height` getter.
    pub fn height(&self) -> f64 {
        Self::compute_height(self)
    }

    /// Port of `Rectangle.computeWidth`.
    pub fn compute_width(rectangle: &Self) -> f64 {
        let mut east = rectangle.east;
        let west = rectangle.west;
        if east < west {
            east += CesiumMath::TWO_PI;
        }
        east - west
    }

    /// Port of `Rectangle.computeHeight`.
    pub fn compute_height(rectangle: &Self) -> f64 {
        rectangle.north - rectangle.south
    }

    /// Port of `Rectangle.fromDegrees`.
    pub fn from_degrees(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self {
            west: CesiumMath::to_radians(west),
            south: CesiumMath::to_radians(south),
            east: CesiumMath::to_radians(east),
            north: CesiumMath::to_radians(north),
        }
    }

    pub fn from_degrees_into(west: f64, south: f64, east: f64, north: f64, result: &mut Self) {
        result.west = CesiumMath::to_radians(west);
        result.south = CesiumMath::to_radians(south);
        result.east = CesiumMath::to_radians(east);
        result.north = CesiumMath::to_radians(north);
    }

    /// Port of `Rectangle.fromRadians`.
    pub fn from_radians(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self { west, south, east, north }
    }

    pub fn from_radians_into(west: f64, south: f64, east: f64, north: f64, result: &mut Self) {
        result.west = west;
        result.south = south;
        result.east = east;
        result.north = north;
    }

    /// Port of `Rectangle.pack`.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: Option<usize>) {
        let idx = starting_index.unwrap_or(0);
        array[idx] = value.west;
        array[idx + 1] = value.south;
        array[idx + 2] = value.east;
        array[idx + 3] = value.north;
    }

    /// Port of `Rectangle.unpack`.
    pub fn unpack(array: &[f64], starting_index: Option<usize>) -> Self {
        let idx = starting_index.unwrap_or(0);
        Self {
            west: array[idx],
            south: array[idx + 1],
            east: array[idx + 2],
            north: array[idx + 3],
        }
    }

    pub fn unpack_into(array: &[f64], starting_index: Option<usize>, result: &mut Self) {
        let idx = starting_index.unwrap_or(0);
        result.west = array[idx];
        result.south = array[idx + 1];
        result.east = array[idx + 2];
        result.north = array[idx + 3];
    }

    /// Port of `Rectangle.packedLength`.
    pub const PACKED_LENGTH: usize = 4;

    /// Port of `Rectangle.clone` (static).
    pub fn clone_static(rectangle: &Self) -> Self {
        *rectangle
    }

    /// Port of `Rectangle.equals` (static).
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(l), Some(r)) => l == r,
            (None, None) => true,
            _ => false,
        }
    }

    /// Port of `Rectangle.equalsEpsilon` (static).
    pub fn equals_epsilon(left: &Self, right: &Self, absolute_epsilon: Option<f64>) -> bool {
        let eps = absolute_epsilon.unwrap_or(0.0);
        (left.west - right.west).abs() <= eps
            && (left.south - right.south).abs() <= eps
            && (left.east - right.east).abs() <= eps
            && (left.north - right.north).abs() <= eps
    }

    /// Instance equals.
    pub fn equals_to(&self, other: &Self) -> bool {
        self == other
    }

    /// Instance equals_epsilon.
    pub fn equals_epsilon_to(&self, other: &Self, epsilon: Option<f64>) -> bool {
        Self::equals_epsilon(self, other, epsilon)
    }

    /// Port of `Rectangle.southwest`.
    pub fn southwest(rectangle: &Self) -> Cartographic {
        Cartographic {
            longitude: rectangle.west,
            latitude: rectangle.south,
            height: 0.0,
        }
    }

    pub fn southwest_into(rectangle: &Self, result: &mut Cartographic) {
        result.longitude = rectangle.west;
        result.latitude = rectangle.south;
        result.height = 0.0;
    }

    /// Port of `Rectangle.northwest`.
    pub fn northwest(rectangle: &Self) -> Cartographic {
        Cartographic {
            longitude: rectangle.west,
            latitude: rectangle.north,
            height: 0.0,
        }
    }

    pub fn northwest_into(rectangle: &Self, result: &mut Cartographic) {
        result.longitude = rectangle.west;
        result.latitude = rectangle.north;
        result.height = 0.0;
    }

    /// Port of `Rectangle.northeast`.
    pub fn northeast(rectangle: &Self) -> Cartographic {
        Cartographic {
            longitude: rectangle.east,
            latitude: rectangle.north,
            height: 0.0,
        }
    }

    pub fn northeast_into(rectangle: &Self, result: &mut Cartographic) {
        result.longitude = rectangle.east;
        result.latitude = rectangle.north;
        result.height = 0.0;
    }

    /// Port of `Rectangle.southeast`.
    pub fn southeast(rectangle: &Self) -> Cartographic {
        Cartographic {
            longitude: rectangle.east,
            latitude: rectangle.south,
            height: 0.0,
        }
    }

    pub fn southeast_into(rectangle: &Self, result: &mut Cartographic) {
        result.longitude = rectangle.east;
        result.latitude = rectangle.south;
        result.height = 0.0;
    }

    /// Port of `Rectangle.center`.
    pub fn center(rectangle: &Self) -> Cartographic {
        let mut east = rectangle.east;
        let west = rectangle.west;
        if east < west {
            east += CesiumMath::TWO_PI;
        }
        let longitude = CesiumMath::negative_pi_to_pi((west + east) * 0.5);
        let latitude = (rectangle.south + rectangle.north) * 0.5;
        Cartographic { longitude, latitude, height: 0.0 }
    }

    pub fn center_into(rectangle: &Self, result: &mut Cartographic) {
        let c = Self::center(rectangle);
        result.longitude = c.longitude;
        result.latitude = c.latitude;
        result.height = 0.0;
    }

    /// Port of `Rectangle.intersection`.
    pub fn intersection(rectangle: &Self, other: &Self) -> Option<Self> {
        let mut rect_east = rectangle.east;
        let mut rect_west = rectangle.west;
        let mut other_east = other.east;
        let mut other_west = other.west;

        if rect_east < rect_west && other_east > 0.0 {
            rect_east += CesiumMath::TWO_PI;
        } else if other_east < other_west && rect_east > 0.0 {
            other_east += CesiumMath::TWO_PI;
        }

        if rect_east < rect_west && other_west < 0.0 {
            other_west += CesiumMath::TWO_PI;
        } else if other_east < other_west && rect_west < 0.0 {
            rect_west += CesiumMath::TWO_PI;
        }

        let west = CesiumMath::negative_pi_to_pi(rect_west.max(other_west));
        let east = CesiumMath::negative_pi_to_pi(rect_east.min(other_east));

        if (rectangle.west < rectangle.east || other.west < other.east) && east <= west {
            return None;
        }

        let south = rectangle.south.max(other.south);
        let north = rectangle.north.min(other.north);

        if south >= north {
            return None;
        }

        Some(Self { west, south, east, north })
    }

    /// Port of `Rectangle.simpleIntersection`.
    pub fn simple_intersection(rectangle: &Self, other: &Self) -> Option<Self> {
        let west = rectangle.west.max(other.west);
        let south = rectangle.south.max(other.south);
        let east = rectangle.east.min(other.east);
        let north = rectangle.north.min(other.north);

        if south >= north || west >= east {
            return None;
        }

        Some(Self { west, south, east, north })
    }

    /// Port of `Rectangle.union`.
    pub fn union(rectangle: &Self, other: &Self) -> Self {
        let mut rect_east = rectangle.east;
        let mut rect_west = rectangle.west;
        let mut other_east = other.east;
        let mut other_west = other.west;

        if rect_east < rect_west && other_east > 0.0 {
            rect_east += CesiumMath::TWO_PI;
        } else if other_east < other_west && rect_east > 0.0 {
            other_east += CesiumMath::TWO_PI;
        }

        if rect_east < rect_west && other_west < 0.0 {
            other_west += CesiumMath::TWO_PI;
        } else if other_east < other_west && rect_west < 0.0 {
            rect_west += CesiumMath::TWO_PI;
        }

        let west = CesiumMath::negative_pi_to_pi(rect_west.min(other_west));
        let east = CesiumMath::negative_pi_to_pi(rect_east.max(other_east));

        Self {
            west,
            south: rectangle.south.min(other.south),
            east,
            north: rectangle.north.max(other.north),
        }
    }

    /// Port of `Rectangle.expand`.
    pub fn expand(rectangle: &Self, cartographic: &Cartographic) -> Self {
        Self {
            west: rectangle.west.min(cartographic.longitude),
            south: rectangle.south.min(cartographic.latitude),
            east: rectangle.east.max(cartographic.longitude),
            north: rectangle.north.max(cartographic.latitude),
        }
    }

    pub fn expand_into(rectangle: &Self, cartographic: &Cartographic, result: &mut Self) {
        result.west = rectangle.west.min(cartographic.longitude);
        result.south = rectangle.south.min(cartographic.latitude);
        result.east = rectangle.east.max(cartographic.longitude);
        result.north = rectangle.north.max(cartographic.latitude);
    }

    /// Port of `Rectangle.contains`.
    pub fn contains(rectangle: &Self, cartographic: &Cartographic) -> bool {
        let mut longitude = cartographic.longitude;
        let latitude = cartographic.latitude;
        let west = rectangle.west;
        let mut east = rectangle.east;

        if east < west {
            east += CesiumMath::TWO_PI;
            if longitude < 0.0 {
                longitude += CesiumMath::TWO_PI;
            }
        }

        (longitude > west || CesiumMath::equals_epsilon(longitude, west, Some(CesiumMath::EPSILON14), None))
            && (longitude < east || CesiumMath::equals_epsilon(longitude, east, Some(CesiumMath::EPSILON14), None))
            && latitude >= rectangle.south
            && latitude <= rectangle.north
    }

    /// Port of `Rectangle.fromCartographicArray`.
    pub fn from_cartographic_array(cartographics: &[Cartographic]) -> Self {
        let mut west = f64::MAX;
        let mut east = f64::MIN;
        let mut west_over_idl = f64::MAX;
        let mut east_over_idl = f64::MIN;
        let mut south = f64::MAX;
        let mut north = f64::MIN;

        for pos in cartographics {
            west = west.min(pos.longitude);
            east = east.max(pos.longitude);
            south = south.min(pos.latitude);
            north = north.max(pos.latitude);

            let lon_adjusted = if pos.longitude >= 0.0 {
                pos.longitude
            } else {
                pos.longitude + CesiumMath::TWO_PI
            };
            west_over_idl = west_over_idl.min(lon_adjusted);
            east_over_idl = east_over_idl.max(lon_adjusted);
        }

        if east - west > east_over_idl - west_over_idl {
            west = west_over_idl;
            east = east_over_idl;
            if east > CesiumMath::PI {
                east -= CesiumMath::TWO_PI;
            }
            if west > CesiumMath::PI {
                west -= CesiumMath::TWO_PI;
            }
        }

        Self { west, south, east, north }
    }

    /// Port of `Rectangle.fromCartesianArray`.
    pub fn from_cartesian_array(cartesians: &[Cartesian3], ellipsoid: Option<&Ellipsoid>) -> Self {
        let ell = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        let mut west = f64::MAX;
        let mut east = f64::MIN;
        let mut west_over_idl = f64::MAX;
        let mut east_over_idl = f64::MIN;
        let mut south = f64::MAX;
        let mut north = f64::MIN;

        for pos in cartesians {
            let mut carto = Cartographic::default();
            if !ell.cartesian_to_cartographic(pos, &mut carto) {
                continue;
            }
            west = west.min(carto.longitude);
            east = east.max(carto.longitude);
            south = south.min(carto.latitude);
            north = north.max(carto.latitude);

            let lon_adjusted = if carto.longitude >= 0.0 {
                carto.longitude
            } else {
                carto.longitude + CesiumMath::TWO_PI
            };
            west_over_idl = west_over_idl.min(lon_adjusted);
            east_over_idl = east_over_idl.max(lon_adjusted);
        }

        if east - west > east_over_idl - west_over_idl {
            west = west_over_idl;
            east = east_over_idl;
            if east > CesiumMath::PI {
                east -= CesiumMath::TWO_PI;
            }
            if west > CesiumMath::PI {
                west -= CesiumMath::TWO_PI;
            }
        }

        Self { west, south, east, north }
    }

    /// Port of `Rectangle.subsample`.
    pub fn subsample(
        rectangle: &Self,
        ellipsoid: Option<&Ellipsoid>,
        surface_height: Option<f64>,
    ) -> Vec<Cartesian3> {
        let ell = ellipsoid.unwrap_or(&Ellipsoid::WGS84);
        let height = surface_height.unwrap_or(0.0);
        let mut result = Vec::new();

        let north = rectangle.north;
        let south = rectangle.south;
        let east = rectangle.east;
        let west = rectangle.west;

        let mut lla = Cartographic { height, ..Default::default() };

        lla.longitude = west;
        lla.latitude = north;
        let mut c = Cartesian3::default();
        ell.cartographic_to_cartesian(&lla, &mut c);
        result.push(c);

        lla.longitude = east;
        let mut c = Cartesian3::default();
        ell.cartographic_to_cartesian(&lla, &mut c);
        result.push(c);

        lla.latitude = south;
        let mut c = Cartesian3::default();
        ell.cartographic_to_cartesian(&lla, &mut c);
        result.push(c);

        lla.longitude = west;
        let mut c = Cartesian3::default();
        ell.cartographic_to_cartesian(&lla, &mut c);
        result.push(c);

        let sample_lat = if north < 0.0 {
            north
        } else if south > 0.0 {
            south
        } else {
            0.0
        };

        for i in 1..8 {
            lla.longitude = -std::f64::consts::PI + i as f64 * CesiumMath::PI_OVER_TWO;
            lla.latitude = sample_lat;
            if Self::contains(rectangle, &lla) {
                let mut c = Cartesian3::default();
                ell.cartographic_to_cartesian(&lla, &mut c);
                result.push(c);
            }
        }

        if sample_lat == 0.0 {
            lla.latitude = 0.0;
            lla.longitude = west;
            let mut c = Cartesian3::default();
            ell.cartographic_to_cartesian(&lla, &mut c);
            result.push(c);
            lla.longitude = east;
            let mut c = Cartesian3::default();
            ell.cartographic_to_cartesian(&lla, &mut c);
            result.push(c);
        }

        result
    }

    /// Port of `Rectangle.subsection`.
    pub fn subsection(
        rectangle: &Self,
        west_lerp: f64,
        south_lerp: f64,
        east_lerp: f64,
        north_lerp: f64,
    ) -> Self {
        let mut result = Self::default();
        Self::subsection_into(rectangle, west_lerp, south_lerp, east_lerp, north_lerp, &mut result);
        result
    }

    pub fn subsection_into(
        rectangle: &Self,
        west_lerp: f64,
        south_lerp: f64,
        east_lerp: f64,
        north_lerp: f64,
        result: &mut Self,
    ) {
        if rectangle.west <= rectangle.east {
            let width = rectangle.east - rectangle.west;
            result.west = rectangle.west + west_lerp * width;
            result.east = rectangle.west + east_lerp * width;
        } else {
            let width = CesiumMath::TWO_PI + rectangle.east - rectangle.west;
            result.west = CesiumMath::negative_pi_to_pi(rectangle.west + west_lerp * width);
            result.east = CesiumMath::negative_pi_to_pi(rectangle.west + east_lerp * width);
        }
        let height = rectangle.north - rectangle.south;
        result.south = rectangle.south + south_lerp * height;
        result.north = rectangle.south + north_lerp * height;

        // Fix floating point precision when t = 1
        if west_lerp == 1.0 {
            result.west = rectangle.east;
        }
        if east_lerp == 1.0 {
            result.east = rectangle.east;
        }
        if south_lerp == 1.0 {
            result.south = rectangle.north;
        }
        if north_lerp == 1.0 {
            result.north = rectangle.north;
        }
    }

    /// Port of `Rectangle.toString` (custom).
    pub fn to_string_repr(&self) -> String {
        format!("({}, {}, {}, {})", self.west, self.south, self.east, self.north)
    }
}
