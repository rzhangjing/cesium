//! Tests for `cesium_core::ellipsoidal_occluder::EllipsoidalOccluder`.
//!
//! Mirrors `packages/engine/Specs/Core/EllipsoidalOccluderSpec.js`.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoidal_occluder::EllipsoidalOccluder;
use cesium_core::intersection_tests::IntersectionTests;
use cesium_core::math::CesiumMath;
use cesium_core::ray::Ray;
use cesium_core::rectangle::Rectangle;
use cesium_test_utils::assert_approx_eq_f64;
use cesium_test_utils::expect_to_throw_dev_error;

#[test]
fn uses_ellipsoid() {
    let ellipsoid = Ellipsoid::new(2.0, 3.0, 4.0);
    let occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), None);
    assert_eq!(occluder.ellipsoid().radii(), ellipsoid.radii());
}

#[test]
fn throws_if_ellipsoid_is_not_provided_to_constructor() {
    expect_to_throw_dev_error(|| {
        EllipsoidalOccluder::new(None, Some(&Cartesian3::new(1.0, 2.0, 3.0)));
    });
}

#[test]
fn is_point_visible_example_works_as_claimed() {
    let camera_position = Cartesian3::new(0.0, 0.0, 2.5);
    let ellipsoid = Ellipsoid::new(1.0, 1.1, 0.9);
    let occluder = EllipsoidalOccluder::new(Some(ellipsoid), Some(&camera_position));
    let point = Cartesian3::new(0.0, -3.0, -3.0);
    assert_eq!(occluder.is_point_visible(&point), true);
}

#[test]
fn is_scaled_space_point_visible_example_works_as_claimed() {
    let camera_position = Cartesian3::new(0.0, 0.0, 2.5);
    let ellipsoid = Ellipsoid::new(1.0, 1.1, 0.9);
    let occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), Some(&camera_position));
    let point = Cartesian3::new(0.0, -3.0, -3.0);
    let mut scaled_space_point = Cartesian3::default();
    ellipsoid.transform_position_to_scaled_space(&point, &mut scaled_space_point);
    assert_eq!(
        occluder.is_scaled_space_point_visible(&scaled_space_point),
        true
    );
}

#[test]
fn is_scaled_space_point_visible_possibly_under_ellipsoid_example_works_as_claimed() {
    // Tests points that are halfway inside a unit sphere:
    // 1) on the diagonal
    // 2) on the +y-axis
    // The camera is on the +z-axis so it will be able to see the diagonal
    // point but not the +y-axis point.

    let camera_position = Cartesian3::new(0.0, 0.0, 1.0);
    let ellipsoid = Ellipsoid::new(1.0, 1.0, 1.0);
    let occluder = EllipsoidalOccluder::new(Some(ellipsoid), Some(&camera_position));
    let height = -0.5;

    let direction = Cartesian3::normalize_new(&Cartesian3::new(1.0, 1.0, 1.0));
    let point = Cartesian3::multiply_by_scalar_new(&direction, 0.5);
    let scaled_space_point = occluder
        .compute_horizon_culling_point_new(&point, &[point])
        .unwrap();
    let scaled_space_point_shrunk = occluder
        .compute_horizon_culling_point_possibly_under_ellipsoid_new(
            &point,
            &[point],
            Some(height),
        )
        .unwrap();

    assert_eq!(occluder.is_scaled_space_point_visible(&scaled_space_point), false);
    assert_eq!(
        occluder.is_scaled_space_point_visible_possibly_under_ellipsoid(
            &scaled_space_point_shrunk,
            Some(height),
        ),
        true
    );

    let direction = Cartesian3::new(0.0, 1.0, 0.0);
    let point = Cartesian3::multiply_by_scalar_new(&direction, 0.5);
    let scaled_space_point = occluder
        .compute_horizon_culling_point_new(&point, &[point])
        .unwrap();
    let scaled_space_point_shrunk = occluder
        .compute_horizon_culling_point_possibly_under_ellipsoid_new(
            &point,
            &[point],
            Some(height),
        )
        .unwrap();

    assert_eq!(occluder.is_scaled_space_point_visible(&scaled_space_point), false);
    assert_eq!(
        occluder.is_scaled_space_point_visible_possibly_under_ellipsoid(
            &scaled_space_point_shrunk,
            Some(height),
        ),
        false
    );
}

#[test]
fn reports_not_visible_when_point_is_directly_behind_ellipsoid() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    occluder.set_camera_position(&Cartesian3::new(7000000.0, 0.0, 0.0));

    let point = Cartesian3::new(-7000000.0, 0.0, 0.0);
    assert_eq!(occluder.is_point_visible(&point), false);
}

#[test]
fn reports_not_visible_when_point_is_directly_behind_ellipsoid_and_camera_is_inside_the_ellipsoid()
{
    let ellipsoid = Ellipsoid::WGS84;
    let mut occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), None);
    occluder.set_camera_position(&Cartesian3::new(
        ellipsoid.minimum_radius() - 100.0,
        0.0,
        0.0,
    ));

    let point = Cartesian3::new(-7000000.0, 0.0, 0.0);
    assert_eq!(occluder.is_point_visible(&point), false);
}

#[test]
fn reports_visible_when_point_is_in_front_of_ellipsoid() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    occluder.set_camera_position(&Cartesian3::new(7000000.0, 0.0, 0.0));

    let point = Cartesian3::new(6900000.0, 0.0, 0.0);
    assert_eq!(occluder.is_point_visible(&point), true);
}

#[test]
fn reports_visible_when_point_is_in_opposite_direction_from_ellipsoid() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    occluder.set_camera_position(&Cartesian3::new(7000000.0, 0.0, 0.0));

    let point = Cartesian3::new(7100000.0, 0.0, 0.0);
    assert_eq!(occluder.is_point_visible(&point), true);
}

#[test]
fn reports_not_visible_when_point_is_over_horizon() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    occluder.set_camera_position(&Cartesian3::new(7000000.0, 0.0, 0.0));

    let point = Cartesian3::new(4510635.0, 4510635.0, 0.0);
    assert_eq!(occluder.is_point_visible(&point), false);
}

// DEVIATION: the JS case "requires directionToPoint and positions" passes
// `undefined` arguments to trigger DeveloperErrors; Rust's type system makes
// both parameters required, so there is nothing to mirror.

#[test]
fn returns_point_on_ellipsoid_when_single_position_is_on_center_line() {
    let ellipsoid = Ellipsoid::new(12345.0, 4567.0, 8910.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let positions = [Cartesian3::new(12345.0, 0.0, 0.0)];
    let direction_to_point = Cartesian3::new(1.0, 0.0, 0.0);

    let result = ellipsoidal_occluder
        .compute_horizon_culling_point_new(&direction_to_point, &positions)
        .unwrap();

    assert_approx_eq_f64!(result.x, 1.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result.y, 0.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result.z, 0.0, CesiumMath::EPSILON14);
}

#[test]
fn returns_undefined_when_horizon_of_single_point_is_parallel_to_center_line() {
    let ellipsoid = Ellipsoid::new(12345.0, 4567.0, 8910.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let positions = [Cartesian3::new(0.0, 4567.0, 0.0)];
    let direction_to_point = Cartesian3::new(1.0, 0.0, 0.0);

    let result =
        ellipsoidal_occluder.compute_horizon_culling_point_new(&direction_to_point, &positions);
    assert!(result.is_none());
}

#[test]
fn returns_undefined_when_single_point_is_in_the_opposite_direction_of_the_center_line() {
    let ellipsoid = Ellipsoid::new(12345.0, 4567.0, 8910.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let positions = [Cartesian3::new(-14000.0, -1000.0, 0.0)];
    let direction_to_point = Cartesian3::new(1.0, 0.0, 0.0);

    let result =
        ellipsoidal_occluder.compute_horizon_culling_point_new(&direction_to_point, &positions);
    assert!(result.is_none());
}

#[test]
fn returns_undefined_when_any_point_is_in_the_opposite_direction_of_the_center_line() {
    let ellipsoid = Ellipsoid::new(1.0, 1.0, 1.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let positions = [
        Cartesian3::new(2.0, 0.0, 0.0),
        Cartesian3::new(-1.0, 0.0, 0.0),
    ];
    let direction_to_point = Cartesian3::new(1.0, 0.0, 0.0);

    let result =
        ellipsoidal_occluder.compute_horizon_culling_point_new(&direction_to_point, &positions);
    assert!(result.is_none());
}

#[test]
fn returns_undefined_when_the_direction_is_zero() {
    let ellipsoid = Ellipsoid::new(1.0, 1.0, 1.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let positions = [Cartesian3::new(1.0, 0.0, 0.0)];
    let direction_to_point = Cartesian3::new(0.0, 0.0, 0.0);

    let result =
        ellipsoidal_occluder.compute_horizon_culling_point_new(&direction_to_point, &positions);
    assert!(result.is_none());
}

#[test]
fn computes_a_point_from_a_single_position_with_a_grazing_altitude_close_to_zero() {
    let ellipsoid = Ellipsoid::new(12345.0, 12345.0, 12345.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), None);

    let positions = [
        Cartesian3::new(-12345.0, 12345.0, 12345.0),
        Cartesian3::new(-12346.0, 12345.0, 12345.0),
    ];
    let bounding_sphere = BoundingSphere::from_points(&positions, None);

    let first_position_array = [positions[0]];
    let result = ellipsoidal_occluder
        .compute_horizon_culling_point_new(&bounding_sphere.center, &first_position_array)
        .unwrap();
    let mut unscaled_result = Cartesian3::default();
    Cartesian3::multiply_components(&result, ellipsoid.radii(), &mut unscaled_result);

    // The grazing altitude of the ray from the horizon culling point to the
    // position used to compute it should be very nearly zero.
    let mut direction = Cartesian3::default();
    Cartesian3::subtract(&positions[0], &unscaled_result, &mut direction);
    let direction = Cartesian3::normalize_new(&direction);
    let ray = Ray::new(Some(&unscaled_result), Some(&direction));
    let nearest = IntersectionTests::grazing_altitude_location(&ray, &ellipsoid).unwrap();
    let mut nearest_cartographic = Cartographic::default();
    assert!(ellipsoid.cartesian_to_cartographic(&nearest, &mut nearest_cartographic));
    assert_approx_eq_f64!(nearest_cartographic.height, 0.0, CesiumMath::EPSILON5);
}

#[test]
fn computes_a_point_from_multiple_positions_with_a_grazing_altitude_close_to_zero_for_one_of_the_positions_and_less_than_zero_for_the_others(
) {
    let ellipsoid = Ellipsoid::new(12345.0, 12345.0, 12345.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), None);

    let positions = [
        Cartesian3::new(-12345.0, 12345.0, 12345.0),
        Cartesian3::new(-12346.0, 12345.0, 12345.0),
        Cartesian3::new(-12446.0, 12445.0, 12445.0),
    ];
    let bounding_sphere = BoundingSphere::from_points(&positions, None);

    let result = ellipsoidal_occluder
        .compute_horizon_culling_point_new(&bounding_sphere.center, &positions)
        .unwrap();
    let mut unscaled_result = Cartesian3::default();
    Cartesian3::multiply_components(&result, ellipsoid.radii(), &mut unscaled_result);

    // The grazing altitude of the ray from the horizon culling point to the
    // position used to compute it should be very nearly zero.
    let mut found_one_near_zero = false;
    for position in &positions {
        let mut direction = Cartesian3::default();
        Cartesian3::subtract(position, &unscaled_result, &mut direction);
        let direction = Cartesian3::normalize_new(&direction);
        let ray = Ray::new(Some(&unscaled_result), Some(&direction));
        let nearest = IntersectionTests::grazing_altitude_location(&ray, &ellipsoid).unwrap();
        let mut nearest_cartographic = Cartographic::default();
        assert!(ellipsoid.cartesian_to_cartographic(&nearest, &mut nearest_cartographic));
        if nearest_cartographic.height.abs() < CesiumMath::EPSILON5 {
            found_one_near_zero = true;
        } else {
            assert!(nearest_cartographic.height < 0.0);
        }
    }

    assert!(found_one_near_zero);
}

#[test]
fn computes_a_point_under_the_ellipsoid_with_compute_horizon_culling_point_possibly_under_ellipsoid(
) {
    let ellipsoid = Ellipsoid::new(12345.0, 4567.0, 8910.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let positions = [Cartesian3::new(12344.0, 0.0, 0.0)];
    let direction_to_point = Cartesian3::new(1.0, 0.0, 0.0);

    let result = ellipsoidal_occluder
        .compute_horizon_culling_point_possibly_under_ellipsoid_new(
            &direction_to_point,
            &positions,
            Some(-1.0),
        )
        .unwrap();

    assert_approx_eq_f64!(result.x, 1.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result.y, 0.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result.z, 0.0, CesiumMath::EPSILON14);
}

#[test]
fn compute_horizon_culling_point_from_vertices_requires_stride() {
    // DEVIATION: the JS case also passes `undefined` for directionToPoint and
    // vertices, which Rust's type system already forbids; only the missing
    // stride check is mirrored.
    let ellipsoid = Ellipsoid::new(12345.0, 12345.0, 12345.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);

    let positions = [
        Cartesian3::new(-12345.0, 12345.0, 12345.0),
        Cartesian3::new(-12346.0, 12345.0, 12345.0),
        Cartesian3::new(-12446.0, 12445.0, 12445.0),
    ];
    let bounding_sphere = BoundingSphere::from_points(&positions, None);

    let mut vertices: Vec<f64> = Vec::new();
    for position in &positions {
        vertices.push(position.x);
        vertices.push(position.y);
        vertices.push(position.z);
        vertices.push(1.0);
        vertices.push(2.0);
        vertices.push(3.0);
        vertices.push(4.0);
    }

    ellipsoidal_occluder
        .compute_horizon_culling_point_from_vertices_new(
            &bounding_sphere.center,
            &vertices,
            Some(7),
            None,
        )
        .unwrap();

    expect_to_throw_dev_error(|| {
        ellipsoidal_occluder.compute_horizon_culling_point_from_vertices_new(
            &bounding_sphere.center,
            &vertices,
            None,
            None,
        );
    });
}

#[test]
fn from_vertices_produces_same_answers_as_compute_horizon_culling_point() {
    let ellipsoid = Ellipsoid::new(12345.0, 12345.0, 12345.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);

    let positions = [
        Cartesian3::new(-12345.0, 12345.0, 12345.0),
        Cartesian3::new(-12346.0, 12345.0, 12345.0),
        Cartesian3::new(-12446.0, 12445.0, 12445.0),
    ];
    let bounding_sphere = BoundingSphere::from_points(&positions, None);

    let center = Cartesian3::new(-12000.0, 12000.0, 12000.0);

    let mut vertices: Vec<f64> = Vec::new();
    for position in &positions {
        vertices.push(position.x - center.x);
        vertices.push(position.y - center.y);
        vertices.push(position.z - center.z);
        vertices.push(1.0);
        vertices.push(2.0);
        vertices.push(3.0);
        vertices.push(4.0);
    }

    let result1 = ellipsoidal_occluder
        .compute_horizon_culling_point_new(&bounding_sphere.center, &positions)
        .unwrap();
    let result2 = ellipsoidal_occluder
        .compute_horizon_culling_point_from_vertices_new(
            &bounding_sphere.center,
            &vertices,
            Some(7),
            Some(&center),
        )
        .unwrap();

    assert_approx_eq_f64!(result1.x, result2.x, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result1.y, result2.y, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result1.z, result2.z, CesiumMath::EPSILON14);
}

#[test]
fn computes_a_point_under_the_ellipsoid_with_compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid(
) {
    let ellipsoid = Ellipsoid::new(12345.0, 4567.0, 8910.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid), None);
    let vertices = [12344.0, 0.0, 0.0];
    let direction_to_point = Cartesian3::new(1.0, 0.0, 0.0);

    let result = ellipsoidal_occluder
        .compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid_new(
            &direction_to_point,
            &vertices,
            Some(3),
            Some(&Cartesian3::ZERO),
            Some(-1.0),
        )
        .unwrap();

    assert_approx_eq_f64!(result.x, 1.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result.y, 0.0, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(result.z, 0.0, CesiumMath::EPSILON14);
}

#[test]
fn from_rectangle_returns_undefined_for_global_rectangle() {
    let ellipsoid = Ellipsoid::new(12345.0, 12345.0, 12345.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), None);
    let rectangle = Rectangle::MAX_VALUE;
    let result =
        ellipsoidal_occluder.compute_horizon_culling_point_from_rectangle_new(&rectangle, &ellipsoid);
    assert!(result.is_none());
}

#[test]
fn from_rectangle_computes_a_point_with_a_grazing_altitude_close_to_zero_for_one_of_the_rectangle_corners_and_less_than_or_equal_to_zero_for_the_others(
) {
    let ellipsoid = Ellipsoid::new(12345.0, 12345.0, 12345.0);
    let ellipsoidal_occluder = EllipsoidalOccluder::new(Some(ellipsoid.clone()), None);

    let rectangle = Rectangle::new(0.1, 0.2, 0.3, 0.4);
    let result = ellipsoidal_occluder
        .compute_horizon_culling_point_from_rectangle_new(&rectangle, &ellipsoid)
        .unwrap();
    let mut unscaled_result = Cartesian3::default();
    Cartesian3::multiply_components(&result, ellipsoid.radii(), &mut unscaled_result);

    // The grazing altitude of the ray from the horizon culling point to the
    // position used to compute it should be very nearly zero.
    let corners = [
        Rectangle::southwest(&rectangle),
        Rectangle::southeast(&rectangle),
        Rectangle::northwest(&rectangle),
        Rectangle::northeast(&rectangle),
    ];
    let mut positions: Vec<Cartesian3> = Vec::new();
    for corner in &corners {
        let mut c = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(corner, &mut c);
        positions.push(c);
    }

    let mut found_one_near_zero = false;
    for position in &positions {
        let mut direction = Cartesian3::default();
        Cartesian3::subtract(position, &unscaled_result, &mut direction);
        let direction = Cartesian3::normalize_new(&direction);
        let ray = Ray::new(Some(&unscaled_result), Some(&direction));
        let nearest = IntersectionTests::grazing_altitude_location(&ray, &ellipsoid).unwrap();
        let mut nearest_cartographic = Cartographic::default();
        assert!(ellipsoid.cartesian_to_cartographic(&nearest, &mut nearest_cartographic));
        if nearest_cartographic.height.abs() < CesiumMath::EPSILON5 {
            found_one_near_zero = true;
        } else {
            assert!(nearest_cartographic.height <= 0.0);
        }
    }

    assert!(found_one_near_zero);
}
