//! Ported from `packages/engine/Source/Core/barycentricCoordinates.js`.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::math::CesiumMath;

/// Computes the barycentric coordinates for a point with respect to a triangle.
///
/// Supports both Cartesian2 and Cartesian3 inputs via an enum.
pub enum BarycentricPoint {
    V2(Cartesian2),
    V3(Cartesian3),
}

/// Computes barycentric coordinates for a Cartesian3 point with respect to a Cartesian3 triangle.
pub fn barycentric_coordinates_3d(
    point: &Cartesian3,
    p0: &Cartesian3,
    p1: &Cartesian3,
    p2: &Cartesian3,
) -> Option<Cartesian3> {
    if Cartesian3::equals_epsilon_method(point, p0, Some(CesiumMath::EPSILON14), None) {
        return Some(Cartesian3::UNIT_X);
    }
    if Cartesian3::equals_epsilon_method(point, p1, Some(CesiumMath::EPSILON14), None) {
        return Some(Cartesian3::UNIT_Y);
    }
    if Cartesian3::equals_epsilon_method(point, p2, Some(CesiumMath::EPSILON14), None) {
        return Some(Cartesian3::UNIT_Z);
    }

    let mut v0 = Cartesian3::ZERO;
    Cartesian3::subtract(p1, p0, &mut v0);
    let mut v1 = Cartesian3::ZERO;
    Cartesian3::subtract(p2, p0, &mut v1);
    let mut v2 = Cartesian3::ZERO;
    Cartesian3::subtract(point, p0, &mut v2);

    let dot00 = Cartesian3::dot(&v0, &v0);
    let dot01 = Cartesian3::dot(&v0, &v1);
    let dot02 = Cartesian3::dot(&v0, &v2);
    let dot11 = Cartesian3::dot(&v1, &v1);
    let dot12 = Cartesian3::dot(&v1, &v2);

    let q = dot00 * dot11 - dot01 * dot01;
    if q == 0.0 {
        return None;
    }

    let inv_q = 1.0 / q;
    let y = (dot11 * dot02 - dot01 * dot12) * inv_q;
    let z = (dot00 * dot12 - dot01 * dot02) * inv_q;
    let x = 1.0 - y - z;
    Some(Cartesian3::new(x, y, z))
}

/// Computes barycentric coordinates for a Cartesian2 point with respect to a Cartesian2 triangle.
pub fn barycentric_coordinates_2d(
    point: &Cartesian2,
    p0: &Cartesian2,
    p1: &Cartesian2,
    p2: &Cartesian2,
) -> Option<Cartesian3> {
    if Cartesian2::equals_epsilon_method(point, p0, Some(CesiumMath::EPSILON14), None) {
        return Some(Cartesian3::UNIT_X);
    }
    if Cartesian2::equals_epsilon_method(point, p1, Some(CesiumMath::EPSILON14), None) {
        return Some(Cartesian3::UNIT_Y);
    }
    if Cartesian2::equals_epsilon_method(point, p2, Some(CesiumMath::EPSILON14), None) {
        return Some(Cartesian3::UNIT_Z);
    }

    let mut v0 = Cartesian2::ZERO;
    Cartesian2::subtract(p1, p0, &mut v0);
    let mut v1 = Cartesian2::ZERO;
    Cartesian2::subtract(p2, p0, &mut v1);
    let mut v2 = Cartesian2::ZERO;
    Cartesian2::subtract(point, p0, &mut v2);

    let dot00 = Cartesian2::dot(&v0, &v0);
    let dot01 = Cartesian2::dot(&v0, &v1);
    let dot02 = Cartesian2::dot(&v0, &v2);
    let dot11 = Cartesian2::dot(&v1, &v1);
    let dot12 = Cartesian2::dot(&v1, &v2);

    let q = dot00 * dot11 - dot01 * dot01;
    if q == 0.0 {
        return None;
    }

    let inv_q = 1.0 / q;
    let y = (dot11 * dot02 - dot01 * dot12) * inv_q;
    let z = (dot00 * dot12 - dot01 * dot02) * inv_q;
    let x = 1.0 - y - z;
    Some(Cartesian3::new(x, y, z))
}
