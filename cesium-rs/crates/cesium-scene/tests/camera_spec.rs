//! B4-1 spec mirror: pure-math cases from
//! `packages/engine/Specs/Scene/CameraSpec.js`.
//!
//! Standalone integration-test entry — the specs aggregator under
//! `specs/tests/` is intentionally untouched.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::math::CesiumMath;
use cesium_core::matrix4::Matrix4;
use cesium_scene::camera::{Camera, CameraProjection};
use cesium_test_utils::assert_approx_eq_f64;

/// Mirrors the spec `beforeEach`: position = UNIT_Z, up = UNIT_Y,
/// direction = -UNIT_Z, right = direction × up.
fn spec_camera() -> Camera {
    let position = Cartesian3::UNIT_Z;
    let up = Cartesian3::UNIT_Y;
    let dir = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_Z, -1.0);
    let right = Cartesian3::cross_new(&dir, &up);

    let mut camera = Camera::new();
    camera.set_position(position);
    camera.set_up(up);
    camera.set_direction(dir);
    camera.set_right(right);
    camera.update();
    camera
}

/// `it("get view matrix")`
#[test]
fn get_view_matrix() {
    let camera = spec_camera();
    let position = *camera.position();
    let up = *camera.up();
    let dir = *camera.direction();
    let right = *camera.right();

    let rotation = Matrix4::new(
        right.x, right.y, right.z, 0.0,
        up.x, up.y, up.z, 0.0,
        -dir.x, -dir.y, -dir.z, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );
    // Matrix4::new takes row-ordered parameters (like the JS constructor).
    let translation = Matrix4::new(
        1.0, 0.0, 0.0, -position.x,
        0.0, 1.0, 0.0, -position.y,
        0.0, 0.0, 1.0, -position.z,
        0.0, 0.0, 0.0, 1.0,
    );
    let expected = Matrix4::multiply_new(&rotation, &translation);
    for i in 0..16 {
        assert_approx_eq_f64!(
            camera.view_matrix().elements[i],
            expected.elements[i],
            CesiumMath::EPSILON14
        );
    }
}

/// `it("get inverse view matrix")`
#[test]
fn get_inverse_view_matrix() {
    let camera = spec_camera();
    let expected = Matrix4::inverse_new(camera.view_matrix()).unwrap();
    for i in 0..16 {
        assert_approx_eq_f64!(
            expected.elements[i],
            camera.inverse_view_matrix().elements[i],
            CesiumMath::EPSILON15
        );
    }
}

/// `it("Computes orthonormal direction, up, and right vectors")`
#[test]
fn computes_orthonormal_direction_up_and_right_vectors() {
    let mut camera = Camera::new();
    camera.set_direction(Cartesian3::new(
        -0.32297853365047874,
        0.9461560708446421,
        0.021761351171635013,
    ));
    camera.set_up(Cartesian3::new(
        0.9327219113001013,
        0.31839266745173644,
        -2.9874778345595487e-10,
    ));
    camera.set_right(Cartesian3::new(
        0.0069286549295528715,
        -0.020297288960790985,
        0.9853344956450351,
    ));

    assert!(
        (Cartesian3::magnitude(camera.right()) - 1.0).abs() > CesiumMath::EPSILON8
    );
    assert!(
        (Cartesian3::magnitude(camera.up()) - 1.0).abs() > CesiumMath::EPSILON8
    );

    // Trigger update_members which normalizes the axes.
    camera.update();
    assert_approx_eq_f64!(Cartesian3::magnitude(camera.right()), 1.0, CesiumMath::EPSILON8);
    assert_approx_eq_f64!(Cartesian3::magnitude(camera.up()), 1.0, CesiumMath::EPSILON8);

    let inverse_affine = Matrix4::inverse_transformation_new(camera.view_matrix());
    let inverse = Matrix4::inverse_new(camera.view_matrix()).unwrap();
    for i in 0..16 {
        assert_approx_eq_f64!(
            inverse_affine.elements[i],
            inverse.elements[i],
            CesiumMath::EPSILON8
        );
    }
}

/// `it("setView with cartesian in 3D")`
#[test]
fn set_view_with_cartesian_in_3d() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut camera = Camera::new();
    let cartesian = Cartesian3::from_degrees_new(-75.0, 0.0, Some(100.0), None);
    camera.set_view(&cartesian, None, None, &ellipsoid);

    let expected_direction =
        Cartesian3::normalize_new(&Cartesian3::multiply_by_scalar_new(&cartesian, -1.0));
    assert_approx_eq_f64!(camera.direction().x, expected_direction.x, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.direction().y, expected_direction.y, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.direction().z, expected_direction.z, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.up().x, Cartesian3::UNIT_Z.x, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.up().y, Cartesian3::UNIT_Z.y, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.up().z, Cartesian3::UNIT_Z.z, CesiumMath::EPSILON6);
    let expected_right = Cartesian3::cross_new(camera.direction(), camera.up());
    assert_approx_eq_f64!(camera.right().x, expected_right.x, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.right().y, expected_right.y, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.right().z, expected_right.z, CesiumMath::EPSILON6);
}

/// `it("setView with direction, up")`
#[test]
fn set_view_with_direction_up() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut camera = Camera::new();
    let direction = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_Z, -1.0);
    let up = Cartesian3::UNIT_Y;
    let destination = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    camera.set_view(&destination, Some(&direction), Some(&up), &ellipsoid);

    assert_approx_eq_f64!(camera.direction().x, direction.x, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.direction().y, direction.y, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.direction().z, direction.z, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.up().x, up.x, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.up().y, up.y, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.up().z, up.z, CesiumMath::EPSILON6);
}

/// `it("lookAt")`
#[test]
fn look_at() {
    let ellipsoid = Ellipsoid::WGS84;
    let target = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let offset = Cartesian3::new(0.0, -1.0, 0.0);

    let mut camera = spec_camera();
    camera.look_at(&target, &offset, &ellipsoid);

    assert_approx_eq_f64!(camera.position().x, offset.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.position().y, offset.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.position().z, offset.z, CesiumMath::EPSILON11);

    let expected_direction =
        Cartesian3::multiply_by_scalar_new(&Cartesian3::normalize_new(&offset), -1.0);
    assert_approx_eq_f64!(camera.direction().x, expected_direction.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.direction().y, expected_direction.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.direction().z, expected_direction.z, CesiumMath::EPSILON11);

    let expected_right = Cartesian3::cross_new(camera.direction(), &Cartesian3::UNIT_Z);
    assert_approx_eq_f64!(camera.right().x, expected_right.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.right().y, expected_right.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.right().z, expected_right.z, CesiumMath::EPSILON11);

    let expected_up = Cartesian3::cross_new(camera.right(), camera.direction());
    assert_approx_eq_f64!(camera.up().x, expected_up.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.up().y, expected_up.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.up().z, expected_up.z, CesiumMath::EPSILON11);

    assert!((1.0 - Cartesian3::magnitude(camera.direction())).abs() < CesiumMath::EPSILON14);
    assert!((1.0 - Cartesian3::magnitude(camera.up())).abs() < CesiumMath::EPSILON14);
    assert!((1.0 - Cartesian3::magnitude(camera.right())).abs() < CesiumMath::EPSILON14);
}

/// `it("lookAt when target is zero")`
#[test]
fn look_at_when_target_is_zero() {
    let ellipsoid = Ellipsoid::WGS84;
    let target = Cartesian3::ZERO;
    let offset = Cartesian3::new(0.0, -1.0, 0.0);

    let mut camera = spec_camera();
    camera.look_at(&target, &offset, &ellipsoid);

    assert_approx_eq_f64!(camera.position().x, offset.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.position().y, offset.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.position().z, offset.z, CesiumMath::EPSILON11);

    let expected_direction =
        Cartesian3::multiply_by_scalar_new(&Cartesian3::normalize_new(&offset), -1.0);
    assert_approx_eq_f64!(camera.direction().x, expected_direction.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.direction().y, expected_direction.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.direction().z, expected_direction.z, CesiumMath::EPSILON11);
}

/// Perspective projection matrix semantics: `fov` is the vertical FOV
/// (CesiumJS `PerspectiveFrustum`), so `m11 = 1/tan(fov/2)` and
/// `m00 = m11 / aspectRatio`.
#[test]
fn perspective_projection_matrix_matches_frustum_semantics() {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_fov(std::f64::consts::FRAC_PI_3);
    camera.set_near(1.0);
    camera.set_far(500.0);
    camera.update();

    let tan_half_fov = (camera.fov() * 0.5).tan();
    let aspect = camera.aspect_ratio();
    let projection = camera.projection_matrix();
    // Column-major: elements[0] = m00, elements[5] = m11.
    assert_approx_eq_f64!(projection.elements[0], 1.0 / (aspect * tan_half_fov));
    assert_approx_eq_f64!(projection.elements[5], 1.0 / tan_half_fov);
    assert_approx_eq_f64!(projection.elements[10], -(500.0 + 1.0) / (500.0 - 1.0));
    assert_approx_eq_f64!(projection.elements[11], -1.0);
    assert_approx_eq_f64!(projection.elements[14], -2.0 * 500.0 / (500.0 - 1.0));

    // Inverse round-trip.
    let inverse = Matrix4::inverse_new(projection).unwrap();
    for i in 0..16 {
        assert_approx_eq_f64!(
            inverse.elements[i],
            camera.inverse_projection_matrix().elements[i],
            CesiumMath::EPSILON12
        );
    }

    // SSE denominator mirrors PerspectiveFrustum#sseDenominator.
    assert_approx_eq_f64!(camera.sse_denominator(), 2.0 * tan_half_fov);
}

/// Orthographic projection: symmetric extents with `height = width / aspect`.
#[test]
fn orthographic_projection_matrix() {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_projection(CameraProjection::Orthographic);
    camera.set_orthographic_width(1000.0);
    camera.set_near(1.0);
    camera.set_far(500.0);
    camera.update();

    let aspect = camera.aspect_ratio();
    let half_width = 500.0;
    let half_height = half_width / aspect;
    let projection = camera.projection_matrix();
    assert_approx_eq_f64!(projection.elements[0], 2.0 / (2.0 * half_width));
    assert_approx_eq_f64!(projection.elements[5], 2.0 / (2.0 * half_height));
}

/// CameraSpec pickEllipsoid geometry: a ray straight down from over the pole
/// hits the polar radius; a horizontal ray misses.
#[test]
fn pick_ellipsoid_hits_and_misses() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_position(Cartesian3::new(0.0, 0.0, 2.0 * ellipsoid.maximum_radius()));
    camera.set_direction(Cartesian3::new(0.0, 0.0, -1.0));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
    camera.update();

    let center = Cartesian2::new(400.0, 300.0);

    // Center pick: ray direction equals the camera direction.
    let ray = camera.get_pick_ray(&center);
    assert_approx_eq_f64!(ray.direction.x, 0.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.y, 0.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.z, -1.0, CesiumMath::EPSILON12);

    let picked = camera.pick_ellipsoid(&center, &ellipsoid).expect("must hit");
    assert_approx_eq_f64!(picked.x, 0.0, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(picked.y, 0.0, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(picked.z, ellipsoid.radii().z, CesiumMath::EPSILON6);

    // Horizontal ray from the same position misses the ellipsoid.
    camera.set_direction(Cartesian3::UNIT_X);
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::new(0.0, 0.0, -1.0));
    camera.update();
    assert!(camera.pick_ellipsoid(&center, &ellipsoid).is_none());
}

/// The perspective pick ray at the window corners follows the frustum's
/// tangent angles (`tanTheta = aspect * tan(fov/2)` horizontal,
/// `tanPhi = tan(fov/2)` vertical).
#[test]
fn get_pick_ray_corner_directions_follow_frustum_angles() {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_position(Cartesian3::new(0.0, 0.0, 100.0));
    camera.set_direction(Cartesian3::new(0.0, 0.0, -1.0));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
    camera.update();

    let tan_phi = (camera.fov() * 0.5).tan();
    let tan_theta = camera.aspect_ratio() * tan_phi;

    // Top-left corner: (-1, +1) NDC → direction ∝ dir - right*tanθ + up*tanφ.
    let ray = camera.get_pick_ray(&Cartesian2::new(0.0, 0.0));
    let expected = Cartesian3::normalize_new(&Cartesian3::new(-tan_theta, tan_phi, -1.0));
    assert_approx_eq_f64!(ray.direction.x, expected.x, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.y, expected.y, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.z, expected.z, CesiumMath::EPSILON12);
}

/// worldToCameraCoordinatesPoint applies the full view matrix (not a bare
/// translation) — mirrors the CameraSpec transform-based expectations.
#[test]
fn world_to_camera_coordinates_uses_view_matrix() {
    let mut camera = Camera::new();
    camera.set_position(Cartesian3::new(10.0, 20.0, 30.0));
    camera.set_direction(Cartesian3::new(0.0, 0.0, -1.0));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
    camera.update();

    let point = Cartesian3::new(15.0, 25.0, 25.0);
    let camera_space = camera.world_to_camera_coordinates(&point);
    assert_approx_eq_f64!(camera_space.x, 5.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera_space.y, 5.0, CesiumMath::EPSILON12);
    // CesiumJS camera space looks down -z: a point 5 m ahead has z = -5.
    assert_approx_eq_f64!(camera_space.z, -5.0, CesiumMath::EPSILON12);

    let round_trip = camera.camera_to_world_coordinates(&camera_space);
    assert_approx_eq_f64!(round_trip.x, point.x, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(round_trip.y, point.y, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(round_trip.z, point.z, CesiumMath::EPSILON12);
}
