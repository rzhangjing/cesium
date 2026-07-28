//! Core/BoundingSphereSpec.js → Rust integration tests
//!
//! Faithful port of CesiumJS `Specs/Core/BoundingSphereSpec.js` (94 `it()` cases).
//!
//! ## Platform adaptations
//! - JS result-parameter variants (`clone(result)`, `fromPoints(p, result)`,
//!   `fromVertices(.., result)`, `fromCornerPoints(.., result)`, `fromEllipsoid(e, result)`,
//!   `fromTransformation(t, result)`, `union(l, r, result)`, `expand(s, p, result)`,
//!   `transform(s, m, result)`, `transformWithoutScale(.., result)`,
//!   `projectTo2D(.., result)`) are merged into the owned-return tests: Rust returns owned
//!   values / uses `Copy`.
//! - JS "throws ..." cases (null/undefined checks and DeveloperError such as
//!   "fromVertices requires a stride of at least 3") are omitted: Rust's type system makes
//!   passing `undefined` impossible, and the stride precondition is a `debug_assert!`.
//! - `createPackableSpecs` (pack/unpack into JS arrays) is omitted: packing is a JS-array
//!   serialization concern not part of the Rust domain API.
//! - JS `EncodedCartesian3.fromCartesian` high/low splitting is a GPU double-precision
//!   emulation technique. Rust `f64` is natively double precision, so the encoded-vertex
//!   tests feed `high = values`, `low = zeros` (`high + low == value`, mathematically
//!   identical to the decoded positions).
//! - Deferred to later tasks: `fromOrientedBoundingBox` (requires
//!   `OrientedBoundingBox.fromPoints`, ported under t8e), `projectTo2D` (requires the
//!   `cartesianToCartographic` + `projectTo2D` pipeline), and `isOccluded`
//!   (Occluder is a C-class rendering dependency).

use cesium_geospatial::bounding::Interval;
use cesium_geospatial::{
    BoundingSphere, Cartographic, Ellipsoid, GeographicProjection, Intersect, MapProjection,
    Rectangle,
};
use cesium_specs::{assert_approx, assert_vec3_epsilon, epsilon};
use glam::{DMat4, DQuat, DVec3};

const POSITIONS_RADIUS: f64 = 1.0;

fn positions_center() -> DVec3 {
    DVec3::new(10000001.0, 0.0, 0.0)
}

fn center_offset() -> DVec3 {
    DVec3::new(10000000.0, 0.0, 0.0)
}

fn get_positions() -> Vec<DVec3> {
    let c = center_offset();
    vec![
        c + DVec3::new(1.0, 0.0, 0.0),
        c + DVec3::new(2.0, 0.0, 0.0),
        c + DVec3::new(0.0, 0.0, 0.0),
        c + DVec3::new(1.0, 1.0, 0.0),
        c + DVec3::new(1.0, -1.0, 0.0),
        c + DVec3::new(1.0, 0.0, 1.0),
        c + DVec3::new(1.0, 0.0, -1.0),
    ]
}

fn get_positions_as_flat_array() -> Vec<f64> {
    let mut result = Vec::new();
    for p in get_positions() {
        result.push(p.x);
        result.push(p.y);
        result.push(p.z);
    }
    result
}

fn get_positions_as_flat_array_with_stride5() -> Vec<f64> {
    let mut result = Vec::new();
    for p in get_positions() {
        result.push(p.x);
        result.push(p.y);
        result.push(p.z);
        result.push(1.23);
        result.push(4.56);
    }
    result
}

/// Asserts every point lies within the sphere's axis-aligned extent (center ± radius),
/// mirroring the JS `contains all points` checks.
fn assert_sphere_contains_points(sphere: &BoundingSphere, points: &[DVec3]) {
    let r = DVec3::splat(sphere.radius);
    let max = sphere.center + r;
    let min = sphere.center - r;
    for p in points {
        assert!(p.x <= max.x && p.x >= min.x, "x of {:?} not in [{}, {}]", p, min.x, max.x);
        assert!(p.y <= max.y && p.y >= min.y, "y of {:?} not in [{}, {}]", p, min.y, max.y);
        assert!(p.z <= max.z && p.z >= min.z, "z of {:?} not in [{}, {}]", p, min.z, max.z);
    }
}

/// Mirrors JS `expectBoundingSphereToContainPoint`: projects the cartographic point and
/// checks its distance from the sphere center is within the radius (with EPSILON9 slack).
fn expect_bounding_sphere_to_contain_point(
    sphere: &BoundingSphere,
    point: Cartographic,
    projection: &GeographicProjection,
) {
    let point_in_cartesian = projection.project(&point);
    let distance_from_center = (point_in_cartesian - sphere.center).length() - epsilon::EPSILON9;
    assert!(
        distance_from_center <= sphere.radius,
        "distance {} > radius {}",
        distance_from_center,
        sphere.radius
    );
}

/// `it("default constructing produces expected values")`
#[test]
fn test_bs_default() {
    let sphere = BoundingSphere::default();
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("constructor sets expected values")`
#[test]
fn test_bs_constructor() {
    let expected_center = DVec3::new(1.0, 2.0, 3.0);
    let sphere = BoundingSphere::new(expected_center, 1.0);
    assert_vec3_epsilon!(sphere.center, expected_center, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 1.0, epsilon::EPSILON15);
}

/// `it("clone without a result parameter")`
#[test]
fn test_bs_clone() {
    let sphere = BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 4.0);
    let result = sphere; // Copy semantics == clone()
    assert!(sphere == result);
}

/// `it("equals")`
#[test]
fn test_bs_equals() {
    let sphere = BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 4.0);
    assert!(sphere == BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 4.0));
    assert!(sphere != BoundingSphere::new(DVec3::new(5.0, 2.0, 3.0), 4.0));
    assert!(sphere != BoundingSphere::new(DVec3::new(1.0, 6.0, 3.0), 4.0));
    assert!(sphere != BoundingSphere::new(DVec3::new(1.0, 2.0, 7.0), 4.0));
    assert!(sphere != BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 8.0));
}

/// `it("fromPoints without positions returns an empty sphere")`
#[test]
fn test_bs_from_points_empty() {
    let sphere = BoundingSphere::from_points(&[]);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromPoints works with one point")`
#[test]
fn test_bs_from_points_one() {
    let expected_center = DVec3::new(1.0, 2.0, 3.0);
    let sphere = BoundingSphere::from_points(&[expected_center]);
    assert_vec3_epsilon!(sphere.center, expected_center, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromPoints computes a center from points")`
#[test]
fn test_bs_from_points_computes_center() {
    let sphere = BoundingSphere::from_points(&get_positions());
    assert_vec3_epsilon!(sphere.center, positions_center(), epsilon::EPSILON15);
    assert_approx!(sphere.radius, POSITIONS_RADIUS, epsilon::EPSILON15);
}

/// `it("fromPoints contains all points (naive)")`
#[test]
fn test_bs_from_points_contains_naive() {
    let positions = get_positions();
    let sphere = BoundingSphere::from_points(&positions);
    assert_sphere_contains_points(&sphere, &positions);
}

/// `it("fromPoints contains all points (ritter)")`
#[test]
fn test_bs_from_points_contains_ritter() {
    let mut positions = get_positions();
    positions.push(DVec3::new(1.0, 1.0, 1.0));
    positions.push(DVec3::new(2.0, 2.0, 2.0));
    positions.push(DVec3::new(3.0, 3.0, 3.0));
    let sphere = BoundingSphere::from_points(&positions);
    assert_sphere_contains_points(&sphere, &positions);
}

/// `it("fromVertices without positions returns an empty sphere")`
#[test]
fn test_bs_from_vertices_empty() {
    let sphere = BoundingSphere::from_vertices(&[], DVec3::ZERO, 3);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromVertices works with one point")`
#[test]
fn test_bs_from_vertices_one() {
    let expected_center = DVec3::new(1.0, 2.0, 3.0);
    let sphere = BoundingSphere::from_vertices(
        &[expected_center.x, expected_center.y, expected_center.z],
        DVec3::ZERO,
        3,
    );
    assert_vec3_epsilon!(sphere.center, expected_center, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromVertices computes a center from points")`
#[test]
fn test_bs_from_vertices_computes_center() {
    let sphere = BoundingSphere::from_vertices(&get_positions_as_flat_array(), DVec3::ZERO, 3);
    assert_vec3_epsilon!(sphere.center, positions_center(), epsilon::EPSILON15);
    assert_approx!(sphere.radius, POSITIONS_RADIUS, epsilon::EPSILON15);
}

/// `it("fromVertices contains all points (naive)")`
#[test]
fn test_bs_from_vertices_contains_naive() {
    let sphere = BoundingSphere::from_vertices(&get_positions_as_flat_array(), DVec3::ZERO, 3);
    assert_sphere_contains_points(&sphere, &get_positions());
}

/// `it("fromVertices contains all points (ritter)")`
#[test]
fn test_bs_from_vertices_contains_ritter() {
    let mut flat = get_positions_as_flat_array();
    flat.extend_from_slice(&[1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 3.0, 3.0, 3.0]);
    let sphere = BoundingSphere::from_vertices(&flat, DVec3::ZERO, 3);
    let mut points = get_positions();
    points.push(DVec3::new(1.0, 1.0, 1.0));
    points.push(DVec3::new(2.0, 2.0, 2.0));
    points.push(DVec3::new(3.0, 3.0, 3.0));
    assert_sphere_contains_points(&sphere, &points);
}

/// `it("fromVertices works with a stride of 5")`
#[test]
fn test_bs_from_vertices_stride5() {
    let sphere =
        BoundingSphere::from_vertices(&get_positions_as_flat_array_with_stride5(), DVec3::ZERO, 5);
    assert_vec3_epsilon!(sphere.center, positions_center(), epsilon::EPSILON15);
    assert_approx!(sphere.radius, POSITIONS_RADIUS, epsilon::EPSILON15);
}

/// `it("fromVertices works with defined center")`
#[test]
fn test_bs_from_vertices_defined_center() {
    let center = DVec3::new(1.0, 2.0, 3.0);
    let sphere =
        BoundingSphere::from_vertices(&get_positions_as_flat_array_with_stride5(), center, 5);
    assert_vec3_epsilon!(sphere.center, positions_center() + center, epsilon::EPSILON15);
    assert_approx!(sphere.radius, POSITIONS_RADIUS, epsilon::EPSILON15);
}

/// `it("fromEncodedCartesianVertices without positions returns an empty sphere")`
#[test]
fn test_bs_from_encoded_empty() {
    let sphere = BoundingSphere::from_encoded_cartesian_vertices(&[], &[]);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromEncodedCartesianVertices without positions of different lengths returns an empty sphere")`
#[test]
fn test_bs_from_encoded_different_lengths() {
    let high = get_positions_as_flat_array();
    let mut low = vec![0.0; high.len()];
    low.pop(); // make the lengths differ
    let sphere = BoundingSphere::from_encoded_cartesian_vertices(&high, &low);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromEncodedCartesianVertices computes a center from points")`
#[test]
fn test_bs_from_encoded_computes_center() {
    let high = get_positions_as_flat_array();
    let low = vec![0.0; high.len()];
    let sphere = BoundingSphere::from_encoded_cartesian_vertices(&high, &low);
    assert_vec3_epsilon!(sphere.center, positions_center(), epsilon::EPSILON15);
    assert_approx!(sphere.radius, POSITIONS_RADIUS, epsilon::EPSILON15);
}

/// `it("fromEncodedCartesianVertices contains all points (naive)")`
#[test]
fn test_bs_from_encoded_contains_naive() {
    let high = get_positions_as_flat_array();
    let low = vec![0.0; high.len()];
    let sphere = BoundingSphere::from_encoded_cartesian_vertices(&high, &low);
    assert_sphere_contains_points(&sphere, &get_positions());
}

/// `it("fromEncodedCartesianVertices contains all points (ritter)")`
///
/// Note: the JS original iterates `positions.length` where `positions` is a `{high, low}`
/// object, so its loop body never executes (vacuous). Here we check containment against the
/// actual decoded positions to keep the test meaningful.
#[test]
fn test_bs_from_encoded_contains_ritter() {
    let mut high = get_positions_as_flat_array();
    let mut low = vec![0.0; high.len()];
    let appended = [
        DVec3::new(1.0, 1.0, 1.0),
        DVec3::new(2.0, 2.0, 2.0),
        DVec3::new(3.0, 3.0, 3.0),
    ];
    let mut points = get_positions();
    for a in appended {
        let p = a + center_offset();
        high.extend_from_slice(&[p.x, p.y, p.z]);
        low.extend_from_slice(&[0.0, 0.0, 0.0]);
        points.push(p);
    }
    let sphere = BoundingSphere::from_encoded_cartesian_vertices(&high, &low);
    assert_sphere_contains_points(&sphere, &points);
}

/// `it("fromRectangle2D")`
#[test]
fn test_bs_from_rectangle_2d() {
    let rectangle = Rectangle::MAX_VALUE;
    let projection = GeographicProjection::new(Ellipsoid::UNIT_SPHERE);
    let expected = BoundingSphere::new(
        DVec3::ZERO,
        (rectangle.east * rectangle.east + rectangle.north * rectangle.north).sqrt(),
    );
    let result = BoundingSphere::from_rectangle_2d(&rectangle, &projection);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("fromRectangle3D")`
#[test]
fn test_bs_from_rectangle_3d() {
    let rectangle = Rectangle::MAX_VALUE;
    let ellipsoid = Ellipsoid::WGS84;
    let expected = BoundingSphere::new(DVec3::ZERO, ellipsoid.maximum_radius());
    let result = BoundingSphere::from_rectangle_3d(&rectangle, &ellipsoid, 0.0);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON10);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON10);
}

/// `it("fromRectangle3D with height")`
#[test]
fn test_bs_from_rectangle_3d_with_height() {
    let rectangle = Rectangle::new(0.1, -0.3, 0.2, -0.4);
    let height = 100000.0;
    let ellipsoid = Ellipsoid::WGS84;
    let points = rectangle.subsample(&ellipsoid, height);
    let expected = BoundingSphere::from_points(&points);
    let result = BoundingSphere::from_rectangle_3d(&rectangle, &ellipsoid, height);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("fromCornerPoints")`
#[test]
fn test_bs_from_corner_points() {
    let sphere =
        BoundingSphere::from_corner_points(DVec3::new(-1.0, 0.0, 0.0), DVec3::new(1.0, 0.0, 0.0));
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 1.0, epsilon::EPSILON15);
}

/// `it("fromEllipsoid")`
#[test]
fn test_bs_from_ellipsoid() {
    let ellipsoid = Ellipsoid::WGS84;
    let sphere = BoundingSphere::from_ellipsoid(&ellipsoid);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, ellipsoid.maximum_radius(), epsilon::EPSILON15);
}

/// `it("fromBoundingSpheres with empty array returns an empty sphere")`
#[test]
fn test_bs_from_bounding_spheres_empty() {
    let sphere = BoundingSphere::from_bounding_spheres(&[]);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("fromBoundingSpheres works with 1 sphere")`
#[test]
fn test_bs_from_bounding_spheres_one() {
    let one = BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 4.0);
    let sphere = BoundingSphere::from_bounding_spheres(&[one]);
    assert!(sphere == one);
}

/// `it("fromBoundingSpheres works with 2 spheres")`
#[test]
fn test_bs_from_bounding_spheres_two() {
    let one = BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 4.0);
    let two = BoundingSphere::new(DVec3::new(5.0, 6.0, 7.0), 8.0);
    let sphere = BoundingSphere::from_bounding_spheres(&[one, two]);
    let expected = one.union(&two);
    assert_vec3_epsilon!(sphere.center, expected.center, epsilon::EPSILON15);
    assert_approx!(sphere.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("fromBoundingSpheres works with 3 spheres")`
#[test]
fn test_bs_from_bounding_spheres_three() {
    let one = BoundingSphere::new(DVec3::new(0.0, 0.0, 0.0), 1.0);
    let two = BoundingSphere::new(DVec3::new(0.0, 3.0, 0.0), 1.0);
    let three = BoundingSphere::new(DVec3::new(0.0, 0.0, 4.0), 1.0);
    let expected = BoundingSphere::new(DVec3::new(0.0, 1.5, 2.0), 3.5);
    let sphere = BoundingSphere::from_bounding_spheres(&[one, two, three]);
    assert_vec3_epsilon!(sphere.center, expected.center, epsilon::EPSILON15);
    assert_approx!(sphere.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("fromTransformation works without a result parameter")`
#[test]
fn test_bs_from_transformation() {
    let translation = DVec3::new(1.0, 2.0, 3.0);
    let rotation = DQuat::from_axis_angle(DVec3::Z, 0.4);
    let scale = DVec3::new(1.0, 2.0, 3.0);
    let expected_radius = 0.5 * scale.length();
    let transformation = DMat4::from_scale_rotation_translation(scale, rotation, translation);
    let sphere = BoundingSphere::from_transformation(&transformation);
    assert_vec3_epsilon!(sphere.center, translation, epsilon::EPSILON14);
    assert_approx!(sphere.radius, expected_radius, epsilon::EPSILON14);
}

/// `it("fromTransformation works with a transformation that has zero scale")`
#[test]
fn test_bs_from_transformation_zero_scale() {
    let transformation = DMat4::from_scale(DVec3::ZERO);
    let sphere = BoundingSphere::from_transformation(&transformation);
    assert_vec3_epsilon!(sphere.center, DVec3::ZERO, epsilon::EPSILON15);
    assert_approx!(sphere.radius, 0.0, epsilon::EPSILON15);
}

/// `it("intersectPlane with sphere on the positive side of a plane")`
#[test]
fn test_bs_intersect_plane_positive() {
    let sphere = BoundingSphere::new(DVec3::ZERO, 0.5);
    let normal = -DVec3::X;
    let position = DVec3::X;
    let distance = -normal.dot(position);
    assert!(sphere.intersect_plane(normal, distance) == Intersect::Inside);
}

/// `it("intersectPlane with sphere on the negative side of a plane")`
#[test]
fn test_bs_intersect_plane_negative() {
    let sphere = BoundingSphere::new(DVec3::ZERO, 0.5);
    let normal = DVec3::X;
    let position = DVec3::X;
    let distance = -normal.dot(position);
    assert!(sphere.intersect_plane(normal, distance) == Intersect::Outside);
}

/// `it("intersectPlane with sphere intersecting a plane")`
#[test]
fn test_bs_intersect_plane_intersecting() {
    let sphere = BoundingSphere::new(DVec3::X, 0.5);
    let normal = DVec3::X;
    let position = DVec3::X;
    let distance = -normal.dot(position);
    assert!(sphere.intersect_plane(normal, distance) == Intersect::Intersecting);
}

/// `it("expands to contain another sphere")` (union)
#[test]
fn test_bs_union() {
    let bs1 = BoundingSphere::new(-DVec3::X, 1.0);
    let bs2 = BoundingSphere::new(DVec3::X, 1.0);
    let expected = BoundingSphere::new(DVec3::ZERO, 2.0);
    let result = bs1.union(&bs2);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("union left sphere encloses right")`
#[test]
fn test_bs_union_left_encloses_right() {
    let bs1 = BoundingSphere::new(DVec3::ZERO, 3.0);
    let bs2 = BoundingSphere::new(DVec3::X, 1.0);
    let result = bs1.union(&bs2);
    assert!(result == bs1);
}

/// `it("union of co-located spheres, right sphere encloses left")`
#[test]
fn test_bs_union_right_encloses_left() {
    let bs1 = BoundingSphere::new(DVec3::X, 1.0);
    let bs2 = BoundingSphere::new(DVec3::X, 2.0);
    let result = bs1.union(&bs2);
    assert!(result == bs2);
}

/// `it("union result parameter is a tight fit")`
#[test]
fn test_bs_union_tight_fit() {
    let bs1 = BoundingSphere::new(-DVec3::X * 3.0, 3.0);
    let bs2 = BoundingSphere::new(DVec3::X, 1.0);
    let expected = BoundingSphere::new(-DVec3::X * 2.0, 4.0);
    let result = bs1.union(&bs2);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("expands to contain another point")`
#[test]
fn test_bs_expand_point() {
    let bs = BoundingSphere::new(-DVec3::X, 1.0);
    let point = DVec3::X;
    let expected = BoundingSphere::new(-DVec3::X, 2.0);
    let result = bs.expand(point);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("applies transform")`
#[test]
fn test_bs_transform() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let transform = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    let expected = BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 1.0);
    let result = bs.transform(&transform);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("applies scale transform")`
#[test]
fn test_bs_transform_scale() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let transform = DMat4::from_scale(DVec3::new(1.0, 2.0, 3.0));
    let expected = BoundingSphere::new(DVec3::ZERO, 3.0);
    let result = bs.transform(&transform);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("applies transform without scale")`
#[test]
fn test_bs_transform_without_scale() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let transform = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0));
    let expected = BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 1.0);
    let result = bs.transform_without_scale(&transform);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("transformWithoutScale ignores scale")`
#[test]
fn test_bs_transform_without_scale_ignores_scale() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let transform = DMat4::from_scale(DVec3::new(1.0, 2.0, 3.0));
    let expected = BoundingSphere::new(DVec3::ZERO, 1.0);
    let result = bs.transform_without_scale(&transform);
    assert_vec3_epsilon!(result.center, expected.center, epsilon::EPSILON15);
    assert_approx!(result.radius, expected.radius, epsilon::EPSILON15);
}

/// `it("finds distances")` (computePlaneDistances)
#[test]
fn test_bs_compute_plane_distances() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let position = DVec3::new(-2.0, 1.0, 0.0);
    let direction = DVec3::X;
    let expected = Interval::new(1.0, 3.0);
    let result = bs.compute_plane_distances(position, direction);
    assert_approx!(result.start, expected.start, epsilon::EPSILON15);
    assert_approx!(result.stop, expected.stop, epsilon::EPSILON15);
}

/// `it("distance squared to point outside of sphere")`
#[test]
fn test_bs_distance_squared_to_outside() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let position = DVec3::new(-2.0, 1.0, 0.0);
    assert_approx!(bs.distance_squared_to(position), 1.52786405, epsilon::EPSILON6);
}

/// `it("distance squared to point inside sphere")`
#[test]
fn test_bs_distance_squared_to_inside() {
    let bs = BoundingSphere::new(DVec3::ZERO, 1.0);
    let position = DVec3::new(-0.5, 0.5, 0.0);
    assert_approx!(bs.distance_squared_to(position), 0.0, epsilon::EPSILON15);
}

/// `it("fromRectangleWithHeights2D includes specified min and max heights")`
#[test]
fn test_bs_from_rectangle_with_heights_2d() {
    let rectangle = Rectangle::new(0.1, 0.5, 0.2, 0.6);
    let projection = GeographicProjection::new(Ellipsoid::WGS84);
    let min_height = -327.0;
    let max_height = 2456.0;
    let sphere = BoundingSphere::from_rectangle_with_heights_2d(
        &rectangle,
        &projection,
        min_height,
        max_height,
    );

    let center = rectangle.center();
    let corners = [
        rectangle.southwest(),
        rectangle.northeast(),
        rectangle.southeast(),
        rectangle.northwest(),
    ];

    let mut test_points: Vec<Cartographic> = Vec::new();
    // Corners at both height extremes.
    for c in corners {
        test_points.push(Cartographic::from_radians(c.longitude, c.latitude, min_height));
        test_points.push(Cartographic::from_radians(c.longitude, c.latitude, max_height));
    }
    // Center at both height extremes.
    test_points.push(Cartographic::from_radians(center.longitude, center.latitude, min_height));
    test_points.push(Cartographic::from_radians(center.longitude, center.latitude, max_height));
    // Edge midpoints at both height extremes.
    let edge_midpoints = [
        (center.longitude, rectangle.south),
        (center.longitude, rectangle.north),
        (rectangle.west, center.latitude),
        (rectangle.east, center.latitude),
    ];
    for (lon, lat) in edge_midpoints {
        test_points.push(Cartographic::from_radians(lon, lat, min_height));
        test_points.push(Cartographic::from_radians(lon, lat, max_height));
    }

    for point in test_points {
        expect_bounding_sphere_to_contain_point(&sphere, point, &projection);
    }
}

/// `it("computes the volume of a BoundingSphere")`
#[test]
fn test_bs_volume() {
    let sphere = BoundingSphere::new(DVec3::ZERO, 1.0);
    let expected = (4.0 / 3.0) * std::f64::consts::PI;
    assert_approx!(sphere.volume(), expected, epsilon::EPSILON6);
}
