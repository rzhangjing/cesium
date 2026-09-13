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

/// Computes a view matrix from eye position, direction, and up vector.
/// Maps to CesiumJS `Matrix4.computeView`
pub fn compute_view(position: DVec3, direction: DVec3, up: DVec3) -> DMat4 {
    let right = direction.cross(up);
    // Column-major: each column is [right, up, -direction, position] transposed
    DMat4::from_cols_array(&[
        right.x, up.x, -direction.x, 0.0,
        right.y, up.y, -direction.y, 0.0,
        right.z, up.z, -direction.z, 0.0,
        -right.dot(position), -up.dot(position), direction.dot(position), 1.0,
    ])
}

/// Creates a Matrix4 from translation, quaternion rotation, and scale.
/// Maps to CesiumJS `Matrix4.fromTranslationQuaternionRotationScale`
pub fn from_translation_quaternion_rotation_scale(
    translation: DVec3,
    rotation: glam::DQuat,
    scale: DVec3,
) -> DMat4 {
    let r = DMat3::from_quat(rotation);
    DMat4::from_cols_array(&[
        r.x_axis.x * scale.x, r.x_axis.y * scale.x, r.x_axis.z * scale.x, 0.0,
        r.y_axis.x * scale.y, r.y_axis.y * scale.y, r.y_axis.z * scale.y, 0.0,
        r.z_axis.x * scale.z, r.z_axis.y * scale.z, r.z_axis.z * scale.z, 0.0,
        translation.x, translation.y, translation.z, 1.0,
    ])
}

/// Multiplies two affine transformation matrices, ignoring the 4th row.
/// The result always has [0,0,0,1] as its 4th row.
/// Maps to CesiumJS `Matrix4.multiplyTransformation`
pub fn multiply_transformation(left: &DMat4, right: &DMat4) -> DMat4 {
    let l = left.to_cols_array();
    let r = right.to_cols_array();
    let mut out = [0.0f64; 16];
    // Columns 0..2: standard 4x4 multiply but with row3 = [0,0,0,1]
    for col in 0..3 {
        for row in 0..3 {
            out[col * 4 + row] = l[row] * r[col * 4]
                + l[4 + row] * r[col * 4 + 1]
                + l[8 + row] * r[col * 4 + 2];
        }
        out[col * 4 + 3] = 0.0;
    }
    // Column 3 (translation): left * right_col3
    for row in 0..3 {
        out[12 + row] = l[row] * r[12]
            + l[4 + row] * r[13]
            + l[8 + row] * r[14]
            + l[12 + row];
    }
    out[15] = 1.0;
    DMat4::from_cols_array(&out)
}

/// Transforms a point by a 4x4 matrix, treating it as a direction (w=0).
/// Translation is ignored.
/// Maps to CesiumJS `Matrix4.multiplyByPointAsVector`
pub fn multiply_by_point_as_vector(matrix: &DMat4, point: DVec3) -> DVec3 {
    DVec3::new(
        matrix.x_axis.x * point.x + matrix.y_axis.x * point.y + matrix.z_axis.x * point.z,
        matrix.x_axis.y * point.x + matrix.y_axis.y * point.y + matrix.z_axis.y * point.z,
        matrix.x_axis.z * point.x + matrix.y_axis.z * point.y + matrix.z_axis.z * point.z,
    )
}

/// Computes the inverse of an affine transformation matrix.
/// More efficient than general inverse for matrices with [0,0,0,1] last row.
/// Maps to CesiumJS `Matrix4.inverseTransformation`
pub fn inverse_transformation(matrix: &DMat4) -> DMat4 {
    // For an affine matrix [R|t; 0|1], inverse is [R^T | -R^T*t; 0 | 1]
    // Extract rotation columns
    let col0 = DVec3::new(matrix.x_axis.x, matrix.x_axis.y, matrix.x_axis.z);
    let col1 = DVec3::new(matrix.y_axis.x, matrix.y_axis.y, matrix.y_axis.z);
    let col2 = DVec3::new(matrix.z_axis.x, matrix.z_axis.y, matrix.z_axis.z);
    let t = DVec3::new(matrix.w_axis.x, matrix.w_axis.y, matrix.w_axis.z);

    // R^T rows are the original columns
    // new_t = -R^T * t = -(col0.dot(t), col1.dot(t), col2.dot(t))
    let nt = DVec3::new(-col0.dot(t), -col1.dot(t), -col2.dot(t));

    // R^T in column-major: column i of R^T = row i of R
    // row 0 of R = (col0.x, col1.x, col2.x)
    // row 1 of R = (col0.y, col1.y, col2.y)
    // row 2 of R = (col0.z, col1.z, col2.z)
    DMat4::from_cols_array(&[
        col0.x, col1.x, col2.x, 0.0,
        col0.y, col1.y, col2.y, 0.0,
        col0.z, col1.z, col2.z, 0.0,
        nt.x, nt.y, nt.z, 1.0,
    ])
}

/// Sets the rotation component (upper-left 3x3) of a matrix.
/// Maps to CesiumJS `Matrix4.setRotation`
pub fn set_rotation(matrix: &DMat4, rotation: &DMat3) -> DMat4 {
    DMat4::from_cols_array(&[
        rotation.x_axis.x, rotation.x_axis.y, rotation.x_axis.z, matrix.x_axis.w,
        rotation.y_axis.x, rotation.y_axis.y, rotation.y_axis.z, matrix.y_axis.w,
        rotation.z_axis.x, rotation.z_axis.y, rotation.z_axis.z, matrix.z_axis.w,
        matrix.w_axis.x, matrix.w_axis.y, matrix.w_axis.z, matrix.w_axis.w,
    ])
}

/// Sets the translation component of a matrix.
/// Maps to CesiumJS `Matrix4.setTranslation`
pub fn set_translation(matrix: &DMat4, translation: DVec3) -> DMat4 {
    DMat4::from_cols_array(&[
        matrix.x_axis.x, matrix.x_axis.y, matrix.x_axis.z, matrix.x_axis.w,
        matrix.y_axis.x, matrix.y_axis.y, matrix.y_axis.z, matrix.y_axis.w,
        matrix.z_axis.x, matrix.z_axis.y, matrix.z_axis.z, matrix.z_axis.w,
        translation.x, translation.y, translation.z, matrix.w_axis.w,
    ])
}

/// Sets the scale component of a matrix (replaces upper-left 3x3 column magnitudes).
/// Maps to CesiumJS `Matrix4.setScale`
pub fn set_scale(matrix: &DMat4, scale: DVec3) -> DMat4 {
    let current = get_scale(matrix);
    let sx = scale.x / current.x;
    let sy = scale.y / current.y;
    let sz = scale.z / current.z;
    DMat4::from_cols_array(&[
        matrix.x_axis.x * sx, matrix.x_axis.y * sx, matrix.x_axis.z * sx, matrix.x_axis.w,
        matrix.y_axis.x * sy, matrix.y_axis.y * sy, matrix.y_axis.z * sy, matrix.y_axis.w,
        matrix.z_axis.x * sz, matrix.z_axis.y * sz, matrix.z_axis.z * sz, matrix.z_axis.w,
        matrix.w_axis.x, matrix.w_axis.y, matrix.w_axis.z, matrix.w_axis.w,
    ])
}

/// Computes an orthographic projection matrix.
/// Maps to CesiumJS `Matrix4.computeOrthographicOffCenter`
pub fn compute_orthographic_off_center(
    left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64,
) -> DMat4 {
    let mut a = 1.0 / (right - left);
    let mut b = 1.0 / (top - bottom);
    let mut c = 1.0 / (far - near);
    let tx = -(right + left) * a;
    let ty = -(top + bottom) * b;
    let tz = -(far + near) * c;
    a *= 2.0;
    b *= 2.0;
    c *= -2.0;

    DMat4::from_cols_array(&[
        a, 0.0, 0.0, 0.0,
        0.0, b, 0.0, 0.0,
        0.0, 0.0, c, 0.0,
        tx, ty, tz, 1.0,
    ])
}

/// Computes a perspective projection matrix from off-center parameters.
/// Maps to CesiumJS `Matrix4.computePerspectiveOffCenter`
pub fn compute_perspective_off_center(
    left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64,
) -> DMat4 {
    let column0_row0 = (2.0 * near) / (right - left);
    let column1_row1 = (2.0 * near) / (top - bottom);
    let column2_row0 = (right + left) / (right - left);
    let column2_row1 = (top + bottom) / (top - bottom);
    let column2_row2 = -(far + near) / (far - near);
    let column2_row3 = -1.0;
    let column3_row2 = (-2.0 * far * near) / (far - near);

    DMat4::from_cols_array(&[
        column0_row0, 0.0, 0.0, 0.0,
        0.0, column1_row1, 0.0, 0.0,
        column2_row0, column2_row1, column2_row2, column2_row3,
        0.0, 0.0, column3_row2, 0.0,
    ])
}

/// Computes an infinite perspective projection matrix.
/// Maps to CesiumJS `Matrix4.computeInfinitePerspectiveOffCenter`
pub fn compute_infinite_perspective_off_center(
    left: f64, right: f64, bottom: f64, top: f64, near: f64,
) -> DMat4 {
    let column0_row0 = (2.0 * near) / (right - left);
    let column1_row1 = (2.0 * near) / (top - bottom);
    let column2_row0 = (right + left) / (right - left);
    let column2_row1 = (top + bottom) / (top - bottom);
    let column2_row2 = -1.0;
    let column2_row3 = -1.0;
    let column3_row2 = -2.0 * near;

    DMat4::from_cols_array(&[
        column0_row0, 0.0, 0.0, 0.0,
        0.0, column1_row1, 0.0, 0.0,
        column2_row0, column2_row1, column2_row2, column2_row3,
        0.0, 0.0, column3_row2, 0.0,
    ])
}

/// Computes a viewport transformation matrix.
/// Maps to CesiumJS `Matrix4.computeViewportTransformation`
pub fn compute_viewport_transformation(
    viewport_x: f64, viewport_y: f64,
    viewport_width: f64, viewport_height: f64,
    near_depth_range: f64, far_depth_range: f64,
) -> DMat4 {
    let half_width = viewport_width * 0.5;
    let half_height = viewport_height * 0.5;
    let half_depth = (far_depth_range - near_depth_range) * 0.5;

    let column0_row0 = half_width;
    let column1_row1 = half_height;
    let column2_row2 = half_depth;
    let column3_row0 = viewport_x + half_width;
    let column3_row1 = viewport_y + half_height;
    let column3_row2 = near_depth_range + half_depth;

    DMat4::from_cols_array(&[
        column0_row0, 0.0, 0.0, 0.0,
        0.0, column1_row1, 0.0, 0.0,
        0.0, 0.0, column2_row2, 0.0,
        column3_row0, column3_row1, column3_row2, 1.0,
    ])
}

/// Computes the element-wise absolute value of a matrix.
/// Maps to CesiumJS `Matrix4.abs`
pub fn abs(matrix: &DMat4) -> DMat4 {
    let cols = matrix.to_cols_array();
    let mut out = [0.0f64; 16];
    for i in 0..16 {
        out[i] = cols[i].abs();
    }
    DMat4::from_cols_array(&out)
}
