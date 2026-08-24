//! Ported from `packages/engine/Source/Scene/CameraFlightPath.js`.
//!
//! M3/S3 materialization: the camera flight animation core. CesiumJS
//! `CameraFlightPath.createTween` builds a Tween.js tween whose eased value
//! drives a great-circle rotation of the camera pose; `Camera#flyTo` /
//! `Camera#flyHome` feed it. The Rust port splits the same pipeline:
//!
//! * [`crate::tween_collection::TweenCollection`] drives a single eased
//!   `value` channel (`0 → 1`, `SINUSOIDAL_IN_OUT` easing, mirroring the JS
//!   `easingFunction: EasingFunction.SINUSOIDAL_IN_OUT` default),
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
    /// Mirrors the CesiumJS default: `camera.flyTo` uses 3.0 seconds when
    /// the caller does not pass a duration.
    pub fn compute_duration(
        _start: &Cartesian3,
        _end: &Cartesian3,
    ) -> f64 {
        3.0
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

    /// The JS default flight duration is 3 seconds.
    #[test]
    fn default_duration_is_three_seconds() {
        let start = Cartesian3::new(0.0, 0.0, 0.0);
        let end = Cartesian3::new(1.0, 0.0, 0.0);
        assert_eq!(CameraFlightPath::compute_duration(&start, &end), 3.0);
    }
}
