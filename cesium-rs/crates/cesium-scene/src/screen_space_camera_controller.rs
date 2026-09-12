//! Ported from `packages/engine/Source/Scene/ScreenSpaceCameraController.js`.
//!
//! Modifies the camera position and orientation based on mouse/touch/wheel
//! input to a canvas.
//!
//! # DEVIATION (scene back-reference → [`SsccSceneContext`])
//!
//! CesiumJS's controller holds `this._scene` and reaches through it for
//! `camera`, `globe`, `mode`, `mapProjection`, `mapMode2D`, `canvas`,
//! `globeHeight`, `pickPositionSupported`, `cameraUnderground`,
//! `verticalExaggeration`, and `ellipsoid`. The port has no scene
//! back-reference (the same pattern the [`crate::camera::Camera`] port uses),
//! so the [`crate::scene::Scene`] publishes those values each frame through a
//! [`SsccSceneContext`] borrow and calls [`ScreenSpaceCameraController::update`].
//! `this._globe` / `this._scene` are therefore *not* stored fields: `globe`
//! arrives as `Option<&mut Globe>` on the context, and `defined(this._globe)`
//! becomes `ctx.globe.is_some()`.
//!
//! # DEVIATION (dynamic member access → enums)
//!
//! CesiumJS keys inertia state by *string* (`"_lastInertiaSpinMovement"`),
//! looks the field up with `object[name]`, and consults an `_inertiaDisablers`
//! map. The port replaces the four strings with the [`InertiaState`] enum and
//! the disabler map with a `match` in
//! [`ScreenSpaceCameraController::activate_inertia`]. Likewise the `eventTypes`
//! union (`CameraEventType | {eventType, modifier} | array | undefined`) becomes
//! a `Vec<`[`CameraEventBinding`]`>` (empty ⇒ `undefined`/disabled), and the
//! `action` function references (`translate2D`, `zoom2D`, …) become the
//! [`SsccAction`] enum dispatched by
//! [`ScreenSpaceCameraController::run_action`].
//!
//! # DEVIATION (depth picking disabled)
//!
//! `pickPosition` reads `scene.pickPositionSupported` and, when true,
//! `scene.pickPositionWorldCoordinates`. The port's scene does not implement
//! depth-texture picking, so `SsccSceneContext::pick_position_supported` is
//! `false` and the depth-intersection branch is unreachable; `pickPosition`
//! returns the globe ray intersection only. Tracked in `docs/deviations.md`.
//!
//! # DEVIATION (`Date` → monotonic clock)
//!
//! `maintainInertia` stamps "now" with `new Date()`. The port uses
//! [`cesium_core::get_timestamp::get_timestamp`] (monotonic milliseconds), the
//! same substitution the [`crate::camera_event_aggregator::CameraEventAggregator`]
//! port makes for press/release times. CesiumJS only ever uses *differences*
//! of these stamps, so a monotonic clock is behaviourally identical and immune
//! to wall-clock jumps.
//!
//! # DEVIATION (live derived getters → explicit refresh)
//!
//! CesiumJS's `camera.positionCartographic` / `positionWC` / `directionWC`
//! getters run `updateMembers` on every access. The [`crate::camera::Camera`]
//! port caches them and refreshes in [`crate::camera::Camera::refresh`]. Where
//! this controller reads a derived getter after the camera may already have
//! moved earlier in the same frame, it calls `ctx.camera.refresh()` first
//! (e.g. at the top of `handle_zoom` and around the `update` dispatch) so the
//! observed value matches the CesiumJS live getter.
//!
//! Staging note: the 2D/Columbus-view/3D motion families (`translate2D`,
//! `zoom2D`, `rotateCV`, `spin3D`, …), `update2D/CV/3D`, and
//! `adjustHeightForTerrain` are landed by the follow-up batches (b3-3d/e); they
//! are present here as compiling stubs so the foundation (constructor state,
//! event registry, inertia, `reactToInput`, `handleZoom`, `pickPosition`, and
//! the underground distance helpers) builds green on its own. The module-level
//! `allow(dead_code)` is removed once they are wired.
#![allow(dead_code)]

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartesian4::Cartesian4;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::geographic_projection::GeographicProjection;
use cesium_core::get_timestamp::get_timestamp;
use cesium_core::intersection_tests::IntersectionTests;
use cesium_core::keyboard_event_modifier::KeyboardEventModifier;
use cesium_core::map_projection::MapProjection;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::plane::Plane;
use cesium_core::quaternion::Quaternion;
use cesium_core::ray::Ray;
use cesium_core::scene_mode::SceneMode;
use cesium_core::transforms::east_north_up_to_fixed_frame_new;
use cesium_core::vertical_exaggeration::VerticalExaggeration;

use crate::camera::{Camera, SetViewOptions, SetViewOrientation};
use crate::camera_event_aggregator::{
    CameraEventAggregator, MouseMovement, Movement, PinchMovement,
};
use crate::camera_event_type::CameraEventType;
use crate::globe::Globe;
use crate::map_mode2_d::MapMode2D;
use crate::scene_transforms::SceneTransforms;
use crate::tween_collection::TweenCollection;

/// If the time between mouse down and mouse up is not between these thresholds,
/// the camera will not move with inertia.
///
/// CesiumJS `inertiaMaxClickTimeThreshold`.
const INERTIA_MAX_CLICK_TIME_THRESHOLD: f64 = 0.4;

/// `decay(time, coefficient)`.
///
/// Returns the value of a decreasing exponential used to taper inertial motion.
fn decay(time: f64, coefficient: f64) -> f64 {
    if time < 0.0 {
        return 0.0;
    }
    let tau = (1.0 - coefficient) * 25.0;
    (-tau * time).exp()
}

/// `sameMousePosition(movement)`.
///
/// CesiumJS reads `movement.startPosition`/`movement.endPosition`; the port
/// takes the two positions directly so it works for both an aggregator
/// [`crate::camera_event_aggregator::LastMovement`] and an
/// [`InertiaMovementState`].
fn same_mouse_position(start_position: &Cartesian2, end_position: &Cartesian2) -> bool {
    Cartesian2::equals_epsilon(
        Some(start_position),
        Some(end_position),
        Some(CesiumMath::EPSILON14),
        None,
    )
}

/// One entry of a CesiumJS `*EventTypes` value.
///
/// CesiumJS allows each entry to be a bare [`CameraEventType`] or an object
/// `{ eventType, modifier }`. The port normalises both to this struct: a bare
/// type has empty `modifiers`, and `{ eventType, modifier }` has one. A whole
/// `*EventTypes` value is a `Vec<CameraEventBinding>` (empty ⇒ `undefined`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraEventBinding {
    /// The `eventType` (or the bare type).
    pub event_type: CameraEventType,
    /// The `modifier`, if any (CesiumJS allows at most one).
    pub modifiers: Vec<KeyboardEventModifier>,
}

impl CameraEventBinding {
    /// A bare `CameraEventType` entry (no modifier).
    pub fn plain(event_type: CameraEventType) -> Self {
        Self {
            event_type,
            modifiers: Vec::new(),
        }
    }

    /// An `{ eventType, modifier }` entry.
    pub fn with_modifier(event_type: CameraEventType, modifier: KeyboardEventModifier) -> Self {
        Self {
            event_type,
            modifiers: vec![modifier],
        }
    }
}

/// The four inertia movement states, replacing CesiumJS's string field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InertiaState {
    /// `_lastInertiaSpinMovement`.
    Spin,
    /// `_lastInertiaZoomMovement`.
    Zoom,
    /// `_lastInertiaTranslateMovement`.
    Translate,
    /// `_lastInertiaTiltMovement`.
    Tilt,
}

/// The `{ startPosition, endPosition, motion, inertiaEnabled }` object CesiumJS
/// stores under each `_lastInertia*Movement` field and hands to an action in
/// place of a live movement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertiaMovementState {
    /// `startPosition`.
    pub start_position: Cartesian2,
    /// `endPosition`.
    pub end_position: Cartesian2,
    /// `motion` — half of the last movement's delta.
    pub motion: Cartesian2,
    /// `inertiaEnabled`.
    pub inertia_enabled: bool,
}

/// The `movement` argument passed to an action.
///
/// CesiumJS hands an action either a live aggregator movement (mouse- or
/// pinch-shaped) or, from `maintainInertia`, an [`InertiaMovementState`]. The
/// actions duck-type on `defined(movement.distance)` / `.angleAndHeight` /
/// `.inertiaEnabled`; this enum makes those three shapes explicit.
#[derive(Debug, Clone, Copy)]
pub enum MovementInput {
    /// A mouse-drag or wheel movement `{ startPosition, endPosition }`.
    Mouse(MouseMovement),
    /// A pinch movement `{ distance, angleAndHeight, prevAngle }`.
    Pinch(PinchMovement),
    /// An inertia movement state.
    Inertia(InertiaMovementState),
}

impl MovementInput {
    /// Builds the input from a live aggregator [`Movement`].
    pub fn from_aggregator(movement: &Movement) -> Self {
        match movement {
            Movement::Mouse(mouse) => MovementInput::Mouse(*mouse),
            Movement::Pinch(pinch) => MovementInput::Pinch(*pinch),
        }
    }

    /// `movement.startPosition` — `None` for a pinch (which has no top-level
    /// start/end, only `distance`/`angleAndHeight`).
    pub fn start_position(&self) -> Option<Cartesian2> {
        match self {
            MovementInput::Mouse(mouse) => Some(mouse.start_position),
            MovementInput::Inertia(state) => Some(state.start_position),
            MovementInput::Pinch(_) => None,
        }
    }

    /// `movement.endPosition` — `None` for a pinch.
    pub fn end_position(&self) -> Option<Cartesian2> {
        match self {
            MovementInput::Mouse(mouse) => Some(mouse.end_position),
            MovementInput::Inertia(state) => Some(state.end_position),
            MovementInput::Pinch(_) => None,
        }
    }

    /// `movement.distance` — `Some` only for a pinch.
    pub fn distance(&self) -> Option<MouseMovement> {
        match self {
            MovementInput::Pinch(pinch) => Some(pinch.distance),
            _ => None,
        }
    }

    /// `movement.angleAndHeight` — `Some` only for a pinch.
    pub fn angle_and_height(&self) -> Option<MouseMovement> {
        match self {
            MovementInput::Pinch(pinch) => Some(pinch.angle_and_height),
            _ => None,
        }
    }

    /// `movement.inertiaEnabled` — `Some` only for an inertia state.
    pub fn inertia_enabled(&self) -> Option<bool> {
        match self {
            MovementInput::Inertia(state) => Some(state.inertia_enabled),
            _ => None,
        }
    }
}

/// The subset of a `movement` that [`ScreenSpaceCameraController::handle_zoom`]
/// consumes, after a zoom action has flattened a pinch to its `distance`
/// sub-movement (CesiumJS `zoom2D`/`zoom3D`/`zoomCV` rebind
/// `movement = movement.distance`).
#[derive(Debug, Clone, Copy)]
pub struct ZoomMovement {
    /// `movement.startPosition`.
    pub start_position: Cartesian2,
    /// `movement.endPosition`.
    pub end_position: Cartesian2,
    /// `movement.inertiaEnabled` (`None` ⇒ fall back to the `_zoomMouseStart`
    /// equality test).
    pub inertia_enabled: Option<bool>,
}

/// The `action` function references CesiumJS passes to `reactToInput`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SsccAction {
    Translate2D,
    Zoom2D,
    Twist2D,
    TranslateCv,
    RotateCv,
    ZoomCv,
    Spin3D,
    Rotate3D,
    Zoom3D,
    Tilt3D,
    Look3D,
}

/// The per-frame scene state the controller reads, published by
/// [`crate::scene::Scene`] (see the module DEVIATION note).
///
/// The borrows are disjoint fields of the scene, so `Scene::update` can build
/// this from `&mut self.camera`, `self.globe.as_mut()`, and
/// `&*self.map_projection` simultaneously.
pub struct SsccSceneContext<'a> {
    /// `scene.camera`.
    pub camera: &'a mut Camera,
    /// `scene.globe` (the controller's `_globe`; `update` clears it when the
    /// camera transform is not the identity).
    pub globe: Option<&'a mut Globe>,
    /// `scene.mode`.
    pub mode: SceneMode,
    /// `scene.mapProjection` (also the source of `scene.ellipsoid`).
    pub map_projection: &'a dyn MapProjection,
    /// `scene.mapMode2D`.
    pub map_mode_2d: MapMode2D,
    /// `scene.canvas.clientWidth`.
    pub canvas_client_width: f64,
    /// `scene.canvas.clientHeight`.
    pub canvas_client_height: f64,
    /// `scene.globeHeight`.
    pub globe_height: Option<f64>,
    /// `scene.pickPositionSupported` (`false` in the port).
    pub pick_position_supported: bool,
    /// `scene.cameraUnderground`.
    pub camera_underground: bool,
    /// `scene.verticalExaggeration`.
    pub vertical_exaggeration: f64,
    /// `scene.verticalExaggerationRelativeHeight`.
    pub vertical_exaggeration_relative_height: f64,
}

/// Modifies the camera position and orientation based on mouse input to a
/// canvas.
///
/// Port of `ScreenSpaceCameraController`. Field names mirror the CesiumJS
/// properties; private (`_`-prefixed in JS) state keeps the underscore so the
/// public/private pairs (`minimumPickingTerrainHeight` vs
/// `_minimumPickingTerrainHeight`) stay distinguishable.
pub struct ScreenSpaceCameraController {
    // ---- Public configuration ----
    /// `enableInputs` (default `true`).
    pub enable_inputs: bool,
    /// `enableTranslate` (default `true`).
    pub enable_translate: bool,
    /// `enableZoom` (default `true`).
    pub enable_zoom: bool,
    /// `enableRotate` (default `true`).
    pub enable_rotate: bool,
    /// `enableTilt` (default `true`).
    pub enable_tilt: bool,
    /// `enableLook` (default `true`).
    pub enable_look: bool,
    /// `inertiaSpin` (default `0.9`).
    pub inertia_spin: f64,
    /// `inertiaTranslate` (default `0.9`).
    pub inertia_translate: f64,
    /// `inertiaZoom` (default `0.8`).
    pub inertia_zoom: f64,
    /// `maximumMovementRatio` (default `0.1`).
    pub maximum_movement_ratio: f64,
    /// `bounceAnimationTime` (default `3.0`).
    pub bounce_animation_time: f64,
    /// `minimumZoomDistance` (default `1.0`).
    pub minimum_zoom_distance: f64,
    /// `maximumZoomDistance` (default `+inf`).
    pub maximum_zoom_distance: f64,
    /// `zoomFactor` (default `5.0`).
    pub zoom_factor: f64,
    /// `translateEventTypes` (default `LEFT_DRAG`).
    pub translate_event_types: Vec<CameraEventBinding>,
    /// `zoomEventTypes` (default `[RIGHT_DRAG, WHEEL, PINCH]`).
    pub zoom_event_types: Vec<CameraEventBinding>,
    /// `rotateEventTypes` (default `LEFT_DRAG`).
    pub rotate_event_types: Vec<CameraEventBinding>,
    /// `tiltEventTypes` (default `[MIDDLE_DRAG, PINCH, LEFT_DRAG+CTRL,
    /// RIGHT_DRAG+CTRL]`).
    pub tilt_event_types: Vec<CameraEventBinding>,
    /// `lookEventTypes` (default `LEFT_DRAG+SHIFT`).
    pub look_event_types: Vec<CameraEventBinding>,
    /// `minimumPickingTerrainHeight`.
    pub minimum_picking_terrain_height: f64,
    /// `minimumPickingTerrainDistanceWithInertia`.
    pub minimum_picking_terrain_distance_with_inertia: f64,
    /// `minimumCollisionTerrainHeight`.
    pub minimum_collision_terrain_height: f64,
    /// `minimumTrackBallHeight`.
    pub minimum_track_ball_height: f64,
    /// `enableCollisionDetection` (default `true`).
    pub enable_collision_detection: bool,
    /// `maximumTiltAngle` (default `undefined`).
    pub maximum_tilt_angle: Option<f64>,

    // ---- Private state ----
    /// `_minimumPickingTerrainHeight` (exaggeration-adjusted in `update`).
    _minimum_picking_terrain_height: f64,
    /// `_minimumCollisionTerrainHeight` (exaggeration-adjusted in `update`).
    _minimum_collision_terrain_height: f64,
    /// `_minimumTrackBallHeight` (exaggeration-adjusted in `update`).
    _minimum_track_ball_height: f64,
    /// `_ellipsoid` — swapped to `UNIT_SPHERE` while a transform is set.
    _ellipsoid: Ellipsoid,
    /// `_lastGlobeHeight`.
    _last_globe_height: f64,
    /// `_aggregator`.
    _aggregator: CameraEventAggregator,
    /// `_lastInertiaSpinMovement`.
    _last_inertia_spin_movement: Option<InertiaMovementState>,
    /// `_lastInertiaZoomMovement`.
    _last_inertia_zoom_movement: Option<InertiaMovementState>,
    /// `_lastInertiaTranslateMovement`.
    _last_inertia_translate_movement: Option<InertiaMovementState>,
    /// `_lastInertiaTiltMovement`.
    _last_inertia_tilt_movement: Option<InertiaMovementState>,
    /// `_tweens` (Columbus-view bounce animations; wired in b3-3d).
    _tweens: TweenCollection,
    /// `_horizontalRotationAxis`.
    _horizontal_rotation_axis: Option<Cartesian3>,
    /// `_tiltCenterMousePosition`.
    _tilt_center_mouse_position: Cartesian2,
    /// `_tiltCenter`.
    _tilt_center: Cartesian3,
    /// `_rotateMousePosition`.
    _rotate_mouse_position: Cartesian2,
    /// `_rotateStartPosition`.
    _rotate_start_position: Cartesian3,
    /// `_strafeStartPosition`.
    _strafe_start_position: Cartesian3,
    /// `_strafeMousePosition`.
    _strafe_mouse_position: Cartesian2,
    /// `_strafeEndMousePosition`.
    _strafe_end_mouse_position: Cartesian2,
    /// `_zoomMouseStart`.
    _zoom_mouse_start: Cartesian2,
    /// `_zoomWorldPosition`.
    _zoom_world_position: Cartesian3,
    /// `_useZoomWorldPosition`.
    _use_zoom_world_position: bool,
    /// CesiumJS module-global `preIntersectionDistance` (read/written by
    /// `zoom3D`). DEVIATION: the port makes it a per-controller field rather
    /// than a process-wide mutable global; with a single controller the two are
    /// indistinguishable, and a field avoids `unsafe`/`thread_local` state.
    _pre_intersection_distance: f64,
    /// `_panLastMousePosition`.
    _pan_last_mouse_position: Cartesian2,
    /// `_panLastWorldPosition`.
    _pan_last_world_position: Cartesian3,
    /// `_translateMousePosition`. DEVIATION: CesiumJS never initialises it in
    /// the constructor (it reads as `undefined` until `translateCV`'s
    /// look-fallback clones into it), so the port models it as an `Option`.
    /// `None` keeps `!Cartesian2.equals(startPosition, undefined)` permanently
    /// true (⇒ `_looking = false` on every fresh drag), matching JS.
    _translate_mouse_position: Option<Cartesian2>,
    /// `_tiltCVOffMap`.
    _tilt_cv_off_map: bool,
    /// `_tiltOnEllipsoid`. CesiumJS never initialises it in the constructor (it
    /// reads as `undefined`/false until `tilt3D` assigns it), so the port
    /// defaults it to `false`.
    _tilt_on_ellipsoid: bool,
    /// `_looking`.
    _looking: bool,
    /// `_rotating`.
    _rotating: bool,
    /// `_strafing`.
    _strafing: bool,
    /// `_zoomingOnVector`.
    _zooming_on_vector: bool,
    /// `_zoomingUnderground`.
    _zooming_underground: bool,
    /// `_rotatingZoom`.
    _rotating_zoom: bool,
    /// `_adjustedHeightForTerrain`.
    _adjusted_height_for_terrain: bool,
    /// `_cameraUnderground`.
    _camera_underground: bool,
    /// `_maxCoord` — `mapProjection.project(Cartographic(π, π/2))`.
    _max_coord: Cartesian3,
    /// `_rotateFactor`.
    _rotate_factor: f64,
    /// `_rotateRateRangeAdjustment`.
    _rotate_rate_range_adjustment: f64,
    /// `_maximumRotateRate`.
    _maximum_rotate_rate: f64,
    /// `_minimumRotateRate`.
    _minimum_rotate_rate: f64,
    /// `_minimumZoomRate`.
    _minimum_zoom_rate: f64,
    /// `_maximumZoomRate` — the Sun-to-Pluto distance in metres.
    _maximum_zoom_rate: f64,
    /// `_minimumUndergroundPickDistance`.
    _minimum_underground_pick_distance: f64,
    /// `_maximumUndergroundPickDistance`.
    _maximum_underground_pick_distance: f64,
    /// Destroyed flag.
    is_destroyed: bool,
}

impl ScreenSpaceCameraController {
    /// Creates a controller.
    ///
    /// `map_projection` supplies both `_maxCoord` and the default `_ellipsoid`
    /// (`scene.ellipsoid` is `scene.mapProjection.ellipsoid` in CesiumJS);
    /// `canvas_client_width` seeds the [`CameraEventAggregator`].
    pub fn new(map_projection: &dyn MapProjection, canvas_client_width: f64) -> Self {
        let ellipsoid = map_projection.ellipsoid();
        let is_wgs84 = Ellipsoid::WGS84.equals(ellipsoid);
        let minimum_radius = ellipsoid.minimum_radius();

        let minimum_picking_terrain_height = if is_wgs84 {
            150000.0
        } else {
            minimum_radius * 0.025
        };
        let minimum_picking_terrain_distance_with_inertia = if is_wgs84 {
            4000.0
        } else {
            minimum_radius * 0.00063
        };
        let minimum_collision_terrain_height = if is_wgs84 {
            15000.0
        } else {
            minimum_radius * 0.0025
        };
        let minimum_track_ball_height = if is_wgs84 {
            7500000.0
        } else {
            minimum_radius * 1.175
        };

        let max_coord = map_projection.project(&Cartographic::new(
            CesiumMath::PI,
            CesiumMath::PI_OVER_TWO,
            0.0,
        ));

        Self {
            enable_inputs: true,
            enable_translate: true,
            enable_zoom: true,
            enable_rotate: true,
            enable_tilt: true,
            enable_look: true,
            inertia_spin: 0.9,
            inertia_translate: 0.9,
            inertia_zoom: 0.8,
            maximum_movement_ratio: 0.1,
            bounce_animation_time: 3.0,
            minimum_zoom_distance: 1.0,
            maximum_zoom_distance: f64::INFINITY,
            zoom_factor: 5.0,
            translate_event_types: vec![CameraEventBinding::plain(CameraEventType::LeftDrag)],
            zoom_event_types: vec![
                CameraEventBinding::plain(CameraEventType::RightDrag),
                CameraEventBinding::plain(CameraEventType::Wheel),
                CameraEventBinding::plain(CameraEventType::Pinch),
            ],
            rotate_event_types: vec![CameraEventBinding::plain(CameraEventType::LeftDrag)],
            tilt_event_types: vec![
                CameraEventBinding::plain(CameraEventType::MiddleDrag),
                CameraEventBinding::plain(CameraEventType::Pinch),
                CameraEventBinding::with_modifier(CameraEventType::LeftDrag, KeyboardEventModifier::Ctrl),
                CameraEventBinding::with_modifier(CameraEventType::RightDrag, KeyboardEventModifier::Ctrl),
            ],
            look_event_types: vec![CameraEventBinding::with_modifier(
                CameraEventType::LeftDrag,
                KeyboardEventModifier::Shift,
            )],
            minimum_picking_terrain_height,
            minimum_picking_terrain_distance_with_inertia,
            minimum_collision_terrain_height,
            minimum_track_ball_height,
            enable_collision_detection: true,
            maximum_tilt_angle: None,

            _minimum_picking_terrain_height: minimum_picking_terrain_height,
            _minimum_collision_terrain_height: minimum_collision_terrain_height,
            _minimum_track_ball_height: minimum_track_ball_height,
            _ellipsoid: ellipsoid.clone(),
            _last_globe_height: 0.0,
            _aggregator: CameraEventAggregator::new(canvas_client_width),
            _last_inertia_spin_movement: None,
            _last_inertia_zoom_movement: None,
            _last_inertia_translate_movement: None,
            _last_inertia_tilt_movement: None,
            _tweens: TweenCollection::new(),
            _horizontal_rotation_axis: None,
            _tilt_center_mouse_position: Cartesian2::new(-1.0, -1.0),
            _tilt_center: Cartesian3::default(),
            _rotate_mouse_position: Cartesian2::new(-1.0, -1.0),
            _rotate_start_position: Cartesian3::default(),
            _strafe_start_position: Cartesian3::default(),
            _strafe_mouse_position: Cartesian2::default(),
            _strafe_end_mouse_position: Cartesian2::default(),
            _zoom_mouse_start: Cartesian2::new(-1.0, -1.0),
            _zoom_world_position: Cartesian3::default(),
            _use_zoom_world_position: false,
            _pre_intersection_distance: 0.0,
            _pan_last_mouse_position: Cartesian2::default(),
            _pan_last_world_position: Cartesian3::default(),
            _translate_mouse_position: None,
            _tilt_cv_off_map: false,
            _tilt_on_ellipsoid: false,
            _looking: false,
            _rotating: false,
            _strafing: false,
            _zooming_on_vector: false,
            _zooming_underground: false,
            _rotating_zoom: false,
            _adjusted_height_for_terrain: false,
            _camera_underground: false,
            _max_coord: max_coord,
            _rotate_factor: 0.0,
            _rotate_rate_range_adjustment: 0.0,
            _maximum_rotate_rate: 1.77,
            _minimum_rotate_rate: 1.0 / 5000.0,
            _minimum_zoom_rate: 20.0,
            _maximum_zoom_rate: 5906376272000.0,
            _minimum_underground_pick_distance: 2000.0,
            _maximum_underground_pick_distance: 10000.0,
            is_destroyed: false,
        }
    }

    /// The mutable input aggregator, so the driver can feed it raw events.
    pub fn aggregator_mut(&mut self) -> &mut CameraEventAggregator {
        &mut self._aggregator
    }

    /// The input aggregator.
    pub fn aggregator(&self) -> &CameraEventAggregator {
        &self._aggregator
    }

    /// `ScreenSpaceCameraController.prototype.update`.
    ///
    /// Resolves the globe/ellipsoid against the camera transform, applies
    /// vertical exaggeration to the terrain-height thresholds, recomputes the
    /// rotate factor, dispatches to the per-mode handler, and finally runs the
    /// terrain collision adjustment and resets the aggregator.
    pub fn update(&mut self, ctx: &mut SsccSceneContext) {
        // `if (!Matrix4.equals(camera.transform, Matrix4.IDENTITY)) { _globe =
        // undefined; _ellipsoid = UNIT_SPHERE } else { _globe = globe; _ellipsoid
        // = scene.ellipsoid ?? default }`.
        if !Matrix4::equals(ctx.camera.transform(), &Matrix4::IDENTITY) {
            ctx.globe = None;
            self._ellipsoid = Ellipsoid::UNIT_SPHERE;
        } else {
            self._ellipsoid = ctx.map_projection.ellipsoid().clone();
        }

        // `VerticalExaggeration.getHeight(...)` for the three thresholds.
        self._minimum_collision_terrain_height = VerticalExaggeration::get_height(
            self.minimum_collision_terrain_height,
            ctx.vertical_exaggeration,
            ctx.vertical_exaggeration_relative_height,
        );
        self._minimum_picking_terrain_height = VerticalExaggeration::get_height(
            self.minimum_picking_terrain_height,
            ctx.vertical_exaggeration,
            ctx.vertical_exaggeration_relative_height,
        );
        self._minimum_track_ball_height = VerticalExaggeration::get_height(
            self.minimum_track_ball_height,
            ctx.vertical_exaggeration,
            ctx.vertical_exaggeration_relative_height,
        );

        // `_cameraUnderground = scene.cameraUnderground && defined(_globe)`.
        self._camera_underground = ctx.camera_underground && ctx.globe.is_some();

        // `_rotateFactor = 1 / radius; _rotateRateRangeAdjustment = radius`.
        let radius = self._ellipsoid.maximum_radius();
        self._rotate_factor = 1.0 / radius;
        self._rotate_rate_range_adjustment = radius;

        self._adjusted_height_for_terrain = false;

        // `previousPosition`/`previousDirection` — refreshed so the comparison
        // sees live world state (see the module DEVIATION note).
        ctx.camera.refresh();
        let previous_position = *ctx.camera.position_wc();
        let previous_direction = *ctx.camera.direction_wc();

        match ctx.mode {
            SceneMode::Scene2D => self.update_2d(ctx),
            SceneMode::ColumbusView => {
                self._horizontal_rotation_axis = Some(Cartesian3::UNIT_Z);
                self.update_cv(ctx);
            }
            SceneMode::Scene3D => {
                self._horizontal_rotation_axis = None;
                self.update_3d(ctx);
            }
            SceneMode::Morphing => {}
        }

        if self.enable_collision_detection && !self._adjusted_height_for_terrain {
            ctx.camera.refresh();
            let camera_changed = !Cartesian3::equals(
                Some(&previous_position),
                Some(ctx.camera.position_wc()),
            ) || !Cartesian3::equals(
                Some(&previous_direction),
                Some(ctx.camera.direction_wc()),
            );
            self.adjust_height_for_terrain(ctx, camera_changed);
        }

        self._aggregator.reset();
    }

    /// `ScreenSpaceCameraController.prototype.onMap`.
    pub fn on_map(&self, ctx: &SsccSceneContext) -> bool {
        if ctx.mode == SceneMode::ColumbusView {
            let position = ctx.camera.position();
            return position.x.abs() - self._max_coord.x < 0.0
                && position.y.abs() - self._max_coord.y < 0.0;
        }
        true
    }

    /// Returns whether this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Removes listeners held by this object and destroys the aggregator.
    pub fn destroy(&mut self) {
        self._tweens.remove_all();
        self._aggregator.destroy();
        self.is_destroyed = true;
    }
}

impl Default for ScreenSpaceCameraController {
    fn default() -> Self {
        // CesiumJS always constructs from a scene; the port's `Default` uses the
        // same WGS84 `GeographicProjection` the `Scene`/`Camera` defaults use.
        let projection = GeographicProjection::new(None);
        Self::new(&projection, 0.0)
    }
}

impl ScreenSpaceCameraController {
    // ---- Inertia plumbing ----

    /// `controller[lastMovementName]` read — the stored movement state for an
    /// [`InertiaState`], replacing CesiumJS's string-keyed member lookup.
    fn inertia_state_ref(&self, state: InertiaState) -> Option<&InertiaMovementState> {
        match state {
            InertiaState::Spin => self._last_inertia_spin_movement.as_ref(),
            InertiaState::Zoom => self._last_inertia_zoom_movement.as_ref(),
            InertiaState::Translate => self._last_inertia_translate_movement.as_ref(),
            InertiaState::Tilt => self._last_inertia_tilt_movement.as_ref(),
        }
    }

    /// `controller[lastMovementName]` write handle.
    fn inertia_state_mut(&mut self, state: InertiaState) -> &mut Option<InertiaMovementState> {
        match state {
            InertiaState::Spin => &mut self._last_inertia_spin_movement,
            InertiaState::Zoom => &mut self._last_inertia_zoom_movement,
            InertiaState::Translate => &mut self._last_inertia_translate_movement,
            InertiaState::Tilt => &mut self._last_inertia_tilt_movement,
        }
    }

    /// `activateInertia(controller, inertiaStateName)`.
    ///
    /// Re-enables inertia on `state` and disables it on the states listed in
    /// CesiumJS's `_inertiaDisablers` map: `Zoom` disables
    /// `[Spin, Translate, Tilt]`, `Tilt` disables `[Spin, Translate]`, and
    /// `Spin`/`Translate` disable nothing. `None` (CesiumJS `undefined`, e.g.
    /// `look3D`) is a no-op.
    fn activate_inertia(&mut self, state: Option<InertiaState>) {
        let state = match state {
            Some(state) => state,
            None => return,
        };

        if let Some(movement_state) = self.inertia_state_mut(state).as_mut() {
            movement_state.inertia_enabled = true;
        }

        let disablers: &[InertiaState] = match state {
            InertiaState::Zoom => &[InertiaState::Spin, InertiaState::Translate, InertiaState::Tilt],
            InertiaState::Tilt => &[InertiaState::Spin, InertiaState::Translate],
            InertiaState::Spin | InertiaState::Translate => &[],
        };
        for &other in disablers {
            if let Some(movement_state) = self.inertia_state_mut(other).as_mut() {
                movement_state.inertia_enabled = false;
            }
        }
    }

    /// `maintainInertia(aggregator, type, modifier, decayCoef, action, object,
    /// lastMovementName)`.
    ///
    /// Tapers the last movement with the [`decay`] exponential and, while the
    /// button is up, keeps feeding it to `action` so the camera coasts to a
    /// stop. The stored [`InertiaMovementState`] is mutated in place exactly as
    /// CesiumJS mutates `movementState`, then snapshotted (it is `Copy`) so the
    /// `&mut self` borrow is released before `action` runs.
    fn maintain_inertia(
        &mut self,
        ctx: &mut SsccSceneContext,
        type_: CameraEventType,
        modifiers: &[KeyboardEventModifier],
        decay_coef: f64,
        action: SsccAction,
        state: InertiaState,
    ) {
        // `movementState = object[name] ?? (object[name] = { ... })`.
        if self.inertia_state_ref(state).is_none() {
            *self.inertia_state_mut(state) = Some(InertiaMovementState {
                start_position: Cartesian2::default(),
                end_position: Cartesian2::default(),
                motion: Cartesian2::default(),
                inertia_enabled: true,
            });
        }

        // `const threshold = ts && tr && (tr - ts) / 1000`. A missing press or
        // release time makes the whole `if (ts && tr && ...)` guard false.
        let (ts, tr) = match (
            self._aggregator.get_button_press_time(type_, modifiers),
            self._aggregator.get_button_release_time(type_, modifiers),
        ) {
            (Some(ts), Some(tr)) => (ts, tr),
            _ => return,
        };
        let threshold = (tr - ts) / 1000.0;
        if threshold >= INERTIA_MAX_CLICK_TIME_THRESHOLD {
            return;
        }

        // `now`/`fromNow` — see the module DEVIATION note (`Date` → monotonic).
        let now = get_timestamp();
        let from_now = (now - tr) / 1000.0;
        let d = decay(from_now, decay_coef);

        let last_movement = match self._aggregator.get_last_movement(type_, modifiers) {
            Some(last_movement) => *last_movement,
            None => return,
        };
        if same_mouse_position(&last_movement.start_position, &last_movement.end_position) {
            return;
        }

        // Copy the state out so the mutation and the later `&mut self` action
        // call do not overlap borrows.
        let mut movement_state = match self.inertia_state_ref(state) {
            Some(movement_state) => *movement_state,
            None => return,
        };
        if !movement_state.inertia_enabled {
            return;
        }

        movement_state.motion.x = (last_movement.end_position.x - last_movement.start_position.x) * 0.5;
        movement_state.motion.y = (last_movement.end_position.y - last_movement.start_position.y) * 0.5;
        movement_state.start_position = last_movement.start_position;
        let scaled = Cartesian2::multiply_by_scalar_new(&movement_state.motion, d);
        movement_state.end_position = Cartesian2::add_new(&movement_state.start_position, &scaled);

        // Write the mutation back — CesiumJS mutates the stored object in place,
        // and it stays mutated even when the guard below returns early.
        *self.inertia_state_mut(state) = Some(movement_state);

        // A near-zero exponential can produce NaN end coordinates.
        if movement_state.end_position.x.is_nan()
            || movement_state.end_position.y.is_nan()
            || Cartesian2::distance(&movement_state.start_position, &movement_state.end_position) < 0.5
        {
            return;
        }

        if !self._aggregator.is_button_down(type_, modifiers) {
            let start_position = self._aggregator.get_start_mouse_position(type_, modifiers);
            let inertia_input = MovementInput::Inertia(movement_state);
            self.run_action(ctx, action, start_position, &inertia_input);
        }
    }

    /// `reactToInput(controller, enabled, eventTypes, action, inertiaConstant,
    /// inertiaStateName)`.
    ///
    /// For each binding in `eventTypes`, runs `action` on the live aggregated
    /// movement (and re-activates inertia), or — when there is no movement and
    /// `inertiaConstant < 1.0` — coasts via [`Self::maintain_inertia`].
    /// `inertia` is `None` for `look3D`, whose CesiumJS `inertiaConstant` is
    /// `undefined` (so `undefined < 1.0` is false and `activateInertia` no-ops).
    ///
    /// The bindings are taken by value (callers `.clone()` the registry `Vec`)
    /// and the movement is copied out of the aggregator, so the `&mut self`
    /// borrow held by `run_action` does not overlap them.
    fn react_to_input(
        &mut self,
        ctx: &mut SsccSceneContext,
        enabled: bool,
        event_types: Vec<CameraEventBinding>,
        action: SsccAction,
        inertia: Option<(f64, InertiaState)>,
    ) {
        for binding in event_types {
            let type_ = binding.event_type;
            let modifiers = &binding.modifiers[..];

            // `isMoving(type, modifier) && getMovement(type, modifier)`.
            let movement = if self._aggregator.is_moving(type_, modifiers) {
                self._aggregator.get_movement(type_, modifiers).copied()
            } else {
                None
            };
            let start_position = self._aggregator.get_start_mouse_position(type_, modifiers);

            if self.enable_inputs && enabled {
                if let Some(movement) = movement {
                    let input = MovementInput::from_aggregator(&movement);
                    self.run_action(ctx, action, start_position, &input);
                    self.activate_inertia(inertia.map(|(_, state)| state));
                } else if let Some((constant, state)) = inertia {
                    if constant < 1.0 {
                        self.maintain_inertia(ctx, type_, modifiers, constant, action, state);
                    }
                }
            }
        }
    }

    /// Dispatches an [`SsccAction`] to its motion handler, replacing CesiumJS's
    /// `action(controller, startPosition, movement)` function reference.
    fn run_action(
        &mut self,
        ctx: &mut SsccSceneContext,
        action: SsccAction,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        match action {
            SsccAction::Translate2D => self.translate_2d(ctx, start_position, movement),
            SsccAction::Zoom2D => self.zoom_2d(ctx, start_position, movement),
            SsccAction::Twist2D => self.twist_2d(ctx, start_position, movement),
            SsccAction::TranslateCv => self.translate_cv(ctx, start_position, movement),
            SsccAction::RotateCv => self.rotate_cv(ctx, start_position, movement),
            SsccAction::ZoomCv => self.zoom_cv(ctx, start_position, movement),
            SsccAction::Spin3D => self.spin_3d(ctx, start_position, movement),
            SsccAction::Rotate3D => self.rotate_3d(ctx, start_position, movement),
            SsccAction::Zoom3D => self.zoom_3d(ctx, start_position, movement),
            SsccAction::Tilt3D => self.tilt_3d(ctx, start_position, movement),
            SsccAction::Look3D => self.look_3d(ctx, start_position, movement),
        }
    }

    // ---- Picking / underground distance helpers ----

    /// `pickPosition(controller, mousePosition, result)`.
    ///
    /// DEVIATION: `scene.pickPositionSupported` is `false` in the port (no
    /// depth-texture picking), so `depthIntersection` is always `undefined` and
    /// the `pickDistance < rayDistance` comparison can never select it; the
    /// globe ray intersection is returned. See the module DEVIATION note.
    fn pick_position(
        &mut self,
        ctx: &mut SsccSceneContext,
        mouse_position: &Cartesian2,
    ) -> Option<Cartesian3> {
        // `let depthIntersection; if (scene.pickPositionSupported) { ... }`.
        let depth_intersection: Option<Cartesian3> = None;

        // `if (!defined(globe)) return Cartesian3.clone(depthIntersection)`.
        if ctx.globe.is_none() {
            return depth_intersection;
        }

        let ray = ctx.camera.get_pick_ray(mouse_position)?;
        let cull_back_faces = !self._camera_underground;
        let mode = ctx.mode;
        let projection: &dyn MapProjection = ctx.map_projection;
        let globe = ctx.globe.as_mut().unwrap();
        let ray_intersection =
            globe.pick_world_coordinates(&ray, mode, Some(projection), Some(cull_back_faces));

        // `pickDistance` is `+inf` (depthIntersection undefined), so the
        // `pickDistance < rayDistance` branch is never taken.
        ray_intersection
    }

    /// `getDistanceFromSurface(controller)` — the camera's height above the
    /// globe surface (ellipsoidal height in 3D, `position.z` otherwise), minus
    /// `scene.globeHeight`, in absolute value.
    fn get_distance_from_surface(&self, ctx: &SsccSceneContext) -> f64 {
        let height = if ctx.mode == SceneMode::Scene3D {
            let mut cartographic = Cartographic::default();
            if self
                ._ellipsoid
                .cartesian_to_cartographic(ctx.camera.position(), &mut cartographic)
            {
                cartographic.height
            } else {
                0.0
            }
        } else {
            ctx.camera.position().z
        };
        let globe_height = ctx.globe_height.unwrap_or(0.0);
        (globe_height - height).abs()
    }

    /// `getZoomDistanceUnderground(controller, ray)` — weights the zoom distance
    /// by how strongly the pick ray points inward (geocentric normal).
    fn get_zoom_distance_underground(&self, ctx: &SsccSceneContext, ray: &Ray) -> f64 {
        let distance_from_surface = self.get_distance_from_surface(ctx);
        let surface_normal = Cartesian3::normalize_new(&ray.origin);
        let strength = Cartesian3::dot(&surface_normal, &ray.direction).abs();
        let strength = strength.max(0.5) * 2.0;
        distance_from_surface * strength
    }

    /// `getTiltCenterUnderground(controller, ray, pickedPosition, result)` —
    /// simulates look-at behaviour by tilting around a small invisible sphere
    /// when the picked position is too far away.
    fn get_tilt_center_underground(
        &self,
        ctx: &SsccSceneContext,
        ray: &Ray,
        picked_position: &Cartesian3,
    ) -> Cartesian3 {
        let mut distance = Cartesian3::distance(&ray.origin, picked_position);
        let distance_from_surface = self.get_distance_from_surface(ctx);
        let maximum_distance = CesiumMath::clamp(
            distance_from_surface * 5.0,
            self._minimum_underground_pick_distance,
            self._maximum_underground_pick_distance,
        );
        if distance > maximum_distance {
            distance = distance.min(distance_from_surface / 5.0);
            distance = distance.max(100.0);
        }
        Ray::get_point_new(ray, Some(distance))
    }

    /// `getStrafeStartPositionUnderground(controller, ray, pickedPosition,
    /// result)` — sets the strafe speed from the picked distance, falling back
    /// to the height above the surface when nothing was picked or it is too far.
    fn get_strafe_start_position_underground(
        &self,
        ctx: &SsccSceneContext,
        ray: &Ray,
        picked_position: Option<Cartesian3>,
    ) -> Cartesian3 {
        let distance = match picked_position {
            None => self.get_distance_from_surface(ctx),
            Some(picked) => {
                let distance = Cartesian3::distance(&ray.origin, &picked);
                if distance > self._maximum_underground_pick_distance {
                    self.get_distance_from_surface(ctx)
                } else {
                    distance
                }
            }
        };
        Ray::get_point_new(ray, Some(distance))
    }

    // ---- Motion families ----
    //
    // The 2D family below is landed (b3-3d). DEVIATION: the Columbus-view / 3D
    // motion handlers are still compiling stubs until b3-3d (CV) / b3-3e (3D);
    // each mirrors the CesiumJS `function xxx(controller, startPosition,
    // movement)` signature.

    /// `translate2D(controller, startPosition, movement)`.
    ///
    /// Picks the world position under the movement's start and end pixels, swaps
    /// into the 2D map frame (`fromElements(y, z, x)`), and moves the camera by
    /// the difference so the grabbed point stays under the cursor.
    fn translate_2d(
        &mut self,
        ctx: &mut SsccSceneContext,
        _start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        let (start_pos, end_pos) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };

        let start = match ctx.camera.get_pick_ray(&start_pos) {
            Some(ray) => ray.origin,
            None => return,
        };
        let end = match ctx.camera.get_pick_ray(&end_pos) {
            Some(ray) => ray.origin,
            None => return,
        };

        let start = Cartesian3::from_elements_new(start.y, start.z, start.x);
        let end = Cartesian3::from_elements_new(end.y, end.z, end.x);

        let direction = Cartesian3::subtract_new(&start, &end);
        let distance = Cartesian3::magnitude(&direction);
        if distance > 0.0 {
            let direction = Cartesian3::normalize_new(&direction);
            ctx.camera.move_camera(&direction, distance);
        }
    }

    /// `zoom2D(controller, startPosition, movement)`.
    fn zoom_2d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `if (defined(movement.distance)) movement = movement.distance` — a
        // pinch zoom flattens to its distance sub-movement.
        let (start, end) = match movement {
            MovementInput::Pinch(pinch) => {
                (pinch.distance.start_position, pinch.distance.end_position)
            }
            MovementInput::Mouse(mouse) => (mouse.start_position, mouse.end_position),
            MovementInput::Inertia(state) => (state.start_position, state.end_position),
        };
        let zoom_movement = ZoomMovement {
            start_position: start,
            end_position: end,
            inertia_enabled: movement.inertia_enabled(),
        };

        let zoom_factor = self.zoom_factor;
        let distance_measure = ctx.camera.get_magnitude().unwrap_or(0.0);
        // `zoom2D` passes no `unitPositionDotDirection` (⇒ `percentage = 1`).
        self.handle_zoom(
            ctx,
            start_position,
            &zoom_movement,
            zoom_factor,
            distance_measure,
            None,
        );
    }

    /// `twist2D(controller, startPosition, movement)`.
    fn twist_2d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // A pinch twist rebinds to its `angleAndHeight` sub-movement.
        if let Some(angle_and_height) = movement.angle_and_height() {
            self.single_axis_twist_2d(ctx, start_position, &angle_and_height);
            return;
        }

        let (start_pos, end_pos) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };

        let width = ctx.canvas_client_width;
        let height = ctx.canvas_client_height;

        let start = Cartesian2::new(
            (2.0 / width) * start_pos.x - 1.0,
            (2.0 / height) * (height - start_pos.y) - 1.0,
        );
        let start = Cartesian2::normalize_new(&start);
        let end = Cartesian2::new(
            (2.0 / width) * end_pos.x - 1.0,
            (2.0 / height) * (height - end_pos.y) - 1.0,
        );
        let end = Cartesian2::normalize_new(&end);

        let mut start_theta = CesiumMath::acos_clamped(start.x);
        if start.y < 0.0 {
            start_theta = CesiumMath::TWO_PI - start_theta;
        }
        let mut end_theta = CesiumMath::acos_clamped(end.x);
        if end.y < 0.0 {
            end_theta = CesiumMath::TWO_PI - end_theta;
        }
        let theta = end_theta - start_theta;

        ctx.camera.twist_right(Some(theta));
    }

    /// `singleAxisTwist2D(controller, startPosition, movement)` — the pinch path
    /// of `twist2D`. `startPosition` is unused, exactly as in CesiumJS.
    fn single_axis_twist_2d(
        &mut self,
        ctx: &mut SsccSceneContext,
        _start_position: Cartesian2,
        movement: &MouseMovement,
    ) {
        let mut rotate_rate = self._rotate_factor * self._rotate_rate_range_adjustment;
        if rotate_rate > self._maximum_rotate_rate {
            rotate_rate = self._maximum_rotate_rate;
        }
        if rotate_rate < self._minimum_rotate_rate {
            rotate_rate = self._minimum_rotate_rate;
        }

        let mut phi_window_ratio =
            (movement.end_position.x - movement.start_position.x) / ctx.canvas_client_width;
        phi_window_ratio = phi_window_ratio.min(self.maximum_movement_ratio);

        let delta_phi = rotate_rate * phi_window_ratio * CesiumMath::PI * 4.0;
        ctx.camera.twist_right(Some(delta_phi));
    }

    /// `translateCV(controller, startPosition, movement)`.
    ///
    /// The Columbus-view translate action. Latches a look or strafe gesture on
    /// the first movement (reused while the start pixel is unchanged),
    /// otherwise grabs the world point under the cursor on the map plane
    /// (`normal = UNIT_X` through `origin.x`) and moves the camera so it stays
    /// under the cursor.
    fn translate_cv(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `if (!Cartesian3.equals(startPosition, _translateMousePosition)) _looking = false`.
        // `_translateMousePosition` is `undefined` (None) until the look-fallback
        // below, so on a fresh drag the latch clears.
        if !Cartesian2::equals(Some(&start_position), self._translate_mouse_position.as_ref()) {
            self._looking = false;
        }
        // `if (!Cartesian3.equals(startPosition, _strafeMousePosition)) _strafing = false`.
        if !Cartesian2::equals(Some(&start_position), Some(&self._strafe_mouse_position)) {
            self._strafing = false;
        }

        if self._looking {
            self.look_3d_impl(ctx, start_position, movement, None);
            return;
        }
        if self._strafing {
            self.continue_strafing(ctx, movement);
            return;
        }

        let (start_mouse, end_mouse) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };

        let camera_underground = self._camera_underground;

        let start_ray = match ctx.camera.get_pick_ray(&start_mouse) {
            Some(ray) => ray,
            None => return,
        };

        // `origin = ZERO` (only `.x` is ever overwritten); `normal = UNIT_X`.
        let mut origin = Cartesian3::ZERO;
        let normal = Cartesian3::UNIT_X;

        // `if (camera.position.z < _minimumPickingTerrainHeight) globePos = pickPosition(startMouse)`.
        let mut globe_pos: Option<Cartesian3> = None;
        if ctx.camera.position().z < self._minimum_picking_terrain_height {
            globe_pos = self.pick_position(ctx, &start_mouse);
            if let Some(gp) = globe_pos {
                origin.x = gp.x;
            }
        }

        // Underground, or the picked ground sits above the camera: strafe instead.
        if camera_underground || (origin.x > ctx.camera.position().z && globe_pos.is_some()) {
            let strafe_start = if camera_underground {
                self.get_strafe_start_position_underground(ctx, &start_ray, globe_pos)
            } else {
                globe_pos.unwrap()
            };
            self._strafe_mouse_position = start_position;
            self._strafe_end_mouse_position = start_position;
            self._strafe_start_position = strafe_start;
            self._strafing = true;
            self.strafe(ctx, end_mouse, strafe_start);
            return;
        }

        let mut plane = Plane::new(&normal, 0.0);
        Plane::from_point_normal(&origin, &normal, &mut plane);

        // CesiumJS recomputes `startRay` here (identical value); the port reuses it.
        let start_plane_pos = IntersectionTests::ray_plane(&start_ray, &plane);
        let end_ray = match ctx.camera.get_pick_ray(&end_mouse) {
            Some(ray) => ray,
            None => return,
        };
        let end_plane_pos = IntersectionTests::ray_plane(&end_ray, &plane);

        let (start_plane_pos, end_plane_pos) = match (start_plane_pos, end_plane_pos) {
            (Some(start), Some(end)) => (start, end),
            _ => {
                // Off the map plane: fall back to a look gesture and latch it.
                self._looking = true;
                self.look_3d_impl(ctx, start_position, movement, None);
                self._translate_mouse_position = Some(start_position);
                return;
            }
        };

        // `diff = startPlanePos - endPlanePos`, then rotate components
        // `(x, y, z) <- (y, z, x)` into the CV map frame.
        let mut diff = Cartesian3::subtract_new(&start_plane_pos, &end_plane_pos);
        let temp = diff.x;
        diff.x = diff.y;
        diff.y = diff.z;
        diff.z = temp;

        let mag = Cartesian3::magnitude(&diff);
        if mag > CesiumMath::EPSILON6 {
            let diff = Cartesian3::normalize_new(&diff);
            ctx.camera.move_camera(&diff, mag);
        }
    }

    /// `rotateCV(controller, startPosition, movement)`.
    ///
    /// The Columbus-view tilt action. A pinch flattens to its `angleAndHeight`
    /// sub-movement; latches a look gesture once `_looking` is set, otherwise
    /// rotates on the map plane (off-map / high camera) or on terrain.
    fn rotate_cv(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `if (defined(movement.angleAndHeight)) movement = movement.angleAndHeight`.
        let flattened = movement.angle_and_height().map(MovementInput::Mouse);
        let movement = flattened.as_ref().unwrap_or(movement);

        // `if (!Cartesian2.equals(startPosition, _tiltCenterMousePosition)) { _tiltCVOffMap = false; _looking = false }`.
        if !Cartesian2::equals(Some(&start_position), Some(&self._tilt_center_mouse_position)) {
            self._tilt_cv_off_map = false;
            self._looking = false;
        }

        if self._looking {
            self.look_3d_impl(ctx, start_position, movement, None);
            return;
        }

        // `if (_tiltCVOffMap || !onMap() || abs(camera.position.z) > _minimumPickingTerrainHeight)`.
        let on_map = self.on_map(ctx);
        let off_map = self._tilt_cv_off_map
            || !on_map
            || ctx.camera.position().z.abs() > self._minimum_picking_terrain_height;
        if off_map {
            self._tilt_cv_off_map = true;
            self.rotate_cv_on_plane(ctx, start_position, movement);
        } else {
            self.rotate_cv_on_terrain(ctx, start_position, movement);
        }
    }

    /// `rotateCVOnPlane(controller, startPosition, movement)`.
    ///
    /// Rotates about the east-north-up frame at the point where the centre pick
    /// ray meets the map plane (`normal = UNIT_X` through the origin); used when
    /// off-map or when the camera is high above the terrain.
    fn rotate_cv_on_plane(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        let window_position = Cartesian2::new(
            ctx.canvas_client_width / 2.0,
            ctx.canvas_client_height / 2.0,
        );
        let ray = match ctx.camera.get_pick_ray(&window_position) {
            Some(ray) => ray,
            None => return,
        };
        let normal = Cartesian3::UNIT_X;

        // Intersect the centre ray with the map plane: `scalar = -dot(n, o) / dot(n, d)`.
        let position = ray.origin;
        let direction = ray.direction;
        let mut scalar: Option<f64> = None;
        let normal_dot_direction = Cartesian3::dot(&normal, &direction);
        if normal_dot_direction.abs() > CesiumMath::EPSILON6 {
            scalar = Some(-Cartesian3::dot(&normal, &position) / normal_dot_direction);
        }

        let scalar = match scalar {
            Some(s) if s > 0.0 => s,
            _ => {
                // The centre ray never meets the map plane in front of the camera:
                // fall back to a look gesture and latch it.
                self._looking = true;
                self.look_3d_impl(ctx, start_position, movement, None);
                self._tilt_center_mouse_position = start_position;
                return;
            }
        };

        let center = Cartesian3::multiply_by_scalar_new(&direction, scalar);
        let center = Cartesian3::add_new(&position, &center);

        // `const projection = scene.mapProjection; const ellipsoid = projection.ellipsoid`.
        let projection = ctx.map_projection;
        let ellipsoid = projection.ellipsoid().clone();

        // Into the CV map frame, unproject to a cartographic, back to a world point.
        let swapped = Cartesian3::from_elements_new(center.y, center.z, center.x);
        let cart = projection.unproject(&swapped);
        let mut center = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&cart, &mut center);

        let transform = east_north_up_to_fixed_frame_new(&center, Some(&ellipsoid));

        let old_globe = ctx.globe.take();
        let old_ellipsoid = self._ellipsoid;
        self._ellipsoid = Ellipsoid::UNIT_SPHERE;
        self._rotate_factor = 1.0;
        self._rotate_rate_range_adjustment = 1.0;

        let old_transform = *ctx.camera.transform();
        ctx.camera.set_transform(transform);

        self.rotate_3d_impl(
            ctx,
            start_position,
            movement,
            Some(Cartesian3::UNIT_Z),
            false,
            false,
        );

        ctx.camera.set_transform(old_transform);
        ctx.globe = old_globe;
        self._ellipsoid = old_ellipsoid;

        let radius = old_ellipsoid.maximum_radius();
        self._rotate_factor = 1.0 / radius;
        self._rotate_rate_range_adjustment = radius;
    }

    /// `rotateCVOnTerrain(controller, startPosition, movement)`.
    ///
    /// Rotates about the picked terrain point. Builds an east-north-up frame at
    /// the centre and a "vertical" frame at the ray/map-plane intersection,
    /// applies the horizontal rotate (under `transform`) then the vertical rotate
    /// (under `verticalTransform`), re-orthogonalises about the constrained axis,
    /// and runs the shared terrain position correction.
    ///
    /// DEVIATION: every early return happens before the globe/ellipsoid swap and
    /// the `_setTransform` calls, so the temporary state is always restored. The
    /// projection is `camera._projection` here (vs `scene.mapProjection` in
    /// `rotateCVOnPlane`), exactly as in CesiumJS.
    fn rotate_cv_on_terrain(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        let camera_underground = self._camera_underground;
        let normal = Cartesian3::UNIT_X;

        // ---- Centre: reuse the latched tilt centre, or pick/intersect a new one ----
        let center: Cartesian3;
        if Cartesian2::equals(Some(&start_position), Some(&self._tilt_center_mouse_position)) {
            center = self._tilt_center;
        } else {
            let mut center_opt: Option<Cartesian3> = None;
            if ctx.camera.position().z < self._minimum_picking_terrain_height {
                center_opt = self.pick_position(ctx, &start_position);
            }

            let mut ray: Option<Ray> = None;
            if center_opt.is_none() {
                let r = match ctx.camera.get_pick_ray(&start_position) {
                    Some(r) => r,
                    None => return,
                };
                let position = r.origin;
                let direction = r.direction;
                let mut scalar: Option<f64> = None;
                let normal_dot_direction = Cartesian3::dot(&normal, &direction);
                if normal_dot_direction.abs() > CesiumMath::EPSILON6 {
                    scalar = Some(-Cartesian3::dot(&normal, &position) / normal_dot_direction);
                }
                let scalar = match scalar {
                    Some(s) if s > 0.0 => s,
                    _ => {
                        self._looking = true;
                        self.look_3d_impl(ctx, start_position, movement, None);
                        self._tilt_center_mouse_position = start_position;
                        return;
                    }
                };
                let c = Cartesian3::multiply_by_scalar_new(&direction, scalar);
                let c = Cartesian3::add_new(&position, &c);
                center_opt = Some(c);
                ray = Some(r);
            }

            if camera_underground {
                let r = match ray {
                    Some(r) => r,
                    None => match ctx.camera.get_pick_ray(&start_position) {
                        Some(r) => r,
                        None => return,
                    },
                };
                let picked = match center_opt {
                    Some(picked) => picked,
                    None => return,
                };
                center_opt = Some(self.get_tilt_center_underground(ctx, &r, &picked));
            }

            let picked = match center_opt {
                Some(picked) => picked,
                None => return,
            };
            self._tilt_center_mouse_position = start_position;
            self._tilt_center = picked;
            center = picked;
        }

        // ---- Vertical centre: intersect the centre-column ray with the map plane ----
        let window_position =
            Cartesian2::new(ctx.canvas_client_width / 2.0, self._tilt_center_mouse_position.y);
        let ray = match ctx.camera.get_pick_ray(&window_position) {
            Some(ray) => ray,
            None => return,
        };

        let mut origin = Cartesian3::ZERO;
        origin.x = center.x;
        let mut plane = Plane::new(&normal, 0.0);
        Plane::from_point_normal(&origin, &normal, &mut plane);
        let vertical_center = IntersectionTests::ray_plane(&ray, &plane);

        // `const projection = camera._projection; const ellipsoid = projection.ellipsoid`.
        let ellipsoid = ctx.camera.map_projection().ellipsoid().clone();

        let swapped = Cartesian3::from_elements_new(center.y, center.z, center.x);
        let cart = ctx.camera.map_projection().unproject(&swapped);
        let mut center_world = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&cart, &mut center_world);
        let transform = east_north_up_to_fixed_frame_new(&center_world, Some(&ellipsoid));

        let vertical_transform = if let Some(vc) = vertical_center {
            let vc_swapped = Cartesian3::from_elements_new(vc.y, vc.z, vc.x);
            let vc_cart = ctx.camera.map_projection().unproject(&vc_swapped);
            let mut vc_world = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&vc_cart, &mut vc_world);
            east_north_up_to_fixed_frame_new(&vc_world, Some(&ellipsoid))
        } else {
            transform
        };

        // ---- Swap region (no early return below) ----
        let old_globe = ctx.globe.take();
        let old_ellipsoid = self._ellipsoid;
        self._ellipsoid = Ellipsoid::UNIT_SPHERE;
        self._rotate_factor = 1.0;
        self._rotate_rate_range_adjustment = 1.0;

        let mut constrained_axis = Some(Cartesian3::UNIT_Z);

        let old_transform = *ctx.camera.transform();
        ctx.camera.set_transform(transform);

        // `tangent = UNIT_Z × normalize(camera.position)`; `dot = camera.right · tangent`
        // (local position/right, as in CesiumJS `rotateCVOnTerrain`).
        let camera_position = *ctx.camera.position();
        let camera_right = *ctx.camera.right();
        let unit_position = Cartesian3::normalize_new(&camera_position);
        let tangent = Cartesian3::cross_new(&Cartesian3::UNIT_Z, &unit_position);
        let dot = Cartesian3::dot(&camera_right, &tangent);

        // Horizontal rotate under `transform`.
        self.rotate_3d_impl(ctx, start_position, movement, constrained_axis, false, true);

        ctx.camera.set_transform(vertical_transform);
        if dot < 0.0 {
            let movement_delta = match (movement.start_position(), movement.end_position()) {
                (Some(start), Some(end)) => start.y - end.y,
                _ => 0.0,
            };
            if (camera_underground && movement_delta < 0.0)
                || (!camera_underground && movement_delta > 0.0)
            {
                // Prevent the camera from flipping past the up axis.
                constrained_axis = None;
            }

            let old_constrained_axis = ctx.camera.constrained_axis();
            ctx.camera.set_constrained_axis(None);
            self.rotate_3d_impl(ctx, start_position, movement, constrained_axis, true, false);
            ctx.camera.set_constrained_axis(old_constrained_axis);
        } else {
            self.rotate_3d_impl(ctx, start_position, movement, constrained_axis, true, false);
        }

        if let Some(constrained) = ctx.camera.constrained_axis() {
            let direction = *ctx.camera.direction();
            let right = Cartesian3::cross_new(&direction, &constrained);
            if !Cartesian3::equals_epsilon(
                Some(&right),
                Some(&Cartesian3::ZERO),
                Some(CesiumMath::EPSILON6),
                None,
            ) {
                let camera_right = *ctx.camera.right();
                let right = if Cartesian3::dot(&right, &camera_right) < 0.0 {
                    Cartesian3::negate_new(&right)
                } else {
                    right
                };
                let up = Cartesian3::cross_new(&right, &direction);
                let new_right = Cartesian3::cross_new(&direction, &up);
                let up = Cartesian3::normalize_new(&up);
                let new_right = Cartesian3::normalize_new(&new_right);
                ctx.camera.set_up(up);
                ctx.camera.set_right(new_right);
            }
        }

        ctx.camera.set_transform(old_transform);
        ctx.globe = old_globe;
        self._ellipsoid = old_ellipsoid;

        let radius = old_ellipsoid.maximum_radius();
        self._rotate_factor = 1.0 / radius;
        self._rotate_rate_range_adjustment = radius;

        self.correct_position_after_terrain(ctx, vertical_transform, old_transform);
    }

    /// `zoomCV(controller, startPosition, movement)`.
    ///
    /// The Columbus-view zoom action. Measures the distance to the picked terrain
    /// (or the underground weighting, or the perpendicular distance to the map
    /// plane) and feeds it to `handleZoom` with no `unitPositionDotDirection`.
    fn zoom_cv(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `if (defined(movement.distance)) movement = movement.distance` — a
        // pinch zoom flattens to its distance sub-movement.
        let (m_start, m_end) = match movement {
            MovementInput::Pinch(pinch) => {
                (pinch.distance.start_position, pinch.distance.end_position)
            }
            MovementInput::Mouse(mouse) => (mouse.start_position, mouse.end_position),
            MovementInput::Inertia(state) => (state.start_position, state.end_position),
        };
        let inertia_movement = movement.inertia_enabled();

        let camera_underground = self._camera_underground;

        let window_position = if camera_underground {
            start_position
        } else {
            Cartesian2::new(ctx.canvas_client_width / 2.0, ctx.canvas_client_height / 2.0)
        };

        let ray = match ctx.camera.get_pick_ray(&window_position) {
            Some(ray) => ray,
            None => return,
        };

        // In Columbus view `camera.position.z` is the height above the map plane.
        let height = ctx.camera.position().z;

        let intersection = if height < self._minimum_picking_terrain_height {
            self.pick_position(ctx, &window_position)
        } else {
            None
        };

        let mut distance: Option<f64> =
            intersection.map(|i| Cartesian3::distance(&ray.origin, &i));

        if camera_underground {
            let distance_underground = self.get_zoom_distance_underground(ctx, &ray);
            distance = Some(match distance {
                Some(d) => d.min(distance_underground),
                None => distance_underground,
            });
        }

        // Off-map: fall back to the perpendicular distance to the `x = 0` plane.
        let distance = match distance {
            Some(d) => d,
            None => {
                let normal = Cartesian3::UNIT_X;
                -Cartesian3::dot(&normal, &ray.origin) / Cartesian3::dot(&normal, &ray.direction)
            }
        };

        let zoom_movement = ZoomMovement {
            start_position: m_start,
            end_position: m_end,
            inertia_enabled: inertia_movement,
        };
        let zoom_factor = self.zoom_factor;
        // `zoomCV` passes no `unitPositionDotDirection` (⇒ `percentage = 1`).
        self.handle_zoom(ctx, start_position, &zoom_movement, zoom_factor, distance, None);
    }

    /// `strafe(controller, movement, strafeStartPosition)`.
    ///
    /// DEVIATION: CesiumJS reads only `movement.endPosition`, so the port takes
    /// that pixel directly instead of the whole movement — `continueStrafing`
    /// temporarily rebinds `movement.endPosition`, which the port expresses by
    /// passing the rebound pixel here.
    fn strafe(
        &mut self,
        ctx: &mut SsccSceneContext,
        end_position: Cartesian2,
        strafe_start_position: Cartesian3,
    ) {
        let ray = match ctx.camera.get_pick_ray(&end_position) {
            Some(ray) => ray,
            None => return,
        };

        let mut direction = *ctx.camera.direction();
        if ctx.mode == SceneMode::ColumbusView {
            direction = Cartesian3::from_elements_new(direction.z, direction.x, direction.y);
        }

        let mut plane = Plane::new(&Cartesian3::UNIT_X, 0.0);
        Plane::from_point_normal(&strafe_start_position, &direction, &mut plane);
        let intersection = match IntersectionTests::ray_plane(&ray, &plane) {
            Some(intersection) => intersection,
            None => return,
        };

        direction = Cartesian3::subtract_new(&strafe_start_position, &intersection);
        if ctx.mode == SceneMode::ColumbusView {
            direction = Cartesian3::from_elements_new(direction.y, direction.z, direction.x);
        }

        let position = Cartesian3::add_new(ctx.camera.position(), &direction);
        ctx.camera.set_position(position);
    }

    /// `continueStrafing(controller, movement)`.
    ///
    /// Accumulates the inertial delta into `_strafeEndMousePosition` (CesiumJS
    /// mutates that field in place through the `endPosition` alias) and strafes
    /// toward it. The temporary `movement.endPosition` rebind is implicit: the
    /// port's [`Self::strafe`] takes the end pixel by value, so the original
    /// movement is never mutated and needs no restore.
    fn continue_strafing(&mut self, ctx: &mut SsccSceneContext, movement: &MovementInput) {
        let (start, end) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };
        let inertial_delta = Cartesian2::subtract_new(&end, &start);
        let end_position = Cartesian2::add_new(&self._strafe_end_mouse_position, &inertial_delta);
        self._strafe_end_mouse_position = end_position;
        let strafe_start = self._strafe_start_position;
        self.strafe(ctx, end_position, strafe_start);
    }

    /// `spin3D(controller, startPosition, movement)`.
    ///
    /// The 3D rotate action: latches a gesture mode (look / rotate / strafe /
    /// pan) on the first movement and reuses it while the start pixel is
    /// unchanged, exactly as CesiumJS does through `_looking` / `_rotating` /
    /// `_strafing` and `_rotateMousePosition`.
    fn spin_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `positionWC` / `directionWC` are read below; refresh so the cached
        // derived state matches the CesiumJS live getters (module DEVIATION).
        ctx.camera.refresh();

        let camera_underground = self._camera_underground;

        // `if (!Matrix4.equals(camera.transform, Matrix4.IDENTITY)) { rotate3D; return; }`.
        if !Matrix4::equals(ctx.camera.transform(), &Matrix4::IDENTITY) {
            self.rotate_3d(ctx, start_position, movement);
            return;
        }

        // `const up = ellipsoid.geodeticSurfaceNormal(camera.position, scratchLookUp)`
        // — `undefined` (None) when the position is degenerate.
        let mut up_vec = Cartesian3::default();
        let up = if self
            ._ellipsoid
            .geodetic_surface_normal(ctx.camera.position(), &mut up_vec)
        {
            Some(up_vec)
        } else {
            None
        };

        // Continuation of an in-progress gesture (start pixel unchanged).
        if Cartesian2::equals(Some(&start_position), Some(&self._rotate_mouse_position)) {
            if self._looking {
                self.look_3d_impl(ctx, start_position, movement, up);
            } else if self._rotating {
                self.rotate_3d(ctx, start_position, movement);
            } else if self._strafing {
                self.continue_strafing(ctx, movement);
            } else {
                // Pan is no longer valid if the camera moves below the pan ellipsoid.
                if Cartesian3::magnitude(ctx.camera.position())
                    < Cartesian3::magnitude(&self._rotate_start_position)
                {
                    return;
                }
                let magnitude = Cartesian3::magnitude(&self._rotate_start_position);
                let radii = Cartesian3::new(magnitude, magnitude, magnitude);
                let ellipsoid = Ellipsoid::from_cartesian3(Some(&radii));
                self.pan_3d(ctx, start_position, movement, &ellipsoid);
            }
            return;
        }
        self._looking = false;
        self._rotating = false;
        self._strafing = false;

        // `const height = ellipsoid.cartesianToCartographic(camera.positionWC, scratch).height`.
        let mut cartographic = Cartographic::default();
        let height = if self
            ._ellipsoid
            .cartesian_to_cartographic(ctx.camera.position_wc(), &mut cartographic)
        {
            cartographic.height
        } else {
            0.0
        };

        let globe_defined = ctx.globe.is_some();
        let movement_start = movement.start_position().unwrap_or(start_position);

        if globe_defined && height < self._minimum_picking_terrain_height {
            if let Some(picked) = self.pick_position(ctx, &movement_start) {
                let strafing;
                let mouse_pos;
                let ray = match ctx.camera.get_pick_ray(&movement_start) {
                    Some(ray) => ray,
                    None => return,
                };
                if camera_underground {
                    strafing = true;
                    mouse_pos =
                        self.get_strafe_start_position_underground(ctx, &ray, Some(picked));
                } else {
                    let mut normal = Cartesian3::default();
                    self._ellipsoid.geodetic_surface_normal(&picked, &mut normal);
                    let tangent_pick = Cartesian3::dot(&ray.direction, &normal).abs() < 0.05;
                    strafing = if tangent_pick {
                        true
                    } else {
                        Cartesian3::magnitude(ctx.camera.position())
                            < Cartesian3::magnitude(&picked)
                    };
                    mouse_pos = picked;
                }

                if strafing {
                    self._strafe_end_mouse_position = start_position;
                    self._strafe_start_position = mouse_pos;
                    self._strafing = true;
                    let strafe_start = self._strafe_start_position;
                    let end = movement.end_position().unwrap_or(movement_start);
                    self.strafe(ctx, end, strafe_start);
                } else {
                    let magnitude = Cartesian3::magnitude(&mouse_pos);
                    let radii = Cartesian3::new(magnitude, magnitude, magnitude);
                    let ellipsoid = Ellipsoid::from_cartesian3(Some(&radii));
                    self.pan_3d(ctx, start_position, movement, &ellipsoid);
                    self._rotate_start_position = mouse_pos;
                }
            } else {
                self._looking = true;
                self.look_3d_impl(ctx, start_position, movement, up);
            }
        } else {
            // `defined(camera.pickEllipsoid(movement.startPosition, _ellipsoid, spin3DPick))`.
            let spin_3d_pick = ctx.camera.pick_ellipsoid(&movement_start, Some(&self._ellipsoid));
            if let Some(pick) = spin_3d_pick {
                let ellipsoid = self._ellipsoid.clone();
                self.pan_3d(ctx, start_position, movement, &ellipsoid);
                self._rotate_start_position = pick;
            } else if height > self._minimum_track_ball_height {
                self._rotating = true;
                self.rotate_3d(ctx, start_position, movement);
            } else {
                self._looking = true;
                self.look_3d_impl(ctx, start_position, movement, up);
            }
        }

        self._rotate_mouse_position = start_position;
    }

    /// `rotate3D(controller, startPosition, movement)` — the three-argument form
    /// CesiumJS calls from `spin3D`/`pan3D` (no constrained axis, both axes free).
    fn rotate_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        self.rotate_3d_impl(ctx, start_position, movement, None, false, false);
    }

    /// `rotate3D(controller, startPosition, movement, constrainedAxis,
    /// rotateOnlyVertical, rotateOnlyHorizontal)`.
    ///
    /// Rotates the camera by the drag delta scaled to an angle, temporarily
    /// applying `constrainedAxis` (restoring the previous value afterwards) and
    /// clamping the tilt to `maximumTiltAngle` when both are defined.
    fn rotate_3d_impl(
        &mut self,
        ctx: &mut SsccSceneContext,
        _start_position: Cartesian2,
        movement: &MovementInput,
        constrained_axis: Option<Cartesian3>,
        rotate_only_vertical: bool,
        rotate_only_horizontal: bool,
    ) {
        let (m_start, m_end) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };

        let old_axis = ctx.camera.constrained_axis();
        if let Some(axis) = constrained_axis {
            ctx.camera.set_constrained_axis(Some(axis));
        }

        let rho = Cartesian3::magnitude(ctx.camera.position());
        let mut rotate_rate = self._rotate_factor * (rho - self._rotate_rate_range_adjustment);
        if rotate_rate > self._maximum_rotate_rate {
            rotate_rate = self._maximum_rotate_rate;
        }
        if rotate_rate < self._minimum_rotate_rate {
            rotate_rate = self._minimum_rotate_rate;
        }

        let mut phi_window_ratio = (m_start.x - m_end.x) / ctx.canvas_client_width;
        let mut theta_window_ratio = (m_start.y - m_end.y) / ctx.canvas_client_height;
        phi_window_ratio = phi_window_ratio.min(self.maximum_movement_ratio);
        theta_window_ratio = theta_window_ratio.min(self.maximum_movement_ratio);

        let delta_phi = rotate_rate * phi_window_ratio * CesiumMath::PI * 2.0;
        let mut delta_theta = rotate_rate * theta_window_ratio * CesiumMath::PI;

        if let (Some(axis), Some(maximum_tilt_angle)) = (constrained_axis, self.maximum_tilt_angle) {
            let dot_product = Cartesian3::dot(ctx.camera.direction(), &axis);
            let tilt = CesiumMath::PI - dot_product.acos() + delta_theta;
            if tilt > maximum_tilt_angle {
                delta_theta -= tilt - maximum_tilt_angle;
            }
        }

        if !rotate_only_vertical {
            ctx.camera.rotate_right(Some(delta_phi));
        }
        if !rotate_only_horizontal {
            ctx.camera.rotate_up(Some(delta_theta));
        }

        ctx.camera.set_constrained_axis(old_axis);
    }

    /// `pan3D(controller, startPosition, movement, ellipsoid)`.
    ///
    /// Grabs the world point under the start pixel and rotates the camera so it
    /// stays under the end pixel. Falls back to `rotate3D` when either pixel
    /// misses the ellipsoid.
    ///
    /// DEVIATION: the `!defined(_globe)` look-at sub-branch (which recomputes
    /// `p1` from `getPixelDimensions`) is unreachable in the port because
    /// [`Self::pick_position`] returns `None` when the globe is `None`, so `p0`
    /// is never defined on a new drag and `_panLastWorldPosition` is never
    /// seeded. It is translated in full for source fidelity; CesiumJS's
    /// `scene.drawingBufferWidth/Height` and `scene.pixelRatio` are sourced from
    /// the camera (`canvas_width/height`, `scene_context().pixel_ratio`), which
    /// the port fixes at a `1.0` pixel ratio.
    fn pan_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
        ellipsoid: &Ellipsoid,
    ) {
        ctx.camera.refresh();

        let (start_mouse_position, end_mouse_position) =
            match (movement.start_position(), movement.end_position()) {
                (Some(start), Some(end)) => (start, end),
                _ => return,
            };

        let mut cartographic = Cartographic::default();
        let height = if ellipsoid.cartesian_to_cartographic(ctx.camera.position_wc(), &mut cartographic)
        {
            cartographic.height
        } else {
            0.0
        };

        // `let p0, p1` — homogeneous points (CesiumJS reuses Cartesian4 scratch).
        let mut p0: Option<Cartesian4> = None;
        let mut p1: Option<Cartesian4> = None;

        let inertia_enabled = movement.inertia_enabled().unwrap_or(false);
        if !inertia_enabled && height < self._minimum_picking_terrain_height {
            // `p0 = Cartesian3.clone(controller._panLastWorldPosition, pan3DP0)`.
            p0 = Some(Cartesian4::new(
                self._pan_last_world_position.x,
                self._pan_last_world_position.y,
                self._pan_last_world_position.z,
                1.0,
            ));

            let globe_undefined = ctx.globe.is_none();

            // Use the last picked world position unless we're starting a new drag.
            if globe_undefined
                && !Cartesian2::equals_epsilon(
                    Some(&start_mouse_position),
                    Some(&self._pan_last_mouse_position),
                    None,
                    None,
                )
            {
                p0 = self
                    .pick_position(ctx, &start_mouse_position)
                    .map(|picked| Cartesian4::new(picked.x, picked.y, picked.z, 1.0));
            }

            if globe_undefined {
                if let Some(p0_val) = p0 {
                    // Read every derived world value before the mutable frustum /
                    // pick-ray calls so the borrows do not overlap.
                    let position_wc = *ctx.camera.position_wc();
                    let direction_wc = *ctx.camera.direction_wc();
                    let right_wc = *ctx.camera.right_wc();
                    let up_wc = *ctx.camera.up_wc();
                    let pixel_ratio = ctx.camera.scene_context().pixel_ratio;
                    let drawing_buffer_width = ctx.camera.canvas_width() as f64;
                    let drawing_buffer_height = ctx.camera.canvas_height() as f64;

                    let p0_cart3 = Cartesian3::new(p0_val.x, p0_val.y, p0_val.z);
                    let to_center = Cartesian3::subtract_new(&p0_cart3, &position_wc);
                    let to_center_proj = Cartesian3::multiply_by_scalar_new(
                        &direction_wc,
                        Cartesian3::dot(&direction_wc, &to_center),
                    );
                    let distance_to_near_plane = Cartesian3::magnitude(&to_center_proj);
                    let pixel_dimensions = ctx.camera.frustum_mut().get_pixel_dimensions(
                        drawing_buffer_width,
                        drawing_buffer_height,
                        distance_to_near_plane,
                        pixel_ratio,
                    );

                    let drag_delta =
                        Cartesian2::subtract_new(&end_mouse_position, &start_mouse_position);

                    // Move the camera the distance the cursor moved in world space.
                    let right = Cartesian3::multiply_by_scalar_new(
                        &right_wc,
                        drag_delta.x * pixel_dimensions.x,
                    );

                    let camera_position_normal = Cartesian3::normalize_new(&position_wc);
                    let end_pick_direction = match ctx.camera.get_pick_ray(&end_mouse_position) {
                        Some(ray) => ray.direction,
                        None => {
                            return self.pan_3d_fallback(ctx, start_position, movement, ellipsoid);
                        }
                    };
                    let mut end_pick_proj = Cartesian3::default();
                    Cartesian3::project_vector(&end_pick_direction, &right_wc, &mut end_pick_proj);
                    let end_pick_proj = Cartesian3::subtract_new(&end_pick_direction, &end_pick_proj);
                    let angle = Cartesian3::angle_between(&end_pick_proj, &direction_wc);
                    let mut forward = 1.0;
                    if ctx.camera.frustum().fov().is_some() {
                        // Clamp so the magnitude is not infinitely large for a small angle.
                        forward = angle.tan().max(0.1);
                    }
                    let mut dot = Cartesian3::dot(&direction_wc, &camera_position_normal).abs();
                    let magnitude =
                        ((-drag_delta.y * pixel_dimensions.y * 2.0) / forward.sqrt()) * (1.0 - dot);
                    let direction = Cartesian3::multiply_by_scalar_new(&end_pick_direction, magnitude);

                    // Move the camera up as it points toward the center.
                    dot = Cartesian3::dot(&up_wc, &camera_position_normal).abs();
                    let up = Cartesian3::multiply_by_scalar_new(
                        &up_wc,
                        -drag_delta.y * (1.0 - dot) * pixel_dimensions.y,
                    );

                    let mut p1_val = Cartesian3::add_new(&p0_cart3, &right);
                    p1_val = Cartesian3::add_new(&p1_val, &direction);
                    p1_val = Cartesian3::add_new(&p1_val, &up);

                    p1 = Some(Cartesian4::new(p1_val.x, p1_val.y, p1_val.z, 1.0));
                    self._pan_last_world_position = p1_val;
                    self._pan_last_mouse_position = end_mouse_position;
                }
            }
        }

        if p0.is_none() || p1.is_none() {
            p0 = ctx
                .camera
                .pick_ellipsoid(&start_mouse_position, Some(ellipsoid))
                .map(|p| Cartesian4::new(p.x, p.y, p.z, 1.0));
            p1 = ctx
                .camera
                .pick_ellipsoid(&end_mouse_position, Some(ellipsoid))
                .map(|p| Cartesian4::new(p.x, p.y, p.z, 1.0));
        }

        let (p0, p1) = match (p0, p1) {
            (Some(p0), Some(p1)) => (p0, p1),
            _ => {
                self.pan_3d_fallback(ctx, start_position, movement, ellipsoid);
                return;
            }
        };

        // `p0 = camera.worldToCameraCoordinates(p0, p0)` (a rigid transform keeps
        // w = 1, so the Cartesian3 part is the camera-space point).
        let p0 = ctx.camera.world_to_camera_coordinates(&p0);
        let p1 = ctx.camera.world_to_camera_coordinates(&p1);
        let p0 = Cartesian3::new(p0.x, p0.y, p0.z);
        let p1 = Cartesian3::new(p1.x, p1.y, p1.z);

        match ctx.camera.constrained_axis() {
            None => {
                let p0n = Cartesian3::normalize_new(&p0);
                let p1n = Cartesian3::normalize_new(&p1);
                let dot = Cartesian3::dot(&p0n, &p1n);
                let axis = Cartesian3::cross_new(&p0n, &p1n);
                if dot < 1.0
                    && !Cartesian3::equals_epsilon(
                        Some(&axis),
                        Some(&Cartesian3::ZERO),
                        Some(CesiumMath::EPSILON14),
                        None,
                    )
                {
                    // dot is in [0, 1].
                    let angle = dot.acos();
                    ctx.camera.rotate(&axis, Some(angle));
                }
            }
            Some(basis0) => {
                let mut basis1 = Cartesian3::default();
                Cartesian3::most_orthogonal_axis(&basis0, &mut basis1);
                basis1 = Cartesian3::cross_new(&basis1, &basis0);
                basis1 = Cartesian3::normalize_new(&basis1);
                let basis2 = Cartesian3::cross_new(&basis0, &basis1);

                let start_rho = Cartesian3::magnitude(&p0);
                let start_dot = Cartesian3::dot(&basis0, &p0);
                let start_theta = (start_dot / start_rho).acos();
                let start_rej = Cartesian3::multiply_by_scalar_new(&basis0, start_dot);
                let start_rej = Cartesian3::subtract_new(&p0, &start_rej);
                let start_rej = Cartesian3::normalize_new(&start_rej);

                let end_rho = Cartesian3::magnitude(&p1);
                let end_dot = Cartesian3::dot(&basis0, &p1);
                let end_theta = (end_dot / end_rho).acos();
                let end_rej = Cartesian3::multiply_by_scalar_new(&basis0, end_dot);
                let end_rej = Cartesian3::subtract_new(&p1, &end_rej);
                let end_rej = Cartesian3::normalize_new(&end_rej);

                let mut start_phi = Cartesian3::dot(&start_rej, &basis1).acos();
                if Cartesian3::dot(&start_rej, &basis2) < 0.0 {
                    start_phi = CesiumMath::TWO_PI - start_phi;
                }
                let mut end_phi = Cartesian3::dot(&end_rej, &basis1).acos();
                if Cartesian3::dot(&end_rej, &basis2) < 0.0 {
                    end_phi = CesiumMath::TWO_PI - end_phi;
                }
                let delta_phi = start_phi - end_phi;

                let camera_position = *ctx.camera.position();
                let east = if Cartesian3::equals_epsilon(
                    Some(&basis0),
                    Some(&camera_position),
                    Some(CesiumMath::EPSILON2),
                    None,
                ) {
                    *ctx.camera.right()
                } else {
                    Cartesian3::cross_new(&basis0, &camera_position)
                };

                let plane_normal = Cartesian3::cross_new(&basis0, &east);
                let side0 =
                    Cartesian3::dot(&plane_normal, &Cartesian3::subtract_new(&p0, &basis0));
                let side1 =
                    Cartesian3::dot(&plane_normal, &Cartesian3::subtract_new(&p1, &basis0));

                let delta_theta = if side0 > 0.0 && side1 > 0.0 {
                    end_theta - start_theta
                } else if side0 > 0.0 && side1 <= 0.0 {
                    if Cartesian3::dot(&camera_position, &basis0) > 0.0 {
                        -start_theta - end_theta
                    } else {
                        start_theta + end_theta
                    }
                } else {
                    start_theta - end_theta
                };

                ctx.camera.rotate_right(Some(delta_phi));
                ctx.camera.rotate_up(Some(delta_theta));
            }
        }
    }

    /// The `!defined(p0) || !defined(p1)` tail of `pan3D`: latch `_rotating` and
    /// delegate to `rotate3D`. Split out so the early returns above stay `()`.
    fn pan_3d_fallback(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
        _ellipsoid: &Ellipsoid,
    ) {
        self._rotating = true;
        self.rotate_3d(ctx, start_position, movement);
    }

    /// `zoom3D(controller, startPosition, movement)`.
    fn zoom_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `if (defined(movement.distance)) movement = movement.distance` — a
        // pinch zoom flattens to its distance sub-movement.
        let (m_start, m_end) = match movement {
            MovementInput::Pinch(pinch) => {
                (pinch.distance.start_position, pinch.distance.end_position)
            }
            MovementInput::Mouse(mouse) => (mouse.start_position, mouse.end_position),
            MovementInput::Inertia(state) => (state.start_position, state.end_position),
        };
        // `const inertiaMovement = movement.inertiaEnabled` — read on the
        // original movement (a pinch/mouse has none; an inertia state carries it).
        let inertia_movement = movement.inertia_enabled();

        let camera_underground = self._camera_underground;

        let window_position = if camera_underground {
            start_position
        } else {
            Cartesian2::new(ctx.canvas_client_width / 2.0, ctx.canvas_client_height / 2.0)
        };

        let ray = match ctx.camera.get_pick_ray(&window_position) {
            Some(ray) => ray,
            None => return,
        };

        // `height` uses the raw `camera.position` (not WC), as in CesiumJS.
        let mut cartographic = Cartographic::default();
        let height = if self
            ._ellipsoid
            .cartesian_to_cartographic(ctx.camera.position(), &mut cartographic)
        {
            cartographic.height
        } else {
            0.0
        };

        let approaching_collision = self._pre_intersection_distance.abs()
            < self.minimum_picking_terrain_distance_with_inertia;
        let need_pick_globe = if inertia_movement == Some(true) {
            approaching_collision
        } else {
            height < self._minimum_picking_terrain_height
        };
        let intersection = if need_pick_globe {
            self.pick_position(ctx, &window_position)
        } else {
            None
        };

        let mut distance: Option<f64> =
            intersection.map(|i| Cartesian3::distance(&ray.origin, &i));

        // In tracking/lookAt mode (`_globe` undefined), `pickPosition` can hit
        // terrain behind the intended target; ignore farther picks.
        if ctx.globe.is_none() {
            if let Some(d) = distance {
                let target_distance = ctx.camera.get_magnitude().unwrap_or(0.0);
                if target_distance < d {
                    distance = None;
                }
            }
        }

        if let Some(d) = distance {
            self._pre_intersection_distance = d;
        }

        if camera_underground {
            let distance_underground = self.get_zoom_distance_underground(ctx, &ray);
            distance = Some(match distance {
                Some(d) => d.min(distance_underground),
                None => distance_underground,
            });
        }

        let distance = distance.unwrap_or(height);

        let unit_position = Cartesian3::normalize_new(ctx.camera.position());
        let unit_position_dot_direction = Cartesian3::dot(&unit_position, ctx.camera.direction());

        let zoom_movement = ZoomMovement {
            start_position: m_start,
            end_position: m_end,
            inertia_enabled: inertia_movement,
        };
        let zoom_factor = self.zoom_factor;
        self.handle_zoom(
            ctx,
            start_position,
            &zoom_movement,
            zoom_factor,
            distance,
            Some(unit_position_dot_direction),
        );
    }

    /// `tilt3D(controller, startPosition, movement)`.
    ///
    /// The 3D tilt action. A pinch tilt flattens to its `angleAndHeight`
    /// sub-movement; the gesture then either looks (once `_looking` is latched),
    /// tilts on the ellipsoid, or tilts on terrain depending on the camera
    /// height and `_tiltOnEllipsoid`.
    fn tilt_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        // `if (!Matrix4.equals(camera.transform, Matrix4.IDENTITY)) return`.
        if !Matrix4::equals(ctx.camera.transform(), &Matrix4::IDENTITY) {
            return;
        }

        // `if (defined(movement.angleAndHeight)) movement = movement.angleAndHeight`.
        let flattened = movement.angle_and_height().map(MovementInput::Mouse);
        let movement = flattened.as_ref().unwrap_or(movement);

        if !Cartesian2::equals(Some(&start_position), Some(&self._tilt_center_mouse_position)) {
            self._tilt_on_ellipsoid = false;
            self._looking = false;
        }

        if self._looking {
            let mut up_vec = Cartesian3::default();
            let up = if self
                ._ellipsoid
                .geodetic_surface_normal(ctx.camera.position(), &mut up_vec)
            {
                Some(up_vec)
            } else {
                None
            };
            self.look_3d_impl(ctx, start_position, movement, up);
            return;
        }

        let mut cartographic = Cartographic::default();
        let height = if self
            ._ellipsoid
            .cartesian_to_cartographic(ctx.camera.position(), &mut cartographic)
        {
            cartographic.height
        } else {
            0.0
        };

        if self._tilt_on_ellipsoid || height > self._minimum_collision_terrain_height {
            self._tilt_on_ellipsoid = true;
            self.tilt_3d_on_ellipsoid(ctx, start_position, movement);
        } else {
            self.tilt_3d_on_terrain(ctx, start_position, movement);
        }
    }

    /// `tilt3DOnEllipsoid(controller, startPosition, movement)`.
    ///
    /// Picks the ellipsoid point under the canvas centre (or the grazing-altitude
    /// point when the centre ray misses), builds an east-north-up frame there and
    /// rotates about its `UNIT_Z` with the ellipsoid temporarily swapped to
    /// `UNIT_SPHERE` (so the rotate rate is frame-independent), restoring
    /// everything afterwards.
    fn tilt_3d_on_ellipsoid(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        let (m_start, m_end) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };

        // `height` reads `camera.positionWC`; refresh so the cache matches the
        // CesiumJS live getter (module DEVIATION).
        ctx.camera.refresh();
        let ellipsoid = self._ellipsoid;
        let mut cartographic = Cartographic::default();
        let height = if ellipsoid.cartesian_to_cartographic(ctx.camera.position_wc(), &mut cartographic)
        {
            cartographic.height
        } else {
            0.0
        };

        let min_height = self.minimum_zoom_distance * 0.25;
        if height - min_height - 1.0 < CesiumMath::EPSILON3 && m_end.y - m_start.y < 0.0 {
            return;
        }

        let window_position = Cartesian2::new(
            ctx.canvas_client_width / 2.0,
            ctx.canvas_client_height / 2.0,
        );
        let ray = match ctx.camera.get_pick_ray(&window_position) {
            Some(ray) => ray,
            None => return,
        };

        let center = if let Some(intersection) = IntersectionTests::ray_ellipsoid(&ray, &ellipsoid) {
            Ray::get_point_new(&ray, Some(intersection.start))
        } else if height > self._minimum_track_ball_height {
            let grazing = match IntersectionTests::grazing_altitude_location(&ray, &ellipsoid) {
                Some(grazing) => grazing,
                None => return,
            };
            let mut grazing_cart = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(&grazing, &mut grazing_cart);
            grazing_cart.height = 0.0;
            let mut center = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&grazing_cart, &mut center);
            center
        } else {
            self._looking = true;
            let mut up_vec = Cartesian3::default();
            let up = if self
                ._ellipsoid
                .geodetic_surface_normal(ctx.camera.position(), &mut up_vec)
            {
                Some(up_vec)
            } else {
                None
            };
            self.look_3d_impl(ctx, start_position, movement, up);
            self._tilt_center_mouse_position = start_position;
            return;
        };

        let transform = east_north_up_to_fixed_frame_new(&center, Some(&ellipsoid));

        let old_globe = ctx.globe.take();
        let old_ellipsoid = self._ellipsoid;
        self._ellipsoid = Ellipsoid::UNIT_SPHERE;
        self._rotate_factor = 1.0;
        self._rotate_rate_range_adjustment = 1.0;

        let old_transform = *ctx.camera.transform();
        ctx.camera.set_transform(transform);

        self.rotate_3d_impl(
            ctx,
            start_position,
            movement,
            Some(Cartesian3::UNIT_Z),
            false,
            false,
        );

        ctx.camera.set_transform(old_transform);
        ctx.globe = old_globe;
        self._ellipsoid = old_ellipsoid;

        let radius = old_ellipsoid.maximum_radius();
        self._rotate_factor = 1.0 / radius;
        self._rotate_rate_range_adjustment = radius;
    }

    /// `tilt3DOnTerrain(controller, startPosition, movement)`.
    ///
    /// Tilts about the picked terrain point. Builds both an east-north-up frame
    /// at the centre and a "vertical" frame at the ray/ellipsoid intersection,
    /// applies the vertical rotate then the horizontal rotate, re-orthogonalises
    /// the orientation about the constrained axis, runs terrain collision
    /// adjustment, and finally rotates the pose back onto the original position
    /// when collision moved the camera.
    ///
    /// DEVIATION: every early return happens before the globe/ellipsoid swap and
    /// the `_setTransform` calls, so the temporary state is always restored (the
    /// CesiumJS swap region likewise has no early return).
    fn tilt_3d_on_terrain(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        let ellipsoid = self._ellipsoid;
        let camera_underground = self._camera_underground;

        let center: Cartesian3;
        if Cartesian2::equals(Some(&start_position), Some(&self._tilt_center_mouse_position)) {
            center = self._tilt_center;
        } else {
            let mut center_opt = self.pick_position(ctx, &start_position);
            let mut start_ray: Option<Ray> = None;

            if center_opt.is_none() {
                let ray = match ctx.camera.get_pick_ray(&start_position) {
                    Some(ray) => ray,
                    None => return,
                };
                match IntersectionTests::ray_ellipsoid(&ray, &ellipsoid) {
                    None => {
                        let mut cartographic = Cartographic::default();
                        let height = if ellipsoid
                            .cartesian_to_cartographic(ctx.camera.position(), &mut cartographic)
                        {
                            cartographic.height
                        } else {
                            0.0
                        };
                        if height <= self._minimum_track_ball_height {
                            self._looking = true;
                            let mut up_vec = Cartesian3::default();
                            let up = if self
                                ._ellipsoid
                                .geodetic_surface_normal(ctx.camera.position(), &mut up_vec)
                            {
                                Some(up_vec)
                            } else {
                                None
                            };
                            self.look_3d_impl(ctx, start_position, movement, up);
                            self._tilt_center_mouse_position = start_position;
                        }
                        return;
                    }
                    Some(intersection) => {
                        center_opt = Some(Ray::get_point_new(&ray, Some(intersection.start)));
                        start_ray = Some(ray);
                    }
                }
            }

            if camera_underground {
                let ray = match start_ray {
                    Some(ray) => ray,
                    None => match ctx.camera.get_pick_ray(&start_position) {
                        Some(ray) => ray,
                        None => return,
                    },
                };
                let picked = match center_opt {
                    Some(picked) => picked,
                    None => return,
                };
                center_opt = Some(self.get_tilt_center_underground(ctx, &ray, &picked));
            }

            let picked = match center_opt {
                Some(picked) => picked,
                None => return,
            };
            self._tilt_center_mouse_position = start_position;
            self._tilt_center = picked;
            center = picked;
        }

        let window_position =
            Cartesian2::new(ctx.canvas_client_width / 2.0, self._tilt_center_mouse_position.y);
        let ray = match ctx.camera.get_pick_ray(&window_position) {
            Some(ray) => ray,
            None => return,
        };

        let mag = Cartesian3::magnitude(&center);
        let radii = Cartesian3::new(mag, mag, mag);
        let new_ellipsoid = Ellipsoid::from_cartesian3(Some(&radii));

        let intersection = match IntersectionTests::ray_ellipsoid(&ray, &new_ellipsoid) {
            Some(intersection) => intersection,
            None => return,
        };

        let t = if Cartesian3::magnitude(&ray.origin) > mag {
            intersection.start
        } else {
            intersection.stop
        };
        let vertical_center = Ray::get_point_new(&ray, Some(t));

        let transform = east_north_up_to_fixed_frame_new(&center, Some(&ellipsoid));
        let vertical_transform =
            east_north_up_to_fixed_frame_new(&vertical_center, Some(&new_ellipsoid));

        let old_globe = ctx.globe.take();
        let old_ellipsoid = self._ellipsoid;
        self._ellipsoid = Ellipsoid::UNIT_SPHERE;
        self._rotate_factor = 1.0;
        self._rotate_rate_range_adjustment = 1.0;

        let mut constrained_axis = Some(Cartesian3::UNIT_Z);

        let old_transform = *ctx.camera.transform();
        // `set_transform` re-publishes the world members, so `positionWC` /
        // `rightWC` below are the same world values CesiumJS reads.
        ctx.camera.set_transform(vertical_transform);

        let position_wc = *ctx.camera.position_wc();
        let right_wc = *ctx.camera.right_wc();
        let tangent = Cartesian3::cross_new(&vertical_center, &position_wc);
        let dot = Cartesian3::dot(&right_wc, &tangent);

        if dot < 0.0 {
            let movement_delta = match (movement.start_position(), movement.end_position()) {
                (Some(start), Some(end)) => start.y - end.y,
                _ => 0.0,
            };
            if (camera_underground && movement_delta < 0.0)
                || (!camera_underground && movement_delta > 0.0)
            {
                // Prevent the camera from flipping past the up axis.
                constrained_axis = None;
            }

            let old_constrained_axis = ctx.camera.constrained_axis();
            ctx.camera.set_constrained_axis(None);
            self.rotate_3d_impl(ctx, start_position, movement, constrained_axis, true, false);
            ctx.camera.set_constrained_axis(old_constrained_axis);
        } else {
            self.rotate_3d_impl(ctx, start_position, movement, constrained_axis, true, false);
        }

        ctx.camera.set_transform(transform);
        self.rotate_3d_impl(ctx, start_position, movement, constrained_axis, false, true);

        if let Some(constrained) = ctx.camera.constrained_axis() {
            let direction = *ctx.camera.direction();
            let right = Cartesian3::cross_new(&direction, &constrained);
            if !Cartesian3::equals_epsilon(
                Some(&right),
                Some(&Cartesian3::ZERO),
                Some(CesiumMath::EPSILON6),
                None,
            ) {
                let camera_right = *ctx.camera.right();
                let right = if Cartesian3::dot(&right, &camera_right) < 0.0 {
                    Cartesian3::negate_new(&right)
                } else {
                    right
                };
                let up = Cartesian3::cross_new(&right, &direction);
                let new_right = Cartesian3::cross_new(&direction, &up);
                let up = Cartesian3::normalize_new(&up);
                let new_right = Cartesian3::normalize_new(&new_right);
                ctx.camera.set_up(up);
                ctx.camera.set_right(new_right);
            }
        }

        ctx.camera.set_transform(old_transform);
        ctx.globe = old_globe;
        self._ellipsoid = old_ellipsoid;

        let radius = old_ellipsoid.maximum_radius();
        self._rotate_factor = 1.0 / radius;
        self._rotate_rate_range_adjustment = radius;

        self.correct_position_after_terrain(ctx, vertical_transform, old_transform);
    }

    /// The shared tail of `tilt3DOnTerrain` and `rotateCVOnTerrain`: after the
    /// temporary transform is restored, run terrain collision adjustment and,
    /// when it moved the camera, rotate the pose back onto the original world
    /// position (clamping the radius so the camera never ends up farther out).
    ///
    /// DEVIATION: CesiumJS inlines this identical block in both
    /// `tilt3DOnTerrain` and `rotateCVOnTerrain`; the port factors it into one
    /// helper (no behaviour change) so the two call sites cannot drift.
    fn correct_position_after_terrain(
        &mut self,
        ctx: &mut SsccSceneContext,
        vertical_transform: Matrix4,
        old_transform: Matrix4,
    ) {
        let original_position = *ctx.camera.position_wc();

        if self.enable_collision_detection {
            self.adjust_height_for_terrain(ctx, true);
        }

        // `adjustHeightForTerrain` may write `camera.position` directly (identity
        // transform path), leaving the world cache stale; refresh before reading.
        ctx.camera.refresh();
        if !Cartesian3::equals(Some(ctx.camera.position_wc()), Some(&original_position)) {
            ctx.camera.set_transform(vertical_transform);
            let original_position = ctx.camera.world_to_camera_coordinates_point(&original_position);

            let mag_sqrd = Cartesian3::magnitude_squared(&original_position);
            let mut camera_position = *ctx.camera.position();
            if Cartesian3::magnitude_squared(&camera_position) > mag_sqrd {
                camera_position = Cartesian3::normalize_new(&camera_position);
                camera_position =
                    Cartesian3::multiply_by_scalar_new(&camera_position, mag_sqrd.sqrt());
                ctx.camera.set_position(camera_position);
            }

            let angle = Cartesian3::angle_between(&original_position, &camera_position);
            let axis = Cartesian3::cross_new(&original_position, &camera_position);
            let axis = Cartesian3::normalize_new(&axis);

            let quaternion = Quaternion::from_axis_angle_new(&axis, angle);
            let rotation = Matrix3::from_quaternion_new(&quaternion);
            let direction = Matrix3::multiply_by_vector_new(&rotation, ctx.camera.direction());
            let up_rotated = Matrix3::multiply_by_vector_new(&rotation, ctx.camera.up());
            let right = Cartesian3::cross_new(&direction, &up_rotated);
            let up = Cartesian3::cross_new(&right, &direction);
            ctx.camera.set_direction(direction);
            ctx.camera.set_up(up);
            ctx.camera.set_right(right);

            ctx.camera.set_transform(old_transform);
        }
    }

    /// `look3D(controller, startPosition, movement)` — the three-argument form
    /// `update3D` binds to the look event types (no rotation axis).
    fn look_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &MovementInput,
    ) {
        self.look_3d_impl(ctx, start_position, movement, None);
    }

    /// `look3D(controller, startPosition, movement, rotationAxis)`.
    ///
    /// Rotates the camera orientation (not position) by the angle subtended
    /// between the pick rays through the start/end pixels, first horizontally
    /// (about `rotationAxis` / `_horizontalRotationAxis` / left) then vertically
    /// (clamped so the direction never crosses the rotation axis).
    fn look_3d_impl(
        &mut self,
        ctx: &mut SsccSceneContext,
        _start_position: Cartesian2,
        movement: &MovementInput,
        rotation_axis: Option<Cartesian3>,
    ) {
        let (m_start, m_end) = match (movement.start_position(), movement.end_position()) {
            (Some(start), Some(end)) => (start, end),
            _ => return,
        };
        let orthographic = ctx.camera.frustum().is_orthographic();

        // ---- Horizontal (x) ----
        let start_pos = Cartesian2::new(m_start.x, 0.0);
        let end_pos = Cartesian2::new(m_end.x, 0.0);
        let start_ray = match ctx.camera.get_pick_ray(&start_pos) {
            Some(ray) => ray,
            None => return,
        };
        let end_ray = match ctx.camera.get_pick_ray(&end_pos) {
            Some(ray) => ray,
            None => return,
        };

        let (start, end) = if orthographic {
            let direction = *ctx.camera.direction();
            let position = *ctx.camera.position();
            let mut s = Cartesian3::add_new(&direction, &start_ray.origin);
            let mut e = Cartesian3::add_new(&direction, &end_ray.origin);
            s = Cartesian3::subtract_new(&s, &position);
            e = Cartesian3::subtract_new(&e, &position);
            (Cartesian3::normalize_new(&s), Cartesian3::normalize_new(&e))
        } else {
            (start_ray.direction, end_ray.direction)
        };

        let mut angle = 0.0;
        let dot = Cartesian3::dot(&start, &end);
        if dot < 1.0 {
            // dot is in [0, 1].
            angle = dot.acos();
        }
        let angle = if m_start.x > m_end.x { -angle } else { angle };

        let horizontal_rotation_axis = self._horizontal_rotation_axis;
        if let Some(axis) = rotation_axis {
            ctx.camera.look(&axis, Some(-angle));
        } else if let Some(axis) = horizontal_rotation_axis {
            ctx.camera.look(&axis, Some(-angle));
        } else {
            ctx.camera.look_left(Some(angle));
        }

        // ---- Vertical (y) ----
        let start_pos = Cartesian2::new(0.0, m_start.y);
        let end_pos = Cartesian2::new(0.0, m_end.y);
        let start_ray = match ctx.camera.get_pick_ray(&start_pos) {
            Some(ray) => ray,
            None => return,
        };
        let end_ray = match ctx.camera.get_pick_ray(&end_pos) {
            Some(ray) => ray,
            None => return,
        };

        let (start, end) = if orthographic {
            let direction = *ctx.camera.direction();
            let position = *ctx.camera.position();
            let mut s = Cartesian3::add_new(&direction, &start_ray.origin);
            let mut e = Cartesian3::add_new(&direction, &end_ray.origin);
            s = Cartesian3::subtract_new(&s, &position);
            e = Cartesian3::subtract_new(&e, &position);
            (Cartesian3::normalize_new(&s), Cartesian3::normalize_new(&e))
        } else {
            (start_ray.direction, end_ray.direction)
        };

        let mut angle = 0.0;
        let dot = Cartesian3::dot(&start, &end);
        if dot < 1.0 {
            // dot is in [0, 1].
            angle = dot.acos();
        }
        let mut angle = if m_start.y > m_end.y { -angle } else { angle };

        let rotation_axis = rotation_axis.or(horizontal_rotation_axis);
        if let Some(axis) = rotation_axis {
            let direction = *ctx.camera.direction();
            let negative_rotation_axis = Cartesian3::negate_new(&axis);
            let north_parallel = Cartesian3::equals_epsilon(
                Some(&direction),
                Some(&axis),
                Some(CesiumMath::EPSILON2),
                None,
            );
            let south_parallel = Cartesian3::equals_epsilon(
                Some(&direction),
                Some(&negative_rotation_axis),
                Some(CesiumMath::EPSILON2),
                None,
            );
            if !north_parallel && !south_parallel {
                let dot = Cartesian3::dot(&direction, &axis);
                let mut angle_to_axis = CesiumMath::acos_clamped(dot);
                if angle > 0.0 && angle > angle_to_axis {
                    angle = angle_to_axis - CesiumMath::EPSILON4;
                }
                let dot = Cartesian3::dot(&direction, &negative_rotation_axis);
                angle_to_axis = CesiumMath::acos_clamped(dot);
                if angle < 0.0 && -angle > angle_to_axis {
                    angle = -angle_to_axis + CesiumMath::EPSILON4;
                }
                let tangent = Cartesian3::cross_new(&axis, &direction);
                ctx.camera.look(&tangent, Some(angle));
            } else if (north_parallel && angle < 0.0) || (south_parallel && angle > 0.0) {
                let right = *ctx.camera.right();
                ctx.camera.look(&right, Some(-angle));
            }
        } else {
            ctx.camera.look_up(Some(angle));
        }
    }

    // ---- Per-mode update dispatch ----
    //
    // DEVIATION: `update2D` / `updateCV` / `update3D` register their
    // `reactToInput` calls in b3-3d/e; they are stubs here so `update` builds.

    /// `update2D(controller)`.
    ///
    /// With a camera transform set (look-at / entity tracking) only zoom — and,
    /// when 2D rotation is allowed, twist — are wired; otherwise translate and
    /// zoom run, with twist bound to the tilt event types.
    fn update_2d(&mut self, ctx: &mut SsccSceneContext) {
        let rotatable_2d = ctx.map_mode_2d == MapMode2D::Rotate;
        let transform_set = !Matrix4::equals(&Matrix4::IDENTITY, ctx.camera.transform());

        if transform_set {
            let (enable_zoom, zoom_types, inertia_zoom) =
                (self.enable_zoom, self.zoom_event_types.clone(), self.inertia_zoom);
            self.react_to_input(
                ctx,
                enable_zoom,
                zoom_types,
                SsccAction::Zoom2D,
                Some((inertia_zoom, InertiaState::Zoom)),
            );
            if rotatable_2d {
                let (enable_rotate, translate_types, inertia_spin) = (
                    self.enable_rotate,
                    self.translate_event_types.clone(),
                    self.inertia_spin,
                );
                self.react_to_input(
                    ctx,
                    enable_rotate,
                    translate_types,
                    SsccAction::Twist2D,
                    Some((inertia_spin, InertiaState::Spin)),
                );
            }
        } else {
            let (enable_translate, translate_types, inertia_translate) = (
                self.enable_translate,
                self.translate_event_types.clone(),
                self.inertia_translate,
            );
            self.react_to_input(
                ctx,
                enable_translate,
                translate_types,
                SsccAction::Translate2D,
                Some((inertia_translate, InertiaState::Translate)),
            );

            let (enable_zoom, zoom_types, inertia_zoom) =
                (self.enable_zoom, self.zoom_event_types.clone(), self.inertia_zoom);
            self.react_to_input(
                ctx,
                enable_zoom,
                zoom_types,
                SsccAction::Zoom2D,
                Some((inertia_zoom, InertiaState::Zoom)),
            );

            if rotatable_2d {
                let (enable_rotate, tilt_types, inertia_spin) = (
                    self.enable_rotate,
                    self.tilt_event_types.clone(),
                    self.inertia_spin,
                );
                self.react_to_input(
                    ctx,
                    enable_rotate,
                    tilt_types,
                    SsccAction::Twist2D,
                    Some((inertia_spin, InertiaState::Tilt)),
                );
            }
        }
    }

    /// `updateCV(controller)` — binds the Columbus-view actions to their event
    /// types. When a tracking transform is set it reuses the 3D rotate/zoom;
    /// otherwise it binds tilt→`rotateCV`, translate→`translateCV`,
    /// zoom→`zoomCV`, look→`look3D`.
    ///
    /// DEVIATION: the CesiumJS bounce-back tween
    /// (`camera.createCorrectPositionTween` + `_tweens.contains/add/update`) is
    /// not ported — `Camera::create_correct_position_tween` is unimplemented and
    /// `SsccSceneContext` carries no frame time for
    /// `TweenCollection::update(&JulianDate)`. The portable half
    /// (`anyButtonDown` ⇒ `_tweens.removeAll()`) is kept; tween creation/update
    /// is deferred to b3-3f, so `_tween` is never assigned (CesiumJS also leaves
    /// it `undefined` until a tween is created).
    fn update_cv(&mut self, ctx: &mut SsccSceneContext) {
        if !Matrix4::equals(&Matrix4::IDENTITY, ctx.camera.transform()) {
            let (enable_rotate, rotate_types, inertia_spin) = (
                self.enable_rotate,
                self.rotate_event_types.clone(),
                self.inertia_spin,
            );
            self.react_to_input(
                ctx,
                enable_rotate,
                rotate_types,
                SsccAction::Rotate3D,
                Some((inertia_spin, InertiaState::Spin)),
            );
            let (enable_zoom, zoom_types, inertia_zoom) = (
                self.enable_zoom,
                self.zoom_event_types.clone(),
                self.inertia_zoom,
            );
            self.react_to_input(
                ctx,
                enable_zoom,
                zoom_types,
                SsccAction::Zoom3D,
                Some((inertia_zoom, InertiaState::Zoom)),
            );
        } else {
            if self._aggregator.any_button_down() {
                self._tweens.remove_all();
            }

            let (enable_tilt, tilt_types, inertia_spin) = (
                self.enable_tilt,
                self.tilt_event_types.clone(),
                self.inertia_spin,
            );
            self.react_to_input(
                ctx,
                enable_tilt,
                tilt_types,
                SsccAction::RotateCv,
                Some((inertia_spin, InertiaState::Tilt)),
            );
            let (enable_translate, translate_types, inertia_translate) = (
                self.enable_translate,
                self.translate_event_types.clone(),
                self.inertia_translate,
            );
            self.react_to_input(
                ctx,
                enable_translate,
                translate_types,
                SsccAction::TranslateCv,
                Some((inertia_translate, InertiaState::Translate)),
            );
            let (enable_zoom, zoom_types, inertia_zoom) = (
                self.enable_zoom,
                self.zoom_event_types.clone(),
                self.inertia_zoom,
            );
            self.react_to_input(
                ctx,
                enable_zoom,
                zoom_types,
                SsccAction::ZoomCv,
                Some((inertia_zoom, InertiaState::Zoom)),
            );
            let (enable_look, look_types) = (self.enable_look, self.look_event_types.clone());
            self.react_to_input(ctx, enable_look, look_types, SsccAction::Look3D, None);

            // DEVIATION: `createCorrectPositionTween` + `_tweens.contains/add/update`
            // deferred (see the method note).
        }
    }

    /// `update3D(controller)` — binds the 3D rotate/zoom/tilt/look actions to
    /// their event types. CesiumJS reuses `inertiaSpin` for both spin and tilt
    /// (storing tilt inertia under `_lastInertiaTiltMovement`), and passes no
    /// inertia for `look3D`.
    fn update_3d(&mut self, ctx: &mut SsccSceneContext) {
        let (enable_rotate, rotate_types, inertia_spin) = (
            self.enable_rotate,
            self.rotate_event_types.clone(),
            self.inertia_spin,
        );
        self.react_to_input(
            ctx,
            enable_rotate,
            rotate_types,
            SsccAction::Spin3D,
            Some((inertia_spin, InertiaState::Spin)),
        );

        let (enable_zoom, zoom_types, inertia_zoom) =
            (self.enable_zoom, self.zoom_event_types.clone(), self.inertia_zoom);
        self.react_to_input(
            ctx,
            enable_zoom,
            zoom_types,
            SsccAction::Zoom3D,
            Some((inertia_zoom, InertiaState::Zoom)),
        );

        let (enable_tilt, tilt_types, inertia_spin) =
            (self.enable_tilt, self.tilt_event_types.clone(), self.inertia_spin);
        self.react_to_input(
            ctx,
            enable_tilt,
            tilt_types,
            SsccAction::Tilt3D,
            Some((inertia_spin, InertiaState::Tilt)),
        );

        let (enable_look, look_types) = (self.enable_look, self.look_event_types.clone());
        self.react_to_input(ctx, enable_look, look_types, SsccAction::Look3D, None);
    }

    /// `adjustHeightForTerrain(controller, cameraChanged)`.
    ///
    /// Keeps the camera at least `minimumZoomDistance` above the globe surface
    /// when it dips below `_minimumCollisionTerrainHeight`, using
    /// `scene.globeHeight`. A non-identity camera transform is temporarily reset
    /// to identity (so height is measured in world space) and restored,
    /// re-deriving the orientation from the adjusted position.
    fn adjust_height_for_terrain(&mut self, ctx: &mut SsccSceneContext, camera_changed: bool) {
        self._adjusted_height_for_terrain = true;

        let mode = ctx.mode;
        if mode == SceneMode::Scene2D || mode == SceneMode::Morphing {
            return;
        }

        // `scene.ellipsoid ?? Ellipsoid.WGS84` — the projection carries the scene
        // ellipsoid in the port (see `SsccSceneContext::map_projection`).
        let ellipsoid = ctx.map_projection.ellipsoid().clone();
        let minimum_zoom_distance = self.minimum_zoom_distance;
        let minimum_collision_terrain_height = self._minimum_collision_terrain_height;
        let globe_height = ctx.globe_height;

        let mut transform: Option<Matrix4> = None;
        let mut mag = 0.0;
        if !Matrix4::equals(ctx.camera.transform(), &Matrix4::IDENTITY) {
            transform = Some(*ctx.camera.transform());
            mag = Cartesian3::magnitude(ctx.camera.position());
            ctx.camera.set_transform(Matrix4::IDENTITY);
        }

        let mut cartographic = if mode == SceneMode::Scene3D {
            let mut c = Cartographic::default();
            ellipsoid.cartesian_to_cartographic(ctx.camera.position(), &mut c);
            c
        } else {
            ctx.map_projection.unproject(ctx.camera.position())
        };

        let mut height_updated = false;
        if cartographic.height < minimum_collision_terrain_height {
            if let Some(globe_height) = globe_height {
                let height = globe_height + minimum_zoom_distance;
                let last_globe_height = self._last_globe_height;
                let difference = globe_height - last_globe_height;
                let percent_difference = difference / last_globe_height;

                // Unless the camera was moved by user input, only update the height
                // once the globe height has been fairly stable across frames (to
                // avoid big jumps during tile loads).
                if cartographic.height < height
                    && (camera_changed || percent_difference.abs() <= 0.1)
                {
                    cartographic.height = height;
                    let position = if mode == SceneMode::Scene3D {
                        let mut p = Cartesian3::default();
                        ellipsoid.cartographic_to_cartesian(&cartographic, &mut p);
                        p
                    } else {
                        ctx.map_projection.project(&cartographic)
                    };
                    ctx.camera.set_position(position);
                    height_updated = true;
                }

                if camera_changed || percent_difference.abs() <= 0.1 {
                    self._last_globe_height = globe_height;
                } else {
                    self._last_globe_height += difference * 0.1;
                }
            }
        }

        if let Some(transform) = transform {
            ctx.camera.set_transform(transform);
            if height_updated {
                let mut position = Cartesian3::normalize_new(ctx.camera.position());
                let mut direction = Cartesian3::negate_new(&position);
                position =
                    Cartesian3::multiply_by_scalar_new(&position, mag.max(minimum_zoom_distance));
                direction = Cartesian3::normalize_new(&direction);
                let up = *ctx.camera.up();
                let right = Cartesian3::cross_new(&direction, &up);
                let up = Cartesian3::cross_new(&right, &direction);
                ctx.camera.set_position(position);
                ctx.camera.set_direction(direction);
                ctx.camera.set_right(right);
                ctx.camera.set_up(up);
            }
        }
    }
}

impl ScreenSpaceCameraController {
    /// `handleZoom(object, startPosition, movement, zoomFactor, distanceMeasure,
    /// unitPositionDotDirection)`.
    ///
    /// The shared zoom engine behind `zoom2D`/`zoom3D`/`zoomCV`. `movement` is
    /// the already-flattened `{ startPosition, endPosition, inertiaEnabled }`
    /// (a pinch zoom rebinds `movement = movement.distance` before calling).
    ///
    /// DEVIATION: a single `ctx.camera.refresh()` at the top synchronises the
    /// cached `positionCartographic` with any movement earlier in the frame;
    /// every `positionCartographic` read below happens before this function
    /// moves the camera, so the cached value matches the CesiumJS live getter.
    fn handle_zoom(
        &mut self,
        ctx: &mut SsccSceneContext,
        start_position: Cartesian2,
        movement: &ZoomMovement,
        zoom_factor: f64,
        distance_measure: f64,
        unit_position_dot_direction: Option<f64>,
    ) {
        ctx.camera.refresh();

        let mut percentage = 1.0;
        if let Some(dot) = unit_position_dot_direction {
            percentage = CesiumMath::clamp(dot.abs(), 0.25, 1.0);
        }

        let diff = movement.end_position.y - movement.start_position.y;

        // `distanceMeasure` is the height above the ellipsoid. Approaching the
        // surface, the zoom rate slows and stops `minimumZoomDistance` above it.
        let approaching_surface = diff > 0.0;
        let min_height = if approaching_surface {
            self.minimum_zoom_distance * percentage
        } else {
            0.0
        };
        let max_height = self.maximum_zoom_distance;

        let min_distance = distance_measure - min_height;
        let zoom_rate = zoom_factor * min_distance;
        let zoom_rate = CesiumMath::clamp(zoom_rate, self._minimum_zoom_rate, self._maximum_zoom_rate);

        let range_window_ratio = diff / ctx.canvas_client_height;
        let range_window_ratio = range_window_ratio.min(self.maximum_movement_ratio);
        let mut distance = zoom_rate * range_window_ratio;

        // `enableCollisionDetection || minimumZoomDistance === 0 || !defined(_globe)`
        // (`_globe` undefined is look-at mode).
        if self.enable_collision_detection || self.minimum_zoom_distance == 0.0 || ctx.globe.is_none() {
            if distance > 0.0 && (distance_measure - min_height).abs() < 1.0 {
                return;
            }
            if distance < 0.0 && (distance_measure - max_height).abs() < 1.0 {
                return;
            }
            if distance_measure - distance < min_height {
                distance = distance_measure - min_height - 1.0;
            } else if distance_measure - distance > max_height {
                distance = distance_measure - max_height;
            }
        }

        let mode = ctx.mode;

        // `scratchZoomViewOptions.orientation` — the pose captured before any
        // movement, re-applied by `setView` at the end.
        let orientation = SetViewOrientation {
            direction: None,
            up: None,
            heading: ctx.camera.heading(),
            pitch: ctx.camera.pitch(),
            roll: ctx.camera.roll(),
        };

        // `movement.inertiaEnabled ?? Cartesian2.equals(startPosition, _zoomMouseStart)`.
        let same_start_position = movement.inertia_enabled.unwrap_or_else(|| {
            Cartesian2::equals(Some(&start_position), Some(&self._zoom_mouse_start))
        });
        let mut zooming_on_vector = self._zooming_on_vector;
        let mut rotating_zoom = self._rotating_zoom;

        if !same_start_position {
            self._zoom_mouse_start = start_position;

            // When a camera transform is set (e.g. tracking an entity) `_globe`
            // is undefined and nothing is picked.
            let mut picked_position: Option<Cartesian3> = None;
            if ctx.globe.is_some() && mode == SceneMode::Scene2D {
                if let Some(ray) = ctx.camera.get_pick_ray(&start_position) {
                    let origin = ray.origin;
                    picked_position =
                        Some(Cartesian3::from_elements_new(origin.y, origin.z, origin.x));
                }
            } else if ctx.globe.is_some() {
                picked_position = self.pick_position(ctx, &start_position);
            }

            if let Some(picked) = picked_position {
                self._use_zoom_world_position = true;
                self._zoom_world_position = picked;
            } else {
                self._use_zoom_world_position = false;
            }

            self._zooming_on_vector = false;
            zooming_on_vector = false;
            self._rotating_zoom = false;
            rotating_zoom = false;
            self._zooming_underground = self._camera_underground;
        }

        if !self._use_zoom_world_position {
            ctx.camera.zoom_in(Some(distance));
            return;
        }

        let mut zoom_on_vector = mode == SceneMode::ColumbusView;

        if ctx.camera.position_cartographic().height < 2000000.0 {
            rotating_zoom = true;
        }

        if !same_start_position || rotating_zoom {
            if mode == SceneMode::Scene2D {
                let world_position = self._zoom_world_position;
                let end_position = *ctx.camera.position();

                if !Cartesian3::equals(Some(&world_position), Some(&end_position))
                    && ctx.camera.position_cartographic().height < self._max_coord.x * 2.0
                {
                    let saved_x = ctx.camera.position().x;

                    let direction = Cartesian3::subtract_new(&world_position, &end_position);
                    let direction = Cartesian3::normalize_new(&direction);

                    let magnitude = ctx.camera.get_magnitude().unwrap_or(0.0);
                    let d = (Cartesian3::distance(&world_position, &end_position) * distance)
                        / (magnitude * 0.5);
                    ctx.camera.move_camera(&direction, d * 0.5);

                    // Re-pick when the camera crossed the map's `x = 0` seam.
                    if (ctx.camera.position().x < 0.0 && saved_x > 0.0)
                        || (ctx.camera.position().x > 0.0 && saved_x < 0.0)
                    {
                        if let Some(ray) = ctx.camera.get_pick_ray(&start_position) {
                            let origin = ray.origin;
                            self._zoom_world_position =
                                Cartesian3::from_elements_new(origin.y, origin.z, origin.x);
                        }
                    }
                }
            } else if mode == SceneMode::Scene3D {
                let camera_position_normal = Cartesian3::normalize_new(ctx.camera.position());
                if self._camera_underground
                    || self._zooming_underground
                    || (ctx.camera.position_cartographic().height < 3000.0
                        && Cartesian3::dot(ctx.camera.direction(), &camera_position_normal).abs()
                            < 0.6)
                {
                    zoom_on_vector = true;
                } else {
                    let center_pixel = Cartesian2::new(
                        ctx.canvas_client_width / 2.0,
                        ctx.canvas_client_height / 2.0,
                    );
                    // `undefined` means the globe does not cover the screen centre.
                    let center_position = self.pick_position(ctx, &center_pixel);

                    match center_position {
                        None => zoom_on_vector = true,
                        Some(center_position) => {
                            if ctx.camera.position_cartographic().height < 1000000.0 {
                                // The great-circle math assumes the camera points
                                // toward the surface; check it here.
                                if Cartesian3::dot(ctx.camera.direction(), &camera_position_normal)
                                    >= -0.5
                                {
                                    zoom_on_vector = true;
                                } else {
                                    self.zoom_toward_target_3d(
                                        ctx,
                                        distance,
                                        orientation,
                                        &camera_position_normal,
                                    );
                                    return;
                                }
                            } else {
                                let position_normal = Cartesian3::normalize_new(&center_position);
                                let picked_normal =
                                    Cartesian3::normalize_new(&self._zoom_world_position);
                                let dot_product = Cartesian3::dot(&picked_normal, &position_normal);

                                if dot_product > 0.0 && dot_product < 1.0 {
                                    let angle = CesiumMath::acos_clamped(dot_product);
                                    let axis = Cartesian3::cross_new(&picked_normal, &position_normal);

                                    let height = ctx.camera.position_cartographic().height;
                                    let denom = if angle.abs() > CesiumMath::to_radians(20.0) {
                                        height * 0.75
                                    } else {
                                        height - distance
                                    };
                                    let scalar = distance / denom;
                                    ctx.camera.rotate(&axis, Some(angle * scalar));
                                }
                            }
                        }
                    }
                }
            }

            self._rotating_zoom = !zoom_on_vector;
        }

        if (!same_start_position && zoom_on_vector) || zooming_on_vector {
            let zoom_mouse_start = SceneTransforms::world_to_window_with_camera(
                &self._zoom_world_position,
                ctx.camera,
            );
            let ray = match zoom_mouse_start {
                Some(zoom_mouse_start)
                    if mode != SceneMode::ColumbusView
                        && Cartesian2::equals(
                            Some(&start_position),
                            Some(&self._zoom_mouse_start),
                        ) =>
                {
                    ctx.camera.get_pick_ray(&zoom_mouse_start)
                }
                _ => ctx.camera.get_pick_ray(&start_position),
            };

            if let Some(ray) = ray {
                let mut ray_direction = ray.direction;
                if mode == SceneMode::ColumbusView || mode == SceneMode::Scene2D {
                    ray_direction = Cartesian3::from_elements_new(
                        ray_direction.y,
                        ray_direction.z,
                        ray_direction.x,
                    );
                }
                ctx.camera.move_camera(&ray_direction, distance);
            }

            self._zooming_on_vector = true;
        } else {
            ctx.camera.zoom_in(Some(distance));
        }

        if !self._camera_underground {
            ctx.camera.set_view_with_options(&SetViewOptions {
                destination: None,
                orientation,
                end_transform: None,
                convert: true,
            });
        }
    }

    /// The great-circle zoom-toward-target block of `handleZoom` (CesiumJS
    /// L763–911), extracted so `handle_zoom` stays readable.
    ///
    /// Moves the camera along the arc toward `_zoomWorldPosition` by `distance`,
    /// keeping the captured `orientation`, then `return`s from `handleZoom` (the
    /// two early-outs below mirror the JS `return`s). `camera_position_normal`
    /// is `normalize(camera.position)` computed by the caller.
    fn zoom_toward_target_3d(
        &mut self,
        ctx: &mut SsccSceneContext,
        distance: f64,
        orientation: SetViewOrientation,
        camera_position_normal: &Cartesian3,
    ) {
        let mut camera_position = *ctx.camera.position();
        let target = self._zoom_world_position;

        let target_normal = Cartesian3::normalize_new(&target);
        if Cartesian3::dot(&target_normal, camera_position_normal) < 0.0 {
            return;
        }

        // `center = cameraPosition + direction * 1000`.
        let forward = *ctx.camera.direction();
        let mut center = Cartesian3::add_new(
            &camera_position,
            &Cartesian3::multiply_by_scalar_new(&forward, 1000.0),
        );

        let position_to_target = Cartesian3::subtract_new(&target, &camera_position);
        let position_to_target_normal = Cartesian3::normalize_new(&position_to_target);

        let alpha_dot = Cartesian3::dot(camera_position_normal, &position_to_target_normal);
        if alpha_dot >= 0.0 {
            // Zoomed past the target; force the next movement to re-pick.
            self._zoom_mouse_start.x = -1.0;
            return;
        }
        let alpha = (-alpha_dot).acos();
        let camera_distance = Cartesian3::magnitude(&camera_position);
        let target_distance = Cartesian3::magnitude(&target);
        let remaining_distance = camera_distance - distance;
        let position_to_target_distance = Cartesian3::magnitude(&position_to_target);

        let gamma = (CesiumMath::clamp(
            (position_to_target_distance / target_distance) * alpha.sin(),
            -1.0,
            1.0,
        ))
        .asin();
        let delta = (CesiumMath::clamp(
            (remaining_distance / target_distance) * alpha.sin(),
            -1.0,
            1.0,
        ))
        .asin();
        let beta = gamma - delta + alpha;

        let mut up = Cartesian3::normalize_new(&camera_position);
        let right = Cartesian3::normalize_new(&Cartesian3::cross_new(&position_to_target_normal, &up));
        let mut forward = Cartesian3::normalize_new(&Cartesian3::cross_new(&up, &right));

        // New position to move to.
        center = Cartesian3::multiply_by_scalar_new(
            &Cartesian3::normalize_new(&center),
            Cartesian3::magnitude(&center) - distance,
        );
        camera_position = Cartesian3::multiply_by_scalar_new(
            &Cartesian3::normalize_new(&camera_position),
            remaining_distance,
        );

        // Pan.
        let p_mid = Cartesian3::multiply_by_scalar_new(
            &Cartesian3::add_new(
                &Cartesian3::multiply_by_scalar_new(&up, beta.cos() - 1.0),
                &Cartesian3::multiply_by_scalar_new(&forward, beta.sin()),
            ),
            remaining_distance,
        );
        camera_position = Cartesian3::add_new(&camera_position, &p_mid);

        up = Cartesian3::normalize_new(&center);
        forward = Cartesian3::normalize_new(&Cartesian3::cross_new(&up, &right));

        let c_mid = Cartesian3::multiply_by_scalar_new(
            &Cartesian3::add_new(
                &Cartesian3::multiply_by_scalar_new(&up, beta.cos() - 1.0),
                &Cartesian3::multiply_by_scalar_new(&forward, beta.sin()),
            ),
            Cartesian3::magnitude(&center),
        );
        center = Cartesian3::add_new(&center, &c_mid);

        // Update the camera: new position, then direction/right/up rebuilt from
        // cross products (the JS `clone(camera.direction, camera.direction)`
        // self-copy is a no-op and is dropped).
        ctx.camera.set_position(camera_position);
        let direction = Cartesian3::normalize_new(&Cartesian3::subtract_new(&center, &camera_position));
        ctx.camera.set_direction(direction);
        let old_up = *ctx.camera.up();
        let right = Cartesian3::cross_new(ctx.camera.direction(), &old_up);
        ctx.camera.set_right(right);
        let up = Cartesian3::cross_new(ctx.camera.right(), ctx.camera.direction());
        ctx.camera.set_up(up);

        ctx.camera.set_view_with_options(&SetViewOptions {
            destination: None,
            orientation,
            end_transform: None,
            convert: true,
        });
    }
}
