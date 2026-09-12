//! Ported from `packages/engine/Source/Core/OrthographicOffCenterFrustum.js`.
//!
//! An orthographic off-center viewing frustum.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::culling_volume::CullingVolume;
use crate::matrix4::Matrix4;

/// An orthographic off-center viewing frustum.
#[derive(Clone, Debug)]
pub struct OrthographicOffCenterFrustum {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
    pub near: f64,
    pub far: f64,
    culling_volume: CullingVolume,
}

impl OrthographicOffCenterFrustum {
    /// Creates a new OrthographicOffCenterFrustum.
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
            top: None,
            bottom: None,
            near: 1.0,
            far: 500_000_000.0,
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

    /// Computes the orthographic projection matrix.
    pub fn compute_projection_matrix(&self) -> Matrix4 {
        let left = self.left.unwrap_or(0.0);
        let right = self.right.unwrap_or(0.0);
        let bottom = self.bottom.unwrap_or(0.0);
        let top = self.top.unwrap_or(0.0);
        let near = self.near;
        let far = self.far;

        let col0x = 2.0 / (right - left);
        let col1y = 2.0 / (top - bottom);
        let col2z = -2.0 / (far - near);
        let col3x = -(right + left) / (right - left);
        let col3y = -(top + bottom) / (top - bottom);
        let col3z = -(far + near) / (far - near);

        Matrix4::new(
            col0x, 0.0, 0.0, 0.0,
            0.0, col1y, 0.0, 0.0,
            0.0, 0.0, col2z, 0.0,
            col3x, col3y, col3z, 1.0,
        )
    }

    /// Creates a culling volume for this frustum.
    ///
    /// Ported 1:1 from `OrthographicOffCenterFrustum.prototype.computeCullingVolume`.
    /// Unlike the perspective variant, `right` is normalised here and every side plane
    /// keeps a constant outward normal (`+right`, `-right`, `+up`, `-up`) whose distance
    /// is measured against the matching near-plane corner instead of `position`.
    pub fn compute_culling_volume(
        &mut self,
        position: &Cartesian3,
        direction: &Cartesian3,
        up: &Cartesian3,
    ) -> &CullingVolume {
        let t = self.top.unwrap_or(0.0);
        let b = self.bottom.unwrap_or(0.0);
        let r = self.right.unwrap_or(0.0);
        let l = self.left.unwrap_or(0.0);
        let n = self.near;
        let f = self.far;

        // `right = normalize(direction x up)`; `up` is deliberately left unnormalised.
        let right = Cartesian3::normalize_new(&Cartesian3::cross_new(direction, up));

        let near_center =
            Cartesian3::add_new(position, &Cartesian3::multiply_by_scalar_new(direction, n));

        // Mirrors `multiplyByScalar(axis, extent, point)` -> `add(nearCenter, point)`.
        let corner = |axis: &Cartesian3, extent: f64| -> Cartesian3 {
            Cartesian3::add_new(
                &near_center,
                &Cartesian3::multiply_by_scalar_new(axis, extent),
            )
        };

        // Left plane: normal = +right
        let point = corner(&right, l);
        let w = -Cartesian3::dot(&right, &point);
        self.culling_volume.planes[0] = Cartesian4::new(right.x, right.y, right.z, w);

        // Right plane: normal = -right
        let neg_right = Cartesian3::negate_new(&right);
        let point = corner(&right, r);
        let w = -Cartesian3::dot(&neg_right, &point);
        self.culling_volume.planes[1] =
            Cartesian4::new(neg_right.x, neg_right.y, neg_right.z, w);

        // Bottom plane: normal = +up
        let point = corner(up, b);
        let w = -Cartesian3::dot(up, &point);
        self.culling_volume.planes[2] = Cartesian4::new(up.x, up.y, up.z, w);

        // Top plane: normal = -up
        let neg_up = Cartesian3::negate_new(up);
        let point = corner(up, t);
        let w = -Cartesian3::dot(&neg_up, &point);
        self.culling_volume.planes[3] = Cartesian4::new(neg_up.x, neg_up.y, neg_up.z, w);

        // Near plane: normal = direction, offset measured at nearCenter
        let w = -Cartesian3::dot(direction, &near_center);
        self.culling_volume.planes[4] =
            Cartesian4::new(direction.x, direction.y, direction.z, w);

        // Far plane: normal = -direction, offset measured at position + direction * far
        let neg_direction = Cartesian3::negate_new(direction);
        let far_point =
            Cartesian3::add_new(position, &Cartesian3::multiply_by_scalar_new(direction, f));
        let w = -Cartesian3::dot(&neg_direction, &far_point);
        self.culling_volume.planes[5] = Cartesian4::new(
            neg_direction.x,
            neg_direction.y,
            neg_direction.z,
            w,
        );

        &self.culling_volume
    }
}

impl Default for OrthographicOffCenterFrustum {
    fn default() -> Self {
        Self::new()
    }
}
