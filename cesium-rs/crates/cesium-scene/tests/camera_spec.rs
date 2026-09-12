//! B4-1 spec mirror: pure-math cases from
//! `packages/engine/Specs/Scene/CameraSpec.js`.
//!
//! Standalone integration-test entry — the specs aggregator under
//! `specs/tests/` is intentionally untouched.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::heading_pitch_range::HeadingPitchRange;
use cesium_core::math::CesiumMath;
use cesium_core::matrix4::Matrix4;
use cesium_core::orthographic_frustum::OrthographicFrustum;
use cesium_core::orthographic_off_center_frustum::OrthographicOffCenterFrustum;
use cesium_core::rectangle::Rectangle;
use cesium_core::scene_mode::SceneMode;
use cesium_core::transforms;
use cesium_scene::camera::{
    Camera, CameraProjection, CameraSceneContext, FlyToBoundingSphereOptions, FlyToOptions,
    LookAtOffset, SetViewDestination, SetViewOptions, SetViewOrientation,
};
use cesium_scene::camera_flight_path::CameraFlightChannel;
use cesium_scene::camera_frustum::CameraFrustum;
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
    camera.update(SceneMode::Scene3D);
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
    camera.update(SceneMode::Scene3D);
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
    camera.look_at(&target, LookAtOffset::Cartesian(offset), &ellipsoid);

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
    camera.look_at(&target, LookAtOffset::Cartesian(offset), &ellipsoid);

    assert_approx_eq_f64!(camera.position().x, offset.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.position().y, offset.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.position().z, offset.z, CesiumMath::EPSILON11);

    let expected_direction =
        Cartesian3::multiply_by_scalar_new(&Cartesian3::normalize_new(&offset), -1.0);
    assert_approx_eq_f64!(camera.direction().x, expected_direction.x, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.direction().y, expected_direction.y, CesiumMath::EPSILON11);
    assert_approx_eq_f64!(camera.direction().z, expected_direction.z, CesiumMath::EPSILON11);
}

/// Perspective projection matrix semantics: with `aspectRatio > 1` CesiumJS
/// treats `fov` as the *horizontal* FOV, and `PerspectiveFrustum#update`
/// recovers the vertical one as
/// `fovy = atan(tan(fov / 2) / aspectRatio) * 2`. Hence
/// `tan(fovy / 2) == tan(fov / 2) / aspectRatio`, and the matrix entries are
/// `m00 = 1 / tan(fov / 2)` and `m11 = aspectRatio / tan(fov / 2)`.
#[test]
fn perspective_projection_matrix_matches_frustum_semantics() {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_fov(std::f64::consts::FRAC_PI_3);
    camera.set_near(1.0);
    camera.set_far(500.0);
    camera.update(SceneMode::Scene3D);

    let tan_half_fov = (camera.fov() * 0.5).tan();
    let tan_half_fovy = (camera.fovy() * 0.5).tan();
    let aspect = camera.aspect_ratio();
    assert!(aspect > 1.0, "the 800x600 canvas must exercise the fov != fovy path");
    // Direct consequence of the fovy definition.
    assert_approx_eq_f64!(tan_half_fovy, tan_half_fov / aspect, CesiumMath::EPSILON14);

    let projection = camera.projection_matrix();
    // Column-major: elements[0] = m00, elements[5] = m11.
    assert_approx_eq_f64!(projection.elements[0], 1.0 / tan_half_fov);
    assert_approx_eq_f64!(projection.elements[5], aspect / tan_half_fov);
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

    // SSE denominator mirrors PerspectiveFrustum#sseDenominator:
    // `2 * tan(0.5 * fovy)` — the vertical FOV, not `fov`.
    assert_approx_eq_f64!(camera.sse_denominator(), 2.0 * tan_half_fovy);
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
    camera.update(SceneMode::Scene3D);

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
    camera.update(SceneMode::Scene3D);

    let center = Cartesian2::new(400.0, 300.0);

    // Center pick: ray direction equals the camera direction.
    let ray = camera.get_pick_ray(&center).expect("canvas has area");
    assert_approx_eq_f64!(ray.direction.x, 0.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.y, 0.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.z, -1.0, CesiumMath::EPSILON12);

    let picked = camera
        .pick_ellipsoid(&center, Some(&ellipsoid))
        .expect("must hit");
    assert_approx_eq_f64!(picked.x, 0.0, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(picked.y, 0.0, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(picked.z, ellipsoid.radii().z, CesiumMath::EPSILON6);

    // Horizontal ray from the same position misses the ellipsoid.
    camera.set_direction(Cartesian3::UNIT_X);
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::new(0.0, 0.0, -1.0));
    camera.update(SceneMode::Scene3D);
    assert!(camera.pick_ellipsoid(&center, Some(&ellipsoid)).is_none());
}

/// The perspective pick ray at the window corners follows the frustum's
/// tangent angles. CesiumJS `getPickRayPerspective` derives both from the
/// *vertical* FOV: `tanPhi = tan(fovy / 2)`,
/// `tanTheta = aspectRatio * tanPhi`.
#[test]
fn get_pick_ray_corner_directions_follow_frustum_angles() {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_position(Cartesian3::new(0.0, 0.0, 100.0));
    camera.set_direction(Cartesian3::new(0.0, 0.0, -1.0));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
    camera.update(SceneMode::Scene3D);

    let tan_phi = (camera.fovy() * 0.5).tan();
    let tan_theta = camera.aspect_ratio() * tan_phi;

    // Top-left corner: (-1, +1) NDC → direction ∝ dir - right*tanθ + up*tanφ.
    let ray = camera
        .get_pick_ray(&Cartesian2::new(0.0, 0.0))
        .expect("canvas has area");
    let expected = Cartesian3::normalize_new(&Cartesian3::new(-tan_theta, tan_phi, -1.0));
    assert_approx_eq_f64!(ray.direction.x, expected.x, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.y, expected.y, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(ray.direction.z, expected.z, CesiumMath::EPSILON12);
}

/// `worldToCameraCoordinatesPoint` / `cameraToWorldCoordinatesPoint` express a
/// point in the camera's *reference frame*: CesiumJS multiplies by
/// `_actualInvTransform` / `_actualTransform`, not by the view matrix, so the
/// camera's own pose is deliberately not involved.
#[test]
fn world_to_camera_coordinates_uses_the_reference_frame() {
    let mut camera = Camera::new();
    camera.set_position(Cartesian3::new(10.0, 20.0, 30.0));
    camera.set_direction(Cartesian3::new(0.0, 0.0, -1.0));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
    camera.update(SceneMode::Scene3D);

    // The default reference frame is the identity, so the conversion is too.
    let point = Cartesian3::new(15.0, 25.0, 25.0);
    let camera_space = camera.world_to_camera_coordinates_point(&point);
    assert_approx_eq_f64!(camera_space.x, point.x, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera_space.y, point.y, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera_space.z, point.z, CesiumMath::EPSILON12);

    // A translated reference frame shifts the point by the inverse translation;
    // the camera's own position (10, 20, 30) must not leak into the result.
    camera.set_transform(Matrix4::from_translation_new(&Cartesian3::new(
        100.0, 0.0, 0.0,
    )));
    let camera_space = camera.world_to_camera_coordinates_point(&point);
    assert_approx_eq_f64!(camera_space.x, point.x - 100.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera_space.y, point.y, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera_space.z, point.z, CesiumMath::EPSILON12);

    let round_trip = camera.camera_to_world_coordinates_point(&camera_space);
    assert_approx_eq_f64!(round_trip.x, point.x, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(round_trip.y, point.y, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(round_trip.z, point.z, CesiumMath::EPSILON12);
}

/// An orthographic camera over the equator, `distance` metres from the
/// ellipsoid centre, looking straight down with the default identity reference
/// frame and an 800×600 drawing buffer.
fn orthographic_camera_at(distance: f64) -> Camera {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);
    camera.set_position(Cartesian3::new(distance, 0.0, 0.0));
    camera.set_direction(Cartesian3::new(-1.0, 0.0, 0.0));
    camera.set_up(Cartesian3::UNIT_Z);
    camera.set_right(Cartesian3::UNIT_Y);
    camera.update(SceneMode::Scene3D);
    camera.switch_to_orthographic_frustum();
    camera
}

/// `calculateOrthographicFrustumWidth`: when the camera is fixed to an object
/// (`transform !== IDENTITY`) the width is the distance from the reference frame
/// origin, so it stays constant while the object moves and nothing the scene
/// publishes is consulted.
#[test]
fn orthographic_width_is_constant_when_fixed_to_an_object() {
    let mut camera = Camera::new();
    camera.set_transform(Matrix4::from_translation_new(&Cartesian3::new(
        1.0e6, 0.0, 0.0,
    )));
    camera.set_position(Cartesian3::new(300.0, 400.0, 0.0));
    assert_approx_eq_f64!(
        camera.calculate_orthographic_frustum_width(),
        500.0,
        CesiumMath::EPSILON12
    );

    camera.set_scene_context(CameraSceneContext {
        pixel_ratio: 1.0,
        ray_intersection: Some(Cartesian3::ZERO),
        ..Default::default()
    });
    assert_approx_eq_f64!(
        camera.calculate_orthographic_frustum_width(),
        500.0,
        CesiumMath::EPSILON12
    );
}

/// `calculateOrthographicFrustumWidth` otherwise measures the distance to what
/// sits under the screen centre, falling back to
/// `Math.max(positionCartographic.height, 0)` when nothing does.
#[test]
fn orthographic_width_uses_the_centre_intersection_or_the_camera_height() {
    let radius = Ellipsoid::WGS84.maximum_radius();
    let height = 5000.0;
    let mut camera = orthographic_camera_at(radius + height);

    // No intersection published → the camera height above the ellipsoid.
    camera.set_scene_context(CameraSceneContext::default());
    assert_approx_eq_f64!(
        camera.calculate_orthographic_frustum_width(),
        height,
        CesiumMath::EPSILON6
    );

    // An intersection on the surface directly below → the same distance, now
    // measured as `Cartesian3.distance(hit, positionWC)`.
    camera.set_scene_context(CameraSceneContext {
        pixel_ratio: 1.0,
        ray_intersection: Some(Cartesian3::new(radius, 0.0, 0.0)),
        ..Default::default()
    });
    assert_approx_eq_f64!(
        camera.calculate_orthographic_frustum_width(),
        height,
        CesiumMath::EPSILON9
    );
}

/// CesiumJS's `mousePosition`: `drawingBuffer{Width,Height} / pixelRatio / 2`.
#[test]
fn centre_window_position_divides_the_drawing_buffer_by_the_pixel_ratio() {
    let mut camera = Camera::new();
    camera.set_canvas_size(800, 600);

    let centre = camera.centre_window_position();
    assert_approx_eq_f64!(centre.x, 400.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(centre.y, 300.0, CesiumMath::EPSILON12);

    camera.set_scene_context(CameraSceneContext {
        pixel_ratio: 2.0,
        ray_intersection: None,
        ..Default::default()
    });
    let centre = camera.centre_window_position();
    assert_approx_eq_f64!(centre.x, 200.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(centre.y, 150.0, CesiumMath::EPSILON12);
}

/// `_adjustOrthographicFrustum` returns early unless the frustum *is* an
/// `OrthographicFrustum`. The off-center variant 2D requires fails that
/// `instanceof` test too, so 2D zooming stays with the bounds rescaling of
/// `zoom2D`.
#[test]
fn adjust_orthographic_frustum_skips_every_other_frustum() {
    let mut camera = Camera::new();
    camera.adjust_orthographic_frustum(true);
    assert!(camera.frustum().is_perspective());
    assert!(camera.frustum().width().is_none());

    let mut camera = Camera::new();
    camera.set_frustum(CameraFrustum::OrthographicOffCenter(
        OrthographicOffCenterFrustum::new(),
    ));
    camera.adjust_orthographic_frustum(true);
    assert!(camera.frustum().is_orthographic_off_center());
    assert!(camera.frustum().width().is_none());
}

// ---------------------------------------------------------------------------
// B3-2d: setView / lookAt / lookAtTransform / getRectangleCameraCoordinates
//
// `Camera::new()` starts with an 800×600 drawing buffer and a 60° perspective
// frustum, i.e. `aspectRatio = 4/3` — the same numbers the CesiumJS spec's
// 1024×768 `FakeScene` produces — so the golden values below are lifted
// straight from `packages/engine/Specs/Scene/CameraSpec.js`.
// ---------------------------------------------------------------------------

fn assert_cartesian_eq(actual: &Cartesian3, expected: &Cartesian3, epsilon: f64) {
    assert_approx_eq_f64!(actual.x, expected.x, epsilon);
    assert_approx_eq_f64!(actual.y, expected.y, epsilon);
    assert_approx_eq_f64!(actual.z, expected.z, epsilon);
}

fn assert_cartographic_eq(actual: &Cartographic, expected: &Cartographic, epsilon: f64) {
    assert_approx_eq_f64!(actual.longitude, expected.longitude, epsilon);
    assert_approx_eq_f64!(actual.latitude, expected.latitude, epsilon);
    assert_approx_eq_f64!(actual.height, expected.height, epsilon);
}

/// `it("setView rectangle in 3D (1)")` — the whole globe, backed off along -x
/// until both poles and both date-line edges fit inside the 60° frustum.
#[test]
fn set_view_rectangle_in_3d_full_globe() {
    let mut camera = Camera::new();
    camera.set_view_with_options(&SetViewOptions {
        destination: Some(SetViewDestination::Rectangle(Rectangle::MAX_VALUE)),
        ..SetViewOptions::default()
    });

    assert_cartesian_eq(
        camera.position(),
        &Cartesian3::new(14680290.639204923, 0.0, 0.0),
        CesiumMath::EPSILON6,
    );
    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::negate_new(&Cartesian3::UNIT_X),
        CesiumMath::EPSILON10,
    );
    assert_cartesian_eq(camera.up(), &Cartesian3::UNIT_Z, CesiumMath::EPSILON10);
    assert_cartesian_eq(camera.right(), &Cartesian3::UNIT_Y, CesiumMath::EPSILON10);
}

/// `it("getRectangleCameraCoordinates rectangle in 3D")` — CesiumJS passes
/// `updateCamera === undefined`, so the module-level `defaultRF` scratch takes
/// the derived axes and the camera's own pose is left exactly as it was.
#[test]
fn get_rectangle_camera_coordinates_in_3d_leaves_the_pose_alone() {
    let mut camera = spec_camera();
    let direction = *camera.direction();
    let up = *camera.up();
    let right = *camera.right();

    let position = camera
        .get_rectangle_camera_coordinates(Rectangle::MAX_VALUE)
        .expect("3D always yields a position");

    assert_cartesian_eq(
        &position,
        &Cartesian3::new(14680290.639204923, 0.0, 0.0),
        CesiumMath::EPSILON6,
    );
    assert_eq!(*camera.direction(), direction);
    assert_eq!(*camera.up(), up);
    assert_eq!(*camera.right(), right);
}

/// `it("gets coordinates for rectangle in 3D across IDL")` — `west > east` bumps
/// `east` by a full turn, which puts the rectangle's centre longitude at π and
/// mirrors the whole-globe answer.
#[test]
fn get_rectangle_camera_coordinates_across_the_idl() {
    let mut camera = spec_camera();
    let rectangle = Rectangle::new(0.1, -CesiumMath::PI_OVER_TWO, -0.1, CesiumMath::PI_OVER_TWO);

    let position = camera
        .get_rectangle_camera_coordinates(rectangle)
        .expect("3D always yields a position");

    assert_cartesian_eq(
        &position,
        &Cartesian3::new(-14680290.639204923, 0.0, 0.0),
        CesiumMath::EPSILON6,
    );
}

/// `it("getRectangleCameraCoordinates")` while morphing returns the JS
/// `undefined`.
#[test]
fn get_rectangle_camera_coordinates_is_undefined_while_morphing() {
    let mut camera = spec_camera();
    camera.update(SceneMode::Morphing);
    assert!(camera
        .get_rectangle_camera_coordinates(Rectangle::MAX_VALUE)
        .is_none());
}

/// `it("setView right rotation order")` — heading/pitch/roll survive the
/// `Quaternion.fromHeadingPitchRoll` round trip in that order.
#[test]
fn set_view_right_rotation_order() {
    let mut camera = Camera::new();
    let position = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let heading = CesiumMath::to_radians(180.0);
    let pitch = CesiumMath::to_radians(0.0);
    let roll = CesiumMath::to_radians(45.0);

    camera.set_view_with_options(&SetViewOptions {
        destination: Some(SetViewDestination::Cartesian(position)),
        orientation: SetViewOrientation {
            heading: Some(heading),
            pitch: Some(pitch),
            roll: Some(roll),
            ..SetViewOrientation::default()
        },
        ..SetViewOptions::default()
    });

    assert_cartesian_eq(camera.position(), &position, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.heading().unwrap(), heading, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.pitch().unwrap(), pitch, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.roll().unwrap(), roll, CesiumMath::EPSILON6);
}

/// `it("setView (1)")` — a second `setView` with no destination keeps
/// `positionWC` and only re-aims.
#[test]
fn set_view_without_a_destination_keeps_the_position() {
    let mut camera = Camera::new();
    let position = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let heading = CesiumMath::to_radians(45.0);
    let pitch = CesiumMath::to_radians(-50.0);
    let roll = CesiumMath::to_radians(45.0);

    camera.set_view_with_options(&SetViewOptions {
        destination: Some(SetViewDestination::Cartesian(position)),
        orientation: SetViewOrientation {
            heading: Some(heading),
            pitch: Some(pitch),
            roll: Some(roll),
            ..SetViewOrientation::default()
        },
        ..SetViewOptions::default()
    });

    let new_heading = CesiumMath::to_radians(200.0);
    let kept_pitch = camera.pitch().unwrap();
    let kept_roll = camera.roll().unwrap();
    camera.set_view_with_options(&SetViewOptions {
        destination: None,
        orientation: SetViewOrientation {
            heading: Some(new_heading),
            pitch: Some(kept_pitch),
            roll: Some(kept_roll),
            ..SetViewOrientation::default()
        },
        ..SetViewOptions::default()
    });

    assert_cartesian_eq(camera.position(), &position, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.heading().unwrap(), new_heading, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.pitch().unwrap(), pitch, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.roll().unwrap(), roll, CesiumMath::EPSILON6);
}

/// `Camera#setView` bails out immediately while morphing.
#[test]
fn set_view_is_a_no_op_while_morphing() {
    let mut camera = spec_camera();
    camera.update(SceneMode::Morphing);

    let before_position = *camera.position();
    let before_direction = *camera.direction();

    camera.set_view_with_options(&SetViewOptions {
        destination: Some(SetViewDestination::Cartesian(Cartesian3::from_degrees_new(
            -75.0,
            0.0,
            Some(100.0),
            None,
        ))),
        ..SetViewOptions::default()
    });

    assert_eq!(*camera.position(), before_position);
    assert_eq!(*camera.direction(), before_direction);
    assert!(camera.heading().is_none());
}

/// `it("setView with cartesian in Columbus View")` — the destination is
/// projected into the map plane and the pose is written there, so
/// `positionCartographic` recovers the original longitude/latitude/height.
#[test]
fn set_view_with_cartesian_in_columbus_view() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut camera = Camera::new();
    camera.update(SceneMode::ColumbusView);

    let cartesian = Cartesian3::from_degrees_new(-75.0, 42.0, Some(100.0), None);
    camera.set_view(&cartesian, None, None, &ellipsoid);

    let mut expected = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&cartesian, &mut expected);
    assert_cartographic_eq(
        camera.position_cartographic(),
        &expected,
        CesiumMath::EPSILON11,
    );
    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::negate_new(&Cartesian3::UNIT_Z),
        CesiumMath::EPSILON6,
    );
    assert_cartesian_eq(camera.up(), &Cartesian3::UNIT_Y, CesiumMath::EPSILON6);
    assert_cartesian_eq(camera.right(), &Cartesian3::UNIT_X, CesiumMath::EPSILON6);
}

/// `it("setView with cartesian in 3D and orthographic frustum")` — the trailing
/// `_adjustOrthographicFrustum(true)` re-derives the width from the new height.
#[test]
fn set_view_with_cartesian_in_3d_and_orthographic_frustum() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut camera = Camera::new();
    let mut frustum = OrthographicFrustum::new();
    frustum.aspect_ratio = Some(800.0 / 600.0);
    // Anything but the answer, to prove it gets overwritten.
    frustum.width = Some(1.0e9);
    camera.set_frustum(CameraFrustum::Orthographic(frustum));

    let cartesian = Cartesian3::from_degrees_new(-75.0, 0.0, Some(100.0), None);
    camera.set_view(&cartesian, None, None, &ellipsoid);

    let mut expected = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&cartesian, &mut expected);
    assert_approx_eq_f64!(
        camera.position_cartographic().height,
        expected.height,
        CesiumMath::EPSILON6
    );
    assert_approx_eq_f64!(
        camera.frustum().width().unwrap(),
        expected.height,
        CesiumMath::EPSILON6
    );
    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::normalize_new(&Cartesian3::negate_new(&cartesian)),
        CesiumMath::EPSILON6,
    );
    assert_cartesian_eq(camera.up(), &Cartesian3::UNIT_Z, CesiumMath::EPSILON6);
}

/// `it("setView with cartesian in 2D")` — the destination's *height* becomes the
/// orthographic frustum width (`right - left`) while the `top / right` ratio is
/// preserved, and the pose keeps the map-plane axes.
#[test]
fn set_view_with_cartesian_in_2d() {
    let ellipsoid = Ellipsoid::WGS84;
    let max_radii = ellipsoid.maximum_radius();

    let mut frustum = OrthographicOffCenterFrustum::new();
    let right = max_radii * std::f64::consts::PI;
    let top = right * (600.0 / 800.0);
    frustum.right = Some(right);
    frustum.left = Some(-right);
    frustum.top = Some(top);
    frustum.bottom = Some(-top);
    frustum.near = 0.01 * max_radii;
    frustum.far = 60.0 * max_radii;
    let ratio = top / right;

    let mut camera = Camera::new();
    camera.set_frustum(CameraFrustum::OrthographicOffCenter(frustum));
    camera.update(SceneMode::Scene2D);

    let cartesian = Cartesian3::from_degrees_new(-75.0, 42.0, Some(100.0), None);
    camera.set_view(&cartesian, None, None, &ellipsoid);

    let mut expected = Cartographic::default();
    ellipsoid.cartesian_to_cartographic(&cartesian, &mut expected);
    assert_cartographic_eq(camera.position_cartographic(), &expected, CesiumMath::EPSILON6);

    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::negate_new(&Cartesian3::UNIT_Z),
        CesiumMath::EPSILON6,
    );
    assert_cartesian_eq(camera.up(), &Cartesian3::UNIT_Y, CesiumMath::EPSILON6);
    assert_cartesian_eq(camera.right(), &Cartesian3::UNIT_X, CesiumMath::EPSILON6);

    let (left, right, top, _bottom) = camera.frustum_mut().bounds();
    assert_approx_eq_f64!(right - left, expected.height, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(top / right, ratio, CesiumMath::EPSILON12);
}

/// `it("lookAtTransform with no offset parameter")` — installing a reference
/// frame alone re-expresses the existing pose in it.
#[test]
fn look_at_transform_with_no_offset() {
    let ellipsoid = Ellipsoid::WGS84;
    let mut cart_origin = Cartographic::from_degrees_new(-75.59777, 40.03883, None);
    let mut origin = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cart_origin, &mut origin);
    let transform = transforms::east_north_up_to_fixed_frame_new(&origin, Some(&ellipsoid));

    let height = 1000.0;
    cart_origin.height = height;
    let mut position = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(&cart_origin, &mut position);

    let mut camera = Camera::new();
    camera.set_position(position);
    let column2 = Matrix4::get_column_new(&transform, 2);
    camera.set_direction(Cartesian3::new(-column2.x, -column2.y, -column2.z));
    let column1 = Matrix4::get_column_new(&transform, 1);
    camera.set_up(Cartesian3::new(column1.x, column1.y, column1.z));
    let column0 = Matrix4::get_column_new(&transform, 0);
    camera.set_right(Cartesian3::new(column0.x, column0.y, column0.z));

    camera.look_at_transform(&transform, None);

    assert_cartesian_eq(
        camera.position(),
        &Cartesian3::new(0.0, 0.0, height),
        CesiumMath::EPSILON9,
    );
    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::negate_new(&Cartesian3::UNIT_Z),
        CesiumMath::EPSILON9,
    );
    assert_cartesian_eq(camera.up(), &Cartesian3::UNIT_Y, CesiumMath::EPSILON9);
    assert_cartesian_eq(camera.right(), &Cartesian3::UNIT_X, CesiumMath::EPSILON9);
}

/// `it("lookAt with heading, pitch and range")` — `offsetFromHeadingPitchRange`
/// places the camera `range` metres from the target at the requested angles.
#[test]
fn look_at_with_heading_pitch_range() {
    let ellipsoid = Ellipsoid::WGS84;
    let target = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let heading = CesiumMath::to_radians(45.0);
    let pitch = CesiumMath::to_radians(-45.0);
    let range = 2.0;

    let mut camera = spec_camera();
    camera.look_at(
        &target,
        LookAtOffset::HeadingPitchRange(HeadingPitchRange::new(heading, pitch, range)),
        &ellipsoid,
    );
    camera.look_at_transform(&Matrix4::IDENTITY, None);

    assert_approx_eq_f64!(
        Cartesian3::distance(camera.position(), &target),
        range,
        CesiumMath::EPSILON6
    );
    assert_approx_eq_f64!(camera.heading().unwrap(), heading, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.pitch().unwrap(), pitch, CesiumMath::EPSILON6);

    assert!((1.0 - Cartesian3::magnitude(camera.direction())).abs() < CesiumMath::EPSILON14);
    assert!((1.0 - Cartesian3::magnitude(camera.up())).abs() < CesiumMath::EPSILON14);
    assert!((1.0 - Cartesian3::magnitude(camera.right())).abs() < CesiumMath::EPSILON14);
}

/// `it("lookAtTransform with heading, pitch and range")`
#[test]
fn look_at_transform_with_heading_pitch_range() {
    let target = Cartesian3::from_degrees_new(0.0, 0.0, None, None);
    let heading = CesiumMath::to_radians(45.0);
    let pitch = CesiumMath::to_radians(-45.0);
    let range = 2.0;
    let transform =
        transforms::east_north_up_to_fixed_frame_new(&target, Some(&Ellipsoid::WGS84));

    let mut camera = spec_camera();
    camera.look_at_transform(
        &transform,
        Some(LookAtOffset::HeadingPitchRange(HeadingPitchRange::new(
            heading, pitch, range,
        ))),
    );
    camera.look_at_transform(&Matrix4::IDENTITY, None);

    assert_approx_eq_f64!(
        Cartesian3::distance(camera.position(), &target),
        range,
        CesiumMath::EPSILON6
    );
    assert_approx_eq_f64!(camera.heading().unwrap(), heading, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.pitch().unwrap(), pitch, CesiumMath::EPSILON6);

    assert!((1.0 - Cartesian3::magnitude(camera.direction())).abs() < CesiumMath::EPSILON14);
    assert!((1.0 - Cartesian3::magnitude(camera.up())).abs() < CesiumMath::EPSILON14);
    assert!((1.0 - Cartesian3::magnitude(camera.right())).abs() < CesiumMath::EPSILON14);
}

/// `Camera#rotate` passes `zooming = false`, so the width is only re-derived
/// once the camera is at least 150 km up; `Camera#move` passes `true` and
/// always re-derives it.
#[test]
fn rotate_only_readjusts_the_orthographic_width_high_up() {
    let radius = Ellipsoid::WGS84.maximum_radius();

    // 1 km up: rotating keeps the framing, moving does not.
    let mut camera = orthographic_camera_at(radius + 1000.0);
    camera.set_orthographic_width(1.0);
    camera.rotate(&Cartesian3::UNIT_Z, Some(0.0));
    assert_approx_eq_f64!(camera.frustum().width().unwrap(), 1.0, CesiumMath::EPSILON12);
    camera.move_camera(&Cartesian3::ZERO, 0.0);
    assert_approx_eq_f64!(
        camera.frustum().width().unwrap(),
        1000.0,
        CesiumMath::EPSILON6
    );

    // 200 km up: rotating re-derives it as well.
    let mut camera = orthographic_camera_at(radius + 200_000.0);
    camera.set_orthographic_width(1.0);
    camera.rotate(&Cartesian3::UNIT_Z, Some(0.0));
    assert_approx_eq_f64!(
        camera.frustum().width().unwrap(),
        200_000.0,
        CesiumMath::EPSILON6
    );
}

/// `_adjustOrthographicFrustum` publishes the measured distance: with a scene
/// ray intersection under the screen centre the width becomes the distance from
/// `positionWC` to it.
#[test]
fn adjust_orthographic_frustum_uses_the_scene_intersection() {
    let radius = Ellipsoid::WGS84.maximum_radius();
    let mut camera = orthographic_camera_at(radius + 200_000.0);
    camera.set_scene_context(CameraSceneContext {
        pixel_ratio: 1.0,
        ray_intersection: Some(Cartesian3::new(radius + 50_000.0, 0.0, 0.0)),
        ..Default::default()
    });
    camera.adjust_orthographic_frustum(true);
    assert_approx_eq_f64!(
        camera.frustum().width().unwrap(),
        150_000.0,
        CesiumMath::EPSILON9
    );
}

/// `switchToPerspectiveFrustum` / `switchToOrthographicFrustum` are no-ops in
/// 2D, which must always be orthographic, and when the frustum already is the
/// requested type.
#[test]
fn switch_frustums_are_no_ops_in_2d_and_for_the_target_type() {
    let mut camera = Camera::new();
    camera.set_frustum(CameraFrustum::OrthographicOffCenter(
        OrthographicOffCenterFrustum::new(),
    ));
    camera.update(SceneMode::Scene2D);
    camera.switch_to_perspective_frustum();
    assert!(camera.frustum().is_orthographic_off_center());
    camera.switch_to_orthographic_frustum();
    assert!(camera.frustum().is_orthographic_off_center());

    // Already perspective in 3D: the hand-tuned `fov` survives.
    let mut camera = Camera::new();
    camera.update(SceneMode::Scene3D);
    camera.set_fov(CesiumMath::to_radians(45.0));
    camera.switch_to_perspective_frustum();
    assert!(camera.frustum().is_perspective());
    assert_approx_eq_f64!(camera.fov(), CesiumMath::to_radians(45.0), CesiumMath::EPSILON12);
}

/// `switchToOrthographicFrustum` measures the width *before* replacing the
/// frustum, because the reconstruction uses the previous one, and installs a
/// fresh `OrthographicFrustum` whose `aspectRatio` comes from the drawing
/// buffer.
#[test]
fn switch_to_orthographic_frustum_measures_the_width_first() {
    let radius = Ellipsoid::WGS84.maximum_radius();
    let camera = orthographic_camera_at(radius + 5000.0);

    assert!(camera.frustum().is_orthographic());
    assert_eq!(camera.projection_type(), CameraProjection::Orthographic);
    assert_approx_eq_f64!(
        camera.frustum().width().unwrap(),
        5000.0,
        CesiumMath::EPSILON6
    );
    assert_approx_eq_f64!(camera.aspect_ratio(), 800.0 / 600.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera.frustum().near(), 1.0, CesiumMath::EPSILON12);
}

/// `switchToPerspectiveFrustum` installs a *fresh* `PerspectiveFrustum`:
/// `fov` is `toRadians(60)`, `aspectRatio` comes from the drawing buffer, and
/// the previous frustum's `near`/`far`/`width` are dropped rather than carried
/// over (which is what `Camera#setProjection` does instead).
#[test]
fn switch_to_perspective_frustum_installs_a_fresh_frustum() {
    let radius = Ellipsoid::WGS84.maximum_radius();
    let mut camera = orthographic_camera_at(radius + 5000.0);
    camera.set_near(10.0);
    camera.set_far(20.0);

    camera.switch_to_perspective_frustum();

    assert!(camera.frustum().is_perspective());
    assert_eq!(camera.projection_type(), CameraProjection::Perspective);
    assert_approx_eq_f64!(camera.fov(), CesiumMath::to_radians(60.0), CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera.aspect_ratio(), 800.0 / 600.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(camera.frustum().near(), 1.0, CesiumMath::EPSILON12);
    assert_approx_eq_f64!(
        camera.frustum().far(),
        500_000_000.0,
        CesiumMath::EPSILON6
    );
    assert!(camera.frustum().width().is_none());
}

/// The `computeViewRectangle` specs' camera: a CesiumJS `FakeScene` camera
/// (512×384 canvas, so the default 4:3 perspective frustum is unchanged) placed
/// at `position`, looking along `direction` with `up`, `right = direction × up`.
fn view_rectangle_camera(position: Cartesian3, up: Cartesian3, direction: Cartesian3) -> Camera {
    let right = Cartesian3::cross_new(&direction, &up);
    let mut camera = Camera::new();
    camera.set_canvas_size(512, 384);
    camera.set_position(position);
    camera.set_up(up);
    camera.set_direction(direction);
    camera.set_right(right);
    camera.update(SceneMode::Scene3D);
    camera
}

/// `it("computeViewRectangle when zoomed in")` — 622 km over the equator at
/// longitude 0, so the visible rectangle is a few degrees on a side.
#[test]
fn compute_view_rectangle_when_zoomed_in() {
    let position = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, 7000000.0);
    let direction = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, -1.0);
    let mut camera = view_rectangle_camera(position, Cartesian3::UNIT_Z, direction);

    let rect = camera
        .compute_view_rectangle(None)
        .expect("the ellipsoid is visible");

    assert_approx_eq_f64!(rect.west, -0.05789100547374969, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.south, -0.04365869998457809, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.east, 0.05789100547374969, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.north, 0.04365869998457809, CesiumMath::EPSILON10);
}

/// `it("computeViewRectangle when zoomed in to pole")` — looking straight down
/// the spin axis the four corner longitudes wrap the globe, so the pole branch
/// snaps `west`/`east` to ±π and `north` to π/2 (`toEqual`, i.e. exact).
#[test]
fn compute_view_rectangle_when_zoomed_in_to_pole() {
    let position = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_Z, 7000000.0);
    let direction = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_Z, -1.0);
    let mut camera = view_rectangle_camera(position, Cartesian3::UNIT_Y, direction);

    let rect = camera
        .compute_view_rectangle(None)
        .expect("the ellipsoid is visible");

    assert_approx_eq_f64!(rect.west, -CesiumMath::PI, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(rect.south, 1.4961779388065022, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.east, CesiumMath::PI, CesiumMath::EPSILON14);
    assert_approx_eq_f64!(rect.north, CesiumMath::PI_OVER_TWO, CesiumMath::EPSILON14);
}

/// `it("computeViewRectangle when zoomed in to IDL")` — the mirror of the
/// zoomed-in case from the far side of the globe, straddling the date line.
#[test]
fn compute_view_rectangle_when_zoomed_in_to_idl() {
    let position = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, -7000000.0);
    let mut camera = view_rectangle_camera(position, Cartesian3::UNIT_Z, Cartesian3::UNIT_X);

    let rect = camera
        .compute_view_rectangle(None)
        .expect("the ellipsoid is visible");

    assert_approx_eq_f64!(rect.west, 3.0837016481160435, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.south, -0.04365869998457809, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.east, -3.0837016481160435, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(rect.north, 0.04365869998457809, CesiumMath::EPSILON10);
}

/// `it("computeViewRectangle when zoomed out")` — far enough that no canvas
/// corner picks the globe, so fewer than two succeed and the whole-globe
/// [`Rectangle::MAX_VALUE`] is returned (`toEqual`, i.e. exact).
#[test]
fn compute_view_rectangle_when_zoomed_out() {
    let position = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, 25000000.0);
    let direction = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, -1.0);
    let mut camera = view_rectangle_camera(position, Cartesian3::UNIT_Z, direction);

    let rect = camera
        .compute_view_rectangle(None)
        .expect("the ellipsoid is visible");

    assert_eq!(rect, Rectangle::MAX_VALUE);
}

/// `it("computeViewRectangle when globe isn't visible")` — looking away from
/// the globe puts its bounding sphere outside the frustum, so the JS returns
/// `undefined`.
#[test]
fn compute_view_rectangle_when_globe_isnt_visible() {
    let position = Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_X, 7000000.0);
    let mut camera = view_rectangle_camera(position, Cartesian3::UNIT_Z, Cartesian3::UNIT_X);

    assert!(camera.compute_view_rectangle(None).is_none());
}

/// The CesiumJS `CameraSpec` `beforeEach` camera for the bounding-sphere specs:
/// `position = UNIT_Z`, `up = UNIT_Y`, `direction = -UNIT_Z`,
/// `right = direction × up = UNIT_X`, in `SCENE3D`, on the `FakeScene` canvas
/// (512×384) with its `screenSpaceCameraController` zoom bounds
/// (0 … 5906376272000 m, the Sun-to-Pluto distance).
fn bounding_sphere_camera() -> Camera {
    let direction = Cartesian3::negate_new(&Cartesian3::UNIT_Z);
    let mut camera = Camera::new();
    camera.set_canvas_size(512, 384);
    camera.set_position(Cartesian3::UNIT_Z);
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_direction(direction);
    camera.set_right(Cartesian3::cross_new(&direction, &Cartesian3::UNIT_Y));
    camera.set_scene_context(CameraSceneContext {
        minimum_zoom_distance: 0.0,
        maximum_zoom_distance: 5906376272000.0,
        ..Default::default()
    });
    camera.update(SceneMode::Scene3D);
    camera
}

/// `it("distanceToBoundingSphere")` — with the `beforeEach` pose the unit-Z
/// camera looks straight down -Z at a sphere of radius 0.5 centred at the
/// origin, so the projected camera→centre distance is 1.0 and the distance to
/// the *front* of the sphere is `1.0 - 0.5`.
#[test]
fn distance_to_bounding_sphere() {
    let mut camera = bounding_sphere_camera();
    let sphere = BoundingSphere::new(Cartesian3::ZERO, 0.5);
    let distance = camera.distance_to_bounding_sphere(&sphere);
    assert_approx_eq_f64!(distance, 0.5, CesiumMath::EPSILON14);
}

/// `it("getPixelSize")` — self-consistent: `getPixelSize` must equal the larger
/// axis of `frustum.getPixelDimensions` evaluated at the same distance, for the
/// `FakeScene` drawing buffer (1024×768) and `pixelRatio = 1.0`.
#[test]
fn get_pixel_size() {
    let mut camera = bounding_sphere_camera();
    let sphere = BoundingSphere::new(Cartesian3::ZERO, 0.5);
    let drawing_buffer_width = 1024.0;
    let drawing_buffer_height = 768.0;

    let distance = camera.distance_to_bounding_sphere(&sphere);
    let pixel_dimensions = camera.frustum_mut().get_pixel_dimensions(
        drawing_buffer_width,
        drawing_buffer_height,
        distance,
        1.0,
    );
    let expected = pixel_dimensions.x.max(pixel_dimensions.y);

    let pixel_size = camera.get_pixel_size(&sphere, drawing_buffer_width, drawing_buffer_height);
    assert_approx_eq_f64!(pixel_size, expected, CesiumMath::EPSILON14);
}

/// `it("viewBoundingSphere")` — with no offset the range is computed to frame
/// the sphere, landing the camera between `radius` and `3× radius` from its
/// centre (`set_transform(IDENTITY)` brings `position` back to world coords).
#[test]
fn view_bounding_sphere() {
    let mut camera = bounding_sphere_camera();
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 10000.0);

    camera.view_bounding_sphere(&sphere, None, &Ellipsoid::WGS84);
    camera.set_transform(Matrix4::IDENTITY);

    let distance = Cartesian3::distance(camera.position(), &sphere.center);
    assert!(
        distance > sphere.radius,
        "{distance} must exceed the radius {}",
        sphere.radius
    );
    assert!(
        distance < sphere.radius * 3.0,
        "{distance} must stay under {}",
        sphere.radius * 3.0
    );
}

/// `it("viewBoundingSphere with offset")` — an explicit non-zero range is used
/// verbatim, so the camera sits exactly `range` from the centre at the given
/// heading/pitch.
#[test]
fn view_bounding_sphere_with_offset() {
    let mut camera = bounding_sphere_camera();
    let heading = CesiumMath::to_radians(45.0);
    let pitch = CesiumMath::to_radians(-45.0);
    let range = 15.0;
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 10.0);

    camera.view_bounding_sphere(
        &sphere,
        Some(HeadingPitchRange::new(heading, pitch, range)),
        &Ellipsoid::WGS84,
    );
    camera.set_transform(Matrix4::IDENTITY);

    let distance = Cartesian3::distance(camera.position(), &sphere.center);
    assert_approx_eq_f64!(distance, range, CesiumMath::EPSILON10);
    assert_approx_eq_f64!(camera.heading().unwrap(), heading, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.pitch().unwrap(), pitch, CesiumMath::EPSILON5);
}

/// `it("viewBoundingSphere does not modify offset.range when it is zero")` —
/// `adjustBoundingSphereOffset` clones the offset (JS `HeadingPitchRange.clone`);
/// the port takes it by value and `HeadingPitchRange` is `Copy`, so the caller's
/// offset is left exactly as constructed.
#[test]
fn view_bounding_sphere_does_not_modify_offset_range() {
    let mut camera = bounding_sphere_camera();
    let heading = CesiumMath::to_radians(45.0);
    let pitch = CesiumMath::to_radians(-45.0);
    let offset = HeadingPitchRange::new(heading, pitch, 0.0);
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 10.0);

    camera.view_bounding_sphere(&sphere, Some(offset), &Ellipsoid::WGS84);

    assert_eq!(offset.heading, heading);
    assert_eq!(offset.pitch, pitch);
    assert_eq!(offset.range, 0.0);
}

/// `it("viewBoundingSphere in 2D")` — the 2D branch of `lookAtTransform`
/// resizes the orthographic off-centre frustum so `right - left` equals the
/// computed range, framing the sphere between `radius` and `3× radius`. The
/// `update(SCENE2D)` rescale only touches the `max2Dfrustum` clone, so the live
/// frustum keeps its `ratio = top/right = 1` when the range is derived.
#[test]
fn view_bounding_sphere_in_2d() {
    let mut camera = Camera::new();
    let mut frustum = OrthographicOffCenterFrustum::new();
    frustum.left = Some(-10.0);
    frustum.right = Some(10.0);
    frustum.bottom = Some(-10.0);
    frustum.top = Some(10.0);
    frustum.near = 1.0;
    frustum.far = 21.0;
    camera.set_frustum(CameraFrustum::OrthographicOffCenter(frustum));
    camera.set_scene_context(CameraSceneContext {
        minimum_zoom_distance: 0.0,
        maximum_zoom_distance: 5906376272000.0,
        ..Default::default()
    });
    camera.update(SceneMode::Scene2D);

    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 10000.0);
    camera.view_bounding_sphere(&sphere, None, &Ellipsoid::WGS84);
    camera.set_transform(Matrix4::IDENTITY);

    let (left, right, _, _) = camera.frustum_mut().bounds();
    let distance = right - left;
    assert!(
        distance > sphere.radius,
        "{distance} must exceed the radius {}",
        sphere.radius
    );
    assert!(
        distance < sphere.radius * 3.0,
        "{distance} must stay under {}",
        sphere.radius * 3.0
    );
}

// ---------------------------------------------------------------------------
// B3-2g: flyTo / flyHome / flyToBoundingSphere / cancelFlight / completeFlight
// ---------------------------------------------------------------------------

/// `it("flyHome works in 3D")` — `flyHome(0)` takes the synchronous shortcut:
/// the home destination is `getRectangleCameraCoordinates(DEFAULT_VIEW_RECTANGLE)`
/// pushed out by `1 + DEFAULT_VIEW_FACTOR`, then `setView` lands the pose looking
/// straight down at it.
#[test]
fn fly_home_works_in_3d() {
    let mut camera = bounding_sphere_camera();
    let destination = Cartesian3::from_degrees_new(30.0, 20.0, Some(1000.0), None);
    camera.set_view(&destination, None, None, &Ellipsoid::WGS84);

    // duration 0 → synchronous, no tween handed back.
    assert!(camera.fly_home(Some(0.0)).is_none());

    assert_cartesian_eq(
        camera.position(),
        &Cartesian3::new(2515865.110478756, -19109892.759980734, 13550929.353715947),
        CesiumMath::EPSILON8,
    );
    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::new(-0.10654051334260287, 0.8092555423939248, -0.5777149696185906),
        CesiumMath::EPSILON8,
    );
    assert_cartesian_eq(
        camera.up(),
        &Cartesian3::new(-0.07540693517283716, 0.5727725379670786, 0.8162385765685121),
        CesiumMath::EPSILON8,
    );
}

/// `it("flyHome works in CV")` — the Columbus View home sits on the `(0,-1,1)`
/// diagonal at `5 × maximumRadius`, pitched down by `-acos(ẑ)`; `convert: false`
/// keeps the destination verbatim.
#[test]
fn fly_home_works_in_cv() {
    let sq2_over2 = std::f64::consts::SQRT_2 * 0.5;
    let mut camera = Camera::new();
    camera.update(SceneMode::ColumbusView);
    let destination = Cartesian3::from_degrees_new(30.0, 20.0, Some(1000.0), None);
    camera.set_view(&destination, None, None, &Ellipsoid::WGS84);

    assert!(camera.fly_home(Some(0.0)).is_none());

    assert_cartesian_eq(
        camera.position(),
        &Cartesian3::new(0.0, -22550119.620184112, 22550119.62018411),
        CesiumMath::EPSILON8,
    );
    assert_cartesian_eq(
        camera.direction(),
        &Cartesian3::new(0.0, sq2_over2, -sq2_over2),
        CesiumMath::EPSILON8,
    );
    assert_cartesian_eq(
        camera.up(),
        &Cartesian3::new(0.0, sq2_over2, sq2_over2),
        CesiumMath::EPSILON8,
    );
}

/// `it("flyTo with heading, pitch and roll")` — the `duration: 0` shortcut runs
/// `setView`, so the heading/pitch/roll read straight back off the final pose.
#[test]
fn fly_to_with_heading_pitch_roll() {
    let mut camera = bounding_sphere_camera();
    let heading = CesiumMath::to_radians(180.0);
    let pitch = 0.0;
    let roll = CesiumMath::to_radians(45.0);
    let destination = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);

    let _ = camera.fly_to(FlyToOptions {
        destination: SetViewDestination::Cartesian(destination),
        orientation: SetViewOrientation {
            heading: Some(heading),
            pitch: Some(pitch),
            roll: Some(roll),
            ..SetViewOrientation::default()
        },
        duration: Some(0.0),
        ..FlyToOptions::default()
    });

    assert_approx_eq_f64!(camera.heading().unwrap(), heading, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.pitch().unwrap(), pitch, CesiumMath::EPSILON6);
    assert_approx_eq_f64!(camera.roll().unwrap(), roll, CesiumMath::EPSILON6);
}

/// `it("flyTo rectangle in 3D")` — an async flight resolves the rectangle
/// through `getRectangleCameraCoordinates` and hands that destination to
/// `CameraFlightPath.createTween`, which installs it as the flight's end
/// position (`equalsEpsilon(expected, 0.1)` in the JS).
#[test]
fn fly_to_rectangle_in_3d() {
    let mut camera = bounding_sphere_camera();
    let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
    camera.set_flight_channel(channel.clone());

    let rectangle = Rectangle::new(
        0.3323436621771766,
        0.8292930502744068,
        0.3325710961342694,
        0.8297059734014236,
    );
    let expected = camera.get_rectangle_camera_coordinates(rectangle).unwrap();

    let tween = camera.fly_to(FlyToOptions {
        destination: SetViewDestination::Rectangle(rectangle),
        ..FlyToOptions::default()
    });
    assert!(tween.is_some());

    let flight = channel.borrow();
    let flight = flight.as_ref().expect("flight installed");
    assert_cartesian_eq(&flight.end_position, &expected, CesiumMath::EPSILON1);
}

/// `it("flyTo rectangle with orientation")` — the `duration: 0` shortcut folds
/// the `direction`/`up` pair into heading/pitch/roll and re-resolves the
/// rectangle, so the final pose keeps the requested orientation at the
/// rectangle's camera coordinates.
#[test]
fn fly_to_rectangle_with_orientation() {
    let mut camera = bounding_sphere_camera();
    let direction = Cartesian3::negate_new(&Cartesian3::UNIT_Z);
    let up = Cartesian3::UNIT_Y;

    let rectangle = Rectangle::new(
        0.3323436621771766,
        0.8292930502744068,
        0.3325710961342694,
        0.8297059734014236,
    );
    let expected = camera.get_rectangle_camera_coordinates(rectangle).unwrap();

    let _ = camera.fly_to(FlyToOptions {
        destination: SetViewDestination::Rectangle(rectangle),
        orientation: SetViewOrientation {
            direction: Some(direction),
            up: Some(up),
            ..SetViewOrientation::default()
        },
        duration: Some(0.0),
        ..FlyToOptions::default()
    });

    assert_cartesian_eq(camera.direction(), &direction, CesiumMath::EPSILON6);
    assert_cartesian_eq(camera.up(), &up, CesiumMath::EPSILON6);
    assert_cartesian_eq(camera.position(), &expected, CesiumMath::EPSILON1);
}

/// `it("flyToBoundingSphere uses CameraFlightPath")` (the `duration: 0` variant)
/// — with no offset the range frames the sphere, landing the camera between
/// `radius` and `3× radius` from its centre.
#[test]
fn fly_to_bounding_sphere_duration_zero() {
    let mut camera = bounding_sphere_camera();
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 10000.0);

    let _ = camera.fly_to_bounding_sphere(
        &sphere,
        FlyToBoundingSphereOptions {
            duration: Some(0.0),
            ..FlyToBoundingSphereOptions::default()
        },
    );

    let distance = Cartesian3::distance(camera.position(), &sphere.center);
    assert!(
        distance > sphere.radius,
        "{distance} must exceed the radius {}",
        sphere.radius
    );
    assert!(
        distance < sphere.radius * 3.0,
        "{distance} must stay under {}",
        sphere.radius * 3.0
    );
}

/// `it("flyToBoundingSphere does not zoom closer than minimumZoomDistance")` —
/// the computed range for a tiny sphere is clamped up to `minimumZoomDistance`.
#[test]
fn fly_to_bounding_sphere_does_not_zoom_closer_than_minimum() {
    let mut camera = bounding_sphere_camera();
    camera.set_scene_context(CameraSceneContext {
        minimum_zoom_distance: 1000.0,
        maximum_zoom_distance: 5906376272000.0,
        ..Default::default()
    });
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 10.0);

    let _ = camera.fly_to_bounding_sphere(
        &sphere,
        FlyToBoundingSphereOptions {
            duration: Some(0.0),
            ..FlyToBoundingSphereOptions::default()
        },
    );

    let distance = Cartesian3::distance(camera.position(), &sphere.center);
    assert_approx_eq_f64!(distance, 1000.0, CesiumMath::EPSILON1);
}

/// `it("flyToBoundingSphere does not zoom further than maximumZoomDistance")` —
/// the computed range for a huge sphere is clamped down to `maximumZoomDistance`.
#[test]
fn fly_to_bounding_sphere_does_not_zoom_further_than_maximum() {
    let mut camera = bounding_sphere_camera();
    camera.set_scene_context(CameraSceneContext {
        minimum_zoom_distance: 0.0,
        maximum_zoom_distance: 10000.0,
        ..Default::default()
    });
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 100000.0);

    let _ = camera.fly_to_bounding_sphere(
        &sphere,
        FlyToBoundingSphereOptions {
            duration: Some(0.0),
            ..FlyToBoundingSphereOptions::default()
        },
    );

    let distance = Cartesian3::distance(camera.position(), &sphere.center);
    assert_approx_eq_f64!(distance, 10000.0, CesiumMath::EPSILON1);
}

/// `it("flyToBoundingSphere does not modify options.offset range if it is
/// zero")` — `adjustBoundingSphereOffset` clones the offset; the port takes it
/// by value and `HeadingPitchRange` is `Copy`, so the caller's offset survives.
#[test]
fn fly_to_bounding_sphere_does_not_modify_offset_range() {
    let mut camera = bounding_sphere_camera();
    let offset = HeadingPitchRange::new(0.0, -1.5, 0.0);
    let center = Cartesian3::from_degrees_new(-117.16, 32.71, Some(0.0), None);
    let sphere = BoundingSphere::new(center, 100000.0);

    let _ = camera.fly_to_bounding_sphere(
        &sphere,
        FlyToBoundingSphereOptions {
            offset: Some(offset),
            ..FlyToBoundingSphereOptions::default()
        },
    );

    assert_eq!(offset.heading, 0.0);
    assert_eq!(offset.pitch, -1.5);
    assert_eq!(offset.range, 0.0);
}

/// `it("can cancel a flight")` (port channel semantics) — an async `flyTo`
/// installs a flight in the shared channel; `cancelFlight` clears it so the next
/// `update` stops applying an interpolated pose.
#[test]
fn cancel_flight_clears_the_flight_channel() {
    let mut camera = bounding_sphere_camera();
    let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
    camera.set_flight_channel(channel.clone());

    let destination = Cartesian3::from_degrees_new(-117.16, 32.71, Some(1000000.0), None);
    let tween = camera.fly_to(FlyToOptions {
        destination: SetViewDestination::Cartesian(destination),
        duration: Some(1.0),
        ..FlyToOptions::default()
    });
    assert!(tween.is_some());
    assert!(channel.borrow().is_some());

    camera.cancel_flight();
    assert!(channel.borrow().is_none());
}

/// `it("can complete a flight")` (port channel semantics) — `completeFlight`
/// snaps the camera to the flight's end pose and clears the channel, mirroring
/// what `apply_flight` does on natural completion.
#[test]
fn complete_flight_applies_the_end_pose_and_clears_the_channel() {
    let mut camera = bounding_sphere_camera();
    let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
    camera.set_flight_channel(channel.clone());

    let destination = Cartesian3::from_degrees_new(-117.16, 32.71, Some(1000000.0), None);
    let tween = camera.fly_to(FlyToOptions {
        destination: SetViewDestination::Cartesian(destination),
        duration: Some(1.0),
        ..FlyToOptions::default()
    });
    assert!(tween.is_some());

    // Snapshot the end pose, releasing the borrow before `complete_flight`
    // (which borrows the channel mutably to clear it).
    let (end_position, end_direction, end_up) = {
        let flight = channel.borrow();
        let flight = flight.as_ref().expect("flight installed");
        (flight.end_position, flight.end_direction, flight.end_up)
    };

    camera.complete_flight();

    assert_cartesian_eq(camera.position(), &end_position, CesiumMath::EPSILON14);
    assert_cartesian_eq(camera.direction(), &end_direction, CesiumMath::EPSILON14);
    assert_cartesian_eq(camera.up(), &end_up, CesiumMath::EPSILON14);
    assert!(channel.borrow().is_none());
}
