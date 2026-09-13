//! Cartesian3 CesiumJS extension functions.
//! Maps to CesiumJS `Core/Cartesian3.js` static methods that go beyond basic vector math.

use crate::ellipsoid::Ellipsoid;
use crate::math_utils;
use crate::spherical::Spherical;
use glam::DVec3;

/// The packed length of a Cartesian3: 3.
pub const PACKED_LENGTH: usize = 3;

/// Converts Spherical coordinates to Cartesian3.
/// Maps to CesiumJS `Cartesian3.fromSpherical`
pub fn from_spherical(spherical: &Spherical) -> DVec3 {
    let clock = spherical.clock;
    let cone = spherical.cone;
    let magnitude = spherical.magnitude;
    let radial = magnitude * cone.sin();
    DVec3::new(
        radial * clock.cos(),
        radial * clock.sin(),
        magnitude * cone.cos(),
    )
}

/// Returns the axis that is most orthogonal to the provided Cartesian.
/// Maps to CesiumJS `Cartesian3.mostOrthogonalAxis`
pub fn most_orthogonal_axis(cartesian: DVec3) -> DVec3 {
    let f = cartesian.normalize_or_zero();
    let f = DVec3::new(f.x.abs(), f.y.abs(), f.z.abs());

    if f.x <= f.y {
        if f.x <= f.z {
            DVec3::X
        } else {
            DVec3::Z
        }
    } else if f.y <= f.z {
        DVec3::Y
    } else {
        DVec3::Z
    }
}

/// Projects vector a onto vector b.
/// Maps to CesiumJS `Cartesian3.projectVector`
pub fn project_vector(a: DVec3, b: DVec3) -> DVec3 {
    let scalar = a.dot(b) / b.dot(b);
    b * scalar
}

/// Computes the midpoint between left and right.
/// Maps to CesiumJS `Cartesian3.midpoint`
pub fn midpoint(left: DVec3, right: DVec3) -> DVec3 {
    DVec3::new(
        (left.x + right.x) * 0.5,
        (left.y + right.y) * 0.5,
        (left.z + right.z) * 0.5,
    )
}

/// Returns true if left and right are equal within the provided epsilon.
/// Maps to CesiumJS `Cartesian3.equalsEpsilon`
pub fn equals_epsilon(
    left: DVec3,
    right: DVec3,
    relative_epsilon: f64,
    absolute_epsilon: f64,
) -> bool {
    math_utils::equals_epsilon(left.x, right.x, relative_epsilon, absolute_epsilon)
        && math_utils::equals_epsilon(left.y, right.y, relative_epsilon, absolute_epsilon)
        && math_utils::equals_epsilon(left.z, right.z, relative_epsilon, absolute_epsilon)
}

/// Packs a Cartesian3 into an array at the given starting index.
/// Maps to CesiumJS `Cartesian3.pack`
pub fn pack(value: DVec3, array: &mut [f64], starting_index: usize) {
    array[starting_index] = value.x;
    array[starting_index + 1] = value.y;
    array[starting_index + 2] = value.z;
}

/// Unpacks a Cartesian3 from an array at the given starting index.
/// Maps to CesiumJS `Cartesian3.unpack`
pub fn unpack(array: &[f64], starting_index: usize) -> DVec3 {
    DVec3::new(
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
    )
}

/// Returns a Cartesian3 position from longitude and latitude values given in degrees.
/// Maps to CesiumJS `Cartesian3.fromDegrees`
pub fn from_degrees(
    longitude: f64,
    latitude: f64,
    height: f64,
    ellipsoid: &Ellipsoid,
) -> DVec3 {
    let lon_rad = math_utils::to_radians(longitude);
    let lat_rad = math_utils::to_radians(latitude);
    from_radians(lon_rad, lat_rad, height, ellipsoid)
}

/// Returns a Cartesian3 position from longitude and latitude values given in radians.
/// Maps to CesiumJS `Cartesian3.fromRadians`
pub fn from_radians(
    longitude: f64,
    latitude: f64,
    height: f64,
    ellipsoid: &Ellipsoid,
) -> DVec3 {
    let radii_squared = ellipsoid.radii_squared();

    let cos_latitude = latitude.cos();
    let mut n = DVec3::new(
        cos_latitude * longitude.cos(),
        cos_latitude * longitude.sin(),
        latitude.sin(),
    );
    n = n.normalize_or_zero();

    let k = DVec3::new(
        radii_squared.x * n.x,
        radii_squared.y * n.y,
        radii_squared.z * n.z,
    );
    let gamma = (n.dot(k)).sqrt();
    let k = k / gamma;
    let n = n * height;

    k + n
}

/// Returns an array of Cartesian3 positions from an array of [lon, lat, lon, lat, ...] in degrees.
/// Maps to CesiumJS `Cartesian3.fromDegreesArray`
pub fn from_degrees_array(coordinates: &[f64], ellipsoid: &Ellipsoid) -> Vec<DVec3> {
    assert!(
        coordinates.len() >= 2 && coordinates.len() % 2 == 0,
        "the number of coordinates must be a multiple of 2 and at least 2"
    );
    let mut result = Vec::with_capacity(coordinates.len() / 2);
    for chunk in coordinates.chunks(2) {
        result.push(from_degrees(chunk[0], chunk[1], 0.0, ellipsoid));
    }
    result
}

/// Returns an array of Cartesian3 positions from an array of [lon, lat, lon, lat, ...] in radians.
/// Maps to CesiumJS `Cartesian3.fromRadiansArray`
pub fn from_radians_array(coordinates: &[f64], ellipsoid: &Ellipsoid) -> Vec<DVec3> {
    assert!(
        coordinates.len() >= 2 && coordinates.len() % 2 == 0,
        "the number of coordinates must be a multiple of 2 and at least 2"
    );
    let mut result = Vec::with_capacity(coordinates.len() / 2);
    for chunk in coordinates.chunks(2) {
        result.push(from_radians(chunk[0], chunk[1], 0.0, ellipsoid));
    }
    result
}

/// Returns an array of Cartesian3 positions from [lon, lat, height, ...] in degrees.
/// Maps to CesiumJS `Cartesian3.fromDegreesArrayHeights`
pub fn from_degrees_array_heights(coordinates: &[f64], ellipsoid: &Ellipsoid) -> Vec<DVec3> {
    assert!(
        coordinates.len() >= 3 && coordinates.len() % 3 == 0,
        "the number of coordinates must be a multiple of 3 and at least 3"
    );
    let mut result = Vec::with_capacity(coordinates.len() / 3);
    for chunk in coordinates.chunks(3) {
        result.push(from_degrees(chunk[0], chunk[1], chunk[2], ellipsoid));
    }
    result
}

/// Returns an array of Cartesian3 positions from [lon, lat, height, ...] in radians.
/// Maps to CesiumJS `Cartesian3.fromRadiansArrayHeights`
pub fn from_radians_array_heights(coordinates: &[f64], ellipsoid: &Ellipsoid) -> Vec<DVec3> {
    assert!(
        coordinates.len() >= 3 && coordinates.len() % 3 == 0,
        "the number of coordinates must be a multiple of 3 and at least 3"
    );
    let mut result = Vec::with_capacity(coordinates.len() / 3);
    for chunk in coordinates.chunks(3) {
        result.push(from_radians(chunk[0], chunk[1], chunk[2], ellipsoid));
    }
    result
}

/// Converts a Cartesian3 to Spherical coordinates.
/// Maps to CesiumJS `Spherical.fromCartesian3` (already in spherical.rs, re-exported here for convenience)
pub fn to_spherical(cartesian: DVec3) -> Spherical {
    Spherical::from_cartesian3(cartesian)
}
