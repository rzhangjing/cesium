//! Ported from `packages/engine/Source/Scene/Camera.js`.
//!
//! The camera defines the view frustum and position from which the scene is
//! rendered. In CesiumJS this is a 3990-line file managing view/projection
//! matrices, flight animations, coordinate transforms and user interaction
//! (look/rotate/move/zoom).
//!
//! B4-1 materialization: the view matrix computation (`updateViewMatrix`), the
//! projection matrices, `getPickRay`, `pickEllipsoid` and the orthonormalizing
//! `updateMembers` semantics are ported one-to-one.
//!
//! B3-2a materialization: the CesiumJS *state model* is reproduced — the
//! four-variant [`CameraFrustum`] union behind `camera.frustum`, the private
//! mirrors `_position`/`_direction`/`_up`/`_right` that `updateMembers` keeps in
//! step with the public pose, the `_actualTransform`/`_actualInvTransform` pair
//! that folds the Columbus View and 2D projections into the view matrix, the
//! world-coordinate getters, the `moveStart`/`moveEnd`/`changed` events together
//! with the `updateCameraDeltas` / `_updateCameraChanged` bookkeeping behind
//! them, and the `TRANSFORM_2D` / `DEFAULT_VIEW_*` / `DEFAULT_OFFSET` constants.
//!
//! M3/S3 materialization: flight animations are driven through the shared
//! flight channel ([`crate::camera_flight_path`]); `Camera::update` applies the
//! in-flight pose each frame. Screen-space controllers remain future work.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::cartographic::Cartographic;
use cesium_core::developer_error::throw_developer_error;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::ellipsoid_geodesic::EllipsoidGeodesic;
use cesium_core::event::Event;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::get_timestamp::get_timestamp;
use cesium_core::heading_pitch_range::HeadingPitchRange;
use cesium_core::heading_pitch_roll::HeadingPitchRoll;
use cesium_core::intersect::Intersect;
use cesium_core::intersection_tests::IntersectionTests;
use cesium_core::map_projection::MapProjection;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::orthographic_frustum::OrthographicFrustum;
use cesium_core::perspective_frustum::PerspectiveFrustum;
use cesium_core::quaternion::Quaternion;
use cesium_core::ray::Ray;
use cesium_core::rectangle::Rectangle;
use cesium_core::scene_mode::SceneMode;
use cesium_core::transforms;

use crate::camera_flight_path::{CameraFlightChannel, CameraFlightPath, CameraFlightTweenOptions};
use crate::camera_frustum::CameraFrustum;
use crate::map_mode2_d::MapMode2D;
use crate::tween_collection::{EasingFn, TweenOptions};

/// The type of camera projection.
///
/// DEVIATION: CesiumJS has no such enum — the projection *is* the concrete class
/// of `camera.frustum` and callers dispatch on `instanceof`. The port keeps this
/// two-value summary because the widgets layer asks it
/// (`VRButtonViewModel`: "is the frustum orthographic?"); it is derived from
/// [`CameraFrustum`], which is the faithful model. Tracked in
/// `docs/deviations.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

/// The scene-derived quantities CesiumJS `calculateOrthographicFrustumWidth`
/// reads through `camera._scene`.
///
/// DEVIATION: the port cannot keep a `camera._scene` back-reference — [`Scene`]
/// *owns* its `Camera` and its `Globe` as plain fields, so `&mut camera` and
/// `&scene` can never coexist, and the JS call happens from inside
/// `Camera#move` / `Camera#rotate` / `setView*`. Instead the owning scene
/// publishes these values with [`Camera::set_scene_context`] once per frame
/// before [`Camera::update`]; [`Camera::centre_window_position`] hands it the
/// exact window position the JS probes so the pick ray it evaluates is the same
/// one CesiumJS would build. Tracked in `docs/deviations.md`.
#[derive(Debug, Clone, Copy)]
pub struct CameraSceneContext {
    /// `scene.pixelRatio`. DEVIATION: the port has no device-pixel-ratio
    /// concept, so this stays `1.0`.
    pub pixel_ratio: f64,
    /// `scene.globe.pickWorldCoordinates(camera.getPickRay(mousePosition),
    /// scene, true, …)`; `None` is the JS `undefined` (no globe, or a miss).
    ///
    /// DEVIATION: `scene.pickPositionSupported` asks the WebGL context for a
    /// depth texture, which the port does not have, so CesiumJS's
    /// `depthIntersection` branch is never taken and its `depthDistance` stays
    /// `Number.POSITIVE_INFINITY` — the `Math.min` therefore always reduces to
    /// this ray distance. Tracked in `docs/deviations.md`.
    pub ray_intersection: Option<Cartesian3>,
    /// `scene.screenSpaceCameraController.minimumZoomDistance`, read by
    /// `viewBoundingSphere`'s `adjustBoundingSphereOffset` to clamp the
    /// computed range.
    ///
    /// DEVIATION: CesiumJS reaches it through
    /// `camera._scene.screenSpaceCameraController`; the port has no scene
    /// back-reference, so the owning scene publishes it here, defaulting to the
    /// `ScreenSpaceCameraController` default of `1.0`. Tracked in
    /// `docs/deviations.md`.
    pub minimum_zoom_distance: f64,
    /// `scene.screenSpaceCameraController.maximumZoomDistance`. The port
    /// defaults to `f64::MAX`, matching [`crate::screen_space_camera_controller::ScreenSpaceCameraController`]
    /// (CesiumJS uses `Number.POSITIVE_INFINITY`, equivalent for any finite
    /// range).
    pub maximum_zoom_distance: f64,
}

impl Default for CameraSceneContext {
    fn default() -> Self {
        Self {
            pixel_ratio: 1.0,
            ray_intersection: None,
            minimum_zoom_distance: 1.0,
            maximum_zoom_distance: f64::MAX,
        }
    }
}

/// The `destination` argument of CesiumJS `Camera#setView`.
///
/// The JS accepts `Cartesian3 | Rectangle` and tells them apart with
/// `defined(destination.west)`.
#[derive(Debug, Clone, Copy)]
pub enum SetViewDestination {
    /// The final position of the camera in world coordinates.
    Cartesian(Cartesian3),
    /// A rectangle that should be visible from a top-down view; resolved
    /// through [`Camera::get_rectangle_camera_coordinates`], which also forces
    /// `convert = false`.
    Rectangle(Rectangle),
}

/// The `orientation` argument of CesiumJS `Camera#setView`.
///
/// The JS is a single bag of five optional properties — either a `direction`/`up`
/// pair or `heading`/`pitch`/`roll` values. `defined(orientation.direction)`
/// selects the pair, which `directionUpToHeadingPitchRoll` then converts into
/// the triple; the unset members of the triple fall back to
/// `heading = 0`, `pitch = -π/2`, `roll = 0`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SetViewOrientation {
    /// `orientation.direction`. `Some` selects the direction/up pair, and
    /// `orientation.heading`/`pitch`/`roll` are then ignored.
    pub direction: Option<Cartesian3>,
    /// `orientation.up`.
    pub up: Option<Cartesian3>,
    /// `orientation.heading`, defaulting to `0.0`.
    pub heading: Option<f64>,
    /// `orientation.pitch`, defaulting to `-CesiumMath.PI_OVER_TWO` (looking
    /// straight down).
    pub pitch: Option<f64>,
    /// `orientation.roll`, defaulting to `0.0`.
    pub roll: Option<f64>,
}

/// CesiumJS's `Camera#setView(options)` argument object.
#[derive(Debug, Clone)]
pub struct SetViewOptions {
    /// `options.destination`, defaulting to `Cartesian3.clone(this.positionWC)`
    /// — i.e. "keep the position, change the orientation".
    pub destination: Option<SetViewDestination>,
    /// `options.orientation ?? Frozen.EMPTY_OBJECT`.
    pub orientation: SetViewOrientation,
    /// `options.endTransform` — the reference frame the camera should end up
    /// in, installed with [`Camera::set_transform`] before anything else.
    pub end_transform: Option<Matrix4>,
    /// `options.convert ?? true`: whether a world-coordinate destination is
    /// converted into scene coordinates. Only relevant outside 3D.
    pub convert: bool,
}

impl Default for SetViewOptions {
    fn default() -> Self {
        Self {
            destination: None,
            orientation: SetViewOrientation::default(),
            end_transform: None,
            convert: true,
        }
    }
}

/// CesiumJS's `Camera#flyTo(options)` argument object.
///
/// DEVIATION: CesiumJS additionally accepts `maximumHeight`,
/// `pitchAdjustHeight`, `flyOverLongitude`, and `flyOverLongitudeWeight`, which
/// shape the flight arc inside `CameraFlightPath.createTween`. The port's
/// [`CameraFlightPath::create_tween`] drives a pose-based flight and ignores
/// them (see [`CameraFlightTweenOptions`]), so they are not modelled here.
/// Tracked in `docs/deviations.md`.
pub struct FlyToOptions {
    /// `options.destination` (required): the final position of the camera in
    /// world coordinates, or a rectangle that would be visible from a top-down
    /// view.
    pub destination: SetViewDestination,
    /// `options.orientation ?? Frozen.EMPTY_OBJECT`: either a `direction`/`up`
    /// pair or `heading`/`pitch`/`roll` values.
    pub orientation: SetViewOrientation,
    /// `options.duration`. `None` derives it from the travel distance; a value
    /// `<= 0.0` takes the synchronous [`Camera::set_view_with_options`]
    /// shortcut.
    pub duration: Option<f64>,
    /// `options.complete`: fired when the flight completes, or immediately on
    /// the `duration <= 0.0` shortcut.
    pub complete: Option<Box<dyn FnOnce()>>,
    /// `options.cancel`: fired if the flight is canceled.
    pub cancel: Option<Box<dyn FnOnce()>>,
    /// `options.endTransform`: the reference frame the camera will be in when
    /// the flight completes.
    pub end_transform: Option<Matrix4>,
    /// `options.convert ?? true`.
    pub convert: bool,
    /// `options.easingFunction`.
    pub easing_function: Option<EasingFn>,
}

impl Default for FlyToOptions {
    fn default() -> Self {
        Self {
            destination: SetViewDestination::Cartesian(Cartesian3::ZERO),
            orientation: SetViewOrientation::default(),
            duration: None,
            complete: None,
            cancel: None,
            end_transform: None,
            convert: true,
            easing_function: None,
        }
    }
}

/// CesiumJS's `Camera#flyToBoundingSphere(boundingSphere, options)` argument
/// object.
///
/// DEVIATION: as [`FlyToOptions`], the `maximumHeight` / `pitchAdjustHeight` /
/// `flyOverLongitude*` arc-shaping options are not modelled. Tracked in
/// `docs/deviations.md`.
pub struct FlyToBoundingSphereOptions {
    /// `options.duration`.
    pub duration: Option<f64>,
    /// `options.offset`: the heading/pitch/range from the target in its local
    /// east-north-up frame. A zero (or `None`) range is computed so the whole
    /// sphere is visible.
    pub offset: Option<HeadingPitchRange>,
    /// `options.complete`.
    pub complete: Option<Box<dyn FnOnce()>>,
    /// `options.cancel`.
    pub cancel: Option<Box<dyn FnOnce()>>,
    /// `options.endTransform`.
    pub end_transform: Option<Matrix4>,
    /// `options.easingFunction`.
    pub easing_function: Option<EasingFn>,
}

impl Default for FlyToBoundingSphereOptions {
    fn default() -> Self {
        Self {
            duration: None,
            offset: None,
            complete: None,
            cancel: None,
            end_transform: None,
            easing_function: None,
        }
    }
}

/// The `offset` argument of CesiumJS `Camera#lookAt` /
/// `Camera#lookAtTransform`.
///
/// The JS accepts `Cartesian3 | HeadingPitchRange` and picks between them with
/// `defined(offset.heading)`.
#[derive(Debug, Clone, Copy)]
pub enum LookAtOffset {
    /// An offset from the centre of the reference frame.
    Cartesian(Cartesian3),
    /// Heading and pitch measured in the reference frame, plus the distance
    /// from its centre. The heading is the angle from the y axis increasing
    /// towards x; positive pitch is below the xy-plane.
    HeadingPitchRange(HeadingPitchRange),
}

/// The camera defining the view frustum and position.
///
/// Mirrors the CesiumJS `Camera` which manages the view and projection
/// matrices, coordinate transforms, and camera movement/rotation.
pub struct Camera {
    // ---- Public pose (`this.position`, `this.direction`, `this.up`, `this.right`) ----
    /// The position of the camera, in the reference frame of [`Camera::transform`].
    position: Cartesian3,
    /// The view direction of the camera.
    direction: Cartesian3,
    /// The up direction of the camera.
    up: Cartesian3,
    /// The right direction of the camera.
    right: Cartesian3,

    // ---- Private mirrors (`_position`, `_direction`, `_up`, `_right`) ----
    /// The last pose `updateMembers` published into the derived state.
    private_position: Cartesian3,
    private_direction: Cartesian3,
    private_up: Cartesian3,
    private_right: Cartesian3,

    // ---- Derived state (`_positionWC`, `_positionCartographic`, `_*WC`) ----
    position_wc: Cartesian3,
    position_cartographic: Cartographic,
    direction_wc: Cartesian3,
    up_wc: Cartesian3,
    right_wc: Cartesian3,

    /// `_viewMatrix`.
    view_matrix: Matrix4,
    /// `_invViewMatrix`.
    inverse_view_matrix: Matrix4,

    // ---- Reference frame (`_transform`, `_invTransform`, `_actualTransform`, …) ----
    /// `_transform`. Read-only in CesiumJS; the mutator is [`Camera::set_transform`].
    transform: Matrix4,
    /// `_invTransform`.
    inverse_transform: Matrix4,
    /// `_actualTransform` — `_transform` with the Columbus View / 2D projection
    /// folded in.
    actual_transform: Matrix4,
    /// `_actualInvTransform`.
    actual_inverse_transform: Matrix4,
    /// `_transformChanged`.
    transform_changed: bool,

    // ---- Projection ----
    /// CesiumJS has no such cache: `frameState` reads
    /// `camera.frustum.projectionMatrix` directly. DEVIATION, tracked in
    /// `docs/deviations.md`.
    projection_matrix: Matrix4,
    /// Port-only companion of [`Camera::projection_matrix`] (CesiumJS computes
    /// the inverse on demand). DEVIATION.
    inverse_projection_matrix: Matrix4,
    /// `this.frustum`.
    frustum: CameraFrustum,
    /// `_max2Dfrustum`.
    max_2d_frustum: Option<CameraFrustum>,

    // ---- Scene mode / map projection ----
    /// `_mode`.
    mode: SceneMode,
    /// `_modeChanged`.
    mode_changed: bool,
    /// `_projection`.
    ///
    /// DEVIATION: CesiumJS shares `scene.mapProjection` by reference; the port
    /// owns one, defaulted to the same `new GeographicProjection(WGS84)` the
    /// scene builds. [`Camera::set_map_projection`] is the injection point.
    map_projection: Box<dyn MapProjection>,
    /// `_scene.mapMode2D`, read by `clampMove2D`.
    map_mode_2d: MapMode2D,
    /// `_maxCoord`.
    max_coord: Cartesian3,

    // ---- Tunables ----
    /// `Camera#maximumZoomDistance`.
    maximum_zoom_distance: f64,
    /// `Camera#minimumZoomDistance`.
    minimum_zoom_distance: f64,
    /// `Camera#defaultMoveAmount`.
    default_move_amount: f64,
    /// `Camera#defaultLookAmount`.
    default_look_amount: f64,
    /// `Camera#defaultRotateAmount`.
    default_rotate_amount: f64,
    /// `Camera#defaultZoomAmount`.
    default_zoom_amount: f64,
    /// `Camera#maximumZoomFactor`.
    maximum_zoom_factor: f64,
    /// `Camera#constrainedAxis` (initially `undefined`).
    constrained_axis: Option<Cartesian3>,
    /// `Camera#percentageChanged`.
    percentage_changed: f64,

    // ---- Events ----
    /// `_moveStart`.
    move_start: Event<()>,
    /// `_moveEnd`.
    move_end: Event<()>,
    /// `_changed`.
    changed_event: Event<f64>,
    /// `_changedPosition`.
    changed_position: Option<Cartesian3>,
    /// `_changedDirection`.
    changed_direction: Option<Cartesian3>,
    /// `_changedFrustum`.
    changed_frustum: Option<CameraFrustum>,
    /// `_changedHeading`.
    changed_heading: Option<f64>,
    /// `_changedRoll`.
    changed_roll: Option<f64>,

    // ---- Move timers ----
    /// `_oldPositionWC`.
    old_position_wc: Option<Cartesian3>,
    /// `Camera#positionWCDeltaMagnitude`.
    position_wc_delta_magnitude: f64,
    /// `Camera#positionWCDeltaMagnitudeLastFrame`.
    position_wc_delta_magnitude_last_frame: f64,
    /// `Camera#timeSinceMoved`.
    time_since_moved: f64,
    /// `_lastMovedTimestamp`.
    last_moved_timestamp: f64,

    /// The drawing buffer width (`scene.drawingBufferWidth`).
    canvas_width: u32,
    /// The drawing buffer height (`scene.drawingBufferHeight`).
    canvas_height: u32,

    /// The shared camera-flight channel installed by the [`crate::scene::Scene`]
    /// (M3/S3). `None` for standalone cameras; when set, [`Camera::update`]
    /// applies the in-flight pose each frame (mirrors CesiumJS `Camera#flyTo`
    /// driving `position`/`direction`/`up` from the tween).
    flight_channel: Option<CameraFlightChannel>,

    /// The scene-derived inputs of `calculateOrthographicFrustumWidth`; see
    /// [`CameraSceneContext`] for why the port publishes them instead of
    /// reaching through `camera._scene`.
    scene_context: CameraSceneContext,
}

impl Camera {
    /// `Camera.TRANSFORM_2D`.
    #[must_use]
    pub fn transform_2d() -> Matrix4 {
        Matrix4::new(
            0.0, 0.0, 1.0, 0.0,
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// `Camera.TRANSFORM_2D_INVERSE`.
    #[must_use]
    pub fn transform_2d_inverse() -> Matrix4 {
        Matrix4::inverse_transformation_new(&Self::transform_2d())
    }

    /// `Camera.DEFAULT_VIEW_RECTANGLE`.
    #[must_use]
    pub fn default_view_rectangle() -> Rectangle {
        Rectangle::from_degrees(-95.0, -20.0, -70.0, 90.0)
    }

    /// `Camera.DEFAULT_VIEW_FACTOR`.
    pub const DEFAULT_VIEW_FACTOR: f64 = 0.5;

    /// `Camera.DEFAULT_OFFSET`.
    #[must_use]
    pub fn default_offset() -> HeadingPitchRange {
        HeadingPitchRange::new(0.0, -CesiumMath::PI_OVER_FOUR, 0.0)
    }

    /// Creates a new camera with default values.
    ///
    /// DEVIATION: CesiumJS takes the owning `scene` and reads
    /// `scene.drawingBufferWidth`/`Height` and `scene.mapProjection` from it.
    /// The port has no back-reference, so the canvas starts at the 800×600 the
    /// rest of the port assumes and the projection at
    /// `GeographicProjection::new(None)` — the same default [`crate::scene::Scene`]
    /// builds. The JS constructor also parks the camera on
    /// `rectangleCameraPosition3D(DEFAULT_VIEW_RECTANGLE)` scaled by
    /// `DEFAULT_VIEW_FACTOR`; that helper arrives with B3-2e, so the port keeps
    /// the origin pose for now. Tracked in `docs/deviations.md`.
    pub fn new() -> Self {
        let map_projection: Box<dyn MapProjection> = Box::new(GeographicProjection::new(None));
        // JS: `_maxCoord = projection.project(new Cartographic(Math.PI, PI_OVER_TWO))`
        let max_coord = map_projection.project(&Cartographic::new(
            std::f64::consts::PI,
            CesiumMath::PI_OVER_TWO,
            0.0,
        ));

        // JS: `this.frustum = new PerspectiveFrustum();`
        //     `this.frustum.aspectRatio = drawingBufferWidth / drawingBufferHeight;`
        //     `this.frustum.fov = CesiumMath.toRadians(60.0);`
        let mut frustum = CameraFrustum::new();
        frustum.set_aspect_ratio(800.0 / 600.0);
        frustum.set_fov(CesiumMath::to_radians(60.0));

        let mut camera = Self {
            position: Cartesian3::ZERO,
            direction: Cartesian3::ZERO,
            up: Cartesian3::ZERO,
            right: Cartesian3::ZERO,
            private_position: Cartesian3::ZERO,
            private_direction: Cartesian3::ZERO,
            private_up: Cartesian3::ZERO,
            private_right: Cartesian3::ZERO,
            position_wc: Cartesian3::ZERO,
            position_cartographic: Cartographic::default(),
            direction_wc: Cartesian3::ZERO,
            up_wc: Cartesian3::ZERO,
            right_wc: Cartesian3::ZERO,
            view_matrix: Matrix4::IDENTITY,
            inverse_view_matrix: Matrix4::IDENTITY,
            transform: Matrix4::IDENTITY,
            inverse_transform: Matrix4::IDENTITY,
            actual_transform: Matrix4::IDENTITY,
            actual_inverse_transform: Matrix4::IDENTITY,
            transform_changed: false,
            projection_matrix: Matrix4::IDENTITY,
            inverse_projection_matrix: Matrix4::IDENTITY,
            frustum,
            max_2d_frustum: None,
            mode: SceneMode::Scene3D,
            mode_changed: true,
            map_projection,
            map_mode_2d: MapMode2D::default(),
            max_coord,
            maximum_zoom_distance: 10.0 * Ellipsoid::WGS84.maximum_radius(),
            minimum_zoom_distance: -1.0 * Ellipsoid::WGS84.maximum_radius(),
            default_move_amount: 100000.0,
            default_look_amount: std::f64::consts::PI / 60.0,
            default_rotate_amount: std::f64::consts::PI / 3600.0,
            default_zoom_amount: 100000.0,
            maximum_zoom_factor: 1.5,
            constrained_axis: None,
            percentage_changed: 0.5,
            move_start: Event::new(),
            move_end: Event::new(),
            changed_event: Event::new(),
            changed_position: None,
            changed_direction: None,
            changed_frustum: None,
            changed_heading: None,
            changed_roll: None,
            old_position_wc: None,
            position_wc_delta_magnitude: 0.0,
            position_wc_delta_magnitude_last_frame: 0.0,
            time_since_moved: 0.0,
            last_moved_timestamp: 0.0,
            canvas_width: 800,
            canvas_height: 600,
            flight_channel: None,
            scene_context: CameraSceneContext::default(),
        };

        // The JS pose starts out all-zero (`new Cartesian3()`); the port keeps
        // the historical default view so the existing scene/widget wiring is
        // unaffected. `set_position`/… are not used here because they would flag
        // a change the constructor has not yet published.
        camera.direction = Cartesian3::new(0.0, 0.0, -1.0);
        camera.up = Cartesian3::new(0.0, 1.0, 0.0);
        camera.right = Cartesian3::new(1.0, 0.0, 0.0);

        // JS constructor: `this._viewMatrix = new Matrix4(); … updateViewMatrix(this);`
        // The port seeds a usable default pose (see the DEVIATION above) whose
        // private mirrors are still zero, so it runs `updateMembers` instead of
        // the bare `updateViewMatrix`: that publishes the mirrors, the
        // world-coordinate quantities and the view matrix in one pass, exactly
        // as the first CesiumJS derived getter would.
        camera.update_members();
        camera
    }

    /// Attaches the shared flight channel (the `Scene` creates it and shares
    /// it with its camera so [`crate::scene::Scene::fly_to`] can drive the
    /// camera through `&self`).
    pub fn set_flight_channel(&mut self, channel: CameraFlightChannel) {
        self.flight_channel = Some(channel);
    }

    // ---- Position and orientation ----

    /// Returns the camera position in the reference frame of [`Camera::transform`].
    pub fn position(&self) -> &Cartesian3 { &self.position }

    /// Returns the camera direction (unit vector).
    pub fn direction(&self) -> &Cartesian3 { &self.direction }

    /// Returns the camera up vector (unit vector).
    pub fn up(&self) -> &Cartesian3 { &self.up }

    /// Returns the camera right vector (unit vector).
    pub fn right(&self) -> &Cartesian3 { &self.right }

    /// Sets the camera position.
    pub fn set_position(&mut self, position: Cartesian3) {
        self.position = position;
    }

    /// Sets the camera direction.
    pub fn set_direction(&mut self, direction: Cartesian3) {
        self.direction = direction;
    }

    /// Sets the camera up vector.
    pub fn set_up(&mut self, up: Cartesian3) {
        self.up = up;
    }

    /// Sets the camera right vector.
    pub fn set_right(&mut self, right: Cartesian3) {
        self.right = right;
    }

    // ---- Derived world-space state ----

    /// `Camera#positionWC`.
    ///
    /// DEVIATION: the CesiumJS getter runs `updateMembers(this)` first; the port
    /// returns the cache and refreshes it in [`Camera::update`] /
    /// [`Camera::refresh`]. See [`Camera::refresh`].
    pub fn position_wc(&self) -> &Cartesian3 { &self.position_wc }

    /// `Camera#positionCartographic`.
    pub fn position_cartographic(&self) -> &Cartographic { &self.position_cartographic }

    /// `Camera#directionWC`.
    pub fn direction_wc(&self) -> &Cartesian3 { &self.direction_wc }

    /// `Camera#upWC`.
    pub fn up_wc(&self) -> &Cartesian3 { &self.up_wc }

    /// `Camera#rightWC`.
    pub fn right_wc(&self) -> &Cartesian3 { &self.right_wc }

    /// Refreshes the derived camera state.
    ///
    /// DEVIATION: CesiumJS has no such method — every derived getter
    /// (`viewMatrix`, `inverseViewMatrix`, `inverseTransform`, `positionWC`,
    /// `directionWC`, `positionCartographic`, …) calls the module-level
    /// `updateMembers(this)` before returning its cached value. Rust cannot do
    /// that behind a shared reference, so the refresh is explicit and the
    /// getters return the cache. [`Camera::update`] runs it at the end of every
    /// frame, which is exactly when CesiumJS `Scene#updateFrameState` reads those
    /// getters, so the observable state is the same. Tracked in
    /// `docs/deviations.md`.
    pub fn refresh(&mut self) {
        self.update_members();
        self.update_projection_matrix();
    }

    // ---- Matrices ----

    /// Returns the view matrix (world-to-camera).
    pub fn view_matrix(&self) -> &Matrix4 { &self.view_matrix }

    /// Returns the inverse view matrix (camera-to-world).
    pub fn inverse_view_matrix(&self) -> &Matrix4 { &self.inverse_view_matrix }

    /// Returns the projection matrix.
    pub fn projection_matrix(&self) -> &Matrix4 { &self.projection_matrix }

    /// Returns the inverse projection matrix.
    pub fn inverse_projection_matrix(&self) -> &Matrix4 { &self.inverse_projection_matrix }

    /// `Camera#transform` — the reference frame; its inverse is appended to the
    /// view matrix. Read-only in CesiumJS.
    pub fn transform(&self) -> &Matrix4 { &self.transform }

    /// `Camera#inverseTransform`.
    pub fn inverse_transform(&self) -> &Matrix4 { &self.inverse_transform }

    /// `_actualTransform` — [`Camera::transform`] with the Columbus View / 2D
    /// projection folded in by `updateMembers`.
    pub fn actual_transform(&self) -> &Matrix4 { &self.actual_transform }

    /// `_actualInvTransform`.
    pub fn actual_inverse_transform(&self) -> &Matrix4 { &self.actual_inverse_transform }

    /// `Camera.prototype._setTransform`.
    ///
    /// Captures the world-space pose, installs the new reference frame, and
    /// re-expresses `position`/`direction`/`up` in it so the camera keeps
    /// looking at the same place; `right` is then rebuilt from the cross
    /// product. `updateMembers` runs twice, exactly as in CesiumJS — once to
    /// publish `_actualInvTransform` for the new frame and once to absorb the
    /// re-expressed pose.
    pub fn set_transform(&mut self, transform: Matrix4) {
        // JS reads `this.positionWC` / `this.upWC` / `this.directionWC`, whose
        // getters run `updateMembers` first.
        self.update_members();
        let position = self.position_wc;
        let up = self.up_wc;
        let direction = self.direction_wc;

        self.transform = transform;
        self.transform_changed = true;
        self.update_members();
        let inverse = self.actual_inverse_transform;

        self.position = Matrix4::multiply_by_point_new(&inverse, &position);
        self.direction = Matrix4::multiply_by_point_as_vector_new(&inverse, &direction);
        self.up = Matrix4::multiply_by_point_as_vector_new(&inverse, &up);
        self.right = Cartesian3::cross_new(&self.direction, &self.up);

        self.update_members();
    }

    // ---- Scene mode / map projection ----

    /// `_mode`.
    pub fn mode(&self) -> SceneMode { self.mode }

    /// `_projection`.
    pub fn map_projection(&self) -> &dyn MapProjection { self.map_projection.as_ref() }

    /// Replaces `_projection` and recomputes `_maxCoord`.
    ///
    /// CesiumJS reads the projection off the scene once in the constructor; the
    /// port has no back-reference, so the scene injects it here.
    pub fn set_map_projection(&mut self, projection: Box<dyn MapProjection>) {
        let max_coord = projection.project(&Cartographic::new(
            std::f64::consts::PI,
            CesiumMath::PI_OVER_TWO,
            0.0,
        ));
        self.map_projection = projection;
        self.max_coord = max_coord;
        // The actual transform is derived from the projection, so the cached one
        // is stale until the next `updateMembers`.
        self.mode_changed = true;
    }

    /// `_scene.mapMode2D`.
    pub fn map_mode_2d(&self) -> MapMode2D { self.map_mode_2d }

    /// Sets `_scene.mapMode2D`.
    pub fn set_map_mode_2d(&mut self, map_mode_2d: MapMode2D) {
        self.map_mode_2d = map_mode_2d;
    }

    /// `_maxCoord`.
    pub fn max_coord(&self) -> &Cartesian3 { &self.max_coord }

    /// `_max2Dfrustum`.
    pub fn max_2d_frustum(&self) -> Option<&CameraFrustum> { self.max_2d_frustum.as_ref() }

    // ---- Frustum ----

    /// `this.frustum` — the faithful four-variant model.
    pub fn frustum(&self) -> &CameraFrustum { &self.frustum }

    /// `this.frustum` — the faithful four-variant model, mutably.
    pub fn frustum_mut(&mut self) -> &mut CameraFrustum { &mut self.frustum }

    /// Replaces `this.frustum` wholesale (`switchTo*Frustum`, `SceneTransitioner`
    /// and `ShadowMap` all assign it directly).
    pub fn set_frustum(&mut self, frustum: CameraFrustum) {
        self.frustum = frustum;
    }

    /// Returns the projection type derived from the frustum variant.
    pub fn projection_type(&self) -> CameraProjection {
        if self.frustum.is_orthographic() || self.frustum.is_orthographic_off_center() {
            CameraProjection::Orthographic
        } else {
            CameraProjection::Perspective
        }
    }

    /// Sets the projection type by swapping the frustum variant.
    ///
    /// DEVIATION: the port-only sibling of CesiumJS
    /// `switchToPerspectiveFrustum` / `switchToOrthographicFrustum`, minus their
    /// `SceneMode.SCENE2D` early-return and minus
    /// `calculateOrthographicFrustumWidth`. `near`/`far`/`aspectRatio` are
    /// carried over, which the JS switches do not do — they install a fresh
    /// frustum. Those two are ported faithfully as
    /// [`Camera::switch_to_perspective_frustum`] /
    /// [`Camera::switch_to_orthographic_frustum`]; this setter is kept for the
    /// widgets layer's projection toggle. Tracked in `docs/deviations.md`.
    pub fn set_projection(&mut self, projection: CameraProjection) {
        let aspect_ratio = self.frustum.aspect_ratio();
        let near = self.frustum.near();
        let far = self.frustum.far();
        let width = self.frustum.width().unwrap_or(1.0);
        let fov = self.frustum.fov().unwrap_or(CesiumMath::to_radians(60.0));

        self.frustum = match projection {
            CameraProjection::Perspective => {
                let mut frustum = PerspectiveFrustum::new();
                frustum.aspect_ratio = aspect_ratio;
                frustum.fov = Some(fov);
                frustum.near = near;
                frustum.far = far;
                CameraFrustum::Perspective(frustum)
            }
            CameraProjection::Orthographic => {
                let mut frustum = OrthographicFrustum::new();
                frustum.aspect_ratio = aspect_ratio;
                frustum.width = Some(width);
                frustum.near = near;
                frustum.far = far;
                CameraFrustum::Orthographic(frustum)
            }
        };
    }

    // ---- Orthographic frustum adaptation ----

    /// Publishes the scene-derived quantities
    /// [`Camera::adjust_orthographic_frustum`] reads. See
    /// [`CameraSceneContext`].
    pub fn set_scene_context(&mut self, context: CameraSceneContext) {
        self.scene_context = context;
    }

    /// The published scene-derived quantities.
    pub fn scene_context(&self) -> &CameraSceneContext {
        &self.scene_context
    }

    /// The window position `calculateOrthographicFrustumWidth` probes:
    /// `mousePosition.x = scene.drawingBufferWidth / scene.pixelRatio / 2.0`,
    /// and likewise for `y`.
    ///
    /// The owning scene evaluates [`Camera::get_pick_ray`] here, hands the ray
    /// to `globe.pickWorldCoordinates` and publishes the hit through
    /// [`Camera::set_scene_context`], which is what closes the loop the JS
    /// closes through `camera._scene`.
    pub fn centre_window_position(&self) -> Cartesian2 {
        let pixel_ratio = self.scene_context.pixel_ratio;
        Cartesian2::new(
            self.canvas_width as f64 / pixel_ratio / 2.0,
            self.canvas_height as f64 / pixel_ratio / 2.0,
        )
    }

    /// The module-level `calculateOrthographicFrustumWidth(camera)`.
    ///
    /// When the camera is fixed to an object (`transform !== IDENTITY`) the
    /// frustum width is simply the distance from the reference frame origin, so
    /// it stays constant while the object moves. Otherwise it is the distance to
    /// whatever sits under the screen centre, falling back to the camera height
    /// above the ellipsoid when nothing does.
    ///
    /// DEVIATION: see [`CameraSceneContext`] for the two scene-dependent inputs
    /// (`rayIntersection` published by the scene, `depthIntersection` never
    /// available). Tracked in `docs/deviations.md`.
    pub fn calculate_orthographic_frustum_width(&self) -> f64 {
        // Camera is fixed to an object, so keep frustum width constant.
        if !Matrix4::equals(&Matrix4::IDENTITY, &self.transform) {
            return Cartesian3::magnitude(&self.position);
        }

        match self.scene_context.ray_intersection {
            Some(ray_intersection) => {
                // `distance = Math.min(depthDistance, rayDistance)` with
                // `depthDistance === Number.POSITIVE_INFINITY`.
                Cartesian3::distance(&ray_intersection, &self.position_wc)
            }
            None => self.position_cartographic.height.max(0.0),
        }
    }

    /// `Camera#_adjustOrthographicFrustum(zooming)`.
    ///
    /// Re-derives an [`OrthographicFrustum`]'s `width` from the distance to
    /// whatever sits under the screen centre, so that zooming with an
    /// orthographic projection in 3D / Columbus view scales the view instead of
    /// moving it. The off-center variants are skipped, which is what keeps 2D
    /// (where [`Camera::zoom_2d`] rescales the bounds directly) untouched.
    ///
    /// `zooming` distinguishes the two call sites: rotations only re-derive the
    /// width when the camera is far enough out (`height >= 150000.0`), because a
    /// close-in orbit keeps its framing; translations always do.
    pub fn adjust_orthographic_frustum(&mut self, zooming: bool) {
        if !self.frustum.is_orthographic() {
            return;
        }

        if !zooming && self.position_cartographic.height < 150000.0 {
            return;
        }

        let width = self.calculate_orthographic_frustum_width();
        self.frustum.set_width(width);
    }

    /// `Camera#switchToPerspectiveFrustum`.
    ///
    /// Switches the frustum/projection to perspective. This is a no-op in 2D,
    /// which must always be orthographic.
    ///
    /// Installs a *fresh* `PerspectiveFrustum` — `near`/`far` revert to the core
    /// defaults, exactly as the JS `this.frustum = new PerspectiveFrustum()`
    /// does.
    pub fn switch_to_perspective_frustum(&mut self) {
        if self.mode == SceneMode::Scene2D || self.frustum.is_perspective() {
            return;
        }

        let mut frustum = PerspectiveFrustum::new();
        frustum.aspect_ratio = Some(self.canvas_width as f64 / self.canvas_height as f64);
        frustum.fov = Some(CesiumMath::to_radians(60.0));
        self.frustum = CameraFrustum::Perspective(frustum);
    }

    /// `Camera#switchToOrthographicFrustum`.
    ///
    /// Switches the frustum/projection to orthographic. This is a no-op in 2D,
    /// which will always be orthographic.
    pub fn switch_to_orthographic_frustum(&mut self) {
        if self.mode == SceneMode::Scene2D || self.frustum.is_orthographic() {
            return;
        }

        // This must be called before changing the frustum because it uses the previous
        // frustum to reconstruct the world space position from the depth buffer.
        let frustum_width = self.calculate_orthographic_frustum_width();

        let mut frustum = OrthographicFrustum::new();
        frustum.aspect_ratio = Some(self.canvas_width as f64 / self.canvas_height as f64);
        frustum.width = Some(frustum_width);
        self.frustum = CameraFrustum::Orthographic(frustum);
    }

    // ---- Projection parameters (façade over `this.frustum`) ----

    /// `frustum.fov`, or `toRadians(60)` when the variant has no such property.
    pub fn fov(&self) -> f64 {
        self.frustum.fov().unwrap_or(CesiumMath::to_radians(60.0))
    }

    /// Sets `frustum.fov`; a no-op unless the frustum is perspective.
    pub fn set_fov(&mut self, fov: f64) {
        self.frustum.set_fov(fov);
    }

    /// `frustum.near`.
    pub fn near(&self) -> f64 { self.frustum.near() }

    /// Sets `frustum.near`.
    pub fn set_near(&mut self, near: f64) {
        self.frustum.set_near(near);
    }

    /// `frustum.far`.
    pub fn far(&self) -> f64 { self.frustum.far() }

    /// Sets `frustum.far`.
    pub fn set_far(&mut self, far: f64) {
        self.frustum.set_far(far);
    }

    /// `frustum.aspectRatio`, or `1.0` when the variant has no such property.
    pub fn aspect_ratio(&self) -> f64 {
        self.frustum.aspect_ratio().unwrap_or(1.0)
    }

    /// Sets `frustum.aspectRatio`.
    pub fn set_aspect_ratio(&mut self, ratio: f64) {
        self.frustum.set_aspect_ratio(ratio);
    }

    /// `frustum.width`, or `1.0` when the frustum is not orthographic.
    pub fn orthographic_width(&self) -> f64 {
        self.frustum.width().unwrap_or(1.0)
    }

    /// Sets `frustum.width`; a no-op unless the frustum is an
    /// [`OrthographicFrustum`] (CesiumJS `Camera#_adjustOrthographicFrustum`
    /// early-returns for the off-center variant too).
    pub fn set_orthographic_width(&mut self, width: f64) {
        self.frustum.set_width(width);
    }

    /// The vertical field of view in radians.
    ///
    /// Mirrors the `_fovy` derived by CesiumJS `PerspectiveFrustum#update`. The
    /// `fov` property denotes the *horizontal* FOV when `aspectRatio > 1` and
    /// the vertical FOV otherwise, so the vertical angle must be recovered:
    /// `aspectRatio <= 1 ? fov : atan(tan(fov * 0.5) / aspectRatio) * 2.0`.
    ///
    /// DEVIATION: CesiumJS `frustum.fovy` is `undefined` for the three
    /// non-perspective variants; this façade keeps returning a number for its
    /// pre-existing call sites by applying the same formula. Use
    /// [`Camera::frustum_mut`] for the faithful `Option`.
    pub fn fovy(&self) -> f64 {
        self.frustum.fovy().unwrap_or_else(|| {
            PerspectiveFrustum::fovy_of(self.fov(), self.aspect_ratio())
        })
    }

    /// Returns the SSE denominator for perspective LOD selection.
    ///
    /// Mirrors CesiumJS `PerspectiveFrustum#sseDenominator`:
    /// `2 * tan(0.5 * fovy)`. Used by the quadtree screen-space error formula
    /// `sse = geometricError * drawingBufferHeight / (distance * sseDenominator)`.
    ///
    /// The *vertical* FOV is the correct input; feeding `fov` directly inflates
    /// the denominator by exactly `aspectRatio` on widescreen canvases and makes
    /// every tile's screen-space error that many times too small.
    ///
    /// DEVIATION: `frustum.sseDenominator` is `undefined` for the non-perspective
    /// variants and their consumers take a different code path in 2D / Columbus
    /// View; `1.0` is the neutral stand-in the port has always used.
    pub fn sse_denominator(&self) -> f64 {
        self.frustum.sse_denominator().unwrap_or(1.0)
    }

    // ---- Tunables ----

    /// Returns the axis the camera is locked to, if any.
    ///
    /// Mirrors CesiumJS `Camera#constrainedAxis` (initially `undefined`).
    pub fn constrained_axis(&self) -> Option<Cartesian3> { self.constrained_axis }

    /// Sets the axis the camera is locked to.
    ///
    /// CesiumJS assigns `Cartesian3.UNIT_Z` for the 3D and Columbus View modes.
    /// The axis selects the orbit axis of `rotate_left`/`rotate_right` and drives
    /// the pole clamping inside `rotate_up`/`rotate_down`.
    pub fn set_constrained_axis(&mut self, axis: Option<Cartesian3>) {
        self.constrained_axis = axis;
    }

    /// `Camera#defaultMoveAmount`.
    pub fn default_move_amount(&self) -> f64 { self.default_move_amount }

    /// Sets `Camera#defaultMoveAmount`.
    pub fn set_default_move_amount(&mut self, amount: f64) {
        self.default_move_amount = amount;
    }

    /// Returns the default look amount in radians.
    ///
    /// Mirrors CesiumJS `Camera#defaultLookAmount` (`Math.PI / 60.0`).
    pub fn default_look_amount(&self) -> f64 { self.default_look_amount }

    /// Sets the default look amount in radians.
    pub fn set_default_look_amount(&mut self, amount: f64) {
        self.default_look_amount = amount;
    }

    /// Returns the default rotate amount in radians.
    ///
    /// Mirrors CesiumJS `Camera#defaultRotateAmount` (`Math.PI / 3600.0`), the
    /// value CesiumJS falls back to when `rotate` is called without an angle.
    pub fn default_rotate_amount(&self) -> f64 { self.default_rotate_amount }

    /// Sets the default rotate amount in radians.
    pub fn set_default_rotate_amount(&mut self, amount: f64) {
        self.default_rotate_amount = amount;
    }

    /// `Camera#defaultZoomAmount`.
    pub fn default_zoom_amount(&self) -> f64 { self.default_zoom_amount }

    /// Sets `Camera#defaultZoomAmount`.
    pub fn set_default_zoom_amount(&mut self, amount: f64) {
        self.default_zoom_amount = amount;
    }

    /// `Camera#maximumZoomFactor`.
    pub fn maximum_zoom_factor(&self) -> f64 { self.maximum_zoom_factor }

    /// Sets `Camera#maximumZoomFactor`.
    pub fn set_maximum_zoom_factor(&mut self, factor: f64) {
        self.maximum_zoom_factor = factor;
    }

    /// `Camera#maximumZoomDistance`.
    pub fn maximum_zoom_distance(&self) -> f64 { self.maximum_zoom_distance }

    /// Sets `Camera#maximumZoomDistance`.
    pub fn set_maximum_zoom_distance(&mut self, distance: f64) {
        self.maximum_zoom_distance = distance;
    }

    /// `Camera#minimumZoomDistance`.
    pub fn minimum_zoom_distance(&self) -> f64 { self.minimum_zoom_distance }

    /// Sets `Camera#minimumZoomDistance`.
    pub fn set_minimum_zoom_distance(&mut self, distance: f64) {
        self.minimum_zoom_distance = distance;
    }

    /// `Camera#percentageChanged`.
    pub fn percentage_changed(&self) -> f64 { self.percentage_changed }

    /// Sets `Camera#percentageChanged`.
    pub fn set_percentage_changed(&mut self, percentage: f64) {
        self.percentage_changed = percentage;
    }

    // ---- Events and the state behind them ----

    /// `Camera#moveStart`.
    pub fn move_start(&self) -> &Event<()> { &self.move_start }

    /// `Camera#moveEnd`.
    pub fn move_end(&self) -> &Event<()> { &self.move_end }

    /// `Camera#changed`.
    pub fn changed(&self) -> &Event<f64> { &self.changed_event }

    /// `Camera#positionWCDeltaMagnitude`.
    pub fn position_wc_delta_magnitude(&self) -> f64 { self.position_wc_delta_magnitude }

    /// `Camera#positionWCDeltaMagnitudeLastFrame`.
    pub fn position_wc_delta_magnitude_last_frame(&self) -> f64 {
        self.position_wc_delta_magnitude_last_frame
    }

    /// `Camera#timeSinceMoved`, in seconds.
    pub fn time_since_moved(&self) -> f64 { self.time_since_moved }

    /// `Camera#canPreloadFlight`.
    ///
    /// CesiumJS tests `defined(this._currentFlight)`; the port's flight lives in
    /// the shared channel installed by the scene.
    pub fn can_preload_flight(&self) -> bool {
        let in_flight = self
            .flight_channel
            .as_ref()
            .is_some_and(|channel| channel.borrow().is_some());
        in_flight && self.mode != SceneMode::Scene2D
    }

    // ---- Heading / pitch / roll ----

    /// `Camera#heading` — `undefined` (i.e. `None`) while morphing.
    ///
    /// CesiumJS temporarily re-bases the camera onto the east-north-up frame at
    /// `positionWC`, reads the angle, and restores the previous transform.
    pub fn heading(&mut self) -> Option<f64> {
        if self.mode == SceneMode::Morphing {
            return None;
        }

        let old_transform = self.transform;
        let transform = self.east_north_up_at_position_wc();
        self.set_transform(transform);

        let heading = get_heading(&self.direction, &self.up);

        self.set_transform(old_transform);

        Some(heading)
    }

    /// `Camera#pitch` — `undefined` (i.e. `None`) while morphing.
    pub fn pitch(&mut self) -> Option<f64> {
        if self.mode == SceneMode::Morphing {
            return None;
        }

        let old_transform = self.transform;
        let transform = self.east_north_up_at_position_wc();
        self.set_transform(transform);

        let pitch = get_pitch(&self.direction);

        self.set_transform(old_transform);

        Some(pitch)
    }

    /// `Camera#roll` — `undefined` (i.e. `None`) while morphing.
    pub fn roll(&mut self) -> Option<f64> {
        if self.mode == SceneMode::Morphing {
            return None;
        }

        let old_transform = self.transform;
        let transform = self.east_north_up_at_position_wc();
        self.set_transform(transform);

        let roll = get_roll(&self.direction, &self.up, &self.right);

        self.set_transform(old_transform);

        Some(roll)
    }

    /// `Transforms.eastNorthUpToFixedFrame(this.positionWC, this._projection.ellipsoid)`.
    ///
    /// The CesiumJS `positionWC` getter runs `updateMembers` first, so this does
    /// too.
    fn east_north_up_at_position_wc(&mut self) -> Matrix4 {
        self.update_members();
        let position_wc = self.position_wc;
        let ellipsoid = self.map_projection.ellipsoid().clone();
        transforms::east_north_up_to_fixed_frame_new(&position_wc, Some(&ellipsoid))
    }
}

impl Camera {
    // ---- Movement ----

    /// `Camera#move` — translates the camera's position by `amount` along
    /// `direction`, in the reference frame of [`Camera::transform`].
    ///
    /// In 2D the position is clamped back inside the map afterwards, which is
    /// what makes [`MapMode2D::InfiniteScroll`] wrap around instead of letting
    /// the camera leave the projected plane. Translating always re-derives an
    /// [`OrthographicFrustum`]'s width, since with an orthographic projection the
    /// distance to the ground *is* the framing.
    pub fn move_camera(&mut self, direction: &Cartesian3, amount: f64) {
        let move_scratch = Cartesian3::multiply_by_scalar_new(direction, amount);
        self.position = Cartesian3::add_new(&self.position, &move_scratch);

        if self.mode == SceneMode::Scene2D {
            let rotatable_2d = self.map_mode_2d == MapMode2D::Rotate;
            let max_coord = self.max_coord;
            clamp_move_2d(rotatable_2d, &max_coord, &mut self.position);
        }
        self.adjust_orthographic_frustum(true);
    }

    /// `Camera#moveForward`.
    ///
    /// In 2D this zooms instead of translating: the camera's `position.z` is the
    /// fixed map height there and the apparent distance is carried by the
    /// orthographic frustum's width (see [`Camera::zoom_2d`]).
    pub fn move_forward(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        if self.mode == SceneMode::Scene2D {
            self.zoom_2d(amount);
        } else {
            let direction = self.direction;
            self.move_camera(&direction, amount);
        }
    }

    /// `Camera#moveBackward`.
    pub fn move_backward(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        if self.mode == SceneMode::Scene2D {
            self.zoom_2d(-amount);
        } else {
            let direction = self.direction;
            self.move_camera(&direction, -amount);
        }
    }

    /// `Camera#moveUp`.
    pub fn move_up(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        let up = self.up;
        self.move_camera(&up, amount);
    }

    /// `Camera#moveDown`.
    pub fn move_down(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        let up = self.up;
        self.move_camera(&up, -amount);
    }

    /// `Camera#moveRight`.
    pub fn move_right(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        let right = self.right;
        self.move_camera(&right, amount);
    }

    /// `Camera#moveLeft`.
    pub fn move_left(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        let right = self.right;
        self.move_camera(&right, -amount);
    }

    // ---- Look ----

    /// `Camera#lookLeft`.
    ///
    /// A no-op in 2D — the JS comment reads "only want view of map to change in
    /// 3D mode, 2D visual is incorrect when look changes".
    pub fn look_left(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_look_amount);
        if self.mode != SceneMode::Scene2D {
            let up = self.up;
            self.look(&up, Some(-amount));
        }
    }

    /// `Camera#lookRight`.
    pub fn look_right(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_look_amount);
        if self.mode != SceneMode::Scene2D {
            let up = self.up;
            self.look(&up, Some(amount));
        }
    }

    /// `Camera#lookUp`.
    pub fn look_up(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_look_amount);
        if self.mode != SceneMode::Scene2D {
            let right = self.right;
            self.look(&right, Some(-amount));
        }
    }

    /// `Camera#lookDown`.
    pub fn look_down(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_look_amount);
        if self.mode != SceneMode::Scene2D {
            let right = self.right;
            self.look(&right, Some(amount));
        }
    }

    /// `Camera#look` — rotates each of the camera's orientation vectors around
    /// `axis` by `angle`, leaving `position` untouched.
    ///
    /// Ported 1:1: the `-turnAngle` quaternion is applied to
    /// `direction`/`up`/`right` directly, with no cross-product re-derivation, so
    /// the three stay exactly as rotated — contrast [`Camera::rotate`], which
    /// rebuilds `right` and `up` from cross products.
    pub fn look(&mut self, axis: &Cartesian3, angle: Option<f64>) {
        let turn_angle = angle.unwrap_or(self.default_look_amount);
        let quaternion = Quaternion::from_axis_angle_new(axis, -turn_angle);
        let rotation = Matrix3::from_quaternion_new(&quaternion);
        self.direction = Matrix3::multiply_by_vector_new(&rotation, &self.direction);
        self.up = Matrix3::multiply_by_vector_new(&rotation, &self.up);
        self.right = Matrix3::multiply_by_vector_new(&rotation, &self.right);
    }

    /// `Camera#twistLeft` — rotates the camera counter-clockwise around its
    /// direction vector.
    pub fn twist_left(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_look_amount);
        let direction = self.direction;
        self.look(&direction, Some(amount));
    }

    /// `Camera#twistRight` — rotates the camera clockwise around its direction
    /// vector.
    pub fn twist_right(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_look_amount);
        let direction = self.direction;
        self.look(&direction, Some(-amount));
    }

    // ---- Rotation ----

    /// `Camera#rotate` — rotates the camera around `axis` by `angle`. The
    /// distance of the camera's position to the centre of its reference frame
    /// stays the same, which is what turns the motion into an orbit.
    ///
    /// Ported 1:1: the axis-angle quaternion is built from `-turnAngle`, the
    /// rotation is applied to `position` as well as to the orientation vectors,
    /// and `right`/`up` are re-derived from cross products rather than rotated
    /// directly so the frame stays orthonormal. `axis` is not normalised here —
    /// CesiumJS documents a unit axis but does not enforce one either.
    ///
    /// Rotating passes `zooming = false`, so an [`OrthographicFrustum`]'s width
    /// is only re-derived once the camera is more than 150 km up; a close-in
    /// orbit keeps the framing it started with.
    pub fn rotate(&mut self, axis: &Cartesian3, angle: Option<f64>) {
        let turn_angle = angle.unwrap_or(self.default_rotate_amount);
        let quaternion = Quaternion::from_axis_angle_new(axis, -turn_angle);
        let rotation = Matrix3::from_quaternion_new(&quaternion);
        self.position = Matrix3::multiply_by_vector_new(&rotation, &self.position);
        self.direction = Matrix3::multiply_by_vector_new(&rotation, &self.direction);
        self.up = Matrix3::multiply_by_vector_new(&rotation, &self.up);
        self.right = Cartesian3::cross_new(&self.direction, &self.up);
        self.up = Cartesian3::cross_new(&self.right, &self.direction);

        self.adjust_orthographic_frustum(false);
    }

    /// `Camera#rotateDown`.
    pub fn rotate_down(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_rotate_amount);
        self.rotate_vertical(angle);
    }

    /// `Camera#rotateUp`.
    pub fn rotate_up(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_rotate_amount);
        self.rotate_vertical(-angle);
    }

    /// `Camera#rotateRight`.
    pub fn rotate_right(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_rotate_amount);
        self.rotate_horizontal(-angle);
    }

    /// `Camera#rotateLeft`.
    pub fn rotate_left(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_rotate_amount);
        self.rotate_horizontal(angle);
    }

    /// The module-level `rotateVertical` helper of CesiumJS.
    ///
    /// With a constrained axis set (and the camera away from the frame origin)
    /// the rotation is re-based onto the tangent of the great circle pointing at
    /// that axis, and the angle is clamped to `angleToAxis - EPSILON4` so the
    /// camera stops just short of the pole instead of flipping over it. At either
    /// pole only the sign that moves away from it is honoured.
    fn rotate_vertical(&mut self, mut angle: f64) {
        let away_from_origin = !Cartesian3::equals_epsilon(
            Some(&self.position),
            Some(&Cartesian3::ZERO),
            Some(CesiumMath::EPSILON2),
            None,
        );
        let constrained_axis = if away_from_origin {
            self.constrained_axis
        } else {
            None
        };

        let constrained_axis = match constrained_axis {
            Some(axis) => axis,
            None => {
                let right = self.right;
                self.rotate(&right, Some(angle));
                return;
            }
        };

        let p = Cartesian3::normalize_new(&self.position);
        let north_parallel = Cartesian3::equals_epsilon(
            Some(&p),
            Some(&constrained_axis),
            Some(CesiumMath::EPSILON2),
            None,
        );
        let south_parallel = Cartesian3::equals_epsilon(
            Some(&p),
            Some(&Cartesian3::negate_new(&constrained_axis)),
            Some(CesiumMath::EPSILON2),
            None,
        );

        if !north_parallel && !south_parallel {
            let axis = Cartesian3::normalize_new(&constrained_axis);

            let dot = Cartesian3::dot(&p, &axis);
            let angle_to_axis = CesiumMath::acos_clamped(dot);
            if angle > 0.0 && angle > angle_to_axis {
                angle = angle_to_axis - CesiumMath::EPSILON4;
            }

            let dot = Cartesian3::dot(&p, &Cartesian3::negate_new(&axis));
            let angle_to_axis = CesiumMath::acos_clamped(dot);
            if angle < 0.0 && -angle > angle_to_axis {
                angle = -angle_to_axis + CesiumMath::EPSILON4;
            }

            let tangent = Cartesian3::cross_new(&axis, &p);
            self.rotate(&tangent, Some(angle));
        } else if (north_parallel && angle < 0.0) || (south_parallel && angle > 0.0) {
            let right = self.right;
            self.rotate(&right, Some(angle));
        }
    }

    /// The module-level `rotateHorizontal` helper of CesiumJS: orbits the
    /// constrained axis when one is set, otherwise the camera's own up vector.
    fn rotate_horizontal(&mut self, angle: f64) {
        match self.constrained_axis {
            Some(axis) => self.rotate(&axis, Some(angle)),
            None => {
                let up = self.up;
                self.rotate(&up, Some(angle));
            }
        }
    }

    // ---- Zoom ----

    /// The module-level `zoom2D` helper of CesiumJS.
    ///
    /// Rescales the off-center orthographic frustum while preserving its aspect
    /// ratio, clamped so the view never grows past the whole map — times
    /// `maximumZoomFactor` when 2D rotation is allowed, since a rotated map has a
    /// longer diagonal. The `top`/`bottom` branch is taken when the frustum is
    /// taller than it is wide, so a very narrow canvas zooms on its height
    /// instead of its width. When the clamp would invert the extents the frustum
    /// snaps to `[-1, 1]`, the tightest view CesiumJS allows.
    fn zoom_2d(&mut self, amount: f64) {
        debug_assert!(
            self.frustum.is_orthographic_off_center(),
            "The camera frustum is expected to be orthographic for 2D camera control."
        );

        let (left, right, top, bottom) = self.frustum.bounds();
        let amount = amount * 0.5;
        let rotatable_2d = self.map_mode_2d == MapMode2D::Rotate;
        let max_coord = self.max_coord;
        let maximum_zoom_factor = self.maximum_zoom_factor;

        let (left, right, top, bottom) = if top.abs() + bottom.abs() > left.abs() + right.abs() {
            let mut new_top = top - amount;
            let mut new_bottom = bottom + amount;

            let mut max_bottom = max_coord.y;
            if rotatable_2d {
                max_bottom *= maximum_zoom_factor;
            }

            if new_bottom > max_bottom {
                new_bottom = max_bottom;
                new_top = -max_bottom;
            }

            if new_top <= new_bottom {
                new_top = 1.0;
                new_bottom = -1.0;
            }

            let ratio = right / top;
            let new_right = new_top * ratio;
            (-new_right, new_right, new_top, new_bottom)
        } else {
            let mut new_right = right - amount;
            let mut new_left = left + amount;

            let mut max_right = max_coord.x;
            if rotatable_2d {
                max_right *= maximum_zoom_factor;
            }

            if new_right > max_right {
                new_right = max_right;
                new_left = -max_right;
            }

            if new_right <= new_left {
                new_right = 1.0;
                new_left = -1.0;
            }

            let ratio = top / right;
            let new_top = new_right * ratio;
            (new_left, new_right, new_top, -new_top)
        };

        self.frustum.set_bounds(left, right, top, bottom);
    }

    /// The module-level `zoom3D` helper of CesiumJS: zooming in 3D and Columbus
    /// view is simply moving along the view vector.
    fn zoom_3d(&mut self, amount: f64) {
        let direction = self.direction;
        self.move_camera(&direction, amount);
    }

    /// `Camera#zoomIn` — zooms `amount` along the camera's view vector.
    pub fn zoom_in(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_zoom_amount);
        if self.mode == SceneMode::Scene2D {
            self.zoom_2d(amount);
        } else {
            self.zoom_3d(amount);
        }
    }

    /// `Camera#zoomOut` — zooms `amount` along the opposite of the camera's view
    /// vector.
    pub fn zoom_out(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_zoom_amount);
        if self.mode == SceneMode::Scene2D {
            self.zoom_2d(-amount);
        } else {
            self.zoom_3d(-amount);
        }
    }

    /// `Camera#getMagnitude`.
    ///
    /// In 3D this is the vector magnitude of the position; in Columbus view the
    /// distance to the map; in 2D the larger frustum extent. CesiumJS returns
    /// `undefined` while morphing, hence the `Option`.
    pub fn get_magnitude(&mut self) -> Option<f64> {
        match self.mode {
            SceneMode::Scene3D => Some(Cartesian3::magnitude(&self.position)),
            SceneMode::ColumbusView => Some(self.position.z.abs()),
            // `frustum.right - frustum.left` / `top - bottom` read the off-center
            // extents of whatever frustum is installed; the JS getters refresh
            // them first, which needs `&mut self` in the port.
            SceneMode::Scene2D => {
                let (left, right, top, bottom) = self.frustum.bounds();
                Some((right - left).max(top - bottom))
            }
            SceneMode::Morphing => None,
        }
    }
}

impl Camera {
    // ---- Coordinate transforms ----
    //
    // CesiumJS' six `*Coordinates*` methods all work in the camera's *reference
    // frame*: they multiply by `_actualInvTransform` / `_actualTransform` —
    // `_transform` with the Columbus View / 2D projection folded in — and not by
    // the view matrix, which additionally carries the camera's own orientation.
    // Each JS method also runs `updateMembers` first; see [`Camera::refresh`] for
    // why the port reads the cached transforms instead.

    /// `Camera#worldToCameraCoordinates`.
    pub fn world_to_camera_coordinates(&self, cartesian: &Cartesian4) -> Cartesian4 {
        Matrix4::multiply_by_vector_new(&self.actual_inverse_transform, cartesian)
    }

    /// `Camera#worldToCameraCoordinatesPoint`.
    pub fn world_to_camera_coordinates_point(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Matrix4::multiply_by_point_new(&self.actual_inverse_transform, cartesian)
    }

    /// `Camera#worldToCameraCoordinatesVector`.
    pub fn world_to_camera_coordinates_vector(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Matrix4::multiply_by_point_as_vector_new(&self.actual_inverse_transform, cartesian)
    }

    /// `Camera#cameraToWorldCoordinates`.
    pub fn camera_to_world_coordinates(&self, cartesian: &Cartesian4) -> Cartesian4 {
        Matrix4::multiply_by_vector_new(&self.actual_transform, cartesian)
    }

    /// `Camera#cameraToWorldCoordinatesPoint`.
    pub fn camera_to_world_coordinates_point(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Matrix4::multiply_by_point_new(&self.actual_transform, cartesian)
    }

    /// `Camera#cameraToWorldCoordinatesVector`.
    pub fn camera_to_world_coordinates_vector(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Matrix4::multiply_by_point_as_vector_new(&self.actual_transform, cartesian)
    }

    // ---- Picking ----

    /// `Camera#getPickRay` — creates a ray from the camera position through the
    /// pixel at `window_position`, in world coordinates.
    ///
    /// Returns `None` when the canvas has no area, exactly as CesiumJS returns
    /// `undefined`.
    pub fn get_pick_ray(&mut self, window_position: &Cartesian2) -> Option<Ray> {
        if self.canvas_width == 0 || self.canvas_height == 0 {
            return None;
        }

        // JS dispatches on
        // `defined(frustum.aspectRatio) && defined(frustum.fov) && defined(frustum.near)`;
        // `near` is a plain field on all four variants, so only the first two
        // discriminate. In practice that selects `PerspectiveFrustum`.
        let perspective = self.frustum.aspect_ratio().is_some() && self.frustum.fov().is_some();
        if perspective {
            Some(self.get_pick_ray_perspective(window_position))
        } else {
            Some(self.get_pick_ray_orthographic(window_position))
        }
    }

    /// The module-level `getPickRayPerspective` helper of CesiumJS.
    ///
    /// Works off the *vertical* FOV (`tanPhi = tan(fovy * 0.5)`,
    /// `tanTheta = aspectRatio * tanPhi`) and off the world-space pose, because
    /// the ray it returns is in world coordinates. Every offset is scaled by
    /// `near` and the result is re-based on `positionWC`, which is what puts the
    /// ray's foot on the near plane rather than at the camera.
    fn get_pick_ray_perspective(&mut self, window_position: &Cartesian2) -> Ray {
        let width = self.canvas_width as f64;
        let height = self.canvas_height as f64;

        // `get_pick_ray` only routes here when `aspectRatio` and `fov` are
        // defined, i.e. for the `Perspective` variant, so both are `Some`.
        let fovy = self.frustum.fovy().unwrap_or(f64::NAN);
        let aspect_ratio = self.frustum.aspect_ratio().unwrap_or(f64::NAN);
        let near = self.frustum.near();

        let tan_phi = (fovy * 0.5).tan();
        let tan_theta = aspect_ratio * tan_phi;

        let x = (2.0 / width) * window_position.x - 1.0;
        let y = (2.0 / height) * (height - window_position.y) - 1.0;

        // `positionWC` / `directionWC` / `rightWC` / `upWC` — their JS getters run
        // `updateMembers` first.
        self.update_members();
        let position = self.position_wc;
        let near_center = Cartesian3::multiply_by_scalar_new(&self.direction_wc, near);
        let near_center = Cartesian3::add_new(&position, &near_center);
        let x_dir = Cartesian3::multiply_by_scalar_new(&self.right_wc, x * near * tan_theta);
        let y_dir = Cartesian3::multiply_by_scalar_new(&self.up_wc, y * near * tan_phi);

        let mut direction = Cartesian3::add_new(&near_center, &x_dir);
        direction = Cartesian3::add_new(&direction, &y_dir);
        direction = Cartesian3::subtract_new(&direction, &position);
        direction = Cartesian3::normalize_new(&direction);

        Ray::new(Some(&position), Some(&direction))
    }

    /// The module-level `getPickRayOrthographic` helper of CesiumJS.
    ///
    /// The ray keeps the camera direction while its origin is shifted inside the
    /// camera plane by the off-center frustum extents; `frustum.offCenterFrustum`
    /// is used when defined, which for an `OrthographicFrustum` means the extents
    /// derived from `width` / `aspectRatio`.
    fn get_pick_ray_orthographic(&mut self, window_position: &Cartesian2) -> Ray {
        let width = self.canvas_width as f64;
        let height = self.canvas_height as f64;

        let (left, right, top, bottom) = self.frustum.bounds();
        let mut x = (2.0 / width) * window_position.x - 1.0;
        x *= (right - left) * 0.5;
        let mut y = (2.0 / height) * (height - window_position.y) - 1.0;
        y *= (top - bottom) * 0.5;

        self.update_members();
        let mut origin = self.position_wc;
        let right_offset = Cartesian3::multiply_by_scalar_new(&self.right_wc, x);
        origin = Cartesian3::add_new(&right_offset, &origin);
        let up_offset = Cartesian3::multiply_by_scalar_new(&self.up_wc, y);
        origin = Cartesian3::add_new(&up_offset, &origin);

        let direction = self.direction_wc;

        // Account for wrap-around in 2D infinite scroll mode.
        if self.mode == SceneMode::Scene2D && self.map_mode_2d == MapMode2D::InfiniteScroll {
            let max_horizontal = self.max_coord.x;
            origin.y = CesiumMath::r#mod(origin.y + max_horizontal, 2.0 * max_horizontal)
                - max_horizontal;
        }

        Ray::new(Some(&origin), Some(&direction))
    }

    /// `Camera#pickEllipsoid` — picks the ellipsoid or the map at the given
    /// window position, returning the surface point in world coordinates.
    ///
    /// Dispatches on the scene mode exactly as CesiumJS does: a ray/ellipsoid
    /// intersection in 3D, and an unprojection of the ray origin (2D) or of the
    /// ray's crossing of the map plane (Columbus view) in the flattened modes.
    /// `None` while morphing, when the canvas has no area, or when the pick falls
    /// outside the projected map.
    pub fn pick_ellipsoid(
        &mut self,
        window_position: &Cartesian2,
        ellipsoid: Option<&Ellipsoid>,
    ) -> Option<Cartesian3> {
        if self.canvas_width == 0 || self.canvas_height == 0 {
            return None;
        }

        match self.mode {
            SceneMode::Scene3D => self.pick_ellipsoid_3d(window_position, ellipsoid),
            SceneMode::Scene2D => self.pick_map_2d(window_position),
            SceneMode::ColumbusView => self.pick_map_columbus_view(window_position),
            SceneMode::Morphing => None,
        }
    }

    /// The module-level `pickEllipsoid3D` helper of CesiumJS.
    ///
    /// `t` falls back to `intersection.stop` when `start` is not ahead of the
    /// camera, which is the case when the camera is inside the ellipsoid.
    fn pick_ellipsoid_3d(
        &mut self,
        window_position: &Cartesian2,
        ellipsoid: Option<&Ellipsoid>,
    ) -> Option<Cartesian3> {
        let ellipsoid = ellipsoid.cloned().unwrap_or(Ellipsoid::WGS84);
        let ray = self.get_pick_ray(window_position)?;
        let intersection = IntersectionTests::ray_ellipsoid(&ray, &ellipsoid)?;
        let t = if intersection.start > 0.0 {
            intersection.start
        } else {
            intersection.stop
        };
        Some(Ray::get_point_new(&ray, Some(t)))
    }

    /// The module-level `pickMap2D` helper of CesiumJS.
    ///
    /// In 2D the projected frame is swizzled to `(z, x, y)`, so the ray origin's
    /// `y`/`z` hold the map's `x`/`y` and can be unprojected directly — the ray's
    /// direction is irrelevant because the map is a plane at constant height.
    fn pick_map_2d(&mut self, window_position: &Cartesian2) -> Option<Cartesian3> {
        let ray = self.get_pick_ray(window_position)?;
        let position = ray.origin;
        let position = Cartesian3::from_elements_new(position.y, position.z, 0.0);
        let cart = self.map_projection.unproject(&position);

        if cart.latitude < -CesiumMath::PI_OVER_TWO || cart.latitude > CesiumMath::PI_OVER_TWO {
            return None;
        }

        let mut result = Cartesian3::default();
        self.map_projection
            .ellipsoid()
            .cartographic_to_cartesian(&cart, &mut result);
        Some(result)
    }

    /// The module-level `pickMapColumbusView` helper of CesiumJS.
    ///
    /// Walks the ray to its crossing of the `x = 0` plane — the map plane in
    /// Columbus view — and unprojects the `y`/`z` of that point.
    fn pick_map_columbus_view(&mut self, window_position: &Cartesian2) -> Option<Cartesian3> {
        let ray = self.get_pick_ray(window_position)?;
        let scalar = -ray.origin.x / ray.direction.x;
        let hit = Ray::get_point_new(&ray, Some(scalar));

        let cart = self
            .map_projection
            .unproject(&Cartesian3::new(hit.y, hit.z, 0.0));

        if cart.latitude < -CesiumMath::PI_OVER_TWO
            || cart.latitude > CesiumMath::PI_OVER_TWO
            || cart.longitude < -std::f64::consts::PI
            || cart.longitude > std::f64::consts::PI
        {
            return None;
        }

        let mut result = Cartesian3::default();
        self.map_projection
            .ellipsoid()
            .cartographic_to_cartesian(&cart, &mut result);
        Some(result)
    }
}

impl Camera {
    // ---- View setup ----

    /// Sets the camera view.
    ///
    /// Port-facing wrapper over [`Camera::set_view_with_options`] covering the
    /// `Cartesian3` destination plus optional `direction`/`up` orientation, the
    /// shape the earlier port exposed.
    ///
    /// DEVIATION: `ellipsoid` is accepted for source compatibility but ignored
    /// — CesiumJS reads `camera._projection.ellipsoid`, which the port owns
    /// through [`Camera::map_projection`]. Tracked in `docs/deviations.md`.
    pub fn set_view(
        &mut self,
        destination: &Cartesian3,
        direction: Option<&Cartesian3>,
        up: Option<&Cartesian3>,
        _ellipsoid: &Ellipsoid,
    ) {
        let options = SetViewOptions {
            destination: Some(SetViewDestination::Cartesian(*destination)),
            orientation: SetViewOrientation {
                direction: direction.copied(),
                up: up.copied(),
                heading: None,
                pitch: None,
                roll: None,
            },
            end_transform: None,
            convert: true,
        };
        self.set_view_with_options(&options);
    }

    /// `Camera.prototype.setView(options)`.
    ///
    /// Returns early while morphing, installs `options.endTransform`, resolves a
    /// [`SetViewDestination::Rectangle`] through
    /// [`Camera::get_rectangle_camera_coordinates`] (which also forces
    /// `convert = false`), folds a `direction`/`up` pair into heading/pitch/roll
    /// with [`Camera::direction_up_to_heading_pitch_roll`], and finally
    /// dispatches on the scene mode to [`Camera::set_view_3d`] /
    /// [`Camera::set_view_cv`] / [`Camera::set_view_2d`].
    ///
    /// DEVIATION: CesiumJS selects the direction/up path on
    /// `defined(orientation.direction)` alone and then throws from inside
    /// `Cartesian3.clone` when `up` is missing; the port treats a missing `up`
    /// as "no direction/up pair" and falls through to heading/pitch/roll.
    /// Tracked in `docs/deviations.md`.
    pub fn set_view_with_options(&mut self, options: &SetViewOptions) {
        let mode = self.mode;
        if mode == SceneMode::Morphing {
            return;
        }

        if let Some(end_transform) = options.end_transform {
            self.set_transform(end_transform);
        }

        let mut convert = options.convert;
        let destination = match options.destination {
            Some(SetViewDestination::Cartesian(cartesian)) => cartesian,
            Some(SetViewDestination::Rectangle(rectangle)) => {
                convert = false;
                match self.get_rectangle_camera_coordinates(rectangle) {
                    Some(destination) => {
                        // >>includeStart('debug', pragmas.debug)
                        if cfg!(debug_assertions)
                            && (destination.x.is_nan() || destination.y.is_nan())
                        {
                            throw_developer_error("destination has a NaN component");
                        }
                        // >>includeEnd('debug');
                        destination
                    }
                    // `getRectangleCameraCoordinates` returns `undefined` while
                    // morphing; unreachable behind the guard above but kept
                    // total here.
                    None => return,
                }
            }
            None => {
                // `Cartesian3.clone(this.positionWC)` — the getter runs
                // `updateMembers` first.
                self.update_members();
                self.position_wc
            }
        };

        let hpr = match (options.orientation.direction, options.orientation.up) {
            (Some(direction), Some(up)) => {
                self.direction_up_to_heading_pitch_roll(&destination, direction, up)
            }
            _ => HeadingPitchRoll {
                heading: options.orientation.heading.unwrap_or(0.0),
                pitch: options.orientation.pitch.unwrap_or(-CesiumMath::PI_OVER_TWO),
                roll: options.orientation.roll.unwrap_or(0.0),
            },
        };

        match mode {
            SceneMode::Scene3D => self.set_view_3d(destination, hpr),
            SceneMode::Scene2D => self.set_view_2d(destination, hpr, convert),
            _ => self.set_view_cv(destination, hpr, convert),
        }
    }

    /// `setView3D(camera, position, hpr)`.
    ///
    /// The destination becomes the origin of a temporary east-north-up frame, so
    /// `position` drops to zero and the heading/pitch/roll — pre-rotated by a
    /// quarter turn so heading zero means north — are read straight off the
    /// rotation matrix columns. The previous reference frame is restored at the
    /// end, which re-expresses the pose in world coordinates.
    fn set_view_3d(&mut self, position: Cartesian3, mut hpr: HeadingPitchRoll) {
        // >>includeStart('debug', pragmas.debug)
        if cfg!(debug_assertions)
            && (position.x.is_nan() || position.y.is_nan() || position.z.is_nan())
        {
            throw_developer_error("position has a NaN component");
        }
        // >>includeEnd('debug');

        let current_transform = self.transform;
        let ellipsoid = self.map_projection.ellipsoid().clone();
        let local_transform =
            transforms::east_north_up_to_fixed_frame_new(&position, Some(&ellipsoid));
        self.set_transform(local_transform);
        self.position = Cartesian3::ZERO;

        hpr.heading -= CesiumMath::PI_OVER_TWO;
        let rot_quat = Quaternion::from_heading_pitch_roll_new(&hpr);
        let rot_mat = Matrix3::from_quaternion_new(&rot_quat);
        self.direction = Matrix3::get_column_new(&rot_mat, 0);
        self.up = Matrix3::get_column_new(&rot_mat, 2);
        self.right = Cartesian3::cross_new(&self.direction, &self.up);

        self.set_transform(current_transform);
        self.adjust_orthographic_frustum(true);
    }

    /// `setViewCV(camera, position, hpr, convert)`.
    ///
    /// Same heading/pitch/roll handling as [`Camera::set_view_3d`], but the
    /// destination is projected into the Columbus View plane instead of being
    /// made the origin of a local frame — so the reference frame is pinned to
    /// the identity while the pose is written.
    fn set_view_cv(&mut self, position: Cartesian3, mut hpr: HeadingPitchRoll, convert: bool) {
        let current_transform = self.transform;
        self.set_transform(Matrix4::IDENTITY);

        // `set_transform` finished with `updateMembers`, so `positionWC` is live.
        let position_wc = self.position_wc;
        if !Cartesian3::equals(Some(&position), Some(&position_wc)) {
            let position = project_destination(self, position, convert);
            self.position = position;
        }

        hpr.heading -= CesiumMath::PI_OVER_TWO;
        let rot_quat = Quaternion::from_heading_pitch_roll_new(&hpr);
        let rot_mat = Matrix3::from_quaternion_new(&rot_quat);
        self.direction = Matrix3::get_column_new(&rot_mat, 0);
        self.up = Matrix3::get_column_new(&rot_mat, 2);
        self.right = Cartesian3::cross_new(&self.direction, &self.up);

        self.set_transform(current_transform);
        self.adjust_orthographic_frustum(true);
    }

    /// `setView2D(camera, position, hpr, convert)`.
    ///
    /// Two 2D-only details are preserved. The destination is written with
    /// `Cartesian2.clone`, so `position.z` survives into the frustum-bounds
    /// derivation that immediately follows. And unlike the 3D and Columbus View
    /// branches this one does **not** finish with `_adjustOrthographicFrustum`;
    /// it restores the reference frame and returns.
    fn set_view_2d(&mut self, position: Cartesian3, mut hpr: HeadingPitchRoll, convert: bool) {
        let current_transform = self.transform;
        self.set_transform(Matrix4::IDENTITY);

        let position_wc = self.position_wc;
        if !Cartesian3::equals(Some(&position), Some(&position_wc)) {
            let position = project_destination(self, position, convert);
            // `Cartesian2.clone(position, camera.position)` writes x/y only.
            self.position.x = position.x;
            self.position.y = position.y;

            let new_left = -position.z * 0.5;
            let new_right = -new_left;
            if new_right > new_left {
                let (_, right, top, _) = self.frustum.bounds();
                let ratio = top / right;
                let new_top = new_right * ratio;
                self.frustum.set_bounds(new_left, new_right, new_top, -new_top);
            }
        }

        if self.map_mode_2d == MapMode2D::Rotate {
            hpr.heading -= CesiumMath::PI_OVER_TWO;
            hpr.pitch = -CesiumMath::PI_OVER_TWO;
            hpr.roll = 0.0;
            let rot_quat = Quaternion::from_heading_pitch_roll_new(&hpr);
            let rot_mat = Matrix3::from_quaternion_new(&rot_quat);
            self.up = Matrix3::get_column_new(&rot_mat, 2);
            self.right = Cartesian3::cross_new(&self.direction, &self.up);
        }

        self.set_transform(current_transform);
    }

    /// `directionUpToHeadingPitchRoll(camera, position, orientation, result)`.
    ///
    /// In 3D the world-space `direction`/`up` are pulled into the local
    /// east-north-up frame at `position` first, so the returned angles are
    /// relative to that frame; the other modes already work in projected
    /// coordinates and use the vectors verbatim.
    fn direction_up_to_heading_pitch_roll(
        &self,
        position: &Cartesian3,
        direction: Cartesian3,
        up: Cartesian3,
    ) -> HeadingPitchRoll {
        let mut direction = direction;
        let mut up = up;

        if self.mode == SceneMode::Scene3D {
            let ellipsoid = self.map_projection.ellipsoid().clone();
            let transform =
                transforms::east_north_up_to_fixed_frame_new(position, Some(&ellipsoid));
            let inv_transform = Matrix4::inverse_transformation_new(&transform);
            direction = Matrix4::multiply_by_point_as_vector_new(&inv_transform, &direction);
            up = Matrix4::multiply_by_point_as_vector_new(&inv_transform, &up);
        }

        let right = Cartesian3::cross_new(&direction, &up);
        HeadingPitchRoll {
            heading: get_heading(&direction, &up),
            pitch: get_pitch(&direction),
            roll: get_roll(&direction, &up, &right),
        }
    }

    /// `Camera.prototype.getRectangleCameraCoordinates(rectangle, result)`.
    ///
    /// The camera position from which `rectangle` fills the view, dispatched on
    /// the scene mode. `None` is the JS `undefined` returned while morphing.
    pub fn get_rectangle_camera_coordinates(&mut self, rectangle: Rectangle) -> Option<Cartesian3> {
        match self.mode {
            SceneMode::Scene3D => Some(self.rectangle_camera_position_3d(rectangle)),
            SceneMode::ColumbusView => {
                Some(self.rectangle_camera_position_columbus_view(rectangle))
            }
            SceneMode::Scene2D => Some(self.rectangle_camera_position_2d(rectangle)),
            SceneMode::Morphing => None,
        }
    }

    /// `rectangleCameraPosition3D(camera, rectangle, result, updateCamera)`.
    ///
    /// Backs the camera off along the negated geodetic surface normal at the
    /// rectangle's centre until all six corner/edge-centre points — plus, when
    /// the rectangle crosses the equator, the two equator points at the west and
    /// east longitudes — fit inside the frustum.
    ///
    /// DEVIATION: CesiumJS's fourth parameter `updateCamera` is `undefined` on
    /// the `getRectangleCameraCoordinates` path, so the module-level `defaultRF`
    /// scratch is used for `direction`/`right`/`up` and the camera's own pose is
    /// left untouched; the port hardcodes that choice because no caller passes
    /// `true`. Tracked in `docs/deviations.md`.
    fn rectangle_camera_position_3d(&mut self, rectangle: Rectangle) -> Cartesian3 {
        let ellipsoid = self.map_projection.ellipsoid().clone();
        let Rectangle { north, south, west, .. } = rectangle;
        let mut east = rectangle.east;

        // If we go across the International Date Line
        if west > east {
            east += CesiumMath::TWO_PI;
        }

        // Find the midpoint latitude.
        //
        // EllipsoidGeodesic will fail if the north and south edges are very
        // close to being on opposite sides of the ellipsoid, and it does not
        // detect that case in optimized builds — so test for it here instead.
        // It can only happen when north is very close to the north pole and
        // south is very close to the south pole, which is handled by centring on
        // latitude 0.
        let longitude = (west + east) * 0.5;
        let latitude = if south < -CesiumMath::PI_OVER_TWO + CesiumMath::RADIANS_PER_DEGREE
            && north > CesiumMath::PI_OVER_TWO - CesiumMath::RADIANS_PER_DEGREE
        {
            0.0
        } else {
            let north_cartographic = Cartographic::new(longitude, north, 0.0);
            let south_cartographic = Cartographic::new(longitude, south, 0.0);
            let mut ellipsoid_geodesic =
                EllipsoidGeodesic::new(None, None, Some(ellipsoid.clone()));
            ellipsoid_geodesic.set_end_points(&north_cartographic, &south_cartographic);
            ellipsoid_geodesic
                .interpolate_using_fraction(0.5)
                .latitude
        };

        let center = cartographic_to_cartesian_new(&ellipsoid, longitude, latitude);

        // JS reuses one scratch cartographic at height 0 for every corner.
        let north_east =
            Cartesian3::subtract_new(&cartographic_to_cartesian_new(&ellipsoid, east, north), &center);
        let north_west =
            Cartesian3::subtract_new(&cartographic_to_cartesian_new(&ellipsoid, west, north), &center);
        let north_center = Cartesian3::subtract_new(
            &cartographic_to_cartesian_new(&ellipsoid, longitude, north),
            &center,
        );
        let south_center = Cartesian3::subtract_new(
            &cartographic_to_cartesian_new(&ellipsoid, longitude, south),
            &center,
        );
        let south_east =
            Cartesian3::subtract_new(&cartographic_to_cartesian_new(&ellipsoid, east, south), &center);
        let south_west =
            Cartesian3::subtract_new(&cartographic_to_cartesian_new(&ellipsoid, west, south), &center);

        let mut direction = Cartesian3::default();
        ellipsoid.geodetic_surface_normal(&center, &mut direction);
        let direction = Cartesian3::negate_new(&direction);
        let right = Cartesian3::normalize_new(&Cartesian3::cross_new(
            &direction,
            &Cartesian3::UNIT_Z,
        ));
        let up = Cartesian3::cross_new(&right, &direction);

        let d = if self.frustum.is_orthographic() {
            let width = Cartesian3::distance(&north_east, &north_west)
                .max(Cartesian3::distance(&south_east, &south_west));
            let height = Cartesian3::distance(&north_east, &south_east)
                .max(Cartesian3::distance(&north_west, &south_west));

            // `camera.frustum._offCenterFrustum`
            let (_, frustum_right, frustum_top, _) = self.frustum.bounds();
            let ratio = frustum_right / frustum_top;
            let height_ratio = height * ratio;
            let (right_scalar, top_scalar) = if width > height_ratio {
                (width, width / ratio)
            } else {
                (height_ratio, height)
            };

            right_scalar.max(top_scalar)
        } else {
            // CesiumJS reads `frustum.fovy` / `frustum.aspectRatio`, which are
            // `undefined` — and so produce NaN — on any other frustum type.
            let tan_phi = (self.frustum.fovy().unwrap_or(f64::NAN) * 0.5).tan();
            let tan_theta = self.frustum.aspect_ratio().unwrap_or(f64::NAN) * tan_phi;

            let mut d = compute_d(&direction, &up, &north_west, tan_phi)
                .max(compute_d(&direction, &up, &south_east, tan_phi))
                .max(compute_d(&direction, &up, &north_east, tan_phi))
                .max(compute_d(&direction, &up, &south_west, tan_phi))
                .max(compute_d(&direction, &up, &north_center, tan_phi))
                .max(compute_d(&direction, &up, &south_center, tan_phi))
                .max(compute_d(&direction, &right, &north_west, tan_theta))
                .max(compute_d(&direction, &right, &south_east, tan_theta))
                .max(compute_d(&direction, &right, &north_east, tan_theta))
                .max(compute_d(&direction, &right, &south_west, tan_theta))
                .max(compute_d(&direction, &right, &north_center, tan_theta))
                .max(compute_d(&direction, &right, &south_center, tan_theta));

            // If the rectangle crosses the equator, compute D at the equator,
            // too, because that's the widest part of the rectangle when
            // projected onto the globe.
            if south < 0.0 && north > 0.0 {
                for equator_longitude in [west, east] {
                    let equator_position = Cartesian3::subtract_new(
                        &cartographic_to_cartesian_new(&ellipsoid, equator_longitude, 0.0),
                        &center,
                    );
                    d = d
                        .max(compute_d(&direction, &up, &equator_position, tan_phi))
                        .max(compute_d(&direction, &right, &equator_position, tan_theta));
                }
            }

            d
        };

        Cartesian3::add_new(
            &center,
            &Cartesian3::multiply_by_scalar_new(&direction, -d),
        )
    }

    /// `rectangleCameraPositionColumbusView(camera, rectangle, result)`.
    ///
    /// The two opposite corners are projected, round-tripped through
    /// `_actualTransform` / `_actualInvTransform` so they land in the camera's
    /// reference frame, and the camera is placed at their midpoint backed off in
    /// z by whatever the frustum needs to frame them.
    fn rectangle_camera_position_columbus_view(&self, rectangle: Rectangle) -> Cartesian3 {
        let rectangle = if rectangle.west > rectangle.east {
            Rectangle::MAX_VALUE
        } else {
            rectangle
        };
        let transform = self.actual_transform;
        let inv_transform = self.actual_inverse_transform;

        let projection = self.map_projection();
        let north_east =
            projection.project(&Cartographic::new(rectangle.east, rectangle.north, 0.0));
        let north_east = Matrix4::multiply_by_point_new(&transform, &north_east);
        let north_east = Matrix4::multiply_by_point_new(&inv_transform, &north_east);

        let south_west =
            projection.project(&Cartographic::new(rectangle.west, rectangle.south, 0.0));
        let south_west = Matrix4::multiply_by_point_new(&transform, &south_west);
        let south_west = Matrix4::multiply_by_point_new(&inv_transform, &south_west);

        let mut result = Cartesian3::default();
        result.x = (north_east.x - south_west.x) * 0.5 + south_west.x;
        result.y = (north_east.y - south_west.y) * 0.5 + south_west.y;

        result.z = match self.frustum.fovy() {
            Some(fovy) => {
                let tan_phi = (fovy * 0.5).tan();
                let tan_theta = self.frustum.aspect_ratio().unwrap_or(f64::NAN) * tan_phi;
                ((north_east.x - south_west.x) / tan_theta)
                    .max((north_east.y - south_west.y) / tan_phi)
                    * 0.5
            }
            None => {
                let width = north_east.x - south_west.x;
                let height = north_east.y - south_west.y;
                width.max(height)
            }
        };

        result
    }

    /// `rectangleCameraPosition2D(camera, rectangle, result)`.
    ///
    /// Like the Columbus View version but with no reference-frame round trip and
    /// an off-center-frustum aspect correction: the projected rectangle is
    /// matched to the frustum's `right`/`top` ratio, and the larger of the two
    /// resulting extents becomes the camera's `z` — which in 2D is the
    /// orthographic "height" the map is drawn at.
    fn rectangle_camera_position_2d(&mut self, rectangle: Rectangle) -> Cartesian3 {
        let mut rectangle = rectangle;

        // Account for the rectangle crossing the International Date Line in 2D mode
        let mut east = rectangle.east;
        if rectangle.west > rectangle.east {
            if self.map_mode_2d == MapMode2D::InfiniteScroll {
                east += CesiumMath::TWO_PI;
            } else {
                rectangle = Rectangle::MAX_VALUE;
                east = rectangle.east;
            }
        }

        let projection = self.map_projection();
        let north_east = projection.project(&Cartographic::new(east, rectangle.north, 0.0));
        let south_west = projection.project(&Cartographic::new(rectangle.west, rectangle.south, 0.0));

        let width = (north_east.x - south_west.x).abs() * 0.5;
        let height = (north_east.y - south_west.y).abs() * 0.5;

        let (_, frustum_right, frustum_top, _) = self.frustum.bounds();
        let ratio = frustum_right / frustum_top;
        let height_ratio = height * ratio;
        let (right, top) = if width > height_ratio {
            (width, width / ratio)
        } else {
            (height_ratio, height)
        };
        let height = (2.0 * right).max(2.0 * top);

        let mut result = Cartesian3::default();
        result.x = (north_east.x - south_west.x) * 0.5 + south_west.x;
        result.y = (north_east.y - south_west.y) * 0.5 + south_west.y;

        let projection = self.map_projection();
        let mut cart = projection.unproject(&result);
        cart.height = height;
        projection.project(&cart)
    }

    /// `Camera.prototype.computeViewRectangle(ellipsoid, result)`.
    ///
    /// The approximate rectangle of the ellipsoid visible from the camera, or
    /// `None` (the JS `undefined`) when the ellipsoid is entirely outside the
    /// view frustum. Each of the four canvas corners is picked against the
    /// ellipsoid; a corner that misses falls back to the matching horizon-quad
    /// point of [`compute_horizon_quad`]. When fewer than two corners hit the
    /// globe the whole-ellipsoid [`Rectangle::MAX_VALUE`] is returned, and a
    /// pole-crossing quad is widened to the full longitude range.
    ///
    /// DEVIATION: CesiumJS reads `canvas.clientWidth`/`clientHeight` off
    /// `this._scene.canvas`; the port uses its own `canvas_width`/
    /// `canvas_height` because it keeps no scene back-reference. Tracked in
    /// `docs/deviations.md`.
    pub fn compute_view_rectangle(
        &mut self,
        ellipsoid: Option<&Ellipsoid>,
    ) -> Option<Rectangle> {
        let ellipsoid = ellipsoid.cloned().unwrap_or(Ellipsoid::WGS84);

        // CesiumJS reads `positionWC`/`directionWC`/`upWC`, whose getters run
        // `updateMembers` first; the port keeps them as plain mirrors, so the
        // culling volume and horizon quad below need an explicit refresh.
        self.update_members();
        let position_wc = self.position_wc;
        let direction_wc = self.direction_wc;
        let up_wc = self.up_wc;
        let culling_volume =
            self.frustum
                .compute_culling_volume(&position_wc, &direction_wc, &up_wc);
        let bounding_sphere =
            BoundingSphere::new(Cartesian3::ZERO, ellipsoid.maximum_radius());
        let visibility = culling_volume.compute_visibility(&bounding_sphere);
        if visibility == Intersect::Outside {
            return None;
        }

        let width = self.canvas_width as f64;
        let height = self.canvas_height as f64;

        let computed_horizon_quad = compute_horizon_quad(&position_wc, &ellipsoid);

        let mut carto_array = [Cartographic::default(); 4];
        let mut successful_pick_count = 0;
        successful_pick_count += self.add_to_view_rectangle_result(
            0.0,
            0.0,
            0,
            &ellipsoid,
            &computed_horizon_quad,
            &mut carto_array,
        );
        successful_pick_count += self.add_to_view_rectangle_result(
            0.0,
            height,
            1,
            &ellipsoid,
            &computed_horizon_quad,
            &mut carto_array,
        );
        successful_pick_count += self.add_to_view_rectangle_result(
            width,
            height,
            2,
            &ellipsoid,
            &computed_horizon_quad,
            &mut carto_array,
        );
        successful_pick_count += self.add_to_view_rectangle_result(
            width,
            0.0,
            3,
            &ellipsoid,
            &computed_horizon_quad,
            &mut carto_array,
        );

        if successful_pick_count < 2 {
            // If we have space non-globe in 3 or 4 corners then return the whole globe
            return Some(Rectangle::MAX_VALUE);
        }

        let mut result = Rectangle::from_cartographic_array(&carto_array);

        // Detect if we go over the poles
        let mut distance = 0.0;
        let mut last_lon = carto_array[3].longitude;
        for carto in carto_array.iter() {
            let lon = carto.longitude;
            let diff = (lon - last_lon).abs();
            if diff > CesiumMath::PI {
                // Crossed the dateline
                distance += CesiumMath::TWO_PI - diff;
            } else {
                distance += diff;
            }
            last_lon = lon;
        }

        // We are over one of the poles so adjust the rectangle accordingly
        if CesiumMath::equals_epsilon(
            distance.abs(),
            CesiumMath::TWO_PI,
            Some(CesiumMath::EPSILON9),
            None,
        ) {
            result.west = -CesiumMath::PI;
            result.east = CesiumMath::PI;
            if carto_array[0].latitude >= 0.0 {
                result.north = CesiumMath::PI_OVER_TWO;
            } else {
                result.south = -CesiumMath::PI_OVER_TWO;
            }
        }

        Some(result)
    }

    /// The module-level `addToResult(x, y, index, camera, ellipsoid,
    /// computedHorizonQuad)` of CesiumJS's `computeViewRectangle`.
    ///
    /// Picks the ellipsoid at the window position `(x, y)`; on a hit it stores
    /// the cartographic of the intersection into `carto_array[index]` and
    /// returns `1`, and on a miss it stores the cartographic of the precomputed
    /// horizon-quad corner and returns `0`.
    fn add_to_view_rectangle_result(
        &mut self,
        x: f64,
        y: f64,
        index: usize,
        ellipsoid: &Ellipsoid,
        computed_horizon_quad: &[Cartesian3; 4],
        carto_array: &mut [Cartographic; 4],
    ) -> i32 {
        let window_position = Cartesian2::new(x, y);
        match self.pick_ellipsoid(&window_position, Some(ellipsoid)) {
            Some(hit) => {
                ellipsoid.cartesian_to_cartographic(&hit, &mut carto_array[index]);
                1
            }
            None => {
                ellipsoid.cartesian_to_cartographic(
                    &computed_horizon_quad[index],
                    &mut carto_array[index],
                );
                0
            }
        }
    }

    /// `Camera.prototype.distanceToBoundingSphere(boundingSphere)`.
    ///
    /// The distance from the camera to the *front* of the bounding sphere: the
    /// component of the camera→centre vector along the view direction, minus the
    /// radius, floored at zero.
    ///
    /// CesiumJS reads `positionWC`/`directionWC`, whose getters run
    /// `updateMembers`; the port keeps them as plain mirrors, so refresh first.
    pub fn distance_to_bounding_sphere(&mut self, bounding_sphere: &BoundingSphere) -> f64 {
        self.update_members();
        let position_wc = self.position_wc;
        let direction_wc = self.direction_wc;
        let to_center = Cartesian3::subtract_new(&position_wc, &bounding_sphere.center);
        let proj = Cartesian3::multiply_by_scalar_new(
            &direction_wc,
            Cartesian3::dot(&to_center, &direction_wc),
        );
        (Cartesian3::magnitude(&proj) - bounding_sphere.radius).max(0.0)
    }

    /// `Camera.prototype.getPixelSize(boundingSphere, drawingBufferWidth,
    /// drawingBufferHeight)`.
    ///
    /// The size of a pixel, in metres, at the front of `bounding_sphere`: the
    /// larger of the two per-axis pixel dimensions the frustum reports for the
    /// distance to the sphere.
    ///
    /// DEVIATION: `pixelRatio` comes from [`CameraSceneContext`]
    /// (`scene.pixelRatio` in CesiumJS), which the port holds at `1.0`. Tracked
    /// in `docs/deviations.md`.
    pub fn get_pixel_size(
        &mut self,
        bounding_sphere: &BoundingSphere,
        drawing_buffer_width: f64,
        drawing_buffer_height: f64,
    ) -> f64 {
        let distance = self.distance_to_bounding_sphere(bounding_sphere);
        let pixel_ratio = self.scene_context.pixel_ratio;
        let pixel_size = self.frustum.get_pixel_dimensions(
            drawing_buffer_width,
            drawing_buffer_height,
            distance,
            pixel_ratio,
        );
        pixel_size.x.max(pixel_size.y)
    }

    /// `Camera.prototype.viewBoundingSphere(boundingSphere, offset)`.
    ///
    /// Places the camera so the view contains `bounding_sphere`, looking at its
    /// centre from `offset` (heading/pitch/range in the local east-north-up
    /// frame). A zero/absent range is computed so the whole sphere is visible
    /// (see [`Camera::adjust_bounding_sphere_offset`]).
    ///
    /// DEVIATION: CesiumJS throws a `DeveloperError` while morphing and reads
    /// the ellipsoid off `scene`; the port skips the morphing guard (as
    /// [`Camera::look_at`] does) and takes the ellipsoid explicitly. Tracked in
    /// `docs/deviations.md`.
    pub fn view_bounding_sphere(
        &mut self,
        bounding_sphere: &BoundingSphere,
        offset: Option<HeadingPitchRange>,
        ellipsoid: &Ellipsoid,
    ) {
        let offset = self.adjust_bounding_sphere_offset(bounding_sphere, offset);
        self.look_at(
            &bounding_sphere.center,
            LookAtOffset::HeadingPitchRange(offset),
            ellipsoid,
        );
    }

    /// The module-level `adjustBoundingSphereOffset(camera, boundingSphere,
    /// offset)` of CesiumJS's `viewBoundingSphere`.
    ///
    /// Clones `offset` (defaulting to [`Camera::default_offset`]) and, when its
    /// range is zero, fills in a range that frames the whole sphere: a fixed
    /// [`MINIMUM_ZOOM`] for a degenerate sphere, otherwise the 2D or 3D framing
    /// distance, clamped to the controller's min/max zoom distance published in
    /// [`CameraSceneContext`].
    fn adjust_bounding_sphere_offset(
        &mut self,
        bounding_sphere: &BoundingSphere,
        offset: Option<HeadingPitchRange>,
    ) -> HeadingPitchRange {
        let mut offset = offset.unwrap_or_else(Camera::default_offset);
        let minimum_zoom = self.scene_context.minimum_zoom_distance;
        let maximum_zoom = self.scene_context.maximum_zoom_distance;
        let range = offset.range;
        // JS: `if (!defined(range) || range === 0.0)`; the port's range is a
        // plain `f64`, so only the zero case applies.
        if range == 0.0 {
            let radius = bounding_sphere.radius;
            // JS: `frustum instanceof OrthographicFrustum || _mode === SCENE2D`.
            let use_2d = self.frustum.is_orthographic() || self.mode == SceneMode::Scene2D;
            if radius == 0.0 {
                offset.range = MINIMUM_ZOOM;
            } else if use_2d {
                offset.range = distance_to_bounding_sphere_2d(&mut self.frustum, radius);
            } else {
                offset.range = distance_to_bounding_sphere_3d(&self.frustum, radius);
            }
            offset.range = CesiumMath::clamp(offset.range, minimum_zoom, maximum_zoom);
        }
        offset
    }

    /// `Camera.prototype.flyTo(options)`.
    ///
    /// Flies the camera from its current position to `options.destination`.
    ///
    /// Faithful control flow:
    /// * returns immediately (no flight) while morphing;
    /// * cancels any in-flight flight first ([`Camera::cancel_flight`]);
    /// * resolves a [`SetViewDestination::Rectangle`] through
    ///   [`Camera::get_rectangle_camera_coordinates`];
    /// * folds a `direction`/`up` orientation into heading/pitch/roll with
    ///   [`Camera::direction_up_to_heading_pitch_roll`];
    /// * `duration <= 0.0` takes the synchronous
    ///   [`Camera::set_view_with_options`] shortcut and fires `complete`
    ///   immediately;
    /// * otherwise builds the flight tween with
    ///   [`CameraFlightPath::create_tween`].
    ///
    /// Returns the tween for the owning [`crate::scene::Scene`] to register and
    /// drive for an animated flight, or `None` when the flight was synchronous
    /// (morphing / `duration <= 0.0`).
    ///
    /// DEVIATION: CesiumJS adds the tween to `scene.tweens` and stores it in
    /// `camera._currentFlight`, reaching the scene through `camera._scene`. The
    /// port's `Camera` has neither a scene back-reference nor a time source to
    /// drive tweens, so it *returns* the tween instead; the in-flight pose is
    /// shared through the flight channel [`Camera::update`] consumes. The async
    /// orientation (heading/pitch/roll) is not honoured because
    /// [`CameraFlightPath::create_tween`] lands in the default straight-down
    /// orientation. Tracked in `docs/deviations.md`.
    pub fn fly_to(&mut self, options: FlyToOptions) -> Option<TweenOptions> {
        // JS: `const mode = this._mode; if (mode === MORPHING) return;`
        if self.mode == SceneMode::Morphing {
            return None;
        }

        // JS: `this.cancelFlight();`
        self.cancel_flight();

        // JS: `if (destination instanceof Rectangle)
        //        destination = getRectangleCameraCoordinates(destination, scratch);`
        let destination = match options.destination {
            SetViewDestination::Cartesian(cartesian) => cartesian,
            // `getRectangleCameraCoordinates` returns `undefined` only while
            // morphing, already excluded above.
            SetViewDestination::Rectangle(rectangle) => {
                self.get_rectangle_camera_coordinates(rectangle)?
            }
        };

        // JS: `if (defined(orientation.direction))
        //        orientation = directionUpToHeadingPitchRoll(this, destination, orientation, scratch);`
        let orientation = match (options.orientation.direction, options.orientation.up) {
            (Some(direction), Some(up)) => {
                let hpr = self.direction_up_to_heading_pitch_roll(&destination, direction, up);
                SetViewOrientation {
                    direction: None,
                    up: None,
                    heading: Some(hpr.heading),
                    pitch: Some(hpr.pitch),
                    roll: Some(hpr.roll),
                }
            }
            _ => options.orientation,
        };

        // JS: `if (defined(options.duration) && options.duration <= 0.0) { …setView…; complete(); return; }`
        if let Some(duration) = options.duration {
            if duration <= 0.0 {
                let set_view_options = SetViewOptions {
                    // JS passes the *original* `options.destination` (rectangle
                    // or cartesian) to `setView`, which re-resolves a rectangle.
                    destination: Some(options.destination),
                    orientation,
                    end_transform: options.end_transform,
                    convert: options.convert,
                };
                self.set_view_with_options(&set_view_options);
                if let Some(complete) = options.complete {
                    complete();
                }
                return None;
            }
        }

        // Async path: build the flight tween for the scene to register. The
        // camera installs a flight channel on itself if it has none, so a
        // standalone camera can still be driven by an external tween collection.
        let channel = match self.flight_channel.clone() {
            Some(channel) => channel,
            None => {
                let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
                self.flight_channel = Some(channel.clone());
                channel
            }
        };
        let tween = CameraFlightPath::create_tween(
            self,
            &channel,
            CameraFlightTweenOptions {
                destination,
                duration: options.duration,
                easing_function: options.easing_function,
                complete: options.complete,
                cancel: options.cancel,
            },
        );
        Some(tween)
    }

    /// `Camera.prototype.flyHome(duration)`.
    ///
    /// Flies to the home view. [`Camera::default_view_rectangle`] sets the
    /// default view for the 3D scene; the 2D and Columbus View home shows the
    /// entire map.
    ///
    /// Returns the tween to register for an animated flight, or `None` when the
    /// flight was synchronous (`duration <= 0.0`) — see [`Camera::fly_to`].
    ///
    /// DEVIATION: CesiumJS calls `this._scene.completeMorph()` when invoked
    /// while morphing; the port has no scene back-reference so that step is
    /// skipped, and — exactly as in the JS, where the captured `mode` is still
    /// `MORPHING` and no branch matches — no flight is started. Tracked in
    /// `docs/deviations.md`.
    pub fn fly_home(&mut self, duration: Option<f64>) -> Option<TweenOptions> {
        let mode = self.mode;

        if mode == SceneMode::Scene2D {
            // JS: `flyTo({ destination: DEFAULT_VIEW_RECTANGLE, duration, endTransform: IDENTITY })`
            self.fly_to(FlyToOptions {
                destination: SetViewDestination::Rectangle(Camera::default_view_rectangle()),
                duration,
                end_transform: Some(Matrix4::IDENTITY),
                ..FlyToOptions::default()
            })
        } else if mode == SceneMode::Scene3D {
            // JS: `destination = getRectangleCameraCoordinates(DEFAULT_VIEW_RECTANGLE);
            //      mag = |destination| + |destination| * DEFAULT_VIEW_FACTOR;
            //      destination = normalize(destination) * mag;`
            let destination =
                self.get_rectangle_camera_coordinates(Camera::default_view_rectangle())?;
            let mut mag = Cartesian3::magnitude(&destination);
            mag += mag * Camera::DEFAULT_VIEW_FACTOR;
            let destination = Cartesian3::multiply_by_scalar_new(
                &Cartesian3::normalize_new(&destination),
                mag,
            );
            self.fly_to(FlyToOptions {
                destination: SetViewDestination::Cartesian(destination),
                duration,
                end_transform: Some(Matrix4::IDENTITY),
                ..FlyToOptions::default()
            })
        } else if mode == SceneMode::ColumbusView {
            // JS: `position = normalize((0,-1,1)) * 5 * maxRadii;
            //      orientation = { heading: 0, pitch: -acos(normalize(position).z), roll: 0 };
            //      convert: false`
            let max_radii = self.map_projection.ellipsoid().maximum_radius();
            let position = Cartesian3::multiply_by_scalar_new(
                &Cartesian3::normalize_new(&Cartesian3::new(0.0, -1.0, 1.0)),
                5.0 * max_radii,
            );
            let pitch = -(Cartesian3::normalize_new(&position).z.acos());
            self.fly_to(FlyToOptions {
                destination: SetViewDestination::Cartesian(position),
                duration,
                orientation: SetViewOrientation {
                    heading: Some(0.0),
                    pitch: Some(pitch),
                    roll: Some(0.0),
                    ..SetViewOrientation::default()
                },
                end_transform: Some(Matrix4::IDENTITY),
                convert: false,
                ..FlyToOptions::default()
            })
        } else {
            // Morphing: no branch matches in the JS either.
            None
        }
    }

    /// `Camera.prototype.flyToBoundingSphere(boundingSphere, options)`.
    ///
    /// Flies the camera so the current view contains `bounding_sphere`. The
    /// offset is heading/pitch/range in the local east-north-up frame centred on
    /// the sphere; a zero range is computed so the whole sphere is visible (see
    /// [`Camera::adjust_bounding_sphere_offset`]).
    ///
    /// Returns the tween to register, or `None` for a synchronous
    /// (`duration <= 0.0`) flight — see [`Camera::fly_to`].
    ///
    /// DEVIATION: CesiumJS reads the ellipsoid off `scene.ellipsoid`; the port
    /// uses the camera's own map-projection ellipsoid. Tracked in
    /// `docs/deviations.md`.
    pub fn fly_to_bounding_sphere(
        &mut self,
        bounding_sphere: &BoundingSphere,
        options: FlyToBoundingSphereOptions,
    ) -> Option<TweenOptions> {
        // JS: `const scene2D = mode === SCENE2D || mode === COLUMBUS_VIEW;`
        let scene2d = self.mode == SceneMode::Scene2D || self.mode == SceneMode::ColumbusView;
        // JS: `this._setTransform(Matrix4.IDENTITY);`
        self.set_transform(Matrix4::IDENTITY);
        // JS: `const offset = adjustBoundingSphereOffset(this, boundingSphere, options.offset);`
        let offset = self.adjust_bounding_sphere_offset(bounding_sphere, options.offset);

        // JS: `position = scene2D ? UNIT_Z * range
        //                        : offsetFromHeadingPitchRange(heading, pitch, range);`
        let local_position = if scene2d {
            Cartesian3::multiply_by_scalar_new(&Cartesian3::UNIT_Z, offset.range)
        } else {
            offset_from_heading_pitch_range(offset.heading, offset.pitch, offset.range)
        };

        let ellipsoid = self.map_projection.ellipsoid().clone();
        // JS: `transform = eastNorthUpToFixedFrame(center, ellipsoid);
        //      position = multiplyByPoint(transform, position);`
        let transform =
            transforms::east_north_up_to_fixed_frame_new(&bounding_sphere.center, Some(&ellipsoid));
        let position = Matrix4::multiply_by_point_new(&transform, &local_position);

        // JS: `if (!scene2D) { direction = normalize(center - position); up = …; }`
        let (direction, up) = if scene2d {
            (None, None)
        } else {
            let direction = Cartesian3::normalize_new(&Cartesian3::subtract_new(
                &bounding_sphere.center,
                &position,
            ));
            let mut up =
                Matrix4::multiply_by_point_as_vector_new(&transform, &Cartesian3::UNIT_Z);
            // JS: `if (1 - |dot(direction, up)| < EPSILON6)` the up vector is
            // nearly parallel to the direction, so rotate the frame's y-axis
            // about `direction` by `heading` to get a stable up.
            if 1.0 - Cartesian3::dot(&direction, &up).abs() < CesiumMath::EPSILON6 {
                let rotate_quat = Quaternion::from_axis_angle_new(&direction, offset.heading);
                let rotation = Matrix3::from_quaternion_new(&rotate_quat);
                let column1 = Matrix4::get_column_new(&transform, 1);
                up = Matrix3::multiply_by_vector_new(
                    &rotation,
                    &Cartesian3::from_cartesian4_new(&column1),
                );
            }
            // JS: `right = cross(direction, up); up = normalize(cross(right, direction));`
            let right = Cartesian3::cross_new(&direction, &up);
            let up = Cartesian3::normalize_new(&Cartesian3::cross_new(&right, &direction));
            (Some(direction), Some(up))
        };

        self.fly_to(FlyToOptions {
            destination: SetViewDestination::Cartesian(position),
            orientation: SetViewOrientation {
                direction,
                up,
                ..SetViewOrientation::default()
            },
            duration: options.duration,
            complete: options.complete,
            cancel: options.cancel,
            end_transform: options.end_transform,
            easing_function: options.easing_function,
            ..FlyToOptions::default()
        })
    }

    /// `Camera.prototype.cancelFlight()`.
    ///
    /// Cancels the current camera flight and leaves the camera at its current
    /// location. If no flight is in progress, this does nothing.
    ///
    /// DEVIATION: CesiumJS calls `this._currentFlight.cancelTween()`, which
    /// fires the tween's cancel callback (and the user's `options.cancel`). The
    /// port's tween — and therefore the user callback — is owned by the
    /// [`crate::scene::Scene`]'s tween collection, not the camera; clearing the
    /// shared flight channel is the camera-visible effect (the next
    /// [`Camera::update`] stops applying an interpolated pose). The user cancel
    /// callback fires when the scene cancels the registered tween. Tracked in
    /// `docs/deviations.md`.
    pub fn cancel_flight(&mut self) {
        if let Some(channel) = self.flight_channel.as_ref() {
            *channel.borrow_mut() = None;
        }
    }

    /// `Camera.prototype.completeFlight()`.
    ///
    /// Completes the current camera flight and moves the camera immediately to
    /// its final destination. If no flight is in progress, this does nothing.
    ///
    /// DEVIATION: CesiumJS cancels the tween, calls `setView` with the flight's
    /// final destination/orientation, and fires `_currentFlight.complete`. The
    /// port applies the flight channel's end pose directly (the same pose
    /// [`Camera::apply_flight`] applies on completion) and clears the channel;
    /// the user complete callback is owned by the scene's tween. Tracked in
    /// `docs/deviations.md`.
    pub fn complete_flight(&mut self) {
        let Some(channel) = self.flight_channel.as_ref() else {
            return;
        };
        let end = {
            let slot = channel.borrow();
            slot.as_ref()
                .map(|flight| (flight.end_position, flight.end_direction, flight.end_up))
        };
        let Some((position, direction, up)) = end else {
            return;
        };
        self.position = position;
        self.direction = direction;
        self.up = up;
        *channel.borrow_mut() = None;
    }

    /// `Camera.prototype.lookAt(target, offset)`.
    ///
    /// Builds the east-north-up frame at `target` and hands it to
    /// [`Camera::look_at_transform`], which leaves the camera locked to that
    /// frame until the transform is reset.
    ///
    /// DEVIATION: CesiumJS reads the ellipsoid off `scene.ellipsoid` and throws
    /// a `DeveloperError` while morphing; the port takes the ellipsoid
    /// explicitly and skips the morphing guard because it has no scene
    /// back-reference. Tracked in `docs/deviations.md`.
    pub fn look_at(&mut self, target: &Cartesian3, offset: LookAtOffset, ellipsoid: &Ellipsoid) {
        let transform = transforms::east_north_up_to_fixed_frame_new(target, Some(ellipsoid));
        self.look_at_transform(&transform, Some(offset));
    }

    /// `Camera.prototype.lookAtTransform(transform, offset)`.
    ///
    /// Installs `transform` as the camera's reference frame and then positions
    /// the camera at `offset` inside it. Passing `None` for the offset only
    /// changes the frame, matching the JS `if (!defined(offset)) { return; }`.
    ///
    /// The 2D branch is a special case: the camera is flattened onto the map
    /// plane (position x/y zeroed, direction = -Z), `up` is taken from the
    /// negated offset with its z discarded, and the orthographic frustum bounds
    /// are resized to frame the offset — the transform is temporarily set to the
    /// identity so those writes land in projected space, then restored.
    ///
    /// DEVIATION: CesiumJS throws a `DeveloperError` while morphing; the port
    /// skips that guard. Tracked in `docs/deviations.md`.
    pub fn look_at_transform(&mut self, transform: &Matrix4, offset: Option<LookAtOffset>) {
        self.set_transform(*transform);

        let offset = match offset {
            Some(offset) => offset,
            None => return,
        };
        let cartesian_offset = match offset {
            LookAtOffset::HeadingPitchRange(hpr) => {
                offset_from_heading_pitch_range(hpr.heading, hpr.pitch, hpr.range)
            }
            LookAtOffset::Cartesian(cartesian) => cartesian,
        };

        if self.mode == SceneMode::Scene2D {
            // `Cartesian2.clone(Cartesian2.ZERO, this.position)` writes x/y only.
            self.position.x = 0.0;
            self.position.y = 0.0;

            let mut up = Cartesian3::negate_new(&cartesian_offset);
            up.z = 0.0;
            if Cartesian3::magnitude_squared(&up) < CesiumMath::EPSILON10 {
                up = Cartesian3::UNIT_Y;
            }
            self.up = Cartesian3::normalize_new(&up);

            self.set_transform(Matrix4::IDENTITY);
            self.direction = Cartesian3::negate_new(&Cartesian3::UNIT_Z);
            self.right = Cartesian3::cross_new(&self.direction, &self.up);
            self.right = Cartesian3::normalize_new(&self.right);

            let (_, right, top, _) = self.frustum.bounds();
            let ratio = top / right;
            let new_right = Cartesian3::magnitude(&cartesian_offset) * 0.5;
            let new_top = ratio * new_right;
            self.frustum.set_bounds(-new_right, new_right, new_top, -new_top);

            self.set_transform(*transform);
            return;
        }

        self.position = cartesian_offset;
        let negated = Cartesian3::negate_new(&self.position);
        self.direction = Cartesian3::normalize_new(&negated);
        let mut right = Cartesian3::cross_new(&self.direction, &Cartesian3::UNIT_Z);
        if Cartesian3::magnitude_squared(&right) < CesiumMath::EPSILON10 {
            right = Cartesian3::UNIT_X;
        }
        self.right = Cartesian3::normalize_new(&right);
        self.up = Cartesian3::normalize_new(&Cartesian3::cross_new(
            &self.right,
            &self.direction,
        ));

        self.adjust_orthographic_frustum(true);
    }

    // ---- Update ----

    /// `Camera.prototype.update(mode)`.
    ///
    /// DEVIATION: two port-only additions. (1) [`Camera::apply_flight`] runs
    /// first — CesiumJS drives the flight from the tween's `update` closure,
    /// which the port folds into the per-frame camera update (M3/S3).
    /// (2) [`Camera::refresh`] runs at the end; CesiumJS recomputes the derived
    /// state lazily from its getters instead. Tracked in `docs/deviations.md`.
    pub fn update(&mut self, mode: SceneMode) {
        self.apply_flight();

        // >>includeStart('debug', pragmas.debug)
        if cfg!(debug_assertions) {
            if mode == SceneMode::Scene2D && !self.frustum.is_orthographic_off_center() {
                throw_developer_error("An OrthographicOffCenterFrustum is required in 2D.");
            }
            if (mode == SceneMode::Scene3D || mode == SceneMode::ColumbusView)
                && !self.frustum.is_perspective()
                && !self.frustum.is_orthographic()
            {
                throw_developer_error(
                    "A PerspectiveFrustum or OrthographicFrustum is required in 3D and Columbus view",
                );
            }
        }
        // >>includeEnd('debug');

        let mut update_frustum = false;
        if mode != self.mode {
            self.mode = mode;
            self.mode_changed = mode != SceneMode::Morphing;
            update_frustum = self.mode == SceneMode::Scene2D;
        }

        if update_frustum {
            // `const frustum = (this._max2Dfrustum = this.frustum.clone())` —
            // the rescaling below applies to the *clone*; the live frustum is
            // never touched here.
            let mut frustum = self.frustum.clone();

            // >>includeStart('debug', pragmas.debug)
            if cfg!(debug_assertions) && !frustum.is_orthographic_off_center() {
                throw_developer_error(
                    "The camera frustum is expected to be orthographic for 2D camera control.",
                );
            }
            // >>includeEnd('debug');

            let max_zoom_out = 2.0;
            let (_left, right, top, _bottom) = frustum.bounds();
            let ratio = top / right;
            let new_right = self.max_coord.x * max_zoom_out;
            let new_left = -new_right;
            let new_top = ratio * new_right;
            let new_bottom = -new_top;
            frustum.set_bounds(new_left, new_right, new_top, new_bottom);

            self.max_2d_frustum = Some(frustum);
        }

        if self.mode == SceneMode::Scene2D {
            let rotatable_2d = self.map_mode_2d == MapMode2D::Rotate;
            let max_coord = self.max_coord;
            clamp_move_2d(rotatable_2d, &max_coord, &mut self.position);
        }

        self.refresh();
    }

    /// Applies the in-flight camera pose from the shared flight channel
    /// (M3/S3, mirrors the CesiumJS `Camera#flyTo` tween update closure).
    ///
    /// While a flight is active the interpolated pose is applied; when the
    /// tween signals completion the exact end pose is applied and the channel
    /// is consumed.
    fn apply_flight(&mut self) {
        let Some(channel) = self.flight_channel.as_ref() else {
            return;
        };
        let mut slot = channel.borrow_mut();
        let Some(flight) = slot.as_ref() else {
            return;
        };
        if flight.completed {
            self.position = flight.end_position;
            self.direction = flight.end_direction;
            self.up = flight.end_up;
            *slot = None;
            return;
        }
        let (position, direction, up) = CameraFlightPath::interpolate(flight, flight.t);
        self.position = position;
        self.direction = direction;
        self.up = up;
    }

    /// The module-level `updateMembers(camera)` of CesiumJS.
    ///
    /// Synchronizes the private mirrors `_position`/`_direction`/`_up`/`_right`
    /// with the public pose, folds the Columbus View / 2D projection into
    /// `_actualTransform`, recomputes the world-coordinate quantities and the
    /// view matrix — but only for the components that actually changed, which
    /// is what makes the CesiumJS getters cheap.
    fn update_members(&mut self) {
        let mode = self.mode;

        // 1. In 2D the camera height is always 12.7 million meters; the
        //    apparent height is half the frustum width.
        let mut height_changed = false;
        let mut height = 0.0;
        if mode == SceneMode::Scene2D {
            let (left, right, _, _) = self.frustum.bounds();
            height = right - left;
            height_changed = height != self.position_cartographic.height;
        }

        // 2.
        let position_changed =
            !Cartesian3::equals(Some(&self.private_position), Some(&self.position))
                || height_changed;
        if position_changed {
            self.private_position = self.position;
        }

        // 3. The public axes are normalized *in place* before being mirrored —
        //    CesiumJS writes back into `camera.direction` / `.up` / `.right`.
        let direction_changed =
            !Cartesian3::equals(Some(&self.private_direction), Some(&self.direction));
        if direction_changed {
            let direction = Cartesian3::normalize_new(&self.direction);
            self.direction = direction;
            self.private_direction = direction;
        }

        let up_changed = !Cartesian3::equals(Some(&self.private_up), Some(&self.up));
        if up_changed {
            let up = Cartesian3::normalize_new(&self.up);
            self.up = up;
            self.private_up = up;
        }

        let right_changed = !Cartesian3::equals(Some(&self.private_right), Some(&self.right));
        if right_changed {
            let right = Cartesian3::normalize_new(&self.right);
            self.right = right;
            self.private_right = right;
        }

        // 4.
        let transform_changed = self.transform_changed || self.mode_changed;
        self.transform_changed = false;

        // 5.
        if transform_changed {
            self.inverse_transform = Matrix4::inverse_transformation_new(&self.transform);

            if self.mode == SceneMode::ColumbusView || self.mode == SceneMode::Scene2D {
                if Matrix4::equals(&Matrix4::IDENTITY, &self.transform) {
                    self.actual_transform = Self::transform_2d();
                } else if self.mode == SceneMode::ColumbusView {
                    self.convert_transform_for_columbus_view();
                } else {
                    self.convert_transform_for_2d();
                }
            } else {
                self.actual_transform = self.transform;
            }

            self.actual_inverse_transform =
                Matrix4::inverse_transformation_new(&self.actual_transform);

            self.mode_changed = false;
        }

        // 6. `_actualTransform` is read *after* step 5.
        if position_changed || transform_changed {
            self.position_wc = Matrix4::multiply_by_point_new(
                &self.actual_transform,
                &self.private_position,
            );

            // Compute the Cartographic position of the camera.
            if mode == SceneMode::Scene3D || mode == SceneMode::Morphing {
                // DEVIATION: JS assigns the getter's return value, so a failed
                // `cartesianToCartographic` leaves `_positionCartographic`
                // `undefined`; the port keeps the previous value instead.
                let mut cartographic = self.position_cartographic;
                if self
                    .map_projection
                    .ellipsoid()
                    .cartesian_to_cartographic(&self.position_wc, &mut cartographic)
                {
                    self.position_cartographic = cartographic;
                }
            } else {
                // The camera position is expressed in the 2D coordinate system
                // where the Y axis is to the East, the Z axis is to the North,
                // and the X axis is out of the map. Express them instead in the
                // ENU axes where X is to the East, Y is to the North, and Z is
                // out of the local horizontal plane.
                let mut position_enu = Cartesian3::new(
                    self.position_wc.y,
                    self.position_wc.z,
                    self.position_wc.x,
                );

                // In 2D, the camera height is always 12.7 million meters.
                // The apparent height is equal to half the frustum width.
                if mode == SceneMode::Scene2D {
                    position_enu.z = height;
                }

                self.position_cartographic = self.map_projection.unproject(&position_enu);
            }
        }

        // 7.
        if direction_changed || up_changed || right_changed {
            let det = Cartesian3::dot(
                &self.private_direction,
                &Cartesian3::cross_new(&self.private_up, &self.private_right),
            );
            if (1.0 - det).abs() > CesiumMath::EPSILON2 {
                // Orthonormalize the axes.
                let inv_up_mag = 1.0 / Cartesian3::magnitude_squared(&self.private_up);
                let scalar =
                    Cartesian3::dot(&self.private_up, &self.private_direction) * inv_up_mag;
                let w0 = Cartesian3::multiply_by_scalar_new(&self.private_direction, scalar);
                let up = Cartesian3::normalize_new(&Cartesian3::subtract_new(&self.private_up, &w0));
                self.private_up = up;
                self.up = up;

                let right = Cartesian3::cross_new(&self.private_direction, &self.private_up);
                self.private_right = right;
                self.right = right;
            }
        }

        // 8.
        if direction_changed || transform_changed {
            let direction_wc = Matrix4::multiply_by_point_as_vector_new(
                &self.actual_transform,
                &self.private_direction,
            );
            self.direction_wc = Cartesian3::normalize_new(&direction_wc);
        }

        if up_changed || transform_changed {
            let up_wc = Matrix4::multiply_by_point_as_vector_new(
                &self.actual_transform,
                &self.private_up,
            );
            self.up_wc = Cartesian3::normalize_new(&up_wc);
        }

        if right_changed || transform_changed {
            let right_wc = Matrix4::multiply_by_point_as_vector_new(
                &self.actual_transform,
                &self.private_right,
            );
            self.right_wc = Cartesian3::normalize_new(&right_wc);
        }

        // 9.
        if position_changed
            || direction_changed
            || up_changed
            || right_changed
            || transform_changed
        {
            self.update_view_matrix();
        }
    }

    /// The module-level `updateViewMatrix(camera)` of CesiumJS.
    ///
    /// Uses the *private* mirrors (already normalized and orthonormalized by
    /// [`Camera::update_members`]) and `_actualInvTransform` — not
    /// `_invTransform`, which ignores the Columbus View / 2D fold.
    fn update_view_matrix(&mut self) {
        let view = Matrix4::compute_view_new(
            &self.private_position,
            &self.private_direction,
            &self.private_up,
            &self.private_right,
        );
        self.view_matrix = Matrix4::multiply_new(&view, &self.actual_inverse_transform);
        self.inverse_view_matrix = Matrix4::inverse_transformation_new(&self.view_matrix);
    }

    /// Caches `frustum.projectionMatrix` and its inverse.
    ///
    /// DEVIATION: CesiumJS reads `camera.frustum.projectionMatrix` on demand
    /// and never caches the inverse; the port publishes both into
    /// [`crate::frame_state::FrameState`]. Tracked in `docs/deviations.md`.
    fn update_projection_matrix(&mut self) {
        self.projection_matrix = self.frustum.projection_matrix();
        self.inverse_projection_matrix =
            Matrix4::inverse_new(&self.projection_matrix).unwrap_or(Matrix4::IDENTITY);
    }

    /// The module-level `updateCameraDeltas(camera)` of CesiumJS — the
    /// bookkeeping behind `positionWCDeltaMagnitude`,
    /// `positionWCDeltaMagnitudeLastFrame` and `timeSinceMoved`.
    fn update_camera_deltas(&mut self) {
        // The JS reads `camera.positionWC`, whose getter runs `updateMembers`.
        self.update_members();

        match self.old_position_wc {
            None => {
                self.old_position_wc = Some(self.position_wc);
            }
            Some(old_position_wc) => {
                self.position_wc_delta_magnitude_last_frame = self.position_wc_delta_magnitude;
                let delta = Cartesian3::subtract_new(&self.position_wc, &old_position_wc);
                self.position_wc_delta_magnitude = Cartesian3::magnitude(&delta);
                self.old_position_wc = Some(self.position_wc);

                // Update move timers.
                if self.position_wc_delta_magnitude > 0.0 {
                    self.time_since_moved = 0.0;
                    self.last_moved_timestamp = get_timestamp();
                } else {
                    self.time_since_moved =
                        (get_timestamp() - self.last_moved_timestamp).max(0.0) / 1000.0;
                }
            }
        }
    }

    /// `Camera.prototype._updateCameraChanged` — raises `Camera#changed` once
    /// the view has drifted past `percentageChanged`.
    pub fn update_camera_changed(&mut self) {
        self.update_camera_deltas();

        if self.changed_event.number_of_listeners() == 0 {
            return;
        }

        let percentage_changed = self.percentage_changed;

        // Check heading.
        let current_heading = self.heading();

        if self.changed_heading.is_none() {
            self.changed_heading = current_heading;
        }

        // DEVIATION: while morphing the JS getters yield `undefined`, so
        // `Math.abs(_changedHeading - currentHeading)` is NaN and every
        // comparison against it is false — nothing is raised. `unwrap_or(NAN)`
        // reproduces that propagation instead of modelling an optional angle.
        let mut heading_delta = (self.changed_heading.unwrap_or(f64::NAN)
            - current_heading.unwrap_or(f64::NAN))
            .abs()
            % CesiumMath::TWO_PI;
        heading_delta = if heading_delta > CesiumMath::PI {
            CesiumMath::TWO_PI - heading_delta
        } else {
            heading_delta
        };

        // Since delta is computed as the shortest distance between two angles
        // the percentage is relative to the half circle.
        let heading_changed_percentage = heading_delta / CesiumMath::PI;

        if heading_changed_percentage > percentage_changed {
            self.changed_heading = current_heading;
        }

        // Check roll.
        let current_roll = self.roll();

        if self.changed_roll.is_none() {
            self.changed_roll = current_roll;
        }

        let mut roll_delta = (self.changed_roll.unwrap_or(f64::NAN)
            - current_roll.unwrap_or(f64::NAN))
            .abs()
            % CesiumMath::TWO_PI;
        roll_delta = if roll_delta > CesiumMath::PI {
            CesiumMath::TWO_PI - roll_delta
        } else {
            roll_delta
        };

        // Since delta is computed as the shortest distance between two angles
        // the percentage is relative to the half circle.
        let roll_changed_percentage = roll_delta / CesiumMath::PI;

        if roll_changed_percentage > percentage_changed {
            self.changed_roll = current_roll;
        }
        if roll_changed_percentage > percentage_changed
            || heading_changed_percentage > percentage_changed
        {
            self.changed_event
                .raise_event(&roll_changed_percentage.max(heading_changed_percentage));
        }

        if self.mode == SceneMode::Scene2D {
            if self.changed_frustum.is_none() {
                self.changed_position = Some(self.position);
                self.changed_frustum = Some(self.frustum.clone());
                return;
            }

            let position = self.position;
            let last_position = self.changed_position.unwrap_or_default();

            let (left, right, top, bottom) = self.frustum.bounds();
            let mut last_frustum = self.changed_frustum.clone().unwrap_or_default();
            let (last_left, last_right, last_top, last_bottom) = last_frustum.bounds();

            let x0 = position.x + left;
            let x1 = position.x + right;
            let x2 = last_position.x + last_left;
            let x3 = last_position.x + last_right;

            let y0 = position.y + bottom;
            let y1 = position.y + top;
            let y2 = last_position.y + last_bottom;
            let y3 = last_position.y + last_top;

            let left_x = x0.max(x2);
            let right_x = x1.min(x3);
            let bottom_y = y0.max(y2);
            let top_y = y1.min(y3);

            // NOTE: CesiumJS compares `bottomY >= y1` here, not `>= topY`. Kept
            // verbatim.
            let area_percentage = if left_x >= right_x || bottom_y >= y1 {
                1.0
            } else {
                // `areaRef` is the *smaller* of the two frustums unless the
                // current one fully contains the previous one.
                let (area_left, area_right, area_top, area_bottom) =
                    if x0 < x2 && x1 > x3 && y0 < y2 && y1 > y3 {
                        (left, right, top, bottom)
                    } else {
                        (last_left, last_right, last_top, last_bottom)
                    };
                1.0 - ((right_x - left_x) * (top_y - bottom_y))
                    / ((area_right - area_left) * (area_top - area_bottom))
            };

            if area_percentage > percentage_changed {
                self.changed_event.raise_event(&area_percentage);
                self.changed_position = Some(self.position);
                self.changed_frustum = Some(self.frustum.clone());
            }
            return;
        }

        if self.changed_direction.is_none() {
            self.changed_position = Some(self.position_wc);
            self.changed_direction = Some(self.direction_wc);
            return;
        }

        let changed_direction = self.changed_direction.unwrap_or_default();
        let changed_position = self.changed_position.unwrap_or_default();

        let dir_angle =
            CesiumMath::acos_clamped(Cartesian3::dot(&self.direction_wc, &changed_direction));

        let dir_percentage = match self.frustum.fovy() {
            Some(fovy) => dir_angle / (fovy * 0.5),
            None => dir_angle,
        };

        let distance = Cartesian3::distance(&self.position_wc, &changed_position);
        let height_percentage = distance / self.position_cartographic.height;

        if dir_percentage > percentage_changed || height_percentage > percentage_changed {
            self.changed_event
                .raise_event(&dir_percentage.max(height_percentage));
            self.changed_position = Some(self.position_wc);
            self.changed_direction = Some(self.direction_wc);
        }
    }

    /// The module-level `convertTransformForColumbusView(camera)` of CesiumJS.
    fn convert_transform_for_columbus_view(&mut self) {
        let transform = self.transform;
        // CesiumJS passes `_actualTransform` itself as the result; the local
        // copy avoids Rust's aliasing restriction on `&self` / `&mut self`.
        let mut actual_transform = self.actual_transform;
        transforms::basis_to_2d(
            self.map_projection.as_ref(),
            &transform,
            &mut actual_transform,
        );
        self.actual_transform = actual_transform;
    }

    /// The module-level `convertTransformFor2D(camera)` of CesiumJS.
    ///
    /// Rebuilds the reference frame in the projected 2D space: the origin is the
    /// projected transform origin swizzled to `(z, x, y)`, the new Z axis is
    /// `UNIT_X` (out of the map), and the X/Y axes are the projections of the
    /// transform's own X/Y axes re-orthogonalized against it.
    ///
    /// DEVIATION: CesiumJS runs the `Cartesian3` helpers on `Cartesian4`
    /// scratches, which silently leaves `w` alone; the `xyz_*` helpers below make
    /// that invariant explicit. Tracked in `docs/deviations.md`.
    fn convert_transform_for_2d(&mut self) {
        let ellipsoid = self.map_projection.ellipsoid().clone();
        let transform = self.transform;

        let origin = Matrix4::get_column_new(&transform, 3);

        let mut cartographic = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(
            &Cartesian3::new(origin.x, origin.y, origin.z),
            &mut cartographic,
        );

        let projected_position = self.map_projection.project(&cartographic);
        let new_origin = Cartesian4::new(
            projected_position.z,
            projected_position.x,
            projected_position.y,
            1.0,
        );

        let new_z_axis = Cartesian4::UNIT_X;

        let column = Matrix4::get_column_new(&transform, 0);
        let x_axis = Cartesian4::add_new(&column, &origin);
        let mut cartographic = Cartographic::default();
        ellipsoid.cartesian_to_cartographic(
            &Cartesian3::new(x_axis.x, x_axis.y, x_axis.z),
            &mut cartographic,
        );
        let projected_position = self.map_projection.project(&cartographic);
        let mut new_x_axis = Cartesian4::new(
            projected_position.z,
            projected_position.x,
            projected_position.y,
            0.0,
        );
        xyz_subtract_in_place(&mut new_x_axis, &new_origin);
        new_x_axis.x = 0.0;

        let mut new_y_axis = Cartesian4::default();
        if xyz_magnitude_squared(&new_x_axis) > CesiumMath::EPSILON10 {
            xyz_cross(&new_z_axis, &new_x_axis, &mut new_y_axis);
        } else {
            let column = Matrix4::get_column_new(&transform, 1);
            let y_axis = Cartesian4::add_new(&column, &origin);
            let mut cartographic = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(
                &Cartesian3::new(y_axis.x, y_axis.y, y_axis.z),
                &mut cartographic,
            );
            let projected_position = self.map_projection.project(&cartographic);
            new_y_axis = Cartesian4::new(
                projected_position.z,
                projected_position.x,
                projected_position.y,
                0.0,
            );
            xyz_subtract_in_place(&mut new_y_axis, &new_origin);
            new_y_axis.x = 0.0;

            if xyz_magnitude_squared(&new_y_axis) < CesiumMath::EPSILON10 {
                new_x_axis = Cartesian4::UNIT_Y;
                new_y_axis = Cartesian4::UNIT_Z;
            }
        }

        xyz_cross(&new_y_axis, &new_z_axis, &mut new_x_axis);
        xyz_normalize_in_place(&mut new_x_axis);
        // The second cross uses the *normalized* X axis, as in CesiumJS.
        xyz_cross(&new_z_axis, &new_x_axis, &mut new_y_axis);
        xyz_normalize_in_place(&mut new_y_axis);

        // `Matrix4.setColumn(_actualTransform, 0..3, …)` — all four columns are
        // written, so assembling the matrix directly is equivalent.
        self.actual_transform = Matrix4::new(
            new_x_axis.x, new_y_axis.x, new_z_axis.x, new_origin.x,
            new_x_axis.y, new_y_axis.y, new_z_axis.y, new_origin.y,
            new_x_axis.z, new_y_axis.z, new_z_axis.z, new_origin.z,
            new_x_axis.w, new_y_axis.w, new_z_axis.w, new_origin.w,
        );
    }

    // ---- Canvas dimensions ----

    /// Records the drawing-buffer size and keeps `frustum.aspectRatio` in step.
    ///
    /// DEVIATION: CesiumJS has no such method — `getPickRay` reads
    /// `scene.canvas.clientWidth/clientHeight` through the camera's `_scene`
    /// back-reference, and `Scene` assigns
    /// `camera.frustum.aspectRatio = viewport.width / viewport.height`. The port
    /// has no back-reference, so the scene pushes the size in here and both
    /// effects happen at once. Tracked in `docs/deviations.md`.
    pub fn set_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width;
        self.canvas_height = height;
        if height > 0 {
            let aspect_ratio = width as f64 / height as f64;
            self.frustum.set_aspect_ratio(aspect_ratio);
        }
    }

    /// The drawing-buffer width `getPickRay` normalizes by.
    pub fn canvas_width(&self) -> u32 { self.canvas_width }

    /// The drawing-buffer height `getPickRay` normalizes by.
    pub fn canvas_height(&self) -> u32 { self.canvas_height }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// The module-level `getHeading(direction, up)` of CesiumJS.
fn get_heading(direction: &Cartesian3, up: &Cartesian3) -> f64 {
    let heading = if !CesiumMath::equals_epsilon(
        direction.z.abs(),
        1.0,
        Some(CesiumMath::EPSILON3),
        None,
    ) {
        direction.y.atan2(direction.x) - CesiumMath::PI_OVER_TWO
    } else {
        up.y.atan2(up.x) - CesiumMath::PI_OVER_TWO
    };

    CesiumMath::TWO_PI - CesiumMath::zero_to_two_pi(heading)
}

/// The module-level `getPitch(direction)` of CesiumJS.
fn get_pitch(direction: &Cartesian3) -> f64 {
    CesiumMath::PI_OVER_TWO - CesiumMath::acos_clamped(direction.z)
}

/// The module-level `getRoll(direction, up, right)` of CesiumJS.
fn get_roll(direction: &Cartesian3, up: &Cartesian3, right: &Cartesian3) -> f64 {
    let mut roll = 0.0;
    if !CesiumMath::equals_epsilon(direction.z.abs(), 1.0, Some(CesiumMath::EPSILON3), None) {
        roll = (-right.z).atan2(up.z);
        roll = CesiumMath::zero_to_two_pi(roll + CesiumMath::TWO_PI);
    }

    roll
}

/// The module-level `offsetFromHeadingPitchRange(heading, pitch, range)` of
/// CesiumJS.
///
/// Rotates the local +x axis by pitch about +y and then by heading about +z,
/// negates it and scales by `range`, giving the camera position that looks at
/// the frame origin from `range` metres away at the requested heading and pitch.
fn offset_from_heading_pitch_range(heading: f64, pitch: f64, range: f64) -> Cartesian3 {
    let pitch = CesiumMath::clamp(pitch, -CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_TWO);
    let heading = CesiumMath::zero_to_two_pi(heading) - CesiumMath::PI_OVER_TWO;

    let pitch_quat = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Y, -pitch);
    let heading_quat = Quaternion::from_axis_angle_new(&Cartesian3::UNIT_Z, -heading);
    let rot_quat = Quaternion::multiply_new(&heading_quat, &pitch_quat);
    let rot_matrix = Matrix3::from_quaternion_new(&rot_quat);

    let offset = Matrix3::multiply_by_vector_new(&rot_matrix, &Cartesian3::UNIT_X);
    let offset = Cartesian3::negate_new(&offset);
    Cartesian3::multiply_by_scalar_new(&offset, range)
}

/// The `convert` step shared by `setViewCV` and `setView2D`:
/// `projection.project(ellipsoid.cartesianToCartographic(position))`.
///
/// CesiumJS reassigns the `position` parameter, which `cartesianToCartographic`
/// leaves untouched (returning `undefined`) when the point has no cartographic
/// image; the port keeps the original in that case.
fn project_destination(camera: &Camera, position: Cartesian3, convert: bool) -> Cartesian3 {
    if !convert {
        return position;
    }
    let projection = camera.map_projection();
    let mut cartographic = Cartographic::default();
    if projection
        .ellipsoid()
        .cartesian_to_cartographic(&position, &mut cartographic)
    {
        projection.project(&cartographic)
    } else {
        position
    }
}

/// The module-level `computeD(direction, upOrRight, corner, tanThetaOrPhi)` of
/// CesiumJS: how far back along `direction` the camera has to sit for `corner`
/// to fall inside the frustum plane spanned by `upOrRight`.
fn compute_d(
    direction: &Cartesian3,
    up_or_right: &Cartesian3,
    corner: &Cartesian3,
    tan_theta_or_phi: f64,
) -> f64 {
    let opposite = Cartesian3::dot(up_or_right, corner).abs();
    opposite / tan_theta_or_phi - Cartesian3::dot(direction, corner)
}

/// The module-level `computeHorizonQuad(camera, ellipsoid)` of CesiumJS's
/// `computeViewRectangle`: the four corners of the ellipsoid's horizon as seen
/// from `position_wc`, returned in `[upperLeft, lowerLeft, lowerRight,
/// upperRight]` order.
///
/// Only `camera.positionWC` is read, so the port passes it directly rather than
/// the whole camera.
fn compute_horizon_quad(position_wc: &Cartesian3, ellipsoid: &Ellipsoid) -> [Cartesian3; 4] {
    let radii = ellipsoid.radii();
    let p = *position_wc;

    // Find the corresponding position in the scaled space of the ellipsoid.
    let q = Cartesian3::multiply_components_new(ellipsoid.one_over_radii(), &p);

    let q_magnitude = Cartesian3::magnitude(&q);
    let q_unit = Cartesian3::normalize_new(&q);

    // Determine the east and north directions at q.
    let (e_unit, n_unit) = if Cartesian3::equals_epsilon(
        Some(&q_unit),
        Some(&Cartesian3::UNIT_Z),
        Some(CesiumMath::EPSILON10),
        None,
    ) {
        (Cartesian3::new(0.0, 1.0, 0.0), Cartesian3::new(0.0, 0.0, 1.0))
    } else {
        let e_unit =
            Cartesian3::normalize_new(&Cartesian3::cross_new(&Cartesian3::UNIT_Z, &q_unit));
        let n_unit = Cartesian3::normalize_new(&Cartesian3::cross_new(&q_unit, &e_unit));
        (e_unit, n_unit)
    };

    // Determine the radius of the 'limb' of the ellipsoid.
    let w_magnitude = (Cartesian3::magnitude_squared(&q) - 1.0).sqrt();

    // Compute the center and offsets.
    let center = Cartesian3::multiply_by_scalar_new(&q_unit, 1.0 / q_magnitude);
    let scalar = w_magnitude / q_magnitude;
    let east_offset = Cartesian3::multiply_by_scalar_new(&e_unit, scalar);
    let north_offset = Cartesian3::multiply_by_scalar_new(&n_unit, scalar);

    // A conservative measure for the longitudes would be to use the min/max
    // longitudes of the bounding frustum.
    let mut upper_left = Cartesian3::add_new(&center, &north_offset);
    upper_left = Cartesian3::subtract_new(&upper_left, &east_offset);
    upper_left = Cartesian3::multiply_components_new(radii, &upper_left);

    let mut lower_left = Cartesian3::subtract_new(&center, &north_offset);
    lower_left = Cartesian3::subtract_new(&lower_left, &east_offset);
    lower_left = Cartesian3::multiply_components_new(radii, &lower_left);

    let mut lower_right = Cartesian3::subtract_new(&center, &north_offset);
    lower_right = Cartesian3::add_new(&lower_right, &east_offset);
    lower_right = Cartesian3::multiply_components_new(radii, &lower_right);

    let mut upper_right = Cartesian3::add_new(&center, &north_offset);
    upper_right = Cartesian3::add_new(&upper_right, &east_offset);
    upper_right = Cartesian3::multiply_components_new(radii, &upper_right);

    [upper_left, lower_left, lower_right, upper_right]
}

/// The module-level `MINIMUM_ZOOM` of CesiumJS's `adjustBoundingSphereOffset`:
/// the range assigned when the bounding sphere is degenerate (radius zero).
const MINIMUM_ZOOM: f64 = 100.0;

/// The module-level `distanceToBoundingSphere3D(camera, radius)` of CesiumJS's
/// `adjustBoundingSphereOffset`: the distance at which a sphere of `radius`
/// exactly fills the perspective frustum, `max(radius/tanTheta, radius/tanPhi)`.
fn distance_to_bounding_sphere_3d(frustum: &CameraFrustum, radius: f64) -> f64 {
    // JS: `tanPhi = tan(frustum.fovy * 0.5); tanTheta = aspectRatio * tanPhi`.
    // Only ever called on the 3D perspective path, where both are defined.
    let fovy = frustum.fovy().unwrap_or(0.0);
    let aspect_ratio = frustum.aspect_ratio().unwrap_or(0.0);
    let tan_phi = (fovy * 0.5).tan();
    let tan_theta = aspect_ratio * tan_phi;
    (radius / tan_theta).max(radius / tan_phi)
}

/// The module-level `distanceToBoundingSphere2D(camera, radius)` of CesiumJS's
/// `adjustBoundingSphereOffset`: `1.5×` the larger of the width/height a sphere
/// of `radius` maps to under the orthographic frustum's aspect ratio.
fn distance_to_bounding_sphere_2d(frustum: &mut CameraFrustum, radius: f64) -> f64 {
    // JS reads `frustum.offCenterFrustum ?? frustum`, then its `right`/`top`;
    // `CameraFrustum::bounds` performs that same off-centre resolution.
    let (_left, right, top, _bottom) = frustum.bounds();
    let ratio = right / top;
    let height_ratio = radius * ratio;
    let (right, top) = if radius > height_ratio {
        (radius, radius / ratio)
    } else {
        (height_ratio, radius)
    };
    right.max(top) * 1.5
}

/// `ellipsoid.cartographicToCartesian(cartographic, result)` returning an owned
/// value at height zero; the port's `Ellipsoid` exposes no `_new` variant.
fn cartographic_to_cartesian_new(
    ellipsoid: &Ellipsoid,
    longitude: f64,
    latitude: f64,
) -> Cartesian3 {
    let mut result = Cartesian3::default();
    ellipsoid.cartographic_to_cartesian(
        &Cartographic::new(longitude, latitude, 0.0),
        &mut result,
    );
    result
}

/// The module-level `clampMove2D(camera, position)` of CesiumJS.
///
/// The `maxX`/`minX` naming is CesiumJS's own and reads backwards: when the map
/// is rotatable they are the symmetric `±maxProjectedX` bounds, and when it is
/// not they are derived from the position itself so that the two `x` comparisons
/// below can only ever pull the camera back to where it already is.
fn clamp_move_2d(rotatable_2d: bool, max_coord: &Cartesian3, position: &mut Cartesian3) {
    let max_projected_x = max_coord.x;
    let max_projected_y = max_coord.y;

    let min_x;
    let max_x;
    if rotatable_2d {
        max_x = max_projected_x;
        min_x = -max_x;
    } else {
        max_x = position.x - max_projected_x * 2.0;
        min_x = position.x + max_projected_x * 2.0;
    }

    if position.x > max_projected_x {
        position.x = max_x;
    }
    if position.x < -max_projected_x {
        position.x = min_x;
    }

    if position.y > max_projected_y {
        position.y = max_projected_y;
    }
    if position.y < -max_projected_y {
        position.y = -max_projected_y;
    }
}

/// `Cartesian3.cross` applied to `Cartesian4` scratch values: reads and writes
/// `x`/`y`/`z` only, leaving `w` untouched. Aliasing-safe, as the JS is.
fn xyz_cross(left: &Cartesian4, right: &Cartesian4, result: &mut Cartesian4) {
    let (lx, ly, lz) = (left.x, left.y, left.z);
    let (rx, ry, rz) = (right.x, right.y, right.z);
    result.x = ly * rz - lz * ry;
    result.y = lz * rx - lx * rz;
    result.z = lx * ry - ly * rx;
}

/// `Cartesian3.subtract` applied in place to a `Cartesian4`; `w` is untouched.
fn xyz_subtract_in_place(left: &mut Cartesian4, right: &Cartesian4) {
    left.x -= right.x;
    left.y -= right.y;
    left.z -= right.z;
}

/// `Cartesian3.magnitudeSquared` over the `x`/`y`/`z` of a `Cartesian4`.
fn xyz_magnitude_squared(cartesian: &Cartesian4) -> f64 {
    cartesian.x * cartesian.x + cartesian.y * cartesian.y + cartesian.z * cartesian.z
}

/// `Cartesian3.normalize` applied in place to a `Cartesian4`; `w` is untouched.
/// Like the JS, a zero-length vector divides by zero rather than being guarded.
fn xyz_normalize_in_place(cartesian: &mut Cartesian4) {
    let magnitude = xyz_magnitude_squared(cartesian).sqrt();
    cartesian.x /= magnitude;
    cartesian.y /= magnitude;
    cartesian.z /= magnitude;
}
