//! Ported from `packages/engine/Source/Core/PerspectiveFrustum.js`.
//!
//! A perspective frustum defined by fov and aspect ratio.

use crate::cartesian3::Cartesian3;
use crate::culling_volume::CullingVolume;
use crate::matrix4::Matrix4;
use crate::perspective_off_center_frustum::PerspectiveOffCenterFrustum;

/// A perspective frustum defined by field-of-view and aspect ratio.
#[derive(Clone, Debug)]
pub struct PerspectiveFrustum {
    pub fov: Option<f64>,
    pub aspect_ratio: Option<f64>,
    pub near: f64,
    pub far: f64,
    /// Offsets the frustum in the x direction.
    pub x_offset: f64,
    /// Offsets the frustum in the y direction.
    pub y_offset: f64,
    /// `false` until the first `update`; mirrors the CesiumJS `_fov` /
    /// `_aspectRatio` / ... caches starting out `undefined` so the very first
    /// `update` always recomputes.
    initialized: bool,
    cached_fov: f64,
    cached_aspect_ratio: f64,
    cached_near: f64,
    cached_far: f64,
    cached_x_offset: f64,
    cached_y_offset: f64,
    /// Mirrors the CesiumJS `_fovy` cache.
    cached_fovy: f64,
    /// Mirrors the CesiumJS `_sseDenominator` cache.
    cached_sse_denominator: f64,
    off_center: PerspectiveOffCenterFrustum,
}

impl PerspectiveFrustum {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 6;

    /// Creates a new PerspectiveFrustum.
    pub fn new() -> Self {
        Self {
            fov: None,
            aspect_ratio: None,
            near: 1.0,
            far: 500_000_000.0,
            x_offset: 0.0,
            y_offset: 0.0,
            initialized: false,
            cached_fov: 0.0,
            cached_aspect_ratio: 0.0,
            cached_near: 0.0,
            cached_far: 0.0,
            cached_x_offset: 0.0,
            cached_y_offset: 0.0,
            cached_fovy: 0.0,
            cached_sse_denominator: 0.0,
            off_center: PerspectiveOffCenterFrustum::new(),
        }
    }

    /// Stores the provided instance into the provided array.
    ///
    /// DEVIATION: JS packs `undefined` fov/aspectRatio as-is; Rust stores NaN.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        let mut i = starting_index;
        array[i] = value.fov.unwrap_or(f64::NAN);
        i += 1;
        array[i] = value.aspect_ratio.unwrap_or(f64::NAN);
        i += 1;
        array[i] = value.near;
        i += 1;
        array[i] = value.far;
        i += 1;
        array[i] = value.x_offset;
        i += 1;
        array[i] = value.y_offset;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize, result: Option<&mut Self>) -> Self {
        let from_f64 = |v: f64| if v.is_nan() { None } else { Some(v) };
        let fov = from_f64(array[starting_index]);
        let aspect_ratio = from_f64(array[starting_index + 1]);
        let near = array[starting_index + 2];
        let far = array[starting_index + 3];
        let x_offset = array[starting_index + 4];
        let y_offset = array[starting_index + 5];

        match result {
            Some(r) => {
                r.fov = fov;
                r.aspect_ratio = aspect_ratio;
                r.near = near;
                r.far = far;
                r.x_offset = x_offset;
                r.y_offset = y_offset;
                r.clone()
            }
            None => Self {
                fov,
                aspect_ratio,
                near,
                far,
                x_offset,
                y_offset,
                initialized: false,
                cached_fov: 0.0,
                cached_aspect_ratio: 0.0,
                cached_near: 0.0,
                cached_far: 0.0,
                cached_x_offset: 0.0,
                cached_y_offset: 0.0,
                cached_fovy: 0.0,
                cached_sse_denominator: 0.0,
                off_center: PerspectiveOffCenterFrustum::new(),
            },
        }
    }

    /// Computes the projection matrix (updates the off-center frustum first).
    pub fn projection_matrix(&mut self) -> Matrix4 {
        self.update();
        self.off_center.compute_projection_matrix()
    }

    /// Returns the off-center frustum bounds after `update`.
    ///
    /// Public because `cesium-scene`'s `CameraFrustum` union has to reach the
    /// derived `left`/`right`/`top`/`bottom` of a `PerspectiveFrustum` the same
    /// way CesiumJS reaches `frustum._offCenterFrustum` (e.g. `Camera#update`
    /// reading `frustum.top / frustum.right` in 2D).
    pub fn off_center_bounds(&mut self) -> (f64, f64, f64, f64) {
        self.update();
        (
            self.off_center.left.unwrap_or(0.0),
            self.off_center.right.unwrap_or(0.0),
            self.off_center.top.unwrap_or(0.0),
            self.off_center.bottom.unwrap_or(0.0),
        )
    }

    /// The vertical field of view in radians.
    ///
    /// Mirrors the CesiumJS `PerspectiveFrustum#fovy` getter, which runs
    /// `update` before returning the cache.
    pub fn fovy(&mut self) -> f64 {
        self.update();
        self.cached_fovy
    }

    /// The screen-space-error denominator, `2.0 * tan(0.5 * fovy)`.
    ///
    /// Mirrors the private CesiumJS `PerspectiveFrustum#sseDenominator` getter.
    pub fn sse_denominator(&mut self) -> f64 {
        self.update();
        self.cached_sse_denominator
    }

    /// Updates the off-center frustum, the vertical FOV and the SSE denominator.
    ///
    /// Ported 1:1 from the module-level `update(frustum)` of CesiumJS,
    /// including its dirty check — nothing is recomputed unless one of `fov`,
    /// `aspectRatio`, `near`, `far`, `xOffset` or `yOffset` changed since the
    /// previous call.
    ///
    /// The key subtlety is that `fov` is the *horizontal* FOV whenever
    /// `aspectRatio > 1` and only equals the vertical FOV otherwise, so `fovy`
    /// has to be recovered before any extent is derived from it.
    pub fn update(&mut self) {
        // CesiumJS throws a DeveloperError when fov/aspectRatio/near/far are
        // undefined. The Rust fields are `Option<f64>`, so fall back to the
        // documented defaults rather than failing on an incomplete frustum.
        let fov = self.fov.unwrap_or(std::f64::consts::FRAC_PI_3);
        let aspect_ratio = self.aspect_ratio.unwrap_or(1.0);

        let changed = !self.initialized
            || fov != self.cached_fov
            || aspect_ratio != self.cached_aspect_ratio
            || self.near != self.cached_near
            || self.far != self.cached_far
            || self.x_offset != self.cached_x_offset
            || self.y_offset != self.cached_y_offset;

        if !changed {
            return;
        }

        self.initialized = true;
        self.cached_fov = fov;
        self.cached_aspect_ratio = aspect_ratio;
        self.cached_near = self.near;
        self.cached_far = self.far;
        self.cached_x_offset = self.x_offset;
        self.cached_y_offset = self.y_offset;

        self.cached_fovy = Self::fovy_of(fov, aspect_ratio);
        self.cached_sse_denominator = Self::sse_denominator_of(self.cached_fovy);

        // `top = near * tan(0.5 * fovy)`, `right = aspectRatio * top`; the
        // offsets are added afterwards so they shift all four side planes.
        let top = self.near * (0.5 * self.cached_fovy).tan();
        let right = aspect_ratio * top;

        self.off_center.top = Some(top + self.y_offset);
        self.off_center.bottom = Some(-top + self.y_offset);
        self.off_center.right = Some(right + self.x_offset);
        self.off_center.left = Some(-right + self.x_offset);
        self.off_center.near = self.near;
        self.off_center.far = self.far;
    }

    /// Computes the culling volume.
    pub fn compute_culling_volume(
        &mut self,
        position: &Cartesian3,
        direction: &Cartesian3,
        up: &Cartesian3,
    ) -> &CullingVolume {
        self.update();
        self.off_center.compute_culling_volume(position, direction, up)
    }

    /// The vertical FOV as a pure function of `fov` and `aspectRatio`.
    ///
    /// Extracted from `update` so that a shared reference can answer
    /// `frustum.fovy`: CesiumJS computes the value inside `update(frustum)` and
    /// caches it, but it depends on nothing else, so the two are equivalent.
    #[must_use]
    pub fn fovy_of(fov: f64, aspect_ratio: f64) -> f64 {
        if aspect_ratio <= 1.0 {
            fov
        } else {
            ((fov * 0.5).tan() / aspect_ratio).atan() * 2.0
        }
    }

    /// `2.0 * tan(0.5 * fovy)`, the pure form of `sseDenominator`.
    #[must_use]
    pub fn sse_denominator_of(fovy: f64) -> f64 {
        2.0 * (0.5 * fovy).tan()
    }

    /// [`PerspectiveFrustum::fovy`] without the cache update, applying the same
    /// `undefined` fallbacks as [`PerspectiveFrustum::update`].
    #[must_use]
    pub fn fovy_pure(&self) -> f64 {
        Self::fovy_of(
            self.fov.unwrap_or(std::f64::consts::FRAC_PI_3),
            self.aspect_ratio.unwrap_or(1.0),
        )
    }

    /// [`PerspectiveFrustum::sse_denominator`] without the cache update.
    #[must_use]
    pub fn sse_denominator_pure(&self) -> f64 {
        Self::sse_denominator_of(self.fovy_pure())
    }
}

impl Default for PerspectiveFrustum {
    fn default() -> Self {
        Self::new()
    }
}
