//! Matrix3 CesiumJS extension functions.
//! Maps to CesiumJS `Core/Matrix3.js` static methods that go beyond basic matrix math (glam).

use glam::{DMat3, DQuat, DVec3};

/// The packed length of a Matrix3: 9.
pub const PACKED_LENGTH: usize = 9;

/// Packs a Matrix3 (column-major) into an array at the given starting index.
/// Maps to CesiumJS `Matrix3.pack`
pub fn pack(value: &DMat3, array: &mut [f64], starting_index: usize) {
    let cols = value.to_cols_array();
    for i in 0..9 {
        array[starting_index + i] = cols[i];
    }
}

/// Unpacks a Matrix3 from a column-major array at the given starting index.
/// Maps to CesiumJS `Matrix3.unpack`
pub fn unpack(array: &[f64], starting_index: usize) -> DMat3 {
    let mut cols = [0.0f64; 9];
    for i in 0..9 {
        cols[i] = array[starting_index + i];
    }
    DMat3::from_cols_array(&cols)
}

/// Creates a Matrix3 from a column-major array at an offset.
/// Maps to CesiumJS `Matrix3.fromColumnMajorArray`
pub fn from_column_major_array(array: &[f64], starting_index: usize) -> DMat3 {
    unpack(array, starting_index)
}

/// Creates a Matrix3 from a row-major array.
/// Maps to CesiumJS `Matrix3.fromRowMajorArray`
pub fn from_row_major_array(array: &[f64]) -> DMat3 {
    // Row-major: array[row*3+col] → column-major DMat3[col][row]
    DMat3::from_cols_array(&[
        array[0], array[3], array[6], // column 0
        array[1], array[4], array[7], // column 1
        array[2], array[5], array[8], // column 2
    ])
}

/// Computes a 3x3 rotation matrix from a quaternion.
/// Maps to CesiumJS `Matrix3.fromQuaternion`
pub fn from_quaternion(quaternion: DQuat) -> DMat3 {
    let x2 = quaternion.x + quaternion.x;
    let y2 = quaternion.y + quaternion.y;
    let z2 = quaternion.z + quaternion.z;

    let xx2 = x2 * quaternion.x;
    let yy2 = y2 * quaternion.y;
    let zz2 = z2 * quaternion.z;
    let xy2 = x2 * quaternion.y;
    let xz2 = x2 * quaternion.z;
    let yz2 = y2 * quaternion.z;
    let wx2 = x2 * quaternion.w;
    let wy2 = y2 * quaternion.w;
    let wz2 = z2 * quaternion.w;

    DMat3::from_cols_array(&[
        1.0 - yy2 - zz2, xy2 + wz2, xz2 - wy2, // column 0
        xy2 - wz2, 1.0 - xx2 - zz2, yz2 + wx2, // column 1
        xz2 + wy2, yz2 - wx2, 1.0 - xx2 - yy2, // column 2
    ])
}

/// Computes a 3x3 rotation matrix from an angle around the X axis.
/// Maps to CesiumJS `Matrix3.fromRotationX`
pub fn from_rotation_x(angle: f64) -> DMat3 {
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();
    DMat3::from_cols_array(&[
        1.0, 0.0, 0.0,
        0.0, cos_angle, sin_angle,
        0.0, -sin_angle, cos_angle,
    ])
}

/// Computes a 3x3 rotation matrix from an angle around the Y axis.
/// Maps to CesiumJS `Matrix3.fromRotationY`
pub fn from_rotation_y(angle: f64) -> DMat3 {
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();
    DMat3::from_cols_array(&[
        cos_angle, 0.0, -sin_angle,
        0.0, 1.0, 0.0,
        sin_angle, 0.0, cos_angle,
    ])
}

/// Computes a 3x3 rotation matrix from an angle around the Z axis.
/// Maps to CesiumJS `Matrix3.fromRotationZ`
pub fn from_rotation_z(angle: f64) -> DMat3 {
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();
    DMat3::from_cols_array(&[
        cos_angle, sin_angle, 0.0,
        -sin_angle, cos_angle, 0.0,
        0.0, 0.0, 1.0,
    ])
}

/// Computes a 3x3 scale matrix from a DVec3 scale.
/// Maps to CesiumJS `Matrix3.fromScale`
pub fn from_scale(scale: DVec3) -> DMat3 {
    DMat3::from_cols_array(&[
        scale.x, 0.0, 0.0,
        0.0, scale.y, 0.0,
        0.0, 0.0, scale.z,
    ])
}

/// Computes a 3x3 uniform scale matrix.
/// Maps to CesiumJS `Matrix3.fromUniformScale`
pub fn from_uniform_scale(scale: f64) -> DMat3 {
    DMat3::from_cols_array(&[
        scale, 0.0, 0.0,
        0.0, scale, 0.0,
        0.0, 0.0, scale,
    ])
}

/// Retrieves a copy of the matrix column at the provided index as a DVec3.
/// Maps to CesiumJS `Matrix3.getColumn`
pub fn get_column(matrix: &DMat3, index: usize) -> DVec3 {
    match index {
        0 => matrix.x_axis,
        1 => matrix.y_axis,
        2 => matrix.z_axis,
        _ => panic!("index must be 0, 1, or 2"),
    }
}

/// Retrieves a copy of the matrix row at the provided index as a DVec3.
/// Maps to CesiumJS `Matrix3.getRow`
pub fn get_row(matrix: &DMat3, index: usize) -> DVec3 {
    let cols = matrix.to_cols_array();
    DVec3::new(cols[index], cols[index + 3], cols[index + 6])
}

/// Computes the length of each column (scale) assuming the matrix is affine.
/// Maps to CesiumJS `Matrix3.getScale`
pub fn get_scale(matrix: &DMat3) -> DVec3 {
    DVec3::new(
        matrix.x_axis.length(),
        matrix.y_axis.length(),
        matrix.z_axis.length(),
    )
}

/// Computes the maximum scale assuming the matrix is affine.
/// Maps to CesiumJS `Matrix3.getMaximumScale`
pub fn get_maximum_scale(matrix: &DMat3) -> f64 {
    let scale = get_scale(matrix);
    scale.x.max(scale.y).max(scale.z)
}

/// Extracts the rotation matrix assuming the matrix is affine (removes scale).
/// Maps to CesiumJS `Matrix3.getRotation`
pub fn get_rotation(matrix: &DMat3) -> DMat3 {
    let scale = get_scale(matrix);
    DMat3::from_cols(
        matrix.x_axis / scale.x,
        matrix.y_axis / scale.y,
        matrix.z_axis / scale.z,
    )
}

/// Sets the rotation assuming the matrix is affine (preserves scale).
/// Maps to CesiumJS `Matrix3.setRotation`
pub fn set_rotation(matrix: &DMat3, rotation: &DMat3) -> DMat3 {
    let scale = get_scale(matrix);
    DMat3::from_cols(
        rotation.x_axis * scale.x,
        rotation.y_axis * scale.y,
        rotation.z_axis * scale.z,
    )
}

/// Computes the sum of two matrices.
/// Maps to CesiumJS `Matrix3.add`
pub fn add(left: &DMat3, right: &DMat3) -> DMat3 {
    *left + *right
}

/// Computes the difference of two matrices.
/// Maps to CesiumJS `Matrix3.subtract`
pub fn subtract(left: &DMat3, right: &DMat3) -> DMat3 {
    *left - *right
}

/// Computes the element-wise absolute value of a matrix.
/// Maps to CesiumJS `Matrix3.abs`
pub fn abs(matrix: &DMat3) -> DMat3 {
    DMat3::from_cols(
        DVec3::new(matrix.x_axis.x.abs(), matrix.x_axis.y.abs(), matrix.x_axis.z.abs()),
        DVec3::new(matrix.y_axis.x.abs(), matrix.y_axis.y.abs(), matrix.y_axis.z.abs()),
        DVec3::new(matrix.z_axis.x.abs(), matrix.z_axis.y.abs(), matrix.z_axis.z.abs()),
    )
}

/// Returns true if left and right are equal within the provided epsilon.
/// Maps to CesiumJS `Matrix3.equalsEpsilon`
pub fn equals_epsilon(left: &DMat3, right: &DMat3, epsilon: f64) -> bool {
    let l = left.to_cols_array();
    let r = right.to_cols_array();
    for i in 0..9 {
        if (l[i] - r[i]).abs() > epsilon {
            return false;
        }
    }
    true
}

/// Creates a 2x2 rotation matrix from an angle (stored as [f64; 4] column-major).
/// Maps to CesiumJS `Matrix2.fromRotation`
pub fn matrix2_from_rotation(angle: f64) -> [f64; 4] {
    let cos_angle = angle.cos();
    let sin_angle = angle.sin();
    // Column-major: [col0row0, col0row1, col1row0, col1row1]
    [cos_angle, sin_angle, -sin_angle, cos_angle]
}

/// Creates a 2x2 scale matrix from a scalar (stored as [f64; 4] column-major).
/// Maps to CesiumJS `Matrix2.fromScale`
pub fn matrix2_from_scale(scale: f64) -> [f64; 4] {
    [scale, 0.0, 0.0, scale]
}

/// Packs a 2x2 matrix (column-major) into an array.
/// Maps to CesiumJS `Matrix2.pack`
pub fn matrix2_pack(value: &[f64; 4], array: &mut [f64], starting_index: usize) {
    array[starting_index] = value[0];
    array[starting_index + 1] = value[1];
    array[starting_index + 2] = value[2];
    array[starting_index + 3] = value[3];
}

/// Unpacks a 2x2 matrix from a column-major array.
/// Maps to CesiumJS `Matrix2.unpack`
pub fn matrix2_unpack(array: &[f64], starting_index: usize) -> [f64; 4] {
    [
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
        array[starting_index + 3],
    ]
}
