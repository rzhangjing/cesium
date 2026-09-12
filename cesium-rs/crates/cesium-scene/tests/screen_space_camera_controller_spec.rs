//! Spec mirror: the portable (DOM-simulation-free) cases from
//! `packages/engine/Specs/Scene/ScreenSpaceCameraControllerSpec.js`.
//!
//! CesiumJS drives the controller through real canvas DOM events
//! (`DomEventSimulator`). The port has no DOM: the controller publishes its
//! [`CameraEventAggregator`] through `aggregator_mut`, so the spec feeds the
//! same `(type, modifiers, payload)` triples the DOM listeners would have
//! produced, then calls [`ScreenSpaceCameraController::update`] with a
//! [`SsccSceneContext`] assembled from the mock scene values, and asserts the
//! resulting camera pose — exactly the observable behaviour the CesiumJS spec
//! checks.
//!
//! Standalone integration-test entry — the specs aggregator under
//! `specs/tests/` is intentionally untouched.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::keyboard_event_modifier::KeyboardEventModifier;
use cesium_core::math::CesiumMath;
use cesium_core::matrix4::Matrix4;
use cesium_core::orthographic_off_center_frustum::OrthographicOffCenterFrustum;
use cesium_core::screen_space_event_handler::{
    MotionEvent, PositionedEvent, ScreenSpaceInputEvent,
};
use cesium_core::screen_space_event_type::ScreenSpaceEventType;
use cesium_core::scene_mode::SceneMode;
use cesium_scene::camera::{Camera, LookAtOffset};
use cesium_scene::camera_frustum::CameraFrustum;
use cesium_scene::map_mode2_d::MapMode2D;
use cesium_scene::screen_space_camera_controller::{
    ScreenSpaceCameraController, SsccSceneContext,
};
use cesium_test_utils::assert_approx_eq_f64;

/// `createCanvas(1024, 768)`.
const WIDTH: f64 = 1024.0;
const HEIGHT: f64 = 768.0;

/// The three mouse buttons the CesiumJS spec drives, expressed as their
/// down/up screen-space event types.
enum Button {
    Left,
    Middle,
    Right,
}

impl Button {
    fn down(&self) -> ScreenSpaceEventType {
        match self {
            Button::Left => ScreenSpaceEventType::LeftDown,
            Button::Middle => ScreenSpaceEventType::MiddleDown,
            Button::Right => ScreenSpaceEventType::RightDown,
        }
    }
    fn up(&self) -> ScreenSpaceEventType {
        match self {
            Button::Left => ScreenSpaceEventType::LeftUp,
            Button::Middle => ScreenSpaceEventType::MiddleUp,
            Button::Right => ScreenSpaceEventType::RightUp,
        }
    }
}

/// `moveMouse(button, startPosition, endPosition, shiftKey)`: a down at
/// `start`, a move to `end`, and an up at `end`.
fn move_mouse(
    controller: &mut ScreenSpaceCameraController,
    button: Button,
    modifiers: &[KeyboardEventModifier],
    start: Cartesian2,
    end: Cartesian2,
) {
    let aggregator = controller.aggregator_mut();
    aggregator.handle_input_event(
        button.down(),
        modifiers,
        &ScreenSpaceInputEvent::Positioned(PositionedEvent { position: start }),
    );
    aggregator.handle_input_event(
        ScreenSpaceEventType::MouseMove,
        modifiers,
        &ScreenSpaceInputEvent::Motion(MotionEvent {
            start_position: start,
            end_position: end,
        }),
    );
    aggregator.handle_input_event(
        button.up(),
        modifiers,
        &ScreenSpaceInputEvent::Positioned(PositionedEvent { position: end }),
    );
}

/// `simulateMouseWheel(wheelDelta)`: the aggregator consumes the already
/// normalised delta (the sign flip lives in the DOM `getWheelDelta` layer the
/// port does not model), so a positive delta zooms in, matching CesiumJS
/// `simulateMouseWheel(120)`.
fn mouse_wheel(controller: &mut ScreenSpaceCameraController, delta: f64) {
    controller
        .aggregator_mut()
        .handle_input_event(
            ScreenSpaceEventType::Wheel,
            &[],
            &ScreenSpaceInputEvent::Wheel(delta),
        );
}

/// Builds the mock scene's camera and the projection shared with the context.
///
/// Mirrors the spec `beforeEach`: `createCamera({ offset, near: 1, far:
/// 500000000 })` on a 1024×768 canvas with a WGS84 `GeographicProjection`.
fn mock_scene() -> (Camera, GeographicProjection) {
    let projection = GeographicProjection::new(Some(Ellipsoid::WGS84));
    let mut camera = Camera::new();
    camera.set_map_projection(Box::new(GeographicProjection::new(Some(
        Ellipsoid::WGS84,
    ))));
    camera.set_map_mode_2d(MapMode2D::InfiniteScroll);
    camera.set_near(1.0);
    camera.set_far(500000000.0);
    (camera, projection)
}

/// Assembles the [`SsccSceneContext`] the controller updates against.
fn context<'a>(
    camera: &'a mut Camera,
    projection: &'a GeographicProjection,
    mode: SceneMode,
) -> SsccSceneContext<'a> {
    SsccSceneContext {
        camera,
        globe: None,
        mode,
        map_projection: projection,
        map_mode_2d: MapMode2D::InfiniteScroll,
        canvas_client_width: WIDTH,
        canvas_client_height: HEIGHT,
        globe_height: Some(0.0),
        pick_position_supported: false,
        camera_underground: false,
        vertical_exaggeration: 1.0,
        vertical_exaggeration_relative_height: 0.0,
    }
}

/// `updateController()`: `camera.update(scene.mode)` then `controller.update()`.
fn update_controller(
    camera: &mut Camera,
    projection: &GeographicProjection,
    controller: &mut ScreenSpaceCameraController,
    mode: SceneMode,
) {
    camera.update(mode);
    let mut ctx = context(camera, projection, mode);
    controller.update(&mut ctx);
}

/// `setUp2D()`.
fn set_up_2d(camera: &mut Camera) {
    let max_radii = Ellipsoid::WGS84.maximum_radius();
    let mut frustum = OrthographicOffCenterFrustum::new();
    let right = max_radii * CesiumMath::PI;
    let top = right * (HEIGHT / WIDTH);
    frustum.right = Some(right);
    frustum.left = Some(-right);
    frustum.top = Some(top);
    frustum.bottom = Some(-top);
    frustum.near = 0.01 * max_radii;
    frustum.far = 60.0 * max_radii;
    camera.set_frustum(CameraFrustum::OrthographicOffCenter(frustum));

    camera.set_position(Cartesian3::new(0.0, 0.0, max_radii));
    camera.set_direction(Cartesian3::negate_new(&Cartesian3::UNIT_Z));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
}

/// `setUpCV()`.
fn set_up_cv(camera: &mut Camera) {
    let max_radii = Ellipsoid::WGS84.maximum_radius();
    camera.set_position(Cartesian3::new(0.0, 0.0, max_radii));
    camera.set_direction(Cartesian3::negate_new(&Cartesian3::UNIT_Z));
    camera.set_up(Cartesian3::UNIT_Y);
    camera.set_right(Cartesian3::UNIT_X);
}

/// `setUp3D()`: the `beforeEach` offset (`normalize(0, -2, 1) * 2.5 *
/// maximumRadius`) applied through `lookAtTransform(IDENTITY, offset)`.
fn set_up_3d(camera: &mut Camera) {
    let max_radii = Ellipsoid::WGS84.maximum_radius();
    let offset = Cartesian3::multiply_by_scalar_new(
        &Cartesian3::normalize_new(&Cartesian3::new(0.0, -2.0, 1.0)),
        2.5 * max_radii,
    );
    camera.look_at_transform(&Matrix4::IDENTITY, Some(LookAtOffset::Cartesian(offset)));
}

// ---- 2D translate (spec "translate * in 2D") ----

#[test]
fn translate_right_in_2d() {
    let (mut camera, projection) = mock_scene();
    set_up_2d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 4.0, HEIGHT / 2.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene2D);

    assert!(position.x < camera.position().x);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
}

#[test]
fn translate_left_in_2d() {
    let (mut camera, projection) = mock_scene();
    set_up_2d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(WIDTH / 4.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene2D);

    assert!(position.x > camera.position().x);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
}

#[test]
fn translate_up_in_2d() {
    let (mut camera, projection) = mock_scene();
    set_up_2d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene2D);

    assert!(position.y > camera.position().y);
    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
}

#[test]
fn translate_down_in_2d() {
    let (mut camera, projection) = mock_scene();
    set_up_2d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene2D);

    assert!(position.y < camera.position().y);
    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
}

// ---- 2D zoom (spec "zoom in 2D", "zoom in 2D with wheel") ----

#[test]
fn zoom_in_2d() {
    let (mut camera, projection) = mock_scene();
    set_up_2d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);
    camera.update(SceneMode::Scene2D);

    let position = *camera.position();
    let frustum_diff = {
        let (left, right, _, _) = camera.frustum_mut().bounds();
        right - left
    };
    move_mouse(
        &mut controller,
        Button::Right,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene2D);

    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
    let (left, right, _, _) = camera.frustum_mut().bounds();
    assert!(frustum_diff > right - left);
}

#[test]
fn zoom_in_2d_with_wheel() {
    let (mut camera, projection) = mock_scene();
    set_up_2d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);
    camera.update(SceneMode::Scene2D);

    let position = *camera.position();
    let frustum_diff = {
        let (left, right, _, _) = camera.frustum_mut().bounds();
        right - left
    };
    mouse_wheel(&mut controller, 120.0);
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene2D);

    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
    let (left, right, _, _) = camera.frustum_mut().bounds();
    assert!(frustum_diff > right - left);
}

// ---- Columbus view (spec "translate/zoom * in Columbus view") ----

#[test]
fn translate_right_in_columbus_view() {
    let (mut camera, projection) = mock_scene();
    set_up_cv(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 4.0, HEIGHT / 2.0),
    );
    update_controller(
        &mut camera,
        &projection,
        &mut controller,
        SceneMode::ColumbusView,
    );

    assert!(position.x < camera.position().x);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
}

#[test]
fn translate_up_in_columbus_view() {
    let (mut camera, projection) = mock_scene();
    set_up_cv(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
    );
    update_controller(
        &mut camera,
        &projection,
        &mut controller,
        SceneMode::ColumbusView,
    );

    assert!(position.y > camera.position().y);
    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
}

#[test]
fn zoom_in_columbus_view() {
    let (mut camera, projection) = mock_scene();
    set_up_cv(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Right,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
    );
    update_controller(
        &mut camera,
        &projection,
        &mut controller,
        SceneMode::ColumbusView,
    );

    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert!(position.z > camera.position().z);
}

#[test]
fn zoom_in_columbus_view_with_wheel() {
    let (mut camera, projection) = mock_scene();
    set_up_cv(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    mouse_wheel(&mut controller, 120.0);
    update_controller(
        &mut camera,
        &projection,
        &mut controller,
        SceneMode::ColumbusView,
    );

    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert!(position.z > camera.position().z);
}

#[test]
fn looks_in_columbus_view() {
    let (mut camera, projection) = mock_scene();
    set_up_cv(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[KeyboardEventModifier::Shift],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
    );
    update_controller(
        &mut camera,
        &projection,
        &mut controller,
        SceneMode::ColumbusView,
    );

    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
    let right = Cartesian3::cross_new(camera.direction(), camera.up());
    assert!(Cartesian3::equals_epsilon(
        Some(&right),
        Some(camera.right()),
        None,
        Some(CesiumMath::EPSILON12)
    ));
}

// ---- 3D (spec "rotates in 3D", "zoom in 3D", "tilts in 3D", "looks in 3D") ----

#[test]
fn rotates_in_3d() {
    let (mut camera, projection) = mock_scene();
    set_up_3d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[],
        Cartesian2::new(0.0, 0.0),
        Cartesian2::new(WIDTH / 4.0, HEIGHT / 4.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene3D);

    assert!(!Cartesian3::equals(Some(&position), Some(camera.position())));
    // The camera keeps looking at the centre: direction ≈ normalize(-position).
    let expected = Cartesian3::normalize_new(&Cartesian3::negate_new(camera.position()));
    assert!(Cartesian3::equals_epsilon(
        Some(camera.direction()),
        Some(&expected),
        None,
        Some(CesiumMath::EPSILON15)
    ));
    let right = Cartesian3::cross_new(camera.direction(), camera.up());
    assert!(Cartesian3::equals_epsilon(
        Some(&right),
        Some(camera.right()),
        None,
        Some(CesiumMath::EPSILON15)
    ));
    let up = Cartesian3::cross_new(camera.right(), camera.direction());
    assert!(Cartesian3::equals_epsilon(
        Some(&up),
        Some(camera.up()),
        None,
        Some(CesiumMath::EPSILON15)
    ));
}

#[test]
fn zoom_in_3d() {
    let (mut camera, projection) = mock_scene();
    set_up_3d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Right,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene3D);

    assert!(Cartesian3::magnitude(&position) > Cartesian3::magnitude(camera.position()));
}

#[test]
fn zoom_in_3d_with_wheel() {
    let (mut camera, projection) = mock_scene();
    set_up_3d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    mouse_wheel(&mut controller, 120.0);
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene3D);

    assert!(Cartesian3::magnitude(&position) > Cartesian3::magnitude(camera.position()));
}

#[test]
fn tilts_in_3d() {
    let (mut camera, projection) = mock_scene();
    set_up_3d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Middle,
        &[],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene3D);

    assert!(!Cartesian3::equals(Some(&position), Some(camera.position())));
    // After a tilt the camera no longer looks straight at the centre.
    let to_center = Cartesian3::normalize_new(&Cartesian3::negate_new(camera.position()));
    assert!(!Cartesian3::equals_epsilon(
        Some(camera.direction()),
        Some(&to_center),
        None,
        Some(CesiumMath::EPSILON14)
    ));
    let right = Cartesian3::cross_new(camera.direction(), camera.up());
    assert!(Cartesian3::equals_epsilon(
        Some(&right),
        Some(camera.right()),
        None,
        Some(CesiumMath::EPSILON14)
    ));
}

#[test]
fn looks_in_3d() {
    let (mut camera, projection) = mock_scene();
    set_up_3d(&mut camera);
    let mut controller = ScreenSpaceCameraController::new(&projection, WIDTH);

    let position = *camera.position();
    move_mouse(
        &mut controller,
        Button::Left,
        &[KeyboardEventModifier::Shift],
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 2.0),
        Cartesian2::new(WIDTH / 2.0, HEIGHT / 4.0),
    );
    update_controller(&mut camera, &projection, &mut controller, SceneMode::Scene3D);

    // Looking rotates in place: the position is untouched, the direction is not.
    assert_approx_eq_f64!(camera.position().x, position.x, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().y, position.y, CesiumMath::EPSILON7);
    assert_approx_eq_f64!(camera.position().z, position.z, CesiumMath::EPSILON7);
    let to_center = Cartesian3::normalize_new(&Cartesian3::negate_new(camera.position()));
    assert!(!Cartesian3::equals(Some(camera.direction()), Some(&to_center)));
    let right = Cartesian3::cross_new(camera.direction(), camera.up());
    assert!(Cartesian3::equals_epsilon(
        Some(&right),
        Some(camera.right()),
        None,
        Some(CesiumMath::EPSILON12)
    ));
    let up = Cartesian3::cross_new(camera.right(), camera.direction());
    assert!(Cartesian3::equals_epsilon(
        Some(&up),
        Some(camera.up()),
        None,
        Some(CesiumMath::EPSILON12)
    ));
}
