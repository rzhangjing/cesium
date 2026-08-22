//! Ported from packages/engine/Source/Core/Ray.js
//!
//! Represents a ray that extends infinitely from the provided origin in the
//! provided direction.

use crate::cartesian3::Cartesian3;
use crate::developer_error::throw_developer_error;

/// Represents a ray that extends infinitely from the provided origin in the
/// provided direction.
///
/// Port of `Ray`.
#[derive(Clone, Debug, PartialEq)]
pub struct Ray {
    /// The origin of the ray.
    ///
    /// Port of `Ray#origin`.
    pub origin: Cartesian3,

    /// The direction of the ray (always normalized).
    ///
    /// Port of `Ray#direction`.
    pub direction: Cartesian3,
}

impl Default for Ray {
    /// Port of `new Ray()` (no arguments).
    fn default() -> Self {
        Self {
            origin: Cartesian3::ZERO,
            direction: Cartesian3::ZERO,
        }
    }
}

impl Ray {
    /// Creates a new `Ray`.
    ///
    /// Port of the `Ray(origin, direction)` constructor.
    ///
    /// The direction is normalized unless it is the zero vector.
    pub fn new(origin: Option<&Cartesian3>, direction: Option<&Cartesian3>) -> Self {
        let dir = direction.copied().unwrap_or(Cartesian3::ZERO);
        let mut direction_val = dir;
        if direction_val != Cartesian3::ZERO {
            let normalized = Cartesian3::normalize_new(&direction_val);
            direction_val = normalized;
        }

        Self {
            origin: origin.copied().unwrap_or(Cartesian3::ZERO),
            direction: direction_val,
        }
    }

    /// Duplicates a `Ray` instance.
    ///
    /// Port of `Ray.clone`.
    ///
    /// Returns `None` if `ray` is `None` (mirrors JS `undefined` → `undefined`).
    pub fn clone(ray: Option<&Self>, result: Option<&mut Self>) -> Option<Self> {
        let ray = ray?;
        if let Some(res) = result {
            res.origin = ray.origin;
            res.direction = ray.direction;
            None // JS returns the mutated result; Rust caller already holds it
        } else {
            Some(Self {
                origin: ray.origin,
                direction: ray.direction,
            })
        }
    }

    /// Allocating variant of [`Ray::clone`] that returns a new `Ray`.
    ///
    /// Returns `None` if `ray` is `None`.
    pub fn clone_new(ray: Option<&Self>) -> Option<Self> {
        let ray = ray?;
        Some(Self {
            origin: ray.origin,
            direction: ray.direction,
        })
    }

    /// Computes the point along the ray given by `r(t) = o + t*d`,
    /// where `o` is the origin of the ray and `d` is the direction.
    ///
    /// Port of `Ray.getPoint`.
    ///
    /// # Panics
    /// Panics with `DeveloperError` if `t` is `None`.
    pub fn get_point(ray: &Self, t: Option<f64>, result: &mut Cartesian3) {
        //>>includeStart('debug', pragmas.debug);
        if t.is_none() {
            throw_developer_error("t is required");
        }
        //>>includeEnd('debug');

        let t_val = t.unwrap();
        let mut scaled = Cartesian3::default();
        Cartesian3::multiply_by_scalar(&ray.direction, t_val, &mut scaled);
        Cartesian3::add(&ray.origin, &scaled, result);
    }

    /// Allocating variant of [`Ray::get_point`].
    ///
    /// # Panics
    /// Panics with `DeveloperError` if `t` is `None`.
    pub fn get_point_new(ray: &Self, t: Option<f64>) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::get_point(ray, t, &mut result);
        result
    }
}
