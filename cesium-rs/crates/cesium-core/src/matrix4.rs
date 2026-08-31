//! Ported from packages/engine/Source/Core/Matrix4.js
//!
//! A 4x4 matrix, stored in column-major order.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::matrix3::Matrix3;
use crate::quaternion::Quaternion;
use crate::translation_rotation_scale::TranslationRotationScale;

/// A 4x4 matrix in column-major order.
///
/// Port of `Matrix4`.
#[derive(Clone, Copy, Debug)]
pub struct Matrix4 {
    /// Column-major storage: 16 elements `[col0r0, col0r1, col0r2, col0r3, col1r0, ...]`.
    pub elements: [f64; 16],
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self {
            elements: [0.0; 16],
        }
    }
}

impl Matrix4 {
    pub const PACKED_LENGTH: usize = 16;

    pub const IDENTITY: Matrix4 = Matrix4 {
        elements: [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ],
    };

    pub const ZERO: Matrix4 = Matrix4 {
        elements: [0.0; 16],
    };

    // Column-major index constants
    pub const COLUMN0ROW0: usize = 0;
    pub const COLUMN0ROW1: usize = 1;
    pub const COLUMN0ROW2: usize = 2;
    pub const COLUMN0ROW3: usize = 3;
    pub const COLUMN1ROW0: usize = 4;
    pub const COLUMN1ROW1: usize = 5;
    pub const COLUMN1ROW2: usize = 6;
    pub const COLUMN1ROW3: usize = 7;
    pub const COLUMN2ROW0: usize = 8;
    pub const COLUMN2ROW1: usize = 9;
    pub const COLUMN2ROW2: usize = 10;
    pub const COLUMN2ROW3: usize = 11;
    pub const COLUMN3ROW0: usize = 12;
    pub const COLUMN3ROW1: usize = 13;
    pub const COLUMN3ROW2: usize = 14;
    pub const COLUMN3ROW3: usize = 15;

    /// Creates a `Matrix4` from 16 individual values.
    ///
    /// Constructor parameters are in row-major order for code readability,
    /// matching the JS constructor. Storage is column-major.
    pub fn new(
        column0_row0: f64, column1_row0: f64, column2_row0: f64, column3_row0: f64,
        column0_row1: f64, column1_row1: f64, column2_row1: f64, column3_row1: f64,
        column0_row2: f64, column1_row2: f64, column2_row2: f64, column3_row2: f64,
        column0_row3: f64, column1_row3: f64, column2_row3: f64, column3_row3: f64,
    ) -> Self {
        Self {
            elements: [
                column0_row0, column0_row1, column0_row2, column0_row3,
                column1_row0, column1_row1, column1_row2, column1_row3,
                column2_row0, column2_row1, column2_row2, column2_row3,
                column3_row0, column3_row1, column3_row2, column3_row3,
            ],
        }
    }

    // --- pack / unpack ---

    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        for i in 0..16 {
            array[starting_index + i] = value.elements[i];
        }
    }

    pub fn unpack(array: &[f64], starting_index: usize, result: &mut Self) {
        for i in 0..16 {
            result.elements[i] = array[starting_index + i];
        }
    }

    pub fn unpack_new(array: &[f64], starting_index: usize) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    pub fn from_array(array: &[f64], starting_index: usize, result: &mut Self) {
        Self::unpack(array, starting_index, result);
    }

    pub fn from_array_new(array: &[f64], starting_index: usize) -> Self {
        Self::unpack_new(array, starting_index)
    }

    pub fn from_column_major_array(values: &[f64], result: &mut Self) {
        result.elements.copy_from_slice(&values[..16]);
    }

    pub fn from_column_major_array_new(values: &[f64]) -> Self {
        let mut result = Self::default();
        Self::from_column_major_array(values, &mut result);
        result
    }

    pub fn from_row_major_array(values: &[f64], result: &mut Self) {
        for col in 0..4 {
            for row in 0..4 {
                result.elements[col * 4 + row] = values[row * 4 + col];
            }
        }
    }

    pub fn from_row_major_array_new(values: &[f64]) -> Self {
        let mut result = Self::default();
        Self::from_row_major_array(values, &mut result);
        result
    }

    /// Port of `Matrix4.fromTranslation`.
    pub fn from_translation(translation: &Cartesian3, result: &mut Self) {
        *result = Self::IDENTITY;
        result.elements[12] = translation.x;
        result.elements[13] = translation.y;
        result.elements[14] = translation.z;
    }

    pub fn from_translation_new(translation: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::from_translation(translation, &mut result);
        result
    }

    /// Port of `Matrix4.fromScale`.
    pub fn from_scale(scale: &Cartesian3, result: &mut Self) {
        result.elements[0] = scale.x;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = scale.y;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = 0.0;
        result.elements[9] = 0.0;
        result.elements[10] = scale.z;
        result.elements[11] = 0.0;
        result.elements[12] = 0.0;
        result.elements[13] = 0.0;
        result.elements[14] = 0.0;
        result.elements[15] = 1.0;
    }

    pub fn from_scale_new(scale: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::from_scale(scale, &mut result);
        result
    }

    /// Port of `Matrix4.fromUniformScale`.
    pub fn from_uniform_scale(scale: f64, result: &mut Self) {
        result.elements[0] = scale;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = scale;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = 0.0;
        result.elements[9] = 0.0;
        result.elements[10] = scale;
        result.elements[11] = 0.0;
        result.elements[12] = 0.0;
        result.elements[13] = 0.0;
        result.elements[14] = 0.0;
        result.elements[15] = 1.0;
    }

    pub fn from_uniform_scale_new(scale: f64) -> Self {
        let mut result = Self::default();
        Self::from_uniform_scale(scale, &mut result);
        result
    }

    /// Port of `Matrix4.fromRotation`.
    pub fn from_rotation(rotation: &Matrix3, result: &mut Self) {
        result.elements[0] = rotation.elements[0];
        result.elements[1] = rotation.elements[1];
        result.elements[2] = rotation.elements[2];
        result.elements[3] = 0.0;
        result.elements[4] = rotation.elements[3];
        result.elements[5] = rotation.elements[4];
        result.elements[6] = rotation.elements[5];
        result.elements[7] = 0.0;
        result.elements[8] = rotation.elements[6];
        result.elements[9] = rotation.elements[7];
        result.elements[10] = rotation.elements[8];
        result.elements[11] = 0.0;
        result.elements[12] = 0.0;
        result.elements[13] = 0.0;
        result.elements[14] = 0.0;
        result.elements[15] = 1.0;
    }

    pub fn from_rotation_new(rotation: &Matrix3) -> Self {
        let mut result = Self::default();
        Self::from_rotation(rotation, &mut result);
        result
    }

    /// Port of `Matrix4.fromRotationTranslation`.
    pub fn from_rotation_translation(rotation: &Matrix3, translation: &Cartesian3, result: &mut Self) {
        result.elements[0] = rotation.elements[0];
        result.elements[1] = rotation.elements[1];
        result.elements[2] = rotation.elements[2];
        result.elements[3] = 0.0;
        result.elements[4] = rotation.elements[3];
        result.elements[5] = rotation.elements[4];
        result.elements[6] = rotation.elements[5];
        result.elements[7] = 0.0;
        result.elements[8] = rotation.elements[6];
        result.elements[9] = rotation.elements[7];
        result.elements[10] = rotation.elements[8];
        result.elements[11] = 0.0;
        result.elements[12] = translation.x;
        result.elements[13] = translation.y;
        result.elements[14] = translation.z;
        result.elements[15] = 1.0;
    }

    pub fn from_rotation_translation_new(rotation: &Matrix3, translation: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::from_rotation_translation(rotation, translation, &mut result);
        result
    }

    /// Port of `Matrix4.fromTranslationQuaternionRotationScale`.
    pub fn from_translation_quaternion_rotation_scale(
        translation: &Cartesian3,
        rotation: &Quaternion,
        scale: &Cartesian3,
        result: &mut Self,
    ) {
        let sx = scale.x;
        let sy = scale.y;
        let sz = scale.z;

        let x2 = rotation.x * rotation.x;
        let xy = rotation.x * rotation.y;
        let xz = rotation.x * rotation.z;
        let xw = rotation.x * rotation.w;
        let y2 = rotation.y * rotation.y;
        let yz = rotation.y * rotation.z;
        let yw = rotation.y * rotation.w;
        let z2 = rotation.z * rotation.z;
        let zw = rotation.z * rotation.w;
        let w2 = rotation.w * rotation.w;

        let m00 = x2 - y2 - z2 + w2;
        let m01 = 2.0 * (xy - zw);
        let m02 = 2.0 * (xz + yw);

        let m10 = 2.0 * (xy + zw);
        let m11 = -x2 + y2 - z2 + w2;
        let m12 = 2.0 * (yz - xw);

        let m20 = 2.0 * (xz - yw);
        let m21 = 2.0 * (yz + xw);
        let m22 = -x2 - y2 + z2 + w2;

        result.elements[0] = m00 * sx;
        result.elements[1] = m10 * sx;
        result.elements[2] = m20 * sx;
        result.elements[3] = 0.0;
        result.elements[4] = m01 * sy;
        result.elements[5] = m11 * sy;
        result.elements[6] = m21 * sy;
        result.elements[7] = 0.0;
        result.elements[8] = m02 * sz;
        result.elements[9] = m12 * sz;
        result.elements[10] = m22 * sz;
        result.elements[11] = 0.0;
        result.elements[12] = translation.x;
        result.elements[13] = translation.y;
        result.elements[14] = translation.z;
        result.elements[15] = 1.0;
    }

    pub fn from_translation_quaternion_rotation_scale_new(
        translation: &Cartesian3,
        rotation: &Quaternion,
        scale: &Cartesian3,
    ) -> Self {
        let mut result = Self::default();
        Self::from_translation_quaternion_rotation_scale(translation, rotation, scale, &mut result);
        result
    }
    /// Port of `Matrix4.packArray`.
    ///
    /// Rust's `Vec<f64>` mirrors the JS "regular array" branch: it is resized
    /// to `array.len() * 16`. The JS typed-array branch (DeveloperError when
    /// the length does not match) is mirrored by `pack_array_into`.
    pub fn pack_array(array: &[Self], result: &mut Vec<f64>) {
        let result_length = array.len() * 16;
        result.resize(result_length, 0.0);
        for (i, matrix) in array.iter().enumerate() {
            Self::pack(matrix, result, i * 16);
        }
    }

    pub fn pack_array_new(array: &[Self]) -> Vec<f64> {
        let mut result = Vec::new();
        Self::pack_array(array, &mut result);
        result
    }

    /// Mirrors the JS typed-array branch of `Matrix4.packArray`: `result`
    /// must have exactly `array.len() * 16` elements, else a DeveloperError
    /// is thrown (debug_assertions-gated).
    pub fn pack_array_into(array: &[Self], result: &mut [f64]) {
        #[cfg(debug_assertions)]
        if result.len() != array.len() * 16 {
            crate::developer_error::throw_developer_error(
                "If result is a typed array, it must have exactly array.length * 16 elements",
            );
        }
        for (i, matrix) in array.iter().enumerate() {
            Self::pack(matrix, result, i * 16);
        }
    }

    /// Port of `Matrix4.unpackArray`.
    pub fn unpack_array(array: &[f64], result: &mut Vec<Self>) {
        #[cfg(debug_assertions)]
        {
            if array.len() < 16 {
                crate::developer_error::throw_developer_error("array.length must be greater than or equal to 16.");
            }
            if array.len() % 16 != 0 {
                crate::developer_error::throw_developer_error("array length must be a multiple of 16.");
            }
        }

        let length = array.len();
        result.resize(length / 16, Self::default());
        let mut i = 0;
        while i < length {
            let index = i / 16;
            Self::unpack(array, i, &mut result[index]);
            i += 16;
        }
    }

    pub fn unpack_array_new(array: &[f64]) -> Vec<Self> {
        let mut result = Vec::new();
        Self::unpack_array(array, &mut result);
        result
    }

    /// Port of `Matrix4.fromTranslationRotationScale`.
    pub fn from_translation_rotation_scale(
        translation_rotation_scale: &TranslationRotationScale,
        result: &mut Self,
    ) {
        Self::from_translation_quaternion_rotation_scale(
            &translation_rotation_scale.translation,
            &translation_rotation_scale.rotation,
            &translation_rotation_scale.scale,
            result,
        );
    }

    pub fn from_translation_rotation_scale_new(
        translation_rotation_scale: &TranslationRotationScale,
    ) -> Self {
        let mut result = Self::default();
        Self::from_translation_rotation_scale(translation_rotation_scale, &mut result);
        result
    }

    /// Port of `Matrix4.fromCamera`.
    ///
    /// Note (docs/deferred.md #19, SEM-9): previously recorded as deferred
    /// because the JS signature takes a Scene `Camera`; now back-filled
    /// against [`CameraView`], which carries exactly the three fields the JS
    /// reads (`position`/`direction`/`up`). Ledger file intentionally left
    /// untouched (ledger updates belong to the follow-up bookkeeping task).
    pub fn from_camera(camera: &CameraView, result: &mut Self) {
        let position = &camera.position;
        let direction = &camera.direction;
        let up = &camera.up;

        let mut f = Cartesian3::default();
        Cartesian3::normalize(direction, &mut f);
        let mut r = Cartesian3::cross_new(&f, up);
        r = Cartesian3::normalize_new(&r);
        let mut u = Cartesian3::cross_new(&r, &f);
        u = Cartesian3::normalize_new(&u);

        let s_x = r.x;
        let s_y = r.y;
        let s_z = r.z;
        let f_x = f.x;
        let f_y = f.y;
        let f_z = f.z;
        let u_x = u.x;
        let u_y = u.y;
        let u_z = u.z;
        let position_x = position.x;
        let position_y = position.y;
        let position_z = position.z;
        let t0 = s_x * -position_x + s_y * -position_y + s_z * -position_z;
        let t1 = u_x * -position_x + u_y * -position_y + u_z * -position_z;
        let t2 = f_x * position_x + f_y * position_y + f_z * position_z;

        result.elements[0] = s_x;
        result.elements[1] = u_x;
        result.elements[2] = -f_x;
        result.elements[3] = 0.0;
        result.elements[4] = s_y;
        result.elements[5] = u_y;
        result.elements[6] = -f_y;
        result.elements[7] = 0.0;
        result.elements[8] = s_z;
        result.elements[9] = u_z;
        result.elements[10] = -f_z;
        result.elements[11] = 0.0;
        result.elements[12] = t0;
        result.elements[13] = t1;
        result.elements[14] = t2;
        result.elements[15] = 1.0;
    }

    pub fn from_camera_new(camera: &CameraView) -> Self {
        let mut result = Self::default();
        Self::from_camera(camera, &mut result);
        result
    }

    /// Port of `Matrix4.computePerspectiveFieldOfView`.
    pub fn compute_perspective_field_of_view(
        fov_y: f64,
        aspect_ratio: f64,
        near: f64,
        far: f64,
        result: &mut Self,
    ) {
        #[cfg(debug_assertions)]
        {
            if fov_y <= 0.0 {
                crate::developer_error::throw_developer_error("fovY must be greater than 0.");
            }
            if fov_y >= std::f64::consts::PI {
                crate::developer_error::throw_developer_error("fovY must be less than PI.");
            }
            if near <= 0.0 {
                crate::developer_error::throw_developer_error("near must be greater than 0.");
            }
            if far <= 0.0 {
                crate::developer_error::throw_developer_error("far must be greater than 0.");
            }
        }

        let bottom = (fov_y * 0.5).tan();

        let column1_row1 = 1.0 / bottom;
        let column0_row0 = column1_row1 / aspect_ratio;
        let column2_row2 = (far + near) / (near - far);
        let column3_row2 = (2.0 * far * near) / (near - far);

        result.elements[0] = column0_row0;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = column1_row1;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = 0.0;
        result.elements[9] = 0.0;
        result.elements[10] = column2_row2;
        result.elements[11] = -1.0;
        result.elements[12] = 0.0;
        result.elements[13] = 0.0;
        result.elements[14] = column3_row2;
        result.elements[15] = 0.0;
    }

    pub fn compute_perspective_field_of_view_new(
        fov_y: f64,
        aspect_ratio: f64,
        near: f64,
        far: f64,
    ) -> Self {
        let mut result = Self::default();
        Self::compute_perspective_field_of_view(fov_y, aspect_ratio, near, far, &mut result);
        result
    }

    /// Port of `Matrix4.computeOrthographicOffCenter`.
    pub fn compute_orthographic_off_center(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
        result: &mut Self,
    ) {
        let mut a = 1.0 / (right - left);
        let mut b = 1.0 / (top - bottom);
        let mut c = 1.0 / (far - near);

        let tx = -(right + left) * a;
        let ty = -(top + bottom) * b;
        let tz = -(far + near) * c;
        a *= 2.0;
        b *= 2.0;
        c *= -2.0;

        result.elements[0] = a;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = b;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = 0.0;
        result.elements[9] = 0.0;
        result.elements[10] = c;
        result.elements[11] = 0.0;
        result.elements[12] = tx;
        result.elements[13] = ty;
        result.elements[14] = tz;
        result.elements[15] = 1.0;
    }

    pub fn compute_orthographic_off_center_new(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
    ) -> Self {
        let mut result = Self::default();
        Self::compute_orthographic_off_center(left, right, bottom, top, near, far, &mut result);
        result
    }

    /// Port of `Matrix4.computePerspectiveOffCenter`.
    pub fn compute_perspective_off_center(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
        result: &mut Self,
    ) {
        let column0_row0 = (2.0 * near) / (right - left);
        let column1_row1 = (2.0 * near) / (top - bottom);
        let column2_row0 = (right + left) / (right - left);
        let column2_row1 = (top + bottom) / (top - bottom);
        let column2_row2 = -(far + near) / (far - near);
        let column2_row3 = -1.0;
        let column3_row2 = (-2.0 * far * near) / (far - near);

        result.elements[0] = column0_row0;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = column1_row1;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = column2_row0;
        result.elements[9] = column2_row1;
        result.elements[10] = column2_row2;
        result.elements[11] = column2_row3;
        result.elements[12] = 0.0;
        result.elements[13] = 0.0;
        result.elements[14] = column3_row2;
        result.elements[15] = 0.0;
    }

    pub fn compute_perspective_off_center_new(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
    ) -> Self {
        let mut result = Self::default();
        Self::compute_perspective_off_center(left, right, bottom, top, near, far, &mut result);
        result
    }

    /// Port of `Matrix4.computeInfinitePerspectiveOffCenter`.
    pub fn compute_infinite_perspective_off_center(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        result: &mut Self,
    ) {
        let column0_row0 = (2.0 * near) / (right - left);
        let column1_row1 = (2.0 * near) / (top - bottom);
        let column2_row0 = (right + left) / (right - left);
        let column2_row1 = (top + bottom) / (top - bottom);
        let column2_row2 = -1.0;
        let column2_row3 = -1.0;
        let column3_row2 = -2.0 * near;

        result.elements[0] = column0_row0;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = column1_row1;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = column2_row0;
        result.elements[9] = column2_row1;
        result.elements[10] = column2_row2;
        result.elements[11] = column2_row3;
        result.elements[12] = 0.0;
        result.elements[13] = 0.0;
        result.elements[14] = column3_row2;
        result.elements[15] = 0.0;
    }

    pub fn compute_infinite_perspective_off_center_new(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
    ) -> Self {
        let mut result = Self::default();
        Self::compute_infinite_perspective_off_center(left, right, bottom, top, near, &mut result);
        result
    }

    /// Port of `Matrix4.computeViewportTransformation`.
    ///
    /// Note (docs/deferred.md #27, SEM-9): previously recorded as deferred;
    /// now back-filled. The JS `viewport` plain object (with all fields
    /// individually defaulting to 0.0) maps to `Option<&Viewport>` where
    /// `None` mirrors `viewport ?? {}`.
    pub fn compute_viewport_transformation(
        viewport: Option<&Viewport>,
        near_depth_range: Option<f64>,
        far_depth_range: Option<f64>,
        result: &mut Self,
    ) {
        let empty = Viewport::default();
        let viewport = viewport.unwrap_or(&empty);
        let x = viewport.x;
        let y = viewport.y;
        let width = viewport.width;
        let height = viewport.height;
        let near_depth_range = near_depth_range.unwrap_or(0.0);
        let far_depth_range = far_depth_range.unwrap_or(1.0);

        let half_width = width * 0.5;
        let half_height = height * 0.5;
        let half_depth = (far_depth_range - near_depth_range) * 0.5;

        let column0_row0 = half_width;
        let column1_row1 = half_height;
        let column2_row2 = half_depth;
        let column3_row0 = x + half_width;
        let column3_row1 = y + half_height;
        let column3_row2 = near_depth_range + half_depth;
        let column3_row3 = 1.0;

        result.elements[0] = column0_row0;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = 0.0;
        result.elements[5] = column1_row1;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = 0.0;
        result.elements[9] = 0.0;
        result.elements[10] = column2_row2;
        result.elements[11] = 0.0;
        result.elements[12] = column3_row0;
        result.elements[13] = column3_row1;
        result.elements[14] = column3_row2;
        result.elements[15] = column3_row3;
    }

    pub fn compute_viewport_transformation_new(
        viewport: Option<&Viewport>,
        near_depth_range: Option<f64>,
        far_depth_range: Option<f64>,
    ) -> Self {
        let mut result = Self::default();
        Self::compute_viewport_transformation(
            viewport,
            near_depth_range,
            far_depth_range,
            &mut result,
        );
        result
    }

    /// Port of `Matrix4.computeView`.
    pub fn compute_view(
        position: &Cartesian3,
        direction: &Cartesian3,
        up: &Cartesian3,
        right: &Cartesian3,
        result: &mut Self,
    ) {
        result.elements[0] = right.x;
        result.elements[1] = up.x;
        result.elements[2] = -direction.x;
        result.elements[3] = 0.0;
        result.elements[4] = right.y;
        result.elements[5] = up.y;
        result.elements[6] = -direction.y;
        result.elements[7] = 0.0;
        result.elements[8] = right.z;
        result.elements[9] = up.z;
        result.elements[10] = -direction.z;
        result.elements[11] = 0.0;
        result.elements[12] = -Cartesian3::dot(right, position);
        result.elements[13] = -Cartesian3::dot(up, position);
        result.elements[14] = Cartesian3::dot(direction, position);
        result.elements[15] = 1.0;
    }

    pub fn compute_view_new(
        position: &Cartesian3,
        direction: &Cartesian3,
        up: &Cartesian3,
        right: &Cartesian3,
    ) -> Self {
        let mut result = Self::default();
        Self::compute_view(position, direction, up, right, &mut result);
        result
    }

    // --- toArray ---

    pub fn to_array(matrix: &Self, result: &mut [f64]) {
        result[..16].copy_from_slice(&matrix.elements);
    }

    pub fn to_array_new(matrix: &Self) -> [f64; 16] {
        let mut result = [0.0; 16];
        Self::to_array(matrix, &mut result);
        result
    }

    pub fn get_element_index(column: usize, row: usize) -> usize {
        column * 4 + row
    }

    // --- getColumn / setColumn ---

    pub fn get_column(matrix: &Self, index: usize, result: &mut Cartesian4) {
        let start = index * 4;
        result.x = matrix.elements[start];
        result.y = matrix.elements[start + 1];
        result.z = matrix.elements[start + 2];
        result.w = matrix.elements[start + 3];
    }

    pub fn get_column_new(matrix: &Self, index: usize) -> Cartesian4 {
        let mut result = Cartesian4::default();
        Self::get_column(matrix, index, &mut result);
        result
    }

    pub fn set_column(matrix: &Self, index: usize, cartesian: &Cartesian4, result: &mut Self) {
        *result = *matrix;
        let start = index * 4;
        result.elements[start] = cartesian.x;
        result.elements[start + 1] = cartesian.y;
        result.elements[start + 2] = cartesian.z;
        result.elements[start + 3] = cartesian.w;
    }

    // --- getRow / setRow ---

    pub fn get_row(matrix: &Self, index: usize, result: &mut Cartesian4) {
        result.x = matrix.elements[index];
        result.y = matrix.elements[index + 4];
        result.z = matrix.elements[index + 8];
        result.w = matrix.elements[index + 12];
    }

    pub fn get_row_new(matrix: &Self, index: usize) -> Cartesian4 {
        let mut result = Cartesian4::default();
        Self::get_row(matrix, index, &mut result);
        result
    }

    pub fn set_row(matrix: &Self, index: usize, cartesian: &Cartesian4, result: &mut Self) {
        *result = *matrix;
        result.elements[index] = cartesian.x;
        result.elements[index + 4] = cartesian.y;
        result.elements[index + 8] = cartesian.z;
        result.elements[index + 12] = cartesian.w;
    }

    // --- getTranslation ---

    pub fn get_translation(matrix: &Self, result: &mut Cartesian3) {
        result.x = matrix.elements[12];
        result.y = matrix.elements[13];
        result.z = matrix.elements[14];
    }

    pub fn get_translation_new(matrix: &Self) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::get_translation(matrix, &mut result);
        result
    }

    // --- setTranslation ---

    pub fn set_translation(matrix: &Self, translation: &Cartesian3, result: &mut Self) {
        *result = *matrix;
        result.elements[12] = translation.x;
        result.elements[13] = translation.y;
        result.elements[14] = translation.z;
    }

    // --- getScale / setScale ---

    pub fn get_scale(matrix: &Self, result: &mut Cartesian3) {
        let col0 = Cartesian3::new(matrix.elements[0], matrix.elements[1], matrix.elements[2]);
        let col1 = Cartesian3::new(matrix.elements[4], matrix.elements[5], matrix.elements[6]);
        let col2 = Cartesian3::new(matrix.elements[8], matrix.elements[9], matrix.elements[10]);
        result.x = Cartesian3::magnitude(&col0);
        result.y = Cartesian3::magnitude(&col1);
        result.z = Cartesian3::magnitude(&col2);
    }

    pub fn get_scale_new(matrix: &Self) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::get_scale(matrix, &mut result);
        result
    }

    pub fn get_maximum_scale(matrix: &Self) -> f64 {
        let scale = Self::get_scale_new(matrix);
        Cartesian3::maximum_component(&scale)
    }

    pub fn set_scale(matrix: &Self, scale: &Cartesian3, result: &mut Self) {
        let existing = Self::get_scale_new(matrix);
        let rx = scale.x / existing.x;
        let ry = scale.y / existing.y;
        let rz = scale.z / existing.z;
        *result = *matrix;
        result.elements[0] *= rx; result.elements[1] *= rx; result.elements[2] *= rx;
        result.elements[4] *= ry; result.elements[5] *= ry; result.elements[6] *= ry;
        result.elements[8] *= rz; result.elements[9] *= rz; result.elements[10] *= rz;
    }

    pub fn set_uniform_scale(matrix: &Self, scale: f64, result: &mut Self) {
        let existing = Self::get_scale_new(matrix);
        let rx = scale / existing.x;
        let ry = scale / existing.y;
        let rz = scale / existing.z;
        *result = *matrix;
        result.elements[0] *= rx; result.elements[1] *= rx; result.elements[2] *= rx;
        result.elements[4] *= ry; result.elements[5] *= ry; result.elements[6] *= ry;
        result.elements[8] *= rz; result.elements[9] *= rz; result.elements[10] *= rz;
    }

    // --- getRotation / setRotation ---

    pub fn get_rotation(matrix: &Self, result: &mut Matrix3) {
        let scale = Self::get_scale_new(matrix);
        result.elements[0] = matrix.elements[0] / scale.x;
        result.elements[1] = matrix.elements[1] / scale.x;
        result.elements[2] = matrix.elements[2] / scale.x;
        result.elements[3] = matrix.elements[4] / scale.y;
        result.elements[4] = matrix.elements[5] / scale.y;
        result.elements[5] = matrix.elements[6] / scale.y;
        result.elements[6] = matrix.elements[8] / scale.z;
        result.elements[7] = matrix.elements[9] / scale.z;
        result.elements[8] = matrix.elements[10] / scale.z;
    }

    pub fn get_rotation_new(matrix: &Self) -> Matrix3 {
        let mut result = Matrix3::default();
        Self::get_rotation(matrix, &mut result);
        result
    }

    pub fn set_rotation(matrix: &Self, rotation: &Matrix3, result: &mut Self) {
        let scale = Self::get_scale_new(matrix);
        *result = *matrix;
        result.elements[0] = rotation.elements[0] * scale.x;
        result.elements[1] = rotation.elements[1] * scale.x;
        result.elements[2] = rotation.elements[2] * scale.x;
        result.elements[4] = rotation.elements[3] * scale.y;
        result.elements[5] = rotation.elements[4] * scale.y;
        result.elements[6] = rotation.elements[5] * scale.y;
        result.elements[8] = rotation.elements[6] * scale.z;
        result.elements[9] = rotation.elements[7] * scale.z;
        result.elements[10] = rotation.elements[8] * scale.z;
    }

    // --- getMatrix3 ---

    pub fn get_matrix3(matrix: &Self, result: &mut Matrix3) {
        result.elements[0] = matrix.elements[0];
        result.elements[1] = matrix.elements[1];
        result.elements[2] = matrix.elements[2];
        result.elements[3] = matrix.elements[4];
        result.elements[4] = matrix.elements[5];
        result.elements[5] = matrix.elements[6];
        result.elements[6] = matrix.elements[8];
        result.elements[7] = matrix.elements[9];
        result.elements[8] = matrix.elements[10];
    }

    pub fn get_matrix3_new(matrix: &Self) -> Matrix3 {
        let mut result = Matrix3::default();
        Self::get_matrix3(matrix, &mut result);
        result
    }

    // --- multiply ---

    pub fn multiply(left: &Self, right: &Self, result: &mut Self) {
        let mut tmp = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += left.elements[k * 4 + row] * right.elements[col * 4 + k];
                }
                tmp[col * 4 + row] = sum;
            }
        }
        result.elements = tmp;
    }

    pub fn multiply_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply(left, right, &mut result);
        result
    }

    /// Port of `Matrix4.multiplyTransformation`.
    /// Assumes both matrices are affine transformation matrices.
    pub fn multiply_transformation(left: &Self, right: &Self, result: &mut Self) {
        // Same as full multiply for correctness
        Self::multiply(left, right, result);
    }

    pub fn multiply_transformation_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply_transformation(left, right, &mut result);
        result
    }

    /// Port of `Matrix4.multiplyByMatrix3`.
    pub fn multiply_by_matrix3(matrix: &Self, rotation: &Matrix3, result: &mut Self) {
        let rot4 = Self::from_rotation_new(rotation);
        Self::multiply(matrix, &rot4, result);
    }

    /// Port of `Matrix4.multiplyByTranslation`.
    pub fn multiply_by_translation(matrix: &Self, translation: &Cartesian3, result: &mut Self) {
        let t4 = Self::from_translation_new(translation);
        Self::multiply(matrix, &t4, result);
    }

    // --- add / subtract ---

    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        for i in 0..16 {
            result.elements[i] = left.elements[i] + right.elements[i];
        }
    }

    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        for i in 0..16 {
            result.elements[i] = left.elements[i] - right.elements[i];
        }
    }

    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    // --- multiplyByVector ---

    pub fn multiply_by_vector(matrix: &Self, cartesian: &Cartesian4, result: &mut Cartesian4) {
        let x = matrix.elements[0]*cartesian.x + matrix.elements[4]*cartesian.y + matrix.elements[8]*cartesian.z + matrix.elements[12]*cartesian.w;
        let y = matrix.elements[1]*cartesian.x + matrix.elements[5]*cartesian.y + matrix.elements[9]*cartesian.z + matrix.elements[13]*cartesian.w;
        let z = matrix.elements[2]*cartesian.x + matrix.elements[6]*cartesian.y + matrix.elements[10]*cartesian.z + matrix.elements[14]*cartesian.w;
        let w = matrix.elements[3]*cartesian.x + matrix.elements[7]*cartesian.y + matrix.elements[11]*cartesian.z + matrix.elements[15]*cartesian.w;
        result.x = x; result.y = y; result.z = z; result.w = w;
    }

    pub fn multiply_by_vector_new(matrix: &Self, cartesian: &Cartesian4) -> Cartesian4 {
        let mut result = Cartesian4::default();
        Self::multiply_by_vector(matrix, cartesian, &mut result);
        result
    }

    /// Port of `Matrix4.multiplyByPointAsVector`.
    pub fn multiply_by_point_as_vector(matrix: &Self, cartesian: &Cartesian3, result: &mut Cartesian3) {
        // JS applies no translation column (w == 0 semantics).
        let x = matrix.elements[0]*cartesian.x + matrix.elements[4]*cartesian.y + matrix.elements[8]*cartesian.z;
        let y = matrix.elements[1]*cartesian.x + matrix.elements[5]*cartesian.y + matrix.elements[9]*cartesian.z;
        let z = matrix.elements[2]*cartesian.x + matrix.elements[6]*cartesian.y + matrix.elements[10]*cartesian.z;
        result.x = x; result.y = y; result.z = z;
    }

    pub fn multiply_by_point_as_vector_new(matrix: &Self, cartesian: &Cartesian3) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::multiply_by_point_as_vector(matrix, cartesian, &mut result);
        result
    }

    /// Port of `Matrix4.multiplyByPoint`.
    pub fn multiply_by_point(matrix: &Self, cartesian: &Cartesian3, result: &mut Cartesian3) {
        // Equivalent to multiplyByVector with w == 1; rows 0/1/2 plus the
        // translation column. CesiumJS performs no perspective division.
        let x = matrix.elements[0]*cartesian.x + matrix.elements[4]*cartesian.y + matrix.elements[8]*cartesian.z + matrix.elements[12];
        let y = matrix.elements[1]*cartesian.x + matrix.elements[5]*cartesian.y + matrix.elements[9]*cartesian.z + matrix.elements[13];
        let z = matrix.elements[2]*cartesian.x + matrix.elements[6]*cartesian.y + matrix.elements[10]*cartesian.z + matrix.elements[14];
        result.x = x;
        result.y = y;
        result.z = z;
    }

    pub fn multiply_by_point_new(matrix: &Self, cartesian: &Cartesian3) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::multiply_by_point(matrix, cartesian, &mut result);
        result
    }

    // --- multiplyByScalar / multiplyByScale ---

    pub fn multiply_by_scalar(matrix: &Self, scalar: f64, result: &mut Self) {
        for i in 0..16 {
            result.elements[i] = matrix.elements[i] * scalar;
        }
    }

    pub fn multiply_by_scalar_new(matrix: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(matrix, scalar, &mut result);
        result
    }

    pub fn multiply_by_scale(matrix: &Self, scale: &Cartesian3, result: &mut Self) {
        result.elements[0] = matrix.elements[0]*scale.x; result.elements[1] = matrix.elements[1]*scale.x;
        result.elements[2] = matrix.elements[2]*scale.x; result.elements[3] = matrix.elements[3]*scale.x;
        result.elements[4] = matrix.elements[4]*scale.y; result.elements[5] = matrix.elements[5]*scale.y;
        result.elements[6] = matrix.elements[6]*scale.y; result.elements[7] = matrix.elements[7]*scale.y;
        result.elements[8] = matrix.elements[8]*scale.z; result.elements[9] = matrix.elements[9]*scale.z;
        result.elements[10] = matrix.elements[10]*scale.z; result.elements[11] = matrix.elements[11]*scale.z;
        result.elements[12] = matrix.elements[12]; result.elements[13] = matrix.elements[13];
        result.elements[14] = matrix.elements[14]; result.elements[15] = matrix.elements[15];
    }

    pub fn multiply_by_uniform_scale(matrix: &Self, scale: f64, result: &mut Self) {
        for i in 0..12 {
            result.elements[i] = matrix.elements[i] * scale;
        }
        result.elements[12] = matrix.elements[12];
        result.elements[13] = matrix.elements[13];
        result.elements[14] = matrix.elements[14];
        result.elements[15] = matrix.elements[15];
    }

    // --- negate / transpose / abs ---

    pub fn negate(matrix: &Self, result: &mut Self) {
        for i in 0..16 { result.elements[i] = -matrix.elements[i]; }
    }

    pub fn negate_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(matrix, &mut result);
        result
    }

    pub fn transpose(matrix: &Self, result: &mut Self) {
        let mut tmp = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                tmp[col * 4 + row] = matrix.elements[row * 4 + col];
            }
        }
        result.elements = tmp;
    }

    pub fn transpose_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::transpose(matrix, &mut result);
        result
    }

    pub fn abs(matrix: &Self, result: &mut Self) {
        for i in 0..16 { result.elements[i] = matrix.elements[i].abs(); }
    }

    pub fn abs_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::abs(matrix, &mut result);
        result
    }

    // --- determinant ---

    pub fn determinant(matrix: &Self) -> f64 {
        let e = &matrix.elements;
        // Column-major naming: mIJ = column I, row J = elements[I*4 + J]
        let m00 = e[0]; let m01 = e[1]; let m02 = e[2]; let m03 = e[3];
        let m10 = e[4]; let m11 = e[5]; let m12 = e[6]; let m13 = e[7];
        let m20 = e[8]; let m21 = e[9]; let m22 = e[10]; let m23 = e[11];
        let m30 = e[12]; let m31 = e[13]; let m32 = e[14]; let m33 = e[15];

        // Laplace expansion along the first row
        let det = m00 * (m11 * (m22 * m33 - m23 * m32) - m21 * (m12 * m33 - m13 * m32) + m31 * (m12 * m23 - m13 * m22))
                - m10 * (m01 * (m22 * m33 - m23 * m32) - m21 * (m02 * m33 - m03 * m32) + m31 * (m02 * m23 - m03 * m22))
                + m20 * (m01 * (m12 * m33 - m13 * m32) - m11 * (m02 * m33 - m03 * m32) + m31 * (m02 * m13 - m03 * m12))
                - m30 * (m01 * (m12 * m23 - m13 * m22) - m11 * (m02 * m23 - m03 * m22) + m21 * (m02 * m13 - m03 * m12));
        det
    }

    // --- inverse ---

    pub fn inverse(matrix: &Self, result: &mut Self) -> bool {
        let e = &matrix.elements;
        let m00 = e[0]; let m01 = e[4]; let m02 = e[8]; let m03 = e[12];
        let m10 = e[1]; let m11 = e[5]; let m12 = e[9]; let m13 = e[13];
        let m20 = e[2]; let m21 = e[6]; let m22 = e[10]; let m23 = e[14];
        let m30 = e[3]; let m31 = e[7]; let m32 = e[11]; let m33 = e[15];

        let a00 = m00*m11 - m01*m10;
        let a01 = m00*m12 - m02*m10;
        let a02 = m00*m13 - m03*m10;
        let a03 = m01*m12 - m02*m11;
        let a04 = m01*m13 - m03*m11;
        let a05 = m02*m13 - m03*m12;
        let a06 = m20*m31 - m21*m30;
        let a07 = m20*m32 - m22*m30;
        let a08 = m20*m33 - m23*m30;
        let a09 = m21*m32 - m22*m31;
        let a10 = m21*m33 - m23*m31;
        let a11 = m22*m33 - m23*m32;

        let det = a00*a11 - a01*a10 + a02*a09 + a03*a08 - a04*a07 + a05*a06;
        if det.abs() <= 1e-15 {
            return false;
        }

        let inv_det = 1.0 / det;
        result.elements[0]  = ( m11*a11 - m12*a10 + m13*a09) * inv_det;
        result.elements[1]  = (-m10*a11 + m12*a08 - m13*a07) * inv_det;
        result.elements[2]  = ( m10*a10 - m11*a08 + m13*a06) * inv_det;
        result.elements[3]  = (-m10*a09 + m11*a07 - m12*a06) * inv_det;
        result.elements[4]  = (-m01*a11 + m02*a10 - m03*a09) * inv_det;
        result.elements[5]  = ( m00*a11 - m02*a08 + m03*a07) * inv_det;
        result.elements[6]  = (-m00*a10 + m01*a08 - m03*a06) * inv_det;
        result.elements[7]  = ( m00*a09 - m01*a07 + m02*a06) * inv_det;
        result.elements[8]  = ( m31*a05 - m32*a04 + m33*a03) * inv_det;
        result.elements[9]  = (-m30*a05 + m32*a02 - m33*a01) * inv_det;
        result.elements[10] = ( m30*a04 - m31*a02 + m33*a00) * inv_det;
        result.elements[11] = (-m30*a03 + m31*a01 - m32*a00) * inv_det;
        result.elements[12] = (-m21*a05 + m22*a04 - m23*a03) * inv_det;
        result.elements[13] = ( m20*a05 - m22*a02 + m23*a01) * inv_det;
        result.elements[14] = (-m20*a04 + m21*a02 - m23*a00) * inv_det;
        result.elements[15] = ( m20*a03 - m21*a01 + m22*a00) * inv_det;
        true
    }

    pub fn inverse_new(matrix: &Self) -> Option<Self> {
        let mut result = Self::default();
        if Self::inverse(matrix, &mut result) { Some(result) } else { None }
    }

    /// Port of `Matrix4.inverseTransformation`.
    /// Efficiently inverts a transformation matrix (rotation + translation).
    pub fn inverse_transformation(matrix: &Self, result: &mut Self) {
        // Transpose the 3x3 rotation part
        result.elements[0]  = matrix.elements[0];
        result.elements[1]  = matrix.elements[4];
        result.elements[2]  = matrix.elements[8];
        result.elements[3]  = 0.0;
        result.elements[4]  = matrix.elements[1];
        result.elements[5]  = matrix.elements[5];
        result.elements[6]  = matrix.elements[9];
        result.elements[7]  = 0.0;
        result.elements[8]  = matrix.elements[2];
        result.elements[9]  = matrix.elements[6];
        result.elements[10] = matrix.elements[10];
        result.elements[11] = 0.0;
        // Compute new translation
        result.elements[12] = -(matrix.elements[0]*matrix.elements[12] + matrix.elements[1]*matrix.elements[13] + matrix.elements[2]*matrix.elements[14]);
        result.elements[13] = -(matrix.elements[4]*matrix.elements[12] + matrix.elements[5]*matrix.elements[13] + matrix.elements[6]*matrix.elements[14]);
        result.elements[14] = -(matrix.elements[8]*matrix.elements[12] + matrix.elements[9]*matrix.elements[13] + matrix.elements[10]*matrix.elements[14]);
        result.elements[15] = 1.0;
    }

    pub fn inverse_transformation_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::inverse_transformation(matrix, &mut result);
        result
    }

    /// Port of `Matrix4.inverseTranspose`.
    pub fn inverse_transpose(matrix: &Self, result: &mut Self) -> bool {
        let mut transposed = Self::default();
        Self::transpose(matrix, &mut transposed);
        Self::inverse(&transposed, result)
    }

    // --- equals ---

    pub fn equals(left: &Self, right: &Self) -> bool {
        left.elements == right.elements
    }

    pub fn equals_epsilon(left: &Self, right: &Self, epsilon: f64) -> bool {
        for i in 0..16 {
            if (left.elements[i] - right.elements[i]).abs() > epsilon { return false; }
        }
        true
    }

    // --- clone ---

    pub fn clone(matrix: &Self, result: &mut Self) {
        result.elements = matrix.elements;
    }

    pub fn clone_new(matrix: &Self) -> Self {
        Self { elements: matrix.elements }
    }

    /// Port of the JS instance getter `length` (returns `packedLength`).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        Self::PACKED_LENGTH
    }

    /// Port of `Matrix4.equalsArray` (`@ignore` in the JS source).
    pub fn equals_array(matrix: &Self, array: &[f64], offset: usize) -> bool {
        for i in 0..16 {
            if matrix.elements[i] != array[offset + i] {
                return false;
            }
        }
        true
    }
}

/// Mirrors the plain-object `viewport` parameter accepted by the JS
/// `computeViewportTransformation` (`{ x, y, width, height }`, all fields
/// individually defaulting to 0.0 via `??`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    /// The x coordinate of the viewport's lower-left corner.
    pub x: f64,
    /// The y coordinate of the viewport's lower-left corner.
    pub y: f64,
    /// The width of the viewport in pixels.
    pub width: f64,
    /// The height of the viewport in pixels.
    pub height: f64,
}

/// Minimal camera data required by [`Matrix4::from_camera`].
///
/// DEVIATION: the JS signature takes the full Scene `Camera`, which lives in
/// cesium-scene and cannot be referenced from cesium-core; this struct
/// carries exactly the three fields the JS reads (`position`, `direction`,
/// `up`).
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraView {
    /// The camera's position (`Camera.position`).
    pub position: Cartesian3,
    /// The camera's view direction (`Camera.direction`).
    pub direction: Cartesian3,
    /// The camera's up direction (`Camera.up`).
    pub up: Cartesian3,
}

impl PartialEq for Matrix4 {
    fn eq(&self, other: &Self) -> bool { Self::equals(self, other) }
}

impl std::fmt::Display for Matrix4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in 0..4 {
            if row > 0 { writeln!(f)?; }
            write!(f, "({}, {}, {}, {})",
                self.elements[row], self.elements[row+4],
                self.elements[row+8], self.elements[row+12])?;
        }
        Ok(())
    }
}
