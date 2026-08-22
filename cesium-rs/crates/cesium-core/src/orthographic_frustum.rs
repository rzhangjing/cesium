//! Ported from `packages/engine/Source/Core/OrthographicFrustum.js`.
//!
//! An orthographic viewing frustum defined by width and aspect ratio.

use crate::cartesian3::Cartesian3;
use crate::culling_volume::CullingVolume;
use crate::orthographic_off_center_frustum::OrthographicOffCenterFrustum;

/// An orthographic frustum defined by width and aspect ratio.
pub struct OrthographicFrustum {
    pub width: Option<f64>,
    pub aspect_ratio: Option<f64>,
    pub near: f64,
    pub far: f64,
    off_center: OrthographicOffCenterFrustum,
}

impl OrthographicFrustum {
    /// Creates a new OrthographicFrustum.
    pub fn new() -> Self {
        Self {
            width: None,
            aspect_ratio: None,
            near: 1.0,
            far: 500_000_000.0,
            off_center: OrthographicOffCenterFrustum::new(),
        }
    }

    /// Updates the off-center frustum from width/aspect ratio.
    pub fn update(&mut self) {
        let width = self.width.unwrap_or(1.0);
        let aspect = self.aspect_ratio.unwrap_or(1.0);
        let half_width = width / 2.0;
        let half_height = half_width / aspect;

        self.off_center.left = Some(-half_width);
        self.off_center.right = Some(half_width);
        self.off_center.top = Some(half_height);
        self.off_center.bottom = Some(-half_height);
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

impl Default for OrthographicFrustum {
    fn default() -> Self {
        Self::new()
    }
}
