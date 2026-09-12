//! The `Camera#frustum` union of CesiumJS.
//!
//! CesiumJS declares the property as
//! `PerspectiveFrustum | PerspectiveOffCenterFrustum | OrthographicFrustum |
//! OrthographicOffCenterFrustum` and dispatches on `instanceof` at runtime:
//! `Camera#update` requires an `OrthographicOffCenterFrustum` in 2D and a
//! `PerspectiveFrustum`/`OrthographicFrustum` in 3D and Columbus view,
//! `Camera#_adjustOrthographicFrustum` early-returns unless the frustum *is* an
//! `OrthographicFrustum`, and `switchToPerspectiveFrustum` /
//! `switchToOrthographicFrustum` test for the target type before replacing it.
//!
//! Rust has no structural subtyping, so the four concrete core frustums are
//! wrapped in an enum and every `instanceof` test becomes a variant match. The
//! per-variant asymmetries of the JS are preserved rather than papered over:
//! `fovy` / `sseDenominator` only exist on `PerspectiveFrustum`, `width` only on
//! `OrthographicFrustum`, and the four side-plane bounds are only *assignable*
//! on the two off-center variants.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::culling_volume::CullingVolume;
use cesium_core::matrix4::Matrix4;
use cesium_core::orthographic_frustum::OrthographicFrustum;
use cesium_core::orthographic_off_center_frustum::OrthographicOffCenterFrustum;
use cesium_core::perspective_frustum::PerspectiveFrustum;
use cesium_core::perspective_off_center_frustum::PerspectiveOffCenterFrustum;

/// The region of space in view, mirroring CesiumJS `Camera#frustum`.
#[derive(Debug, Clone)]
pub enum CameraFrustum {
    /// `new PerspectiveFrustum()` — the default in 3D and Columbus view.
    Perspective(PerspectiveFrustum),
    /// `new PerspectiveOffCenterFrustum()` — used by the VR / stereo paths.
    PerspectiveOffCenter(PerspectiveOffCenterFrustum),
    /// `new OrthographicFrustum()` — produced by `switchToOrthographicFrustum`.
    Orthographic(OrthographicFrustum),
    /// `new OrthographicOffCenterFrustum()` — required in 2D.
    OrthographicOffCenter(OrthographicOffCenterFrustum),
}

impl CameraFrustum {
    /// Creates the default frustum: CesiumJS `Camera`'s constructor assigns
    /// `this.frustum = new PerspectiveFrustum()`.
    pub fn new() -> Self {
        CameraFrustum::Perspective(PerspectiveFrustum::new())
    }

    /// `frustum instanceof PerspectiveFrustum`.
    pub fn is_perspective(&self) -> bool {
        matches!(self, CameraFrustum::Perspective(_))
    }

    /// `frustum instanceof OrthographicFrustum`.
    ///
    /// Deliberately *not* satisfied by [`CameraFrustum::OrthographicOffCenter`]:
    /// `Camera#_adjustOrthographicFrustum` returns early for the off-center
    /// variant even though it is also orthographic.
    pub fn is_orthographic(&self) -> bool {
        matches!(self, CameraFrustum::Orthographic(_))
    }

    /// `frustum instanceof OrthographicOffCenterFrustum`.
    pub fn is_orthographic_off_center(&self) -> bool {
        matches!(self, CameraFrustum::OrthographicOffCenter(_))
    }

    /// `frustum.projectionMatrix`.
    pub fn projection_matrix(&mut self) -> Matrix4 {
        match self {
            CameraFrustum::Perspective(f) => f.projection_matrix(),
            CameraFrustum::PerspectiveOffCenter(f) => f.compute_projection_matrix(),
            CameraFrustum::Orthographic(f) => f.projection_matrix(),
            CameraFrustum::OrthographicOffCenter(f) => f.compute_projection_matrix(),
        }
    }

    /// `frustum.computeCullingVolume(position, direction, up)`.
    ///
    /// Returns an owned volume: the core frustums hand back a reference into
    /// their own cached `CullingVolume`, which cannot escape the `&mut self`
    /// borrow of the enum.
    pub fn compute_culling_volume(
        &mut self,
        position: &Cartesian3,
        direction: &Cartesian3,
        up: &Cartesian3,
    ) -> CullingVolume {
        match self {
            CameraFrustum::Perspective(f) => f.compute_culling_volume(position, direction, up).clone(),
            CameraFrustum::PerspectiveOffCenter(f) => {
                f.compute_culling_volume(position, direction, up).clone()
            }
            CameraFrustum::Orthographic(f) => f.compute_culling_volume(position, direction, up).clone(),
            CameraFrustum::OrthographicOffCenter(f) => {
                f.compute_culling_volume(position, direction, up).clone()
            }
        }
    }

    /// `frustum.getPixelDimensions(drawingBufferWidth, drawingBufferHeight,
    /// distance, pixelRatio, result)`.
    ///
    /// CesiumJS implements this on each concrete frustum: the two off-centre
    /// variants compute it directly, while `PerspectiveFrustum` /
    /// `OrthographicFrustum` `update()` themselves and delegate to their
    /// internal `_offCenterFrustum`. [`CameraFrustum::bounds`] already performs
    /// exactly that update-then-read-off-centre step, so the perspective and
    /// orthographic formulas are applied to its `(left, right, top, bottom)`.
    ///
    /// The perspective branch scales the near-plane `top`/`right` by
    /// `distance / near`; the orthographic branch is distance-independent and
    /// uses the full frustum width/height. Returns `(x = width, y = height)` in
    /// metres, matching the JS `result.x`/`result.y`.
    pub fn get_pixel_dimensions(
        &mut self,
        drawing_buffer_width: f64,
        drawing_buffer_height: f64,
        distance: f64,
        pixel_ratio: f64,
    ) -> Cartesian2 {
        let (left, right, top, bottom) = self.bounds();
        let near = self.near();
        match self {
            CameraFrustum::Perspective(_) | CameraFrustum::PerspectiveOffCenter(_) => {
                // JS: `inverseNear = 1/near; tanTheta = top*inverseNear;
                //      pixelHeight = 2*pixelRatio*distance*tanTheta/height`
                let inverse_near = 1.0 / near;
                let pixel_height =
                    (2.0 * pixel_ratio * distance * top * inverse_near) / drawing_buffer_height;
                let pixel_width =
                    (2.0 * pixel_ratio * distance * right * inverse_near) / drawing_buffer_width;
                Cartesian2::new(pixel_width, pixel_height)
            }
            CameraFrustum::Orthographic(_) | CameraFrustum::OrthographicOffCenter(_) => {
                // JS: `frustumWidth = right-left; frustumHeight = top-bottom;
                //      pixelWidth = pixelRatio*frustumWidth/width`
                let frustum_width = right - left;
                let frustum_height = top - bottom;
                let pixel_width = (pixel_ratio * frustum_width) / drawing_buffer_width;
                let pixel_height = (pixel_ratio * frustum_height) / drawing_buffer_height;
                Cartesian2::new(pixel_width, pixel_height)
            }
        }
    }

    /// `frustum.fovy`, which is `undefined` for every variant except
    /// [`CameraFrustum::Perspective`].
    ///
    /// `Camera#_updateCameraChanged` tests `defined(camera.frustum.fovy)` before
    /// dividing the direction delta by it, so the `None` case is load-bearing.
    ///
    /// Takes `&self`: the CesiumJS getter runs `update()` purely to refresh the
    /// cached value, and `_fovy` is a function of `fov` and `aspectRatio` alone
    /// ([`PerspectiveFrustum::fovy_pure`]).
    pub fn fovy(&self) -> Option<f64> {
        match self {
            CameraFrustum::Perspective(f) => Some(f.fovy_pure()),
            _ => None,
        }
    }

    /// `frustum.sseDenominator`, a `PerspectiveFrustum`-only private getter.
    pub fn sse_denominator(&self) -> Option<f64> {
        match self {
            CameraFrustum::Perspective(f) => Some(f.sse_denominator_pure()),
            _ => None,
        }
    }

    /// `frustum.near`.
    pub fn near(&self) -> f64 {
        match self {
            CameraFrustum::Perspective(f) => f.near,
            CameraFrustum::PerspectiveOffCenter(f) => f.near,
            CameraFrustum::Orthographic(f) => f.near,
            CameraFrustum::OrthographicOffCenter(f) => f.near,
        }
    }

    /// `frustum.near = value`.
    pub fn set_near(&mut self, near: f64) {
        match self {
            CameraFrustum::Perspective(f) => f.near = near,
            CameraFrustum::PerspectiveOffCenter(f) => f.near = near,
            CameraFrustum::Orthographic(f) => f.near = near,
            CameraFrustum::OrthographicOffCenter(f) => f.near = near,
        }
    }

    /// `frustum.far`.
    pub fn far(&self) -> f64 {
        match self {
            CameraFrustum::Perspective(f) => f.far,
            CameraFrustum::PerspectiveOffCenter(f) => f.far,
            CameraFrustum::Orthographic(f) => f.far,
            CameraFrustum::OrthographicOffCenter(f) => f.far,
        }
    }

    /// `frustum.far = value`.
    pub fn set_far(&mut self, far: f64) {
        match self {
            CameraFrustum::Perspective(f) => f.far = far,
            CameraFrustum::PerspectiveOffCenter(f) => f.far = far,
            CameraFrustum::Orthographic(f) => f.far = far,
            CameraFrustum::OrthographicOffCenter(f) => f.far = far,
        }
    }

    /// `frustum.aspectRatio`, `undefined` until assigned.
    pub fn aspect_ratio(&self) -> Option<f64> {
        match self {
            CameraFrustum::Perspective(f) => f.aspect_ratio,
            CameraFrustum::Orthographic(f) => f.aspect_ratio,
            // The off-center variants are defined by their four side planes and
            // carry no `aspectRatio` property in CesiumJS.
            CameraFrustum::PerspectiveOffCenter(_)
            | CameraFrustum::OrthographicOffCenter(_) => None,
        }
    }

    /// `frustum.aspectRatio = value`.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f64) {
        match self {
            CameraFrustum::Perspective(f) => f.aspect_ratio = Some(aspect_ratio),
            CameraFrustum::Orthographic(f) => f.aspect_ratio = Some(aspect_ratio),
            CameraFrustum::PerspectiveOffCenter(_)
            | CameraFrustum::OrthographicOffCenter(_) => {}
        }
    }

    /// `frustum.fov`, present only on the perspective variants.
    pub fn fov(&self) -> Option<f64> {
        match self {
            CameraFrustum::Perspective(f) => f.fov,
            // `ShadowMap` probes `defined(this._lightCamera.frustum.fov)` to
            // decide whether the light is a spot light; the off-center variants
            // carry no `fov` property in CesiumJS either.
            CameraFrustum::PerspectiveOffCenter(_)
            | CameraFrustum::Orthographic(_)
            | CameraFrustum::OrthographicOffCenter(_) => None,
        }
    }

    /// `frustum.fov = value`; a no-op unless the frustum is a
    /// [`CameraFrustum::Perspective`].
    pub fn set_fov(&mut self, fov: f64) {
        if let CameraFrustum::Perspective(f) = self {
            f.fov = Some(fov);
        }
    }

    /// `frustum.width`, present only on [`CameraFrustum::Orthographic`].
    pub fn width(&self) -> Option<f64> {
        match self {
            CameraFrustum::Orthographic(f) => f.width,
            _ => None,
        }
    }

    /// `frustum.width = value`; the assignment
    /// `Camera#_adjustOrthographicFrustum` performs.
    pub fn set_width(&mut self, width: f64) {
        if let CameraFrustum::Orthographic(f) = self {
            f.width = Some(width);
        }
    }

    /// The four side-plane bounds `(left, right, top, bottom)`.
    ///
    /// For [`CameraFrustum::Perspective`] / [`CameraFrustum::Orthographic`] these
    /// are the *derived* values of the internal off-center frustum, reached in
    /// CesiumJS through `frustum._offCenterFrustum`; the frustum is updated
    /// first, exactly as the JS getters do.
    pub fn bounds(&mut self) -> (f64, f64, f64, f64) {
        match self {
            CameraFrustum::Perspective(f) => f.off_center_bounds(),
            CameraFrustum::Orthographic(f) => f.off_center_bounds(),
            CameraFrustum::PerspectiveOffCenter(f) => (
                f.left.unwrap_or(0.0),
                f.right.unwrap_or(0.0),
                f.top.unwrap_or(0.0),
                f.bottom.unwrap_or(0.0),
            ),
            CameraFrustum::OrthographicOffCenter(f) => (
                f.left.unwrap_or(0.0),
                f.right.unwrap_or(0.0),
                f.top.unwrap_or(0.0),
                f.bottom.unwrap_or(0.0),
            ),
        }
    }

    /// Assigns the four side-plane bounds.
    ///
    /// Only the off-center variants store them: this is what makes
    /// `Camera#update`'s 2D branch (`frustum.right = maxCoord.x * maxZoomOut; …`)
    /// legal there and a no-op elsewhere, matching CesiumJS where writing
    /// `left`/`right`/`top`/`bottom` onto a `PerspectiveFrustum` would only add
    /// inert expando properties.
    pub fn set_bounds(&mut self, left: f64, right: f64, top: f64, bottom: f64) {
        match self {
            CameraFrustum::PerspectiveOffCenter(f) => {
                f.left = Some(left);
                f.right = Some(right);
                f.top = Some(top);
                f.bottom = Some(bottom);
            }
            CameraFrustum::OrthographicOffCenter(f) => {
                f.left = Some(left);
                f.right = Some(right);
                f.top = Some(top);
                f.bottom = Some(bottom);
            }
            CameraFrustum::Perspective(_) | CameraFrustum::Orthographic(_) => {}
        }
    }
}

impl Default for CameraFrustum {
    fn default() -> Self {
        Self::new()
    }
}
