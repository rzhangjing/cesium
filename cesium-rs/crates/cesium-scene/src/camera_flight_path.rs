//! Ported from `packages/engine/Source/Scene/CameraFlightPath.js`.
//!
//! M3/S3 materialization: the camera flight animation core. CesiumJS
//! `CameraFlightPath.createTween` builds a Tween.js tween whose eased value
//! drives a great-circle rotation of the camera pose; `Camera#flyTo` /
//! `Camera#flyHome` feed it. The Rust port splits the same pipeline:
//!
//! * [`crate::tween_collection::TweenCollection`] drives a single eased
//!   `value` channel (`0 → 1`; the default easing is selected by
//!   [`CameraFlightPath::create_tween`] — `QUINTIC_IN_OUT`, or `CUBIC_OUT`
//!   when descending from above 11500 m, mirroring the JS default),
//! * the tween callbacks write the eased value into a [`CameraFlight`]
//!   shared through a [`CameraFlightChannel`] (an `Rc<RefCell<...>>` the
//!   `Scene` and the `Camera` both hold — this is what lets `Scene::fly_to`
//!   work through `&self`, mirroring the JS `scene.camera.flyTo(...)`),
//! * [`Camera::update`](crate::camera::Camera::update) reads the channel
//!   each frame and applies the interpolated pose via
//!   [`CameraFlightPath::interpolate`].
//!
//! DEVIATION: CesiumJS rotates the position along a great circle around the
//! ellipsoid center (`axisRotation` on `startPosition × endPosition`) which
//! produces an arced flight; the port linearly interpolates the position and
//! slerps the direction/up unit vectors. The endpoint and timing semantics
//! match; the mid-flight arc is a straight chord.

use std::cell::RefCell;
use std::rc::Rc;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::easing_function::{cubic_out, linear_none};
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::transforms;

use crate::camera::Camera;
use crate::tween_collection::{EasingFn, TweenOptions};

/// The shared flight channel: `Scene` installs a [`CameraFlight`] when a
/// flight starts (and clears it via the cancel callback), the flight tween
/// updates `t` / `completed`, and `Camera::update` consumes it.
pub type CameraFlightChannel = Rc<RefCell<Option<CameraFlight>>>;

/// One in-flight camera animation (mirrors the JS tween closure state of
/// `CameraFlightPath.createTween`: the captured start/end poses plus the
/// eased progress).
pub struct CameraFlight {
    /// The camera position when the flight started.
    pub start_position: Cartesian3,
    /// The camera direction when the flight started.
    pub start_direction: Cartesian3,
    /// The camera up vector when the flight started.
    pub start_up: Cartesian3,
    /// The destination position.
    pub end_position: Cartesian3,
    /// The destination direction.
    pub end_direction: Cartesian3,
    /// The destination up vector.
    pub end_up: Cartesian3,
    /// The eased progress in `[0, 1]`, written by the tween update callback.
    pub t: f64,
    /// Set by the tween complete callback; `Camera::update` applies the
    /// exact end pose and clears the channel when it sees this.
    pub completed: bool,
}

/// The options consumed by [`CameraFlightPath::create_tween`], mirroring
/// the CesiumJS `CameraFlightPath.createTween(scene, options)` options
/// object (`destination`, `duration`, `easingFunction`, `complete`,
/// `cancel`).
///
/// DEVIATION: CesiumJS additionally accepts `convert`, `maximumHeight`,
/// `flyOverLongitude`, `flyOverLongitudeWeight`, `pitchAdjustHeight`,
/// `heading`/`pitch`/`roll`, and `endTransform`. The port drives a
/// pose-based flight (position/direction/up) and lands in the JS default
/// orientation (heading 0, pitch -π/2, roll 0 — straight down), so the
/// orientation/height-shaping options are not ported; `endTransform` is
/// deferred.
pub struct CameraFlightTweenOptions {
    /// The destination to fly to (JS `options.destination`, required).
    pub destination: Cartesian3,
    /// The flight duration in seconds (JS `options.duration`). When `None`
    /// it is derived from the travel distance, mirroring the JS default.
    pub duration: Option<f64>,
    /// The easing function (JS `options.easingFunction`). When `None` the
    /// JS default is selected (`CUBIC_OUT` when descending from above
    /// 11500 m, otherwise `QUINTIC_IN_OUT`).
    pub easing_function: Option<EasingFn>,
    /// Called once when the flight completes (JS `options.complete`).
    pub complete: Option<Box<dyn FnOnce()>>,
    /// Called when the flight is canceled (JS `options.cancel`).
    pub cancel: Option<Box<dyn FnOnce()>>,
}

/// CesiumJS `EasingFunction.QUINTIC_IN_OUT` (Tween.js `Quintic.InOut`).
/// Defined locally because the `cesium-core` easing table (treated as
/// read-only for this task) does not yet include the quintic family.
fn quintic_in_out(t: f64) -> f64 {
    let t2 = t * 2.0;
    if t2 < 1.0 {
        0.5 * t2 * t2 * t2 * t2 * t2
    } else {
        let t3 = t2 - 2.0;
        0.5 * (t3 * t3 * t3 * t3 * t3 + 2.0)
    }
}

/// Computes a flight path for the camera between two positions.
///
/// This is used to animate smooth camera transitions.
pub struct CameraFlightPath;

impl CameraFlightPath {
    /// Interpolates the flight pose at the eased progress `t`.
    ///
    /// Position is lerped, direction/up are slerped as unit vectors; the up
    /// vector is then re-orthonormalized against the direction (mirroring
    /// the JS `updateMembers` pass that runs when the camera view matrix is
    /// recomputed).
    pub fn interpolate(
        flight: &CameraFlight,
        t: f64,
    ) -> (Cartesian3, Cartesian3, Cartesian3) {
        let t = t.clamp(0.0, 1.0);
        let mut position = Cartesian3::new(0.0, 0.0, 0.0);
        Cartesian3::lerp(&flight.start_position, &flight.end_position, t, &mut position);
        let direction = slerp_unit(&flight.start_direction, &flight.end_direction, t);
        let up_raw = slerp_unit(&flight.start_up, &flight.end_up, t);
        // Re-orthonormalize up against the interpolated direction.
        let right = Cartesian3::cross_new(&direction, &up_raw);
        let up = if Cartesian3::magnitude(&right) > 1e-10 {
            let right = Cartesian3::normalize_new(&right);
            Cartesian3::cross_new(&right, &direction)
        } else {
            up_raw
        };
        (position, direction, up)
    }

    /// Creates a flight animation from the current camera position to the destination.
    pub fn create_rotation(
        _start: &Cartesian3,
        _end: &Cartesian3,
        _duration: f64,
    ) -> Vec<(f64, Cartesian3)> {
        // DEVIATION: Requires spline interpolation
        Vec::new()
    }

    /// Computes the duration for a flight based on distance.
    ///
    /// Mirrors the CesiumJS `createTween` default when the caller does not
    /// pass a duration: `min(ceil(distance / 1e6) + 2, 3)` seconds.
    pub fn compute_duration(
        start: &Cartesian3,
        end: &Cartesian3,
    ) -> f64 {
        let distance = Cartesian3::distance(start, end);
        ((distance / 1_000_000.0).ceil() + 2.0).min(3.0)
    }

    /// Creates the camera-flight tween for a destination.
    ///
    /// Rust port of CesiumJS `CameraFlightPath.createTween(scene, options)`.
    /// Returns a [`TweenOptions`] ready to be added to the scene's
    /// [`crate::tween_collection::TweenCollection`] (the JS returns the
    /// object passed to `scene.tweens.add`). The eased value channel drives
    /// a [`CameraFlight`] installed on `flight_channel`, which
    /// [`crate::camera::Camera::update`] consumes each frame.
    ///
    /// Faithful edge semantics:
    /// * `duration <= 0` → an "empty flight" that applies the exact end pose
    ///   and fires `complete` on the very next update (JS `emptyFlight` with
    ///   an immediate `update({ time: 1.0 })`);
    /// * an explicit `easing_function` (e.g. `linear_none`) is honored;
    /// * canceling the returned tween fires `cancel` and never `complete`
    ///   (enforced by the [`crate::tween_collection::TweenCollection`]).
    pub fn create_tween(
        camera: &Camera,
        flight_channel: &CameraFlightChannel,
        options: CameraFlightTweenOptions,
    ) -> TweenOptions {
        let ellipsoid = Ellipsoid::WGS84;
        let destination = options.destination;

        // The end pose lands in the CesiumJS default orientation
        // (heading 0, pitch -PI/2, roll 0): looking straight down from the
        // east-north-up frame at the destination (mirrors `setView`
        // default inside the JS `createUpdate3D`).
        let frame =
            transforms::east_north_up_to_fixed_frame_new(&destination, Some(&ellipsoid));
        let e = &frame.elements;
        let end_up = Cartesian3::new(e[4], e[5], e[6]);
        let end_direction = Cartesian3::new(-e[8], -e[9], -e[10]);

        *flight_channel.borrow_mut() = Some(CameraFlight {
            start_position: *camera.position(),
            start_direction: *camera.direction(),
            start_up: *camera.up(),
            end_position: destination,
            end_direction,
            end_up,
            t: 0.0,
            completed: false,
        });

        // Default duration mirrors the JS:
        // `min(ceil(distance / 1e6) + 2, 3)`.
        let duration = options
            .duration
            .unwrap_or_else(|| Self::compute_duration(camera.position(), &destination));

        // `duration <= 0` → empty flight: a zero-duration tween whose
        // complete callback snaps to the exact end pose (the camera applies
        // it on the next update) and then invokes the user callback.
        if duration <= 0.0 {
            let complete_channel = flight_channel.clone();
            let user_complete = options.complete;
            let cancel_channel = flight_channel.clone();
            let user_cancel = options.cancel;
            return TweenOptions {
                start_object: Vec::new(),
                stop_object: Vec::new(),
                duration: 0.0,
                delay: 0.0,
                easing_function: linear_none,
                update: None,
                complete: Some(Box::new(move || {
                    if let Some(flight) = complete_channel.borrow_mut().as_mut() {
                        flight.completed = true;
                    }
                    if let Some(complete) = user_complete {
                        complete();
                    }
                })),
                cancel: Some(Box::new(move || {
                    *cancel_channel.borrow_mut() = None;
                    if let Some(cancel) = user_cancel {
                        cancel();
                    }
                })),
            };
        }

        // Default easing mirrors the JS: `CUBIC_OUT` when descending from
        // above 11500 m, otherwise `QUINTIC_IN_OUT`.
        let easing_function = options.easing_function.unwrap_or_else(|| {
            let mut start_carto = Cartographic { longitude: 0.0, latitude: 0.0, height: 0.0 };
            let mut end_carto = Cartographic { longitude: 0.0, latitude: 0.0, height: 0.0 };
            let start_ok =
                ellipsoid.cartesian_to_cartographic(camera.position(), &mut start_carto);
            let end_ok = ellipsoid.cartesian_to_cartographic(&destination, &mut end_carto);
            if start_ok
                && end_ok
                && start_carto.height > end_carto.height
                && start_carto.height > 11500.0
            {
                cubic_out
            } else {
                quintic_in_out
            }
        });

        let mut tween = TweenOptions::new(
            vec![("time".to_string(), 0.0)],
            vec![("time".to_string(), 1.0)],
            duration,
        );
        tween.easing_function = easing_function;

        let update_channel = flight_channel.clone();
        tween.update = Some(Box::new(move |values| {
            if let Some(flight) = update_channel.borrow_mut().as_mut() {
                flight.t = values[0].1;
            }
        }));

        let complete_channel = flight_channel.clone();
        let user_complete = options.complete;
        tween.complete = Some(Box::new(move || {
            if let Some(flight) = complete_channel.borrow_mut().as_mut() {
                flight.completed = true;
            }
            if let Some(complete) = user_complete {
                complete();
            }
        }));

        let cancel_channel = flight_channel.clone();
        let user_cancel = options.cancel;
        tween.cancel = Some(Box::new(move || {
            *cancel_channel.borrow_mut() = None;
            if let Some(cancel) = user_cancel {
                cancel();
            }
        }));

        tween
    }
}

/// Spherical linear interpolation between two unit vectors.
fn slerp_unit(start: &Cartesian3, end: &Cartesian3, t: f64) -> Cartesian3 {
    let dot = Cartesian3::dot(start, end).clamp(-1.0, 1.0);
    let theta = dot.acos();
    if theta < 1e-10 {
        return *start;
    }
    let sin_theta = theta.sin();
    let weight_start = ((1.0 - t) * theta).sin() / sin_theta;
    let weight_end = (t * theta).sin() / sin_theta;
    let combined = Cartesian3::add_new(
        &Cartesian3::multiply_by_scalar_new(start, weight_start),
        &Cartesian3::multiply_by_scalar_new(end, weight_end),
    );
    Cartesian3::normalize_new(&combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    use cesium_core::julian_date::JulianDate;

    use crate::tween_collection::TweenCollection;

    fn flight() -> CameraFlight {
        CameraFlight {
            start_position: Cartesian3::new(0.0, 0.0, 1.0),
            start_direction: Cartesian3::new(0.0, 0.0, -1.0),
            start_up: Cartesian3::new(0.0, 1.0, 0.0),
            end_position: Cartesian3::new(2.0, 0.0, 1.0),
            end_direction: Cartesian3::new(1.0, 0.0, 0.0),
            end_up: Cartesian3::new(0.0, 1.0, 0.0),
            t: 0.0,
            completed: false,
        }
    }

    /// The endpoints are reproduced exactly at t = 0 and t = 1.
    #[test]
    fn interpolate_endpoints() {
        let flight = flight();
        let (position, direction, up) = CameraFlightPath::interpolate(&flight, 0.0);
        assert!((position.z - 1.0).abs() < 1e-12);
        assert!((direction.z + 1.0).abs() < 1e-9);
        assert!((up.y - 1.0).abs() < 1e-9);

        let (position, direction, _) = CameraFlightPath::interpolate(&flight, 1.0);
        assert!((position.x - 2.0).abs() < 1e-12);
        assert!((direction.x - 1.0).abs() < 1e-9);
    }

    /// Position lerps and the direction stays a unit vector mid-flight.
    #[test]
    fn interpolate_midpoint() {
        let flight = flight();
        let (position, direction, up) = CameraFlightPath::interpolate(&flight, 0.5);
        assert!((position.x - 1.0).abs() < 1e-12);
        assert!((Cartesian3::magnitude(&direction) - 1.0).abs() < 1e-9);
        // direction halfway between -Z and +X: normalized (-1,0,1)/sqrt(2)
        assert!((direction.x - 0.7071067811865476).abs() < 1e-9);
        // up stays orthogonal to the direction.
        assert!(Cartesian3::dot(&direction, &up).abs() < 1e-9);
    }

    /// The JS default flight duration is distance-derived:
    /// `min(ceil(distance / 1e6) + 2, 3)`.
    #[test]
    fn default_duration_is_distance_derived() {
        // No travel → ceil(0) + 2 = 2 s.
        assert_eq!(
            CameraFlightPath::compute_duration(
                &Cartesian3::new(0.0, 0.0, 0.0),
                &Cartesian3::new(0.0, 0.0, 0.0),
            ),
            2.0,
        );
        // 1.5e6 m apart → ceil(1.5) + 2 = 4, capped at 3 s.
        assert_eq!(
            CameraFlightPath::compute_duration(
                &Cartesian3::new(0.0, 0.0, 0.0),
                &Cartesian3::new(1_500_000.0, 0.0, 0.0),
            ),
            3.0,
        );
    }

    /// `create_tween` installs a flight on the channel whose end pose is
    /// the JS default orientation (straight down) at the destination, and
    /// derives the duration from the travel distance.
    #[test]
    fn create_tween_installs_flight_with_default_pose_and_duration() {
        let camera = Camera::new();
        let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let destination = Cartesian3::new(Ellipsoid::WGS84.maximum_radius() + 1.0, 0.0, 0.0);
        let tween = CameraFlightPath::create_tween(
            &camera,
            &channel,
            CameraFlightTweenOptions {
                destination,
                duration: None,
                easing_function: None,
                complete: None,
                cancel: None,
            },
        );

        // Distance ≈ 6.378e6 → ceil(6.378) + 2 = 9, capped at 3 s.
        assert!((tween.duration - 3.0).abs() < 1e-12);

        let flight = channel.borrow();
        let flight = flight.as_ref().expect("flight installed");
        assert_eq!(flight.end_position, destination);
        // Destination on the equator at +X: straight down is -X, up is +Z.
        assert!((flight.end_direction.x + 1.0).abs() < 1e-9);
        assert!(flight.end_direction.y.abs() < 1e-9);
        assert!(flight.end_direction.z.abs() < 1e-9);
        assert!(flight.end_up.x.abs() < 1e-9);
        assert!((flight.end_up.z - 1.0).abs() < 1e-9);
        assert_eq!(flight.t, 0.0);
        assert!(!flight.completed);
    }

    /// `duration <= 0` → the JS "empty flight": a zero-duration tween
    /// whose completion snaps the channel to `completed` and fires the
    /// user callback.
    #[test]
    fn create_tween_zero_duration_is_empty_flight() {
        let camera = Camera::new();
        let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let completed = Rc::new(Cell::new(false));
        let tween;
        {
            let completed = completed.clone();
            tween = CameraFlightPath::create_tween(
                &camera,
                &channel,
                CameraFlightTweenOptions {
                    destination: Cartesian3::new(Ellipsoid::WGS84.maximum_radius(), 0.0, 0.0),
                    duration: Some(0.0),
                    easing_function: None,
                    complete: Some(Box::new(move || completed.set(true))),
                    cancel: None,
                },
            );
        }
        assert_eq!(tween.duration, 0.0);

        let mut tweens = TweenCollection::new();
        tweens.add(tween);
        tweens.update(&JulianDate::now());
        assert!(completed.get());
        assert!(channel.borrow().as_ref().unwrap().completed);
        assert!(tweens.is_empty());
    }

    /// The default easing is `QUINTIC_IN_OUT` for short low flights:
    /// quintic_in_out(0.25) = 0.015625.
    #[test]
    fn create_tween_default_easing_is_quintic_in_out() {
        let camera = Camera::new();
        let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let tween = CameraFlightPath::create_tween(
            &camera,
            &channel,
            CameraFlightTweenOptions {
                destination: Cartesian3::new(Ellipsoid::WGS84.maximum_radius() + 1.0, 0.0, 0.0),
                duration: Some(4.0),
                easing_function: None,
                complete: None,
                cancel: None,
            },
        );
        let mut tweens = TweenCollection::new();
        tweens.add(tween);
        let start = JulianDate::now();
        tweens.update(&start);
        tweens.update(&JulianDate::add_seconds_new(&start, 1.0));
        assert!((channel.borrow().as_ref().unwrap().t - 0.015625).abs() < 1e-12);
    }

    /// Descending from above 11500 m defaults to `CUBIC_OUT`:
    /// cubic_out(0.25) = 0.578125.
    #[test]
    fn create_tween_descents_from_height_use_cubic_out() {
        let mut camera = Camera::new();
        let radius = Ellipsoid::WGS84.maximum_radius();
        camera.set_position(Cartesian3::new(radius + 100_000.0, 0.0, 0.0));
        let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let tween = CameraFlightPath::create_tween(
            &camera,
            &channel,
            CameraFlightTweenOptions {
                destination: Cartesian3::new(radius + 1_000.0, 0.0, 0.0),
                duration: Some(4.0),
                easing_function: None,
                complete: None,
                cancel: None,
            },
        );
        let mut tweens = TweenCollection::new();
        tweens.add(tween);
        let start = JulianDate::now();
        tweens.update(&start);
        tweens.update(&JulianDate::add_seconds_new(&start, 1.0));
        assert!((channel.borrow().as_ref().unwrap().t - 0.578125).abs() < 1e-12);
    }

    /// Canceling a flight tween fires the user cancel callback, clears the
    /// channel, and never fires complete (JS cancel semantics).
    #[test]
    fn create_tween_cancel_clears_channel_and_skips_complete() {
        let camera = Camera::new();
        let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let completed = Rc::new(Cell::new(false));
        let canceled = Rc::new(Cell::new(false));
        let tween;
        {
            let completed = completed.clone();
            let canceled = canceled.clone();
            tween = CameraFlightPath::create_tween(
                &camera,
                &channel,
                CameraFlightTweenOptions {
                    destination: Cartesian3::new(Ellipsoid::WGS84.maximum_radius(), 0.0, 0.0),
                    duration: Some(2.0),
                    easing_function: None,
                    complete: Some(Box::new(move || completed.set(true))),
                    cancel: Some(Box::new(move || canceled.set(true))),
                },
            );
        }
        let mut tweens = TweenCollection::new();
        let id = tweens.add(tween);
        tweens.cancel(id);
        tweens.update(&JulianDate::now());
        assert!(canceled.get());
        assert!(!completed.get());
        assert!(channel.borrow().is_none());
    }

    /// An explicit `easing_function` (LINEAR_NONE) is honored over the
    /// default selection.
    #[test]
    fn create_tween_honors_explicit_linear_easing() {
        let camera = Camera::new();
        let channel: CameraFlightChannel = Rc::new(RefCell::new(None));
        let tween = CameraFlightPath::create_tween(
            &camera,
            &channel,
            CameraFlightTweenOptions {
                destination: Cartesian3::new(Ellipsoid::WGS84.maximum_radius() + 1.0, 0.0, 0.0),
                duration: Some(4.0),
                easing_function: Some(linear_none),
                complete: None,
                cancel: None,
            },
        );
        let mut tweens = TweenCollection::new();
        tweens.add(tween);
        let start = JulianDate::now();
        tweens.update(&start);
        tweens.update(&JulianDate::add_seconds_new(&start, 1.0));
        assert!((channel.borrow().as_ref().unwrap().t - 0.25).abs() < 1e-12);
    }
}
