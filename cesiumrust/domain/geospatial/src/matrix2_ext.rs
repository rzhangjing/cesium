//! CesiumJS `Matrix2.js` faithful port — 2×2 matrix as column-major `[f64; 4]`.
//!
//! Layout (column-major): `[col0row0, col0row1, col1row0, col1row1]`
//! i.e. index = column * 2 + row.

use glam::DVec2;

/// Packed length of a Matrix2.
pub const PACKED_LENGTH: usize = 4;

/// Identity matrix.
pub const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 1.0];

/// Zero matrix.
pub const ZERO: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

// ---------------------------------------------------------------------------
// Pack / Unpack
// ---------------------------------------------------------------------------

/// Pack a Matrix2 into `array` starting at `starting_index`.
pub fn pack(value: &[f64; 4], array: &mut [f64], starting_index: usize) {
    array[starting_index] = value[0];
    array[starting_index + 1] = value[1];
    array[starting_index + 2] = value[2];
    array[starting_index + 3] = value[3];
}

/// Unpack a Matrix2 from `array` starting at `starting_index`.
pub fn unpack(array: &[f64], starting_index: usize) -> [f64; 4] {
    [
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
        array[starting_index + 3],
    ]
}

/// Alias for `unpack`.
pub fn from_array(array: &[f64], starting_index: usize) -> [f64; 4] {
    unpack(array, starting_index)
}

/// Pack an array of Matrix2 values into a flat array.
pub fn pack_array(array: &[[f64; 4]], result: &mut Vec<f64>) {
    result.resize(array.len() * 4, 0.0);
    for (i, m) in array.iter().enumerate() {
        pack(m, result, i * 4);
    }
}

/// Unpack a flat array into an array of Matrix2 values.
pub fn unpack_array(array: &[f64]) -> Vec<[f64; 4]> {
    let count = array.len() / 4;
    (0..count).map(|i| unpack(array, i * 4)).collect()
}

// ---------------------------------------------------------------------------
// Construction
// ---------------------------------------------------------------------------

/// Create from column-major values: `[col0row0, col0row1, col1row0, col1row1]`.
pub fn from_column_major_array(values: &[f64]) -> [f64; 4] {
    [values[0], values[1], values[2], values[3]]
}

/// Create from row-major values: `[row0col0, row0col1, row1col0, row1col1]`.
pub fn from_row_major_array(values: &[f64]) -> [f64; 4] {
    // row-major: [r0c0, r0c1, r1c0, r1c1]
    // column-major: [r0c0, r1c0, r0c1, r1c1]
    [values[0], values[2], values[1], values[3]]
}

/// Create a scale matrix from a non-uniform scale.
pub fn from_scale(scale: DVec2) -> [f64; 4] {
    [scale.x, 0.0, 0.0, scale.y]
}

/// Create a uniform scale matrix.
pub fn from_uniform_scale(scale: f64) -> [f64; 4] {
    [scale, 0.0, 0.0, scale]
}

/// Create a 2D rotation matrix from an angle in radians (counter-clockwise).
pub fn from_rotation(angle: f64) -> [f64; 4] {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    // Column-major: col0 = (cos, sin), col1 = (-sin, cos)
    [cos_a, sin_a, -sin_a, cos_a]
}

// ---------------------------------------------------------------------------
// Element access
// ---------------------------------------------------------------------------

/// Get the flat index for a given (column, row).
pub fn get_element_index(column: usize, row: usize) -> usize {
    column * 2 + row
}

/// Get a column as a Cartesian2.
pub fn get_column(matrix: &[f64; 4], index: usize) -> DVec2 {
    let start = index * 2;
    DVec2::new(matrix[start], matrix[start + 1])
}

/// Set a column from a Cartesian2.
pub fn set_column(matrix: &[f64; 4], index: usize, cartesian: DVec2) -> [f64; 4] {
    let mut result = *matrix;
    let start = index * 2;
    result[start] = cartesian.x;
    result[start + 1] = cartesian.y;
    result
}

/// Get a row as a Cartesian2.
pub fn get_row(matrix: &[f64; 4], index: usize) -> DVec2 {
    DVec2::new(matrix[index], matrix[index + 2])
}

/// Set a row from a Cartesian2.
pub fn set_row(matrix: &[f64; 4], index: usize, cartesian: DVec2) -> [f64; 4] {
    let mut result = *matrix;
    result[index] = cartesian.x;
    result[index + 2] = cartesian.y;
    result
}

// ---------------------------------------------------------------------------
// Scale / Rotation extraction
// ---------------------------------------------------------------------------

/// Set the scale of a matrix, preserving rotation.
pub fn set_scale(matrix: &[f64; 4], scale: DVec2) -> [f64; 4] {
    let mut result = *matrix;
    // Scale column 0
    let col0_len = (matrix[0] * matrix[0] + matrix[1] * matrix[1]).sqrt();
    if col0_len > 0.0 {
        result[0] = matrix[0] / col0_len * scale.x;
        result[1] = matrix[1] / col0_len * scale.x;
    }
    // Scale column 1
    let col1_len = (matrix[2] * matrix[2] + matrix[3] * matrix[3]).sqrt();
    if col1_len > 0.0 {
        result[2] = matrix[2] / col1_len * scale.y;
        result[3] = matrix[3] / col1_len * scale.y;
    }
    result
}

/// Set a uniform scale, preserving rotation.
pub fn set_uniform_scale(matrix: &[f64; 4], scale: f64) -> [f64; 4] {
    set_scale(matrix, DVec2::splat(scale))
}

/// Get the scale from a matrix.
pub fn get_scale(matrix: &[f64; 4]) -> DVec2 {
    let sx = (matrix[0] * matrix[0] + matrix[1] * matrix[1]).sqrt();
    let sy = (matrix[2] * matrix[2] + matrix[3] * matrix[3]).sqrt();
    DVec2::new(sx, sy)
}

/// Get the maximum scale component.
pub fn get_maximum_scale(matrix: &[f64; 4]) -> f64 {
    let s = get_scale(matrix);
    s.x.max(s.y)
}

/// Set the rotation of a matrix, preserving scale.
pub fn set_rotation(matrix: &[f64; 4], rotation: &[f64; 4]) -> [f64; 4] {
    let scale = get_scale(matrix);
    [
        rotation[0] * scale.x,
        rotation[1] * scale.x,
        rotation[2] * scale.y,
        rotation[3] * scale.y,
    ]
}

/// Extract the rotation (removing scale) from a matrix.
pub fn get_rotation(matrix: &[f64; 4]) -> [f64; 4] {
    let scale = get_scale(matrix);
    let sx = if scale.x > 0.0 { scale.x } else { 1.0 };
    let sy = if scale.y > 0.0 { scale.y } else { 1.0 };
    [
        matrix[0] / sx,
        matrix[1] / sx,
        matrix[2] / sy,
        matrix[3] / sy,
    ]
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Multiply two 2×2 matrices: `left * right`.
pub fn multiply(left: &[f64; 4], right: &[f64; 4]) -> [f64; 4] {
    // Column-major: result[col*2+row] = sum_k left[k*2+row] * right[col*2+k]
    [
        left[0] * right[0] + left[2] * right[1],
        left[1] * right[0] + left[3] * right[1],
        left[0] * right[2] + left[2] * right[3],
        left[1] * right[2] + left[3] * right[3],
    ]
}

/// Add two matrices element-wise.
pub fn add(left: &[f64; 4], right: &[f64; 4]) -> [f64; 4] {
    [
        left[0] + right[0],
        left[1] + right[1],
        left[2] + right[2],
        left[3] + right[3],
    ]
}

/// Subtract two matrices element-wise.
pub fn subtract(left: &[f64; 4], right: &[f64; 4]) -> [f64; 4] {
    [
        left[0] - right[0],
        left[1] - right[1],
        left[2] - right[2],
        left[3] - right[3],
    ]
}

/// Multiply a matrix by a column vector.
pub fn multiply_by_vector(matrix: &[f64; 4], cartesian: DVec2) -> DVec2 {
    DVec2::new(
        matrix[0] * cartesian.x + matrix[2] * cartesian.y,
        matrix[1] * cartesian.x + matrix[3] * cartesian.y,
    )
}

/// Multiply a matrix by a scalar.
pub fn multiply_by_scalar(matrix: &[f64; 4], scalar: f64) -> [f64; 4] {
    [
        matrix[0] * scalar,
        matrix[1] * scalar,
        matrix[2] * scalar,
        matrix[3] * scalar,
    ]
}

/// Multiply a matrix by a non-uniform scale (column-wise).
pub fn multiply_by_scale(matrix: &[f64; 4], scale: DVec2) -> [f64; 4] {
    [
        matrix[0] * scale.x,
        matrix[1] * scale.x,
        matrix[2] * scale.y,
        matrix[3] * scale.y,
    ]
}

/// Multiply a matrix by a uniform scale.
pub fn multiply_by_uniform_scale(matrix: &[f64; 4], scale: f64) -> [f64; 4] {
    multiply_by_scalar(matrix, scale)
}

/// Negate all elements.
pub fn negate(matrix: &[f64; 4]) -> [f64; 4] {
    [-matrix[0], -matrix[1], -matrix[2], -matrix[3]]
}

/// Transpose the matrix.
pub fn transpose(matrix: &[f64; 4]) -> [f64; 4] {
    [matrix[0], matrix[2], matrix[1], matrix[3]]
}

/// Absolute value of all elements.
pub fn abs(matrix: &[f64; 4]) -> [f64; 4] {
    [
        matrix[0].abs(),
        matrix[1].abs(),
        matrix[2].abs(),
        matrix[3].abs(),
    ]
}

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Exact equality.
pub fn equals(left: &[f64; 4], right: &[f64; 4]) -> bool {
    left[0] == right[0] && left[1] == right[1] && left[2] == right[2] && left[3] == right[3]
}

/// Check if matrix elements equal array elements at offset.
pub fn equals_array(matrix: &[f64; 4], array: &[f64], offset: usize) -> bool {
    matrix[0] == array[offset]
        && matrix[1] == array[offset + 1]
        && matrix[2] == array[offset + 2]
        && matrix[3] == array[offset + 3]
}

/// Epsilon equality.
pub fn equals_epsilon(left: &[f64; 4], right: &[f64; 4], epsilon: f64) -> bool {
    (left[0] - right[0]).abs() <= epsilon
        && (left[1] - right[1]).abs() <= epsilon
        && (left[2] - right[2]).abs() <= epsilon
        && (left[3] - right[3]).abs() <= epsilon
}
