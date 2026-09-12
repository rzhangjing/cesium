//! Ported from `packages/engine/Source/Core/PerspectiveOffCenterFrustum.js`.
//!
//! Defines a perspective off-center viewing frustum.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::culling_volume::CullingVolume;
use crate::matrix4::Matrix4;

/// A viewing frustum defined by 6 planes (perspective, off-center).
#[derive(Clone, Debug)]
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

        // Mirrors Matrix4.computePerspectiveOffCenter: column2Row0/Row1 land
        // in elements[8]/[9], column2Row3 (-1) in elements[11] and
        // column3Row2 in elements[14] (Matrix4::new parameters are
        // row-ordered, storage is column-major).
        Matrix4::new(
            col0x, 0.0, col2x, 0.0,
            0.0, col1y, col2y, 0.0,
            0.0, 0.0, col2z, col3z,
            0.0, 0.0, col2w, 0.0,
        )
    }

    /// Creates a culling volume for this frustum.
    ///
    /// Ported 1:1 from `PerspectiveOffCenterFrustum.prototype.computeCullingVolume`.
    /// The four side planes are built from the near-plane corners measured relative
    /// to `position`; the left plane normalises twice (the JS does the same) while
    /// the right/bottom/top planes normalise once after the cross product.
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

        // `right = direction x up`; CesiumJS does not normalise direction, up or right.
        let right = Cartesian3::cross_new(direction, up);

        let near_center =
            Cartesian3::add_new(position, &Cartesian3::multiply_by_scalar_new(direction, n));

        let far_center =
            Cartesian3::add_new(position, &Cartesian3::multiply_by_scalar_new(direction, f));

        // Mirrors the JS scratch sequence
        // `multiplyByScalar(axis, extent, normal)` -> `add(nearCenter, normal)`
        // -> `subtract(normal, position)`.
        let corner_from_position = |axis: &Cartesian3, extent: f64| -> Cartesian3 {
            let corner = Cartesian3::add_new(
                &near_center,
                &Cartesian3::multiply_by_scalar_new(axis, extent),
            );
            Cartesian3::subtract_new(&corner, position)
        };

        // Left plane computation
        let normal = Cartesian3::normalize_new(&corner_from_position(&right, l));
        let normal = Cartesian3::normalize_new(&Cartesian3::cross_new(&normal, up));
        let w = -Cartesian3::dot(&normal, position);
        self.culling_volume.planes[0] = Cartesian4::new(normal.x, normal.y, normal.z, w);

        // Right plane computation
        let normal = corner_from_position(&right, r);
        let normal = Cartesian3::normalize_new(&Cartesian3::cross_new(up, &normal));
        let w = -Cartesian3::dot(&normal, position);
        self.culling_volume.planes[1] = Cartesian4::new(normal.x, normal.y, normal.z, w);

        // Bottom plane computation
        let normal = corner_from_position(&up, b);
        let normal = Cartesian3::normalize_new(&Cartesian3::cross_new(&right, &normal));
        let w = -Cartesian3::dot(&normal, position);
        self.culling_volume.planes[2] = Cartesian4::new(normal.x, normal.y, normal.z, w);

        // Top plane computation
        let normal = corner_from_position(&up, t);
        let normal = Cartesian3::normalize_new(&Cartesian3::cross_new(&normal, &right));
        let w = -Cartesian3::dot(&normal, position);
        self.culling_volume.planes[3] = Cartesian4::new(normal.x, normal.y, normal.z, w);

        // Near plane computation
        let w = -Cartesian3::dot(direction, &near_center);
        self.culling_volume.planes[4] =
            Cartesian4::new(direction.x, direction.y, direction.z, w);

        // Far plane computation (`Cartesian3.negate(direction, normal)`)
        let neg_direction = Cartesian3::multiply_by_scalar_new(direction, -1.0);
        let w = -Cartesian3::dot(&neg_direction, &far_center);
        self.culling_volume.planes[5] = Cartesian4::new(
            neg_direction.x,
            neg_direction.y,
            neg_direction.z,
            w,
        );

        &self.culling_volume
    }
}

impl Default for PerspectiveOffCenterFrustum {
    fn default() -> Self {
        Self::new()
    }
}
