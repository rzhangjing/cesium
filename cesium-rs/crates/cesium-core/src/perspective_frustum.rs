//! Ported from `packages/engine/Source/Core/PerspectiveFrustum.js`.
//!
//! A perspective frustum defined by fov and aspect ratio.

use crate::cartesian3::Cartesian3;
use crate::culling_volume::CullingVolume;
use crate::perspective_off_center_frustum::PerspectiveOffCenterFrustum;

/// A perspective frustum defined by field-of-view and aspect ratio.
pub struct PerspectiveFrustum {
    pub fov: Option<f64>,
    pub aspect_ratio: Option<f64>,
    pub near: f64,
    pub far: f64,
    off_center: PerspectiveOffCenterFrustum,
}

impl PerspectiveFrustum {
    /// Creates a new PerspectiveFrustum.
    pub fn new() -> Self {
        Self {
            fov: None,
            aspect_ratio: None,
            near: 1.0,
            far: 500_000_000.0,
            off_center: PerspectiveOffCenterFrustum::new(),
        }
    }

    /// Updates the off-center frustum from fov/aspect ratio.
    pub fn update(&mut self) {
        let fov = self.fov.unwrap_or(std::f64::consts::FRAC_PI_3);
        let aspect = self.aspect_ratio.unwrap_or(1.0);
        let tan_half_fov = (fov / 2.0).tan();
        let right = self.near * tan_half_fov;
        let top = self.near * tan_half_fov / aspect;

        self.off_center.left = Some(-right);
        self.off_center.right = Some(right);
        self.off_center.top = Some(top);
        self.off_center.bottom = Some(-top);
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
}

impl Default for PerspectiveFrustum {
    fn default() -> Self {
        Self::new()
    }
}
