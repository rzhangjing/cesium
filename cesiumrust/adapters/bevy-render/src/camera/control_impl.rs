//! `CameraControl` driving-port adapter (M2.5).
//!
//! Exposes the domain camera algorithms — [`CameraController`], [`CameraFlight`]
//! and the `compute_*` flight helpers — through the [`CameraControl`] driving
//! port (`cesium-ports-driving`), so external code (a script, a widget, a test)
//! can programmatically fly/set/orient/zoom the camera without touching the
//! mouse / keyboard / touch input path.
//!
//! # Layering
//! * All camera *math* stays in the domain: great-arc slerp flight
//!   ([`CameraFlight::fly_to`] → [`CameraFlight::update`]), `set_view` /
//!   `look_at` pose construction ([`compute_set_view`] / [`compute_look_at`]) and
//!   zoom ([`CameraController::zoom`]). This adapter only
//!   1. converts the port's [`Cartographic`] API into the domain's ECEF `DVec3`
//!      vocabulary, and
//!   2. does the meters→normalized-delta boundary conversion for zoom, exactly
//!      mirroring the pixel→world conversions in [`super::controller_system`] /
//!      [`super::touch_system`] (magnitude at the boundary, semantics in the
//!      domain).
//! * Everything is `f64` (domain); the single `f32` GPU boundary remains
//!   [`super::update_system`], which this module does not touch.
//!
//! # Bevy bridge
//! [`CameraControlImpl`] is a self-contained, ECS-free object (directly
//! instantiable in a unit/integration test). [`CameraControlPort`] wraps it as a
//! Bevy [`Resource`] and [`camera_control_port_system`] syncs it to the live
//! [`CesiumCamera`] entity each `PostUpdate` (before the Transform writer):
//! * when a command was issued or a flight is active, the port is authoritative
//!   and its camera is pushed to the entity;
//! * otherwise the entity is authoritative (mouse/touch/keyboard drove it) and is
//!   copied back into the port, so the next command starts from the current pose.
//!
//! The system is inert without a [`CesiumCamera`] entity, so registering it never
//! perturbs apps that drive the camera another way.

use bevy::prelude::*;
use cesium_camera::Camera;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::Cartographic;
use cesium_interaction::{
    compute_flight_duration, compute_look_at, compute_set_view, select_flight_easing,
    CameraController, CameraControllerConfig, CameraFlight,
};
use cesium_ports_driving::CameraControl;
use glam::{DQuat, DVec3};
use std::f64::consts::{FRAC_PI_2, TAU};

use crate::camera::components::CesiumCamera;

/// A read-only snapshot of the camera pose returned by
/// [`CameraControlImpl::get_camera_state`].
///
/// `heading`/`pitch`/`roll` follow the CesiumJS `Camera` convention (radians):
/// heading `0` = north, increasing eastward; pitch `0` = horizon, `-π/2` =
/// straight down; roll `0` = level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    /// Camera position (ECEF, meters).
    pub position: DVec3,
    /// View direction (unit).
    pub direction: DVec3,
    /// Up direction (unit).
    pub up: DVec3,
    /// Right direction (unit).
    pub right: DVec3,
    /// Heading in radians (`[0, 2π)`).
    pub heading: f64,
    /// Pitch in radians (`[-π/2, π/2]`).
    pub pitch: f64,
    /// Roll in radians.
    pub roll: f64,
}

/// The `CameraControl` driving-port implementation.
///
/// Owns a domain [`Camera`], the reference [`Ellipsoid`], and at most one active
/// [`CameraFlight`]. Every port method delegates the geometry to the domain and
/// bumps an internal generation counter so the Bevy bridge can tell "a command
/// was issued" from "idle".
pub struct CameraControlImpl {
    camera: Camera,
    ellipsoid: Ellipsoid,
    flight: Option<CameraFlight>,
    generation: u64,
}

impl CameraControlImpl {
    /// Creates a control bound to `camera` on `ellipsoid`.
    pub fn new(camera: Camera, ellipsoid: Ellipsoid) -> Self {
        Self {
            camera,
            ellipsoid,
            flight: None,
            generation: 0,
        }
    }

    /// Borrow the underlying domain camera.
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Mutably borrow the underlying domain camera (used by the Bevy bridge to
    /// re-seed the port from the live entity).
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// The reference ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Command generation; increments on every port method call.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Whether a flight is currently active.
    pub fn is_flying(&self) -> bool {
        self.flight.is_some()
    }

    /// Advances the active flight by `dt` seconds, applying it to the camera.
    /// Returns `true` while the flight is still in progress after this step.
    pub fn update(&mut self, dt: f64) -> bool {
        let Some(flight) = self.flight.as_mut() else {
            return false;
        };
        let still_flying = flight.apply_to_camera(&mut self.camera, dt);
        if !still_flying {
            self.flight = None;
        }
        still_flying
    }

    /// Snapshots the current pose (position/orientation + heading/pitch/roll).
    pub fn get_camera_state(&self) -> CameraState {
        let (heading, pitch, roll) = compute_heading_pitch_roll(
            self.camera.position,
            self.camera.direction,
            self.camera.up,
        );
        CameraState {
            position: self.camera.position,
            direction: self.camera.direction,
            up: self.camera.up,
            right: self.camera.right,
            heading,
            pitch,
            roll,
        }
    }

    /// The camera's current cartographic position (lon/lat/height), if it is not
    /// at the ellipsoid center.
    pub fn camera_cartographic(&self) -> Option<Cartographic> {
        self.ellipsoid.cartesian_to_cartographic(self.camera.position)
    }

    /// Builds a default-configured domain controller (magnitudes come from the
    /// port call, semantics/limits from the domain).
    #[inline]
    fn controller(&self) -> CameraController {
        CameraController {
            config: CameraControllerConfig::default(),
            ellipsoid: self.ellipsoid,
        }
    }

    /// Assigns an absolute pose, re-orthonormalizing right/up like [`Camera::new`].
    fn apply_pose(&mut self, position: DVec3, direction: DVec3, up: DVec3) {
        self.camera.position = position;
        let direction = direction.normalize();
        self.camera.direction = direction;
        let right = direction.cross(up).normalize();
        self.camera.right = right;
        self.camera.up = right.cross(direction).normalize();
    }

    /// Default zoom step when the port passes `None`: 10 % of the current height
    /// above the surface (matches [`CameraController::zoom`]'s own scaling).
    #[inline]
    fn default_zoom_step(&self) -> f64 {
        let height = (self.camera.position.length() - self.ellipsoid.maximum_radius()).abs();
        (height * 0.1).max(1.0)
    }

    /// Moves the camera along its view direction by `meters` (positive = forward
    /// / zoom in). Converts the metric amount into the normalized delta the
    /// domain [`CameraController::zoom`] consumes, keeping collision + speed
    /// limits in the domain.
    fn zoom_by_meters(&mut self, meters: f64) {
        let ctrl = self.controller();
        let height = (self.camera.position.length() - self.ellipsoid.maximum_radius())
            .abs()
            .max(1000.0);
        // domain zoom moves by `height * 0.1 * delta * zoom_speed` (zoom_speed = 1).
        let delta = meters / (height * 0.1);
        ctrl.zoom(&mut self.camera, delta);
        ctrl.enforce_collision(&mut self.camera);
        self.flight = None;
        self.generation += 1;
    }
}

impl Default for CameraControlImpl {
    fn default() -> Self {
        let ellipsoid = Ellipsoid::WGS84;
        let position = Camera::default_home_position(&ellipsoid);
        let direction = -position.normalize();
        Self::new(Camera::new(position, direction, DVec3::Z), ellipsoid)
    }
}

impl CameraControl for CameraControlImpl {
    fn set_view(&mut self, position: Cartographic, heading: f64, pitch: f64, roll: f64) {
        // `compute_set_view` places the camera at `height` above (lon, lat) and
        // orients it by heading/pitch; roll is applied about the view direction.
        let (pos, dir, up) =
            compute_set_view(&position, position.height, heading, pitch, &self.ellipsoid);
        let (dir, up) = apply_roll(dir, up, roll);
        self.apply_pose(pos, dir, up);
        self.flight = None;
        self.generation += 1;
    }

    fn fly_to(
        &mut self,
        destination: Cartographic,
        heading: Option<f64>,
        pitch: Option<f64>,
        roll: Option<f64>,
        duration_secs: f64,
    ) {
        // The end pose is exactly a `set_view` at the destination: heading/pitch
        // default to "look straight down" (CesiumJS `flyTo` default) which
        // `compute_set_view(0, -π/2)` reproduces as `-destination.normalize()`.
        let h = heading.unwrap_or(0.0);
        let p = pitch.unwrap_or(-FRAC_PI_2);
        let r = roll.unwrap_or(0.0);
        let (dest_ecef, dir, up) =
            compute_set_view(&destination, destination.height, h, p, &self.ellipsoid);
        let (dir, up) = apply_roll(dir, up, r);

        // Great-arc flight: `CameraFlight::update` slerps about the center with a
        // parabolic arch; duration/easing derive from the travelled distance when
        // the caller leaves `duration_secs` non-positive.
        let distance = (dest_ecef - self.camera.position).length();
        let duration = if duration_secs > 0.0 {
            duration_secs
        } else {
            compute_flight_duration(distance)
        };
        let mut flight =
            CameraFlight::fly_to(&self.camera, dest_ecef, Some(dir), Some(up), duration);
        flight.easing = select_flight_easing(distance);
        self.flight = Some(flight);
        self.generation += 1;
    }

    fn look_at(&mut self, target: Cartographic, heading: f64, pitch: f64, range: f64) {
        let target_ecef = self.ellipsoid.cartographic_to_cartesian(&target);
        let up = target_ecef.normalize();
        let (east, north, _) = local_enu(up);

        // Camera sits `range` away from the target at elevation `-pitch` and the
        // azimuth opposite the look heading (heading = direction the camera looks).
        let horiz = range * pitch.cos();
        let vert = -range * pitch.sin();
        let dir_h = -(north * heading.cos() + east * heading.sin());
        let offset = dir_h * horiz + up * vert;

        let (pos, dir, cam_up) = compute_look_at(target_ecef, offset);
        self.apply_pose(pos, dir, cam_up);
        self.flight = None;
        self.generation += 1;
    }

    fn zoom_in(&mut self, amount: Option<f64>) {
        let meters = amount.unwrap_or_else(|| self.default_zoom_step());
        self.zoom_by_meters(meters.abs());
    }

    fn zoom_out(&mut self, amount: Option<f64>) {
        let meters = amount.unwrap_or_else(|| self.default_zoom_step());
        self.zoom_by_meters(-meters.abs());
    }

    fn home(&mut self) {
        let dest = Camera::default_home_position(&self.ellipsoid);
        let distance = (dest - self.camera.position).length();
        let duration = compute_flight_duration(distance);
        let dir = -dest.normalize();
        let mut flight = CameraFlight::fly_to(&self.camera, dest, Some(dir), Some(DVec3::Z), duration);
        flight.easing = select_flight_easing(distance);
        self.flight = Some(flight);
        self.generation += 1;
    }
}

/// Rotates `up` about `direction` by `roll` (no-op when `roll == 0`).
#[inline]
fn apply_roll(direction: DVec3, up: DVec3, roll: f64) -> (DVec3, DVec3) {
    if roll == 0.0 {
        return (direction, up);
    }
    let q = DQuat::from_axis_angle(direction.normalize(), roll);
    (direction, (q * up).normalize())
}

/// Local East-North-Up basis for a geodetic up vector `up` (unit). Falls back to
/// a `Y`-derived east at the poles where `Z × up` degenerates.
fn local_enu(up: DVec3) -> (DVec3, DVec3, DVec3) {
    let mut east = DVec3::Z.cross(up);
    if east.length_squared() < 1e-12 {
        east = DVec3::Y.cross(up);
    }
    let east = east.normalize();
    let north = up.cross(east).normalize();
    (east, north, up)
}

/// Derives CesiumJS-convention heading/pitch/roll from a pose.
fn compute_heading_pitch_roll(position: DVec3, direction: DVec3, up: DVec3) -> (f64, f64, f64) {
    let n = position.normalize();
    if !n.is_finite() || n.length_squared() < 0.5 {
        // Degenerate (camera at/near the center): no meaningful local frame.
        return (0.0, 0.0, 0.0);
    }
    let (east, north, _) = local_enu(n);

    // Pitch: elevation of the view direction below the local horizon.
    let pitch = direction.dot(n).clamp(-1.0, 1.0).asin();

    // Heading: azimuth of the direction's horizontal projection, from north.
    let horiz = direction - n * direction.dot(n);
    let heading = if horiz.length_squared() < 1e-18 {
        0.0
    } else {
        let h = horiz.normalize();
        h.dot(east).atan2(h.dot(north)).rem_euclid(TAU)
    };

    // Roll: signed angle from the "level" up to the camera up, about direction.
    let up_level = n - direction * n.dot(direction);
    let roll = if up_level.length_squared() < 1e-18 {
        0.0
    } else {
        let up_level = up_level.normalize();
        let right_level = direction.cross(up_level).normalize();
        up.dot(right_level).atan2(up.dot(up_level))
    };

    (heading, pitch, roll)
}

/// Bevy [`Resource`] wrapping a [`CameraControlImpl`] so the driving port can be
/// fetched from the world and used to programmatically control the camera.
///
/// Implements [`CameraControl`] by forwarding to the inner control, so callers
/// can `port.fly_to(...)` directly (with the trait in scope).
#[derive(Resource)]
pub struct CameraControlPort {
    control: CameraControlImpl,
    last_gen: u64,
}

impl Default for CameraControlPort {
    fn default() -> Self {
        let control = CameraControlImpl::default();
        Self {
            last_gen: control.generation(),
            control,
        }
    }
}

impl CameraControlPort {
    /// Wraps an existing control implementation.
    pub fn new(control: CameraControlImpl) -> Self {
        Self {
            last_gen: control.generation(),
            control,
        }
    }

    /// Borrow the inner control.
    pub fn control(&self) -> &CameraControlImpl {
        &self.control
    }

    /// Mutably borrow the inner control.
    pub fn control_mut(&mut self) -> &mut CameraControlImpl {
        &mut self.control
    }
}

impl CameraControl for CameraControlPort {
    fn set_view(&mut self, position: Cartographic, heading: f64, pitch: f64, roll: f64) {
        self.control.set_view(position, heading, pitch, roll);
    }

    fn fly_to(
        &mut self,
        destination: Cartographic,
        heading: Option<f64>,
        pitch: Option<f64>,
        roll: Option<f64>,
        duration_secs: f64,
    ) {
        self.control
            .fly_to(destination, heading, pitch, roll, duration_secs);
    }

    fn look_at(&mut self, target: Cartographic, heading: f64, pitch: f64, range: f64) {
        self.control.look_at(target, heading, pitch, range);
    }

    fn zoom_in(&mut self, amount: Option<f64>) {
        self.control.zoom_in(amount);
    }

    fn zoom_out(&mut self, amount: Option<f64>) {
        self.control.zoom_out(amount);
    }

    fn home(&mut self) {
        self.control.home();
    }
}

/// `PostUpdate` bridge between the [`CameraControlPort`] driving port and the
/// live [`CesiumCamera`] entity.
///
/// Runs before [`super::camera_update_system`] so the Transform writer sees the
/// port-driven pose. Authoritative direction per frame:
/// * a command was issued this frame (`generation` changed) **or** a flight is
///   active → push the port's camera onto the entity;
/// * otherwise → copy the entity back into the port, so the next programmatic
///   command (e.g. a great-arc `fly_to`) starts from the pose the mouse/touch
///   path left the camera in.
///
/// Inert when no [`CesiumCamera`] entity exists.
pub fn camera_control_port_system(
    mut cameras: Query<&mut CesiumCamera>,
    mut port: ResMut<CameraControlPort>,
    time: Res<Time>,
) {
    let dt = time.delta_secs_f64();
    let was_flying = port.control.is_flying();
    let still_flying = port.control.update(dt);
    let commanded = port.control.generation() != port.last_gen;
    port.last_gen = port.control.generation();

    let Ok(mut cesium_cam) = cameras.get_single_mut() else {
        return;
    };
    if was_flying || still_flying || commanded {
        cesium_cam.camera = port.control.camera().clone();
    } else {
        *port.control.camera_mut() = cesium_cam.camera.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f64 = 6378137.0;

    fn equator_camera() -> Camera {
        Camera::new(
            DVec3::new(R * 2.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    }

    #[test]
    fn heading_pitch_roll_of_a_level_downward_camera() {
        // Equator, looking straight down, up = north → heading 0, pitch -π/2, roll 0.
        let cam = equator_camera();
        let (h, p, r) = compute_heading_pitch_roll(cam.position, cam.direction, cam.up);
        assert!(h.abs() < 1e-9 || (h - TAU).abs() < 1e-9, "heading {h}");
        // `asin` amplifies a 1-ulp dot-product error to ~1.5e-8 rad at nadir, so
        // the tolerance is 1e-6 rad (still ~3e-6 degrees — negligible).
        assert!((p + FRAC_PI_2).abs() < 1e-6, "pitch {p}");
        assert!(r.abs() < 1e-6, "roll {r}");
    }

    #[test]
    fn set_view_places_camera_above_target_looking_down() {
        let mut ctrl = CameraControlImpl::new(equator_camera(), Ellipsoid::WGS84);
        let carto = Cartographic::from_degrees(10.0, 20.0, 500_000.0);
        ctrl.set_view(carto, 0.0, -FRAC_PI_2, 0.0);
        let expected = Ellipsoid::WGS84.cartographic_to_cartesian(&carto);
        let state = ctrl.get_camera_state();
        assert!((state.position - expected).length() < 1e-3, "{}", state.position);
        // Looking straight down ⇒ direction ≈ -surface normal, pitch ≈ -π/2.
        assert!((state.pitch + FRAC_PI_2).abs() < 1e-6, "pitch {}", state.pitch);
    }

    #[test]
    fn fly_to_great_arc_lands_on_destination() {
        let mut ctrl = CameraControlImpl::new(equator_camera(), Ellipsoid::WGS84);
        let dest = Cartographic::from_degrees(-75.0, 40.0, 1_000_000.0);
        ctrl.fly_to(dest, None, None, None, 0.0);
        assert!(ctrl.is_flying());
        let expected = Ellipsoid::WGS84.cartographic_to_cartesian(&dest);
        for _ in 0..1000 {
            if !ctrl.update(0.05) {
                break;
            }
        }
        assert!(!ctrl.is_flying(), "flight must complete");
        let err = (ctrl.get_camera_state().position - expected).length();
        assert!(err / R < 1e-9, "fly_to error {err} m");
    }

    #[test]
    fn look_at_holds_range_and_aims_at_target() {
        let mut ctrl = CameraControlImpl::new(equator_camera(), Ellipsoid::WGS84);
        let target = Cartographic::from_degrees(0.0, 0.0, 0.0);
        let range = 2_000_000.0;
        ctrl.look_at(target, 0.0, -FRAC_PI_2, range);
        let target_ecef = Ellipsoid::WGS84.cartographic_to_cartesian(&target);
        let state = ctrl.get_camera_state();
        let dist = (state.position - target_ecef).length();
        assert!((dist - range).abs() / range < 1e-9, "range {dist}");
        let aim = (target_ecef - state.position).normalize();
        assert!(aim.dot(state.direction) > 1.0 - 1e-9, "must aim at target");
    }

    #[test]
    fn zoom_in_then_out_moves_along_view() {
        let mut ctrl = CameraControlImpl::new(equator_camera(), Ellipsoid::WGS84);
        let len0 = ctrl.get_camera_state().position.length();
        ctrl.zoom_in(Some(100_000.0));
        let len1 = ctrl.get_camera_state().position.length();
        assert!((len0 - len1 - 100_000.0).abs() < 1.0, "in: {len0}→{len1}");
        ctrl.zoom_out(Some(250_000.0));
        let len2 = ctrl.get_camera_state().position.length();
        assert!((len2 - len1 - 250_000.0).abs() < 1.0, "out: {len1}→{len2}");
    }
}
