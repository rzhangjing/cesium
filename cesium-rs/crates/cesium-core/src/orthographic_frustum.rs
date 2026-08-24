//! Ported from `packages/engine/Source/Core/OrthographicFrustum.js`.
//!
//! An orthographic viewing frustum defined by width and aspect ratio.

use crate::cartesian3::Cartesian3;
use crate::culling_volume::CullingVolume;
use crate::matrix4::Matrix4;
use crate::orthographic_off_center_frustum::OrthographicOffCenterFrustum;

/// An orthographic frustum defined by width and aspect ratio.
#[derive(Clone, Debug)]
pub struct OrthographicFrustum {
    pub width: Option<f64>,
    pub aspect_ratio: Option<f64>,
    pub near: f64,
    pub far: f64,
    off_center: OrthographicOffCenterFrustum,
}

impl OrthographicFrustum {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 4;

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

    /// Stores the provided instance into the provided array.
    ///
    /// DEVIATION: JS packs `undefined` width/aspectRatio as-is; Rust stores NaN.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.width.unwrap_or(f64::NAN);
        array[starting_index + 1] = value.aspect_ratio.unwrap_or(f64::NAN);
        array[starting_index + 2] = value.near;
        array[starting_index + 3] = value.far;
    }

    /// Retrieves an instance from a packed array.
    pub fn unpack(array: &[f64], starting_index: usize, result: Option<&mut Self>) -> Self {
        let from_f64 = |v: f64| if v.is_nan() { None } else { Some(v) };
        let width = from_f64(array[starting_index]);
        let aspect_ratio = from_f64(array[starting_index + 1]);
        let near = array[starting_index + 2];
        let far = array[starting_index + 3];

        match result {
            Some(r) => {
                r.width = width;
                r.aspect_ratio = aspect_ratio;
                r.near = near;
                r.far = far;
                r.clone()
            }
            None => Self {
                width,
                aspect_ratio,
                near,
                far,
                off_center: OrthographicOffCenterFrustum::new(),
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
