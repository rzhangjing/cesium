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
    pub(crate) fn off_center_bounds(&mut self) -> (f64, f64, f64, f64) {
        self.update();
        (
            self.off_center.left.unwrap_or(0.0),
            self.off_center.right.unwrap_or(0.0),
            self.off_center.top.unwrap_or(0.0),
            self.off_center.bottom.unwrap_or(0.0),
        )
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
