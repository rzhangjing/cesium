//! Ported from packages/engine/Source/Core/Matrix2.js
//!
//! A 2x2 matrix, stored in column-major order:
//! ```text
//!   [0] [2]     column0Row0  column1Row0
//!   [1] [3]  =  column0Row1  column1Row1
//! ```

use crate::cartesian2::Cartesian2;

/// A 2x2 matrix in column-major order.
///
/// Port of `Matrix2`.
#[derive(Clone, Copy, Debug)]
pub struct Matrix2 {
    /// Column-major storage: `[column0Row0, column0Row1, column1Row0, column1Row1]`.
    pub elements: [f64; 4],
}

impl Default for Matrix2 {
    fn default() -> Self {
        Self {
            elements: [0.0; 4],
        }
    }
}

impl Matrix2 {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 4;

    /// An immutable `Matrix2` initialized to the identity matrix.
    pub const IDENTITY: Matrix2 = Matrix2 {
        elements: [1.0, 0.0, 0.0, 1.0],
    };

    /// An immutable `Matrix2` initialized to the zero matrix.
    pub const ZERO: Matrix2 = Matrix2 {
        elements: [0.0; 4],
    };

    /// Index constants (column-major).
    pub const COLUMN0ROW0: usize = 0;
    pub const COLUMN0ROW1: usize = 1;
    pub const COLUMN1ROW0: usize = 2;
    pub const COLUMN1ROW1: usize = 3;

    /// Creates a `Matrix2` from individual elements (column-major order).
    ///
    /// Port of the `Matrix2(column0Row0, column1Row0, column0Row1, column1Row1)` constructor.
    pub fn new(column0_row0: f64, column1_row0: f64, column0_row1: f64, column1_row1: f64) -> Self {
        Self {
            elements: [column0_row0, column0_row1, column1_row0, column1_row1],
        }
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Port of `Matrix2.pack`.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.elements[0];
        array[starting_index + 1] = value.elements[1];
        array[starting_index + 2] = value.elements[2];
        array[starting_index + 3] = value.elements[3];
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Port of `Matrix2.unpack`.
    pub fn unpack(array: &[f64], starting_index: usize, result: &mut Self) {
        result.elements[0] = array[starting_index];
        result.elements[1] = array[starting_index + 1];
        result.elements[2] = array[starting_index + 2];
        result.elements[3] = array[starting_index + 3];
    }

    /// Allocating variant of [`Matrix2::unpack`].
    pub fn unpack_new(array: &[f64], starting_index: usize) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    /// Alias for [`Matrix2::unpack`].
    ///
    /// Port of `Matrix2.fromArray` (= `Matrix2.unpack`).
    pub fn from_array(array: &[f64], starting_index: usize, result: &mut Self) {
        Self::unpack(array, starting_index, result);
    }

    /// Allocating variant of [`Matrix2::from_array`].
    pub fn from_array_new(array: &[f64], starting_index: usize) -> Self {
        Self::unpack_new(array, starting_index)
    }

    /// Creates a `Matrix2` from a column-major order array.
    ///
    /// Port of `Matrix2.fromColumnMajorArray`.
    pub fn from_column_major_array(values: &[f64], result: &mut Self) {
        result.elements[0] = values[0];
        result.elements[1] = values[1];
        result.elements[2] = values[2];
        result.elements[3] = values[3];
    }

    /// Allocating variant of [`Matrix2::from_column_major_array`].
    pub fn from_column_major_array_new(values: &[f64]) -> Self {
        let mut result = Self::default();
        Self::from_column_major_array(values, &mut result);
        result
    }

    /// Creates a `Matrix2` from a row-major order array.
    ///
    /// Port of `Matrix2.fromRowMajorArray`.
    pub fn from_row_major_array(values: &[f64], result: &mut Self) {
        result.elements[0] = values[0];
        result.elements[1] = values[2];
        result.elements[2] = values[1];
        result.elements[3] = values[3];
    }

    /// Allocating variant of [`Matrix2::from_row_major_array`].
    pub fn from_row_major_array_new(values: &[f64]) -> Self {
        let mut result = Self::default();
        Self::from_row_major_array(values, &mut result);
        result
    }

    /// Computes a `Matrix2` representing a non-uniform scale.
    ///
    /// Port of `Matrix2.fromScale`.
    pub fn from_scale(scale: &Cartesian2, result: &mut Self) {
        result.elements[0] = scale.x;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = scale.y;
    }

    /// Allocating variant of [`Matrix2::from_scale`].
    pub fn from_scale_new(scale: &Cartesian2) -> Self {
        let mut result = Self::default();
        Self::from_scale(scale, &mut result);
        result
    }

    /// Computes a `Matrix2` representing a uniform scale.
    ///
    /// Port of `Matrix2.fromUniformScale`.
    pub fn from_uniform_scale(scale: f64, result: &mut Self) {
        result.elements[0] = scale;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = scale;
    }

    /// Allocating variant of [`Matrix2::from_uniform_scale`].
    pub fn from_uniform_scale_new(scale: f64) -> Self {
        let mut result = Self::default();
        Self::from_uniform_scale(scale, &mut result);
        result
    }

    /// Creates a rotation matrix.
    ///
    /// Port of `Matrix2.fromRotation`.
    pub fn from_rotation(angle: f64, result: &mut Self) {
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        result.elements[0] = cos_angle;
        result.elements[1] = sin_angle;
        result.elements[2] = -sin_angle;
        result.elements[3] = cos_angle;
    }

    /// Allocating variant of [`Matrix2::from_rotation`].
    pub fn from_rotation_new(angle: f64) -> Self {
        let mut result = Self::default();
        Self::from_rotation(angle, &mut result);
        result
    }

    /// Creates an array from the provided `Matrix2`.
    ///
    /// Port of `Matrix2.toArray`.
    pub fn to_array(matrix: &Self, result: &mut [f64]) {
        result[0] = matrix.elements[0];
        result[1] = matrix.elements[1];
        result[2] = matrix.elements[2];
        result[3] = matrix.elements[3];
    }

    /// Allocating variant of [`Matrix2::to_array`].
    pub fn to_array_new(matrix: &Self) -> [f64; 4] {
        let mut result = [0.0; 4];
        Self::to_array(matrix, &mut result);
        result
    }

    /// Computes the array index of the element at the provided row and column.
    ///
    /// Port of `Matrix2.getElementIndex`.
    pub fn get_element_index(column: usize, row: usize) -> usize {
        column * 2 + row
    }

    /// Retrieves a column at the provided index as a `Cartesian2`.
    ///
    /// Port of `Matrix2.getColumn`.
    pub fn get_column(matrix: &Self, index: usize, result: &mut Cartesian2) {
        let start = index * 2;
        result.x = matrix.elements[start];
        result.y = matrix.elements[start + 1];
    }

    /// Allocating variant of [`Matrix2::get_column`].
    pub fn get_column_new(matrix: &Self, index: usize) -> Cartesian2 {
        let mut result = Cartesian2::default();
        Self::get_column(matrix, index, &mut result);
        result
    }

    /// Replaces the specified column in the provided matrix.
    ///
    /// Port of `Matrix2.setColumn`.
    pub fn set_column(matrix: &Self, index: usize, cartesian: &Cartesian2, result: &mut Self) {
        *result = *matrix;
        let start = index * 2;
        result.elements[start] = cartesian.x;
        result.elements[start + 1] = cartesian.y;
    }

    /// Retrieves a row at the provided index as a `Cartesian2`.
    ///
    /// Port of `Matrix2.getRow`.
    pub fn get_row(matrix: &Self, index: usize, result: &mut Cartesian2) {
        result.x = matrix.elements[index];
        result.y = matrix.elements[index + 2];
    }

    /// Allocating variant of [`Matrix2::get_row`].
    pub fn get_row_new(matrix: &Self, index: usize) -> Cartesian2 {
        let mut result = Cartesian2::default();
        Self::get_row(matrix, index, &mut result);
        result
    }

    /// Replaces the specified row in the provided matrix.
    ///
    /// Port of `Matrix2.setRow`.
    pub fn set_row(matrix: &Self, index: usize, cartesian: &Cartesian2, result: &mut Self) {
        *result = *matrix;
        result.elements[index] = cartesian.x;
        result.elements[index + 2] = cartesian.y;
    }

    /// Extracts the non-uniform scale assuming the matrix is an affine
    /// transformation.
    ///
    /// Port of `Matrix2.getScale`.
    pub fn get_scale(matrix: &Self, result: &mut Cartesian2) {
        let col0 = Cartesian2::new(matrix.elements[0], matrix.elements[1]);
        let col1 = Cartesian2::new(matrix.elements[2], matrix.elements[3]);
        result.x = Cartesian2::magnitude(&col0);
        result.y = Cartesian2::magnitude(&col1);
    }

    /// Allocating variant of [`Matrix2::get_scale`].
    pub fn get_scale_new(matrix: &Self) -> Cartesian2 {
        let mut result = Cartesian2::default();
        Self::get_scale(matrix, &mut result);
        result
    }

    /// Computes the maximum scale assuming the matrix is an affine
    /// transformation.
    ///
    /// Port of `Matrix2.getMaximumScale`.
    pub fn get_maximum_scale(matrix: &Self) -> f64 {
        let scale = Self::get_scale_new(matrix);
        Cartesian2::maximum_component(&scale)
    }

    /// Computes a new matrix that replaces the scale with the provided scale.
    ///
    /// Port of `Matrix2.setScale`.
    pub fn set_scale(matrix: &Self, scale: &Cartesian2, result: &mut Self) {
        let existing_scale = Self::get_scale_new(matrix);
        let ratio_x = scale.x / existing_scale.x;
        let ratio_y = scale.y / existing_scale.y;

        result.elements[0] = matrix.elements[0] * ratio_x;
        result.elements[1] = matrix.elements[1] * ratio_x;
        result.elements[2] = matrix.elements[2] * ratio_y;
        result.elements[3] = matrix.elements[3] * ratio_y;
    }

    /// Computes a new matrix that replaces the scale with a uniform scale.
    ///
    /// Port of `Matrix2.setUniformScale`.
    pub fn set_uniform_scale(matrix: &Self, scale: f64, result: &mut Self) {
        let existing_scale = Self::get_scale_new(matrix);
        let ratio_x = scale / existing_scale.x;
        let ratio_y = scale / existing_scale.y;

        result.elements[0] = matrix.elements[0] * ratio_x;
        result.elements[1] = matrix.elements[1] * ratio_x;
        result.elements[2] = matrix.elements[2] * ratio_y;
        result.elements[3] = matrix.elements[3] * ratio_y;
    }

    /// Sets the rotation assuming the matrix is an affine transformation.
    ///
    /// Port of `Matrix2.setRotation`.
    pub fn set_rotation(matrix: &Self, rotation: &Self, result: &mut Self) {
        let scale = Self::get_scale_new(matrix);

        result.elements[0] = rotation.elements[0] * scale.x;
        result.elements[1] = rotation.elements[1] * scale.x;
        result.elements[2] = rotation.elements[2] * scale.y;
        result.elements[3] = rotation.elements[3] * scale.y;
    }

    /// Extracts the rotation matrix.
    ///
    /// Port of `Matrix2.getRotation`.
    pub fn get_rotation(matrix: &Self, result: &mut Self) {
        let scale = Self::get_scale_new(matrix);

        result.elements[0] = matrix.elements[0] / scale.x;
        result.elements[1] = matrix.elements[1] / scale.x;
        result.elements[2] = matrix.elements[2] / scale.y;
        result.elements[3] = matrix.elements[3] / scale.y;
    }

    /// Allocating variant of [`Matrix2::get_rotation`].
    pub fn get_rotation_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::get_rotation(matrix, &mut result);
        result
    }

    /// Computes the product of two matrices.
    ///
    /// Port of `Matrix2.multiply`.
    pub fn multiply(left: &Self, right: &Self, result: &mut Self) {
        let col0_row0 = left.elements[0] * right.elements[0] + left.elements[2] * right.elements[1];
        let col1_row0 = left.elements[0] * right.elements[2] + left.elements[2] * right.elements[3];
        let col0_row1 = left.elements[1] * right.elements[0] + left.elements[3] * right.elements[1];
        let col1_row1 = left.elements[1] * right.elements[2] + left.elements[3] * right.elements[3];

        result.elements[0] = col0_row0;
        result.elements[1] = col0_row1;
        result.elements[2] = col1_row0;
        result.elements[3] = col1_row1;
    }

    /// Allocating variant of [`Matrix2::multiply`].
    pub fn multiply_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply(left, right, &mut result);
        result
    }

    /// Computes the sum of two matrices.
    ///
    /// Port of `Matrix2.add`.
    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        result.elements[0] = left.elements[0] + right.elements[0];
        result.elements[1] = left.elements[1] + right.elements[1];
        result.elements[2] = left.elements[2] + right.elements[2];
        result.elements[3] = left.elements[3] + right.elements[3];
    }

    /// Allocating variant of [`Matrix2::add`].
    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    /// Computes the difference of two matrices.
    ///
    /// Port of `Matrix2.subtract`.
    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        result.elements[0] = left.elements[0] - right.elements[0];
        result.elements[1] = left.elements[1] - right.elements[1];
        result.elements[2] = left.elements[2] - right.elements[2];
        result.elements[3] = left.elements[3] - right.elements[3];
    }

    /// Allocating variant of [`Matrix2::subtract`].
    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    /// Computes the product of a matrix and a column vector.
    ///
    /// Port of `Matrix2.multiplyByVector`.
    pub fn multiply_by_vector(matrix: &Self, cartesian: &Cartesian2, result: &mut Cartesian2) {
        let x = matrix.elements[0] * cartesian.x + matrix.elements[2] * cartesian.y;
        let y = matrix.elements[1] * cartesian.x + matrix.elements[3] * cartesian.y;
        result.x = x;
        result.y = y;
    }

    /// Allocating variant of [`Matrix2::multiply_by_vector`].
    pub fn multiply_by_vector_new(matrix: &Self, cartesian: &Cartesian2) -> Cartesian2 {
        let mut result = Cartesian2::default();
        Self::multiply_by_vector(matrix, cartesian, &mut result);
        result
    }

    /// Computes the product of a matrix and a scalar.
    ///
    /// Port of `Matrix2.multiplyByScalar`.
    pub fn multiply_by_scalar(matrix: &Self, scalar: f64, result: &mut Self) {
        result.elements[0] = matrix.elements[0] * scalar;
        result.elements[1] = matrix.elements[1] * scalar;
        result.elements[2] = matrix.elements[2] * scalar;
        result.elements[3] = matrix.elements[3] * scalar;
    }

    /// Allocating variant of [`Matrix2::multiply_by_scalar`].
    pub fn multiply_by_scalar_new(matrix: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(matrix, scalar, &mut result);
        result
    }

    /// Computes the product of a matrix times a non-uniform scale.
    ///
    /// Port of `Matrix2.multiplyByScale`.
    pub fn multiply_by_scale(matrix: &Self, scale: &Cartesian2, result: &mut Self) {
        result.elements[0] = matrix.elements[0] * scale.x;
        result.elements[1] = matrix.elements[1] * scale.x;
        result.elements[2] = matrix.elements[2] * scale.y;
        result.elements[3] = matrix.elements[3] * scale.y;
    }

    /// Computes the product of a matrix times a uniform scale.
    ///
    /// Port of `Matrix2.multiplyByUniformScale`.
    pub fn multiply_by_uniform_scale(matrix: &Self, scale: f64, result: &mut Self) {
        result.elements[0] = matrix.elements[0] * scale;
        result.elements[1] = matrix.elements[1] * scale;
        result.elements[2] = matrix.elements[2] * scale;
        result.elements[3] = matrix.elements[3] * scale;
    }

    /// Creates a negated copy of the provided matrix.
    ///
    /// Port of `Matrix2.negate`.
    pub fn negate(matrix: &Self, result: &mut Self) {
        result.elements[0] = -matrix.elements[0];
        result.elements[1] = -matrix.elements[1];
        result.elements[2] = -matrix.elements[2];
        result.elements[3] = -matrix.elements[3];
    }

    /// Allocating variant of [`Matrix2::negate`].
    pub fn negate_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(matrix, &mut result);
        result
    }

    /// Computes the transpose of the provided matrix.
    ///
    /// Port of `Matrix2.transpose`.
    pub fn transpose(matrix: &Self, result: &mut Self) {
        let col0_row0 = matrix.elements[0];
        let col0_row1 = matrix.elements[2];
        let col1_row0 = matrix.elements[1];
        let col1_row1 = matrix.elements[3];

        result.elements[0] = col0_row0;
        result.elements[1] = col0_row1;
        result.elements[2] = col1_row0;
        result.elements[3] = col1_row1;
    }

    /// Allocating variant of [`Matrix2::transpose`].
    pub fn transpose_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::transpose(matrix, &mut result);
        result
    }

    /// Computes a matrix with absolute values of the provided matrix's elements.
    ///
    /// Port of `Matrix2.abs`.
    pub fn abs(matrix: &Self, result: &mut Self) {
        result.elements[0] = matrix.elements[0].abs();
        result.elements[1] = matrix.elements[1].abs();
        result.elements[2] = matrix.elements[2].abs();
        result.elements[3] = matrix.elements[3].abs();
    }

    /// Allocating variant of [`Matrix2::abs`].
    pub fn abs_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::abs(matrix, &mut result);
        result
    }

    /// Duplicates a `Matrix2` instance.
    ///
    /// Port of `Matrix2.clone`.
    pub fn clone(matrix: &Self, result: &mut Self) {
        result.elements = matrix.elements;
    }

    /// Allocating variant of [`Matrix2::clone`].
    pub fn clone_new(matrix: &Self) -> Self {
        Self {
            elements: matrix.elements,
        }
    }

    /// Compares two matrices componentwise.
    ///
    /// Port of `Matrix2.equals`.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.elements == right.elements
    }

    /// Compares two matrices componentwise within epsilon.
    ///
    /// Port of `Matrix2.equalsEpsilon`.
    pub fn equals_epsilon(left: &Self, right: &Self, epsilon: f64) -> bool {
        (left.elements[0] - right.elements[0]).abs() <= epsilon
            && (left.elements[1] - right.elements[1]).abs() <= epsilon
            && (left.elements[2] - right.elements[2]).abs() <= epsilon
            && (left.elements[3] - right.elements[3]).abs() <= epsilon
    }
}

impl PartialEq for Matrix2 {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}

impl std::fmt::Display for Matrix2 {
    /// Port of `Matrix2.prototype.toString`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {})\n({}, {})",
            self.elements[0], self.elements[2], self.elements[1], self.elements[3]
        )
    }
}
