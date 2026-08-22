//! Ported from `packages/engine/Source/Core/PerspectiveOffCenterFrustum.js`.
//!
//! Defines a perspective off-center viewing frustum.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::culling_volume::CullingVolume;
use crate::matrix4::Matrix4;

/// A viewing frustum defined by 6 planes (perspective, off-center).
pub struct PerspectiveOffCenterFrustum {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
    pub near: f64,
    pub far: f64,
    _projection_matrix: Option<Matrix4>,
    culling_volume: CullingVolume,
}

impl PerspectiveOffCenterFrustum {
    /// Creates a new PerspectiveOffCenterFrustum.
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
            top: None,
            bottom: None,
            near: 1.0,
            far: 500_000_000.0,
            _projection_matrix: None,
            culling_volume: CullingVolume {
                planes: vec![
                    Cartesian4::default(),
                    Cartesian4::default(),
                    Cartesian4::default(),
                    Cartesian4::default(),
                    Cartesian4::default(),
                    Cartesian4::default(),
                ],
            },
        }
    }

    /// Computes the projection matrix.
    pub fn compute_projection_matrix(&mut self) -> Matrix4 {
        let left = self.left.unwrap_or(0.0);
        let right = self.right.unwrap_or(0.0);
        let bottom = self.bottom.unwrap_or(0.0);
        let top = self.top.unwrap_or(0.0);
        let near = self.near;
        let far = self.far;

        let col0x = 2.0 * near / (right - left);
        let col1y = 2.0 * near / (top - bottom);
        let col2x = (right + left) / (right - left);
        let col2y = (top + bottom) / (top - bottom);
        let col2z = -(far + near) / (far - near);
        let col2w = -1.0;
        let col3z = -2.0 * far * near / (far - near);

        Matrix4::new(
            col0x, 0.0, 0.0, 0.0,
            0.0, col1y, 0.0, 0.0,
            col2x, col2y, col2z, col2w,
            0.0, 0.0, col3z, 0.0,
        )
    }

    /// Computes the culling volume.
    pub fn compute_culling_volume(
        &mut self,
        position: &Cartesian3,
        direction: &Cartesian3,
        up: &Cartesian3,
    ) -> &CullingVolume {
        // Simplified culling volume computation
        let right_dir = Cartesian3::cross_new(direction, up);
        let _near_center = Cartesian3::add_new(
            position,
            &Cartesian3::multiply_by_scalar_new(direction, self.near),
        );

        // Compute 6 planes
        let left_normal = Cartesian3::multiply_by_scalar_new(&right_dir, -1.0);
        self.culling_volume.planes[0] = Cartesian4::new(
            left_normal.x, left_normal.y, left_normal.z,
            -Cartesian3::dot(&left_normal, position),
        );

        self.culling_volume.planes[1] = Cartesian4::new(
            right_dir.x, right_dir.y, right_dir.z,
            -Cartesian3::dot(&right_dir, position),
        );

        let down_normal = Cartesian3::multiply_by_scalar_new(up, -1.0);
        self.culling_volume.planes[2] = Cartesian4::new(
            down_normal.x, down_normal.y, down_normal.z,
            -Cartesian3::dot(&down_normal, position),
        );

        self.culling_volume.planes[3] = Cartesian4::new(
            up.x, up.y, up.z,
            -Cartesian3::dot(up, position),
        );

        self.culling_volume.planes[4] = Cartesian4::new(
            direction.x, direction.y, direction.z,
            -Cartesian3::dot(direction, position),
        );

        let neg_dir = Cartesian3::multiply_by_scalar_new(direction, -1.0);
        let far_point = Cartesian3::add_new(
            position,
            &Cartesian3::multiply_by_scalar_new(direction, self.far),
        );
        self.culling_volume.planes[5] = Cartesian4::new(
            neg_dir.x, neg_dir.y, neg_dir.z,
            -Cartesian3::dot(&neg_dir, &far_point),
        );

        &self.culling_volume
    }
}

impl Default for PerspectiveOffCenterFrustum {
    fn default() -> Self {
        Self::new()
    }
}
