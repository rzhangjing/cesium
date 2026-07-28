//! Matrix4 CesiumJS extension functions.
//! Maps to CesiumJS `Core/Matrix4.js` static methods beyond basic glam operations.
//! Note: CesiumJS stores matrices in column-major order, same as glam DMat4.

use crate::math_utils;
use glam::{DMat3, DMat4, DVec3};

/// The packed length of a Matrix4: 16.
pub const PACKED_LENGTH: usize = 16;

/// Creates a Matrix4 from a rotation (Matrix3) and translation.
/// Maps to CesiumJS `Matrix4.fromRotationTranslation`
pub fn from_rotation_translation(rotation: &DMat3, translation: DVec3) -> DMat4 {
    DMat4::from_cols_array(&[
        rotation.x_axis.x,
        rotation.x_axis.y,
        rotation.x_axis.z,
        0.0,
        rotation.y_axis.x,
        rotation.y_axis.y,
        rotation.y_axis.z,
        0.0,
        rotation.z_axis.x,
        rotation.z_axis.y,
        rotation.z_axis.z,
        0.0,
        translation.x,
        translation.y,
        translation.z,
        1.0,
    ])
}

/// Creates a Matrix4 from a translation vector.
/// Maps to CesiumJS `Matrix4.fromTranslation`
pub fn from_translation(translation: DVec3) -> DMat4 {
    DMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        translation.x, translation.y, translation.z, 1.0,
    ])
}

/// Creates a Matrix4 from a non-uniform scale.
/// Maps to CesiumJS `Matrix4.fromScale`
pub fn from_scale(scale: DVec3) -> DMat4 {
    DMat4::from_cols_array(&[
        scale.x, 0.0, 0.0, 0.0,
        0.0, scale.y, 0.0, 0.0,
        0.0, 0.0, scale.z, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ])
}

/// Creates a Matrix4 from a uniform scale.
/// Maps to CesiumJS `Matrix4.fromUniformScale`
pub fn from_uniform_scale(scale: f64) -> DMat4 {
    from_scale(DVec3::splat(scale))
}

/// Gets the translation component of an affine transformation matrix.
/// Maps to CesiumJS `Matrix4.getTranslation`
pub fn get_translation(matrix: &DMat4) -> DVec3 {
    DVec3::new(matrix.w_axis.x, matrix.w_axis.y, matrix.w_axis.z)
}

/// Gets the scale component of an affine transformation matrix.
/// Maps to CesiumJS `Matrix4.getScale`
pub fn get_scale(matrix: &DMat4) -> DVec3 {
    let sx = DVec3::new(matrix.x_axis.x, matrix.x_axis.y, matrix.x_axis.z).length();
    let sy = DVec3::new(matrix.y_axis.x, matrix.y_axis.y, matrix.y_axis.z).length();
    let sz = DVec3::new(matrix.z_axis.x, matrix.z_axis.y, matrix.z_axis.z).length();
    DVec3::new(sx, sy, sz)
}

/// Gets the maximum scale of an affine transformation matrix.
/// Maps to CesiumJS `Matrix4.getMaximumScale`
pub fn get_maximum_scale(matrix: &DMat4) -> f64 {
    let scale = get_scale(matrix);
    scale.x.max(scale.y).max(scale.z)
}

/// Gets the rotation component (upper-left 3x3 normalized by scale).
/// Maps to CesiumJS `Matrix4.getRotation`
pub fn get_rotation(matrix: &DMat4) -> DMat3 {
    let scale = get_scale(matrix);
    DMat3::from_cols_array(&[
        matrix.x_axis.x / scale.x,
        matrix.x_axis.y / scale.x,
        matrix.x_axis.z / scale.x,
        matrix.y_axis.x / scale.y,
        matrix.y_axis.y / scale.y,
        matrix.y_axis.z / scale.y,
        matrix.z_axis.x / scale.z,
        matrix.z_axis.y / scale.z,
        matrix.z_axis.z / scale.z,
    ])
}

/// Multiplies an affine transformation matrix by an implicit translation.
/// Equivalent to matrix * fromTranslation(translation) but more efficient.
/// Maps to CesiumJS `Matrix4.multiplyByTranslation`
pub fn multiply_by_translation(matrix: &DMat4, translation: DVec3) -> DMat4 {
    let x = translation.x;
    let y = translation.y;
    let z = translation.z;

    let tx = x * matrix.x_axis.x
        + y * matrix.y_axis.x
        + z * matrix.z_axis.x
        + matrix.w_axis.x;
    let ty = x * matrix.x_axis.y
        + y * matrix.y_axis.y
        + z * matrix.z_axis.y
        + matrix.w_axis.y;
    let tz = x * matrix.x_axis.z
        + y * matrix.y_axis.z
        + z * matrix.z_axis.z
        + matrix.w_axis.z;

    DMat4::from_cols_array(&[
        matrix.x_axis.x, matrix.x_axis.y, matrix.x_axis.z, matrix.x_axis.w,
        matrix.y_axis.x, matrix.y_axis.y, matrix.y_axis.z, matrix.y_axis.w,
        matrix.z_axis.x, matrix.z_axis.y, matrix.z_axis.z, matrix.z_axis.w,
        tx, ty, tz, matrix.w_axis.w,
    ])
}

/// Multiplies an affine transformation matrix by an implicit non-uniform scale.
/// Maps to CesiumJS `Matrix4.multiplyByScale`
pub fn multiply_by_scale(matrix: &DMat4, scale: DVec3) -> DMat4 {
    if scale.x == 1.0 && scale.y == 1.0 && scale.z == 1.0 {
        return *matrix;
    }

    DMat4::from_cols_array(&[
        scale.x * matrix.x_axis.x,
        scale.x * matrix.x_axis.y,
        scale.x * matrix.x_axis.z,
        matrix.x_axis.w,
        scale.y * matrix.y_axis.x,
        scale.y * matrix.y_axis.y,
        scale.y * matrix.y_axis.z,
        matrix.y_axis.w,
        scale.z * matrix.z_axis.x,
        scale.z * matrix.z_axis.y,
        scale.z * matrix.z_axis.z,
        matrix.z_axis.w,
        matrix.w_axis.x,
        matrix.w_axis.y,
        matrix.w_axis.z,
        matrix.w_axis.w,
    ])
}

/// Computes a perspective projection matrix from field of view.
/// Maps to CesiumJS `Matrix4.computePerspectiveFieldOfView`
pub fn compute_perspective_field_of_view(
    fov_y: f64,
    aspect_ratio: f64,
    near: f64,
    far: f64,
) -> DMat4 {
    let bottom = (fov_y * 0.5).tan();
    let column1_row1 = 1.0 / bottom;
    let column0_row0 = column1_row1 / aspect_ratio;
    let column2_row2 = (far + near) / (near - far);
    let column3_row2 = (2.0 * far * near) / (near - far);

    DMat4::from_cols_array(&[
        column0_row0, 0.0, 0.0, 0.0,
        0.0, column1_row1, 0.0, 0.0,
        0.0, 0.0, column2_row2, -1.0,
        0.0, 0.0, column3_row2, 0.0,
    ])
}

/// Packs a Matrix4 into an array at the given starting index.
/// Maps to CesiumJS `Matrix4.pack`
pub fn pack(value: &DMat4, array: &mut [f64], starting_index: usize) {
    let cols = value.to_cols_array();
    for (i, &v) in cols.iter().enumerate() {
        array[starting_index + i] = v;
    }
}

/// Unpacks a Matrix4 from an array at the given starting index.
/// Maps to CesiumJS `Matrix4.unpack`
pub fn unpack(array: &[f64], starting_index: usize) -> DMat4 {
    let mut cols = [0.0f64; 16];
    for i in 0..16 {
        cols[i] = array[starting_index + i];
    }
    DMat4::from_cols_array(&cols)
}

/// Returns true if left and right are equal within the provided epsilon.
/// Maps to CesiumJS `Matrix4.equalsEpsilon`
pub fn equals_epsilon(left: &DMat4, right: &DMat4, epsilon: f64) -> bool {
    let l = left.to_cols_array();
    let r = right.to_cols_array();
    l.iter()
        .zip(r.iter())
        .all(|(&a, &b)| math_utils::equals_epsilon(a, b, 0.0, epsilon))
}
