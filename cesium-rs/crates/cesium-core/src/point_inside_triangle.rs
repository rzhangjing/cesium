//! Ported from `packages/engine/Source/Core/pointInsideTriangle.js`.

use crate::cartesian3::Cartesian3;

/// Determines if a point is inside a triangle using barycentric coordinates.
pub fn point_inside_triangle(
    point: &Cartesian3,
    p0: &Cartesian3,
    p1: &Cartesian3,
    p2: &Cartesian3,
) -> bool {
    let coords = barycentric_coordinates(point, p0, p1, p2);
    coords.x > 0.0 && coords.y > 0.0 && coords.z > 0.0
}

/// Computes the barycentric coordinates of a point with respect to a triangle.
fn barycentric_coordinates(
    point: &Cartesian3,
    p0: &Cartesian3,
    p1: &Cartesian3,
    p2: &Cartesian3,
) -> Cartesian3 {
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

    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    let w = 1.0 - u - v;

    Cartesian3::new(u, v, w)
}
