//! Ported from packages/engine/Source/Core/Matrix3.js
//!
//! A 3x3 matrix, stored in column-major order:
//! ```text
//!   [0] [3] [6]     column0Row0  column1Row0  column2Row0
//!   [1] [4] [7]  =  column0Row1  column1Row1  column2Row1
//!   [2] [5] [8]     column0Row2  column1Row2  column2Row2
//! ```

use crate::cartesian3::Cartesian3;
use crate::heading_pitch_roll::HeadingPitchRoll;
use crate::quaternion::Quaternion;

/// Result of eigen-decomposition.
pub struct EigenDecompositionResult {
    pub unitary: Matrix3,
    pub diagonal: Matrix3,
}

/// A 3x3 matrix in column-major order.
///
/// Port of `Matrix3`.
#[derive(Clone, Copy, Debug)]
pub struct Matrix3 {
    /// Column-major storage: `[col0row0, col0row1, col0row2, col1row0, col1row1, col1row2, col2row0, col2row1, col2row2]`.
    pub elements: [f64; 9],
}

impl Default for Matrix3 {
    fn default() -> Self {
        Self {
            elements: [0.0; 9],
        }
    }
}

impl Matrix3 {
    /// The number of elements used to pack the object into an array.
    pub const PACKED_LENGTH: usize = 9;

    /// An immutable `Matrix3` initialized to the identity matrix.
    pub const IDENTITY: Matrix3 = Matrix3 {
        elements: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };

    /// An immutable `Matrix3` initialized to the zero matrix.
    pub const ZERO: Matrix3 = Matrix3 {
        elements: [0.0; 9],
    };

    /// Index constants (column-major).
    pub const COLUMN0ROW0: usize = 0;
    pub const COLUMN0ROW1: usize = 1;
    pub const COLUMN0ROW2: usize = 2;
    pub const COLUMN1ROW0: usize = 3;
    pub const COLUMN1ROW1: usize = 4;
    pub const COLUMN1ROW2: usize = 5;
    pub const COLUMN2ROW0: usize = 6;
    pub const COLUMN2ROW1: usize = 7;
    pub const COLUMN2ROW2: usize = 8;

    /// Creates a `Matrix3` from individual elements (column-major order).
    ///
    /// Port of the `Matrix3(column0Row0, column1Row0, column2Row0, column0Row1, ...)` constructor.
    pub fn new(
        column0_row0: f64,
        column1_row0: f64,
        column2_row0: f64,
        column0_row1: f64,
        column1_row1: f64,
        column2_row1: f64,
        column0_row2: f64,
        column1_row2: f64,
        column2_row2: f64,
    ) -> Self {
        Self {
            elements: [
                column0_row0, column0_row1, column0_row2,
                column1_row0, column1_row1, column1_row2,
                column2_row0, column2_row1, column2_row2,
            ],
        }
    }

    /// Stores the provided instance into the provided array.
    ///
    /// Port of `Matrix3.pack`.
    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        for i in 0..9 {
            array[starting_index + i] = value.elements[i];
        }
    }

    /// Retrieves an instance from a packed array.
    ///
    /// Port of `Matrix3.unpack`.
    pub fn unpack(array: &[f64], starting_index: usize, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = array[starting_index + i];
        }
    }

    /// Allocating variant of [`Matrix3::unpack`].
    pub fn unpack_new(array: &[f64], starting_index: usize) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    /// Alias for [`Matrix3::unpack`].
    ///
    /// Port of `Matrix3.fromArray` (= `Matrix3.unpack`).
    pub fn from_array(array: &[f64], starting_index: usize, result: &mut Self) {
        Self::unpack(array, starting_index, result);
    }

    /// Allocating variant of [`Matrix3::from_array`].
    pub fn from_array_new(array: &[f64], starting_index: usize) -> Self {
        Self::unpack_new(array, starting_index)
    }

    /// Creates a `Matrix3` from a column-major order array.
    ///
    /// Port of `Matrix3.fromColumnMajorArray`.
    pub fn from_column_major_array(values: &[f64], result: &mut Self) {
        result.elements.copy_from_slice(&values[..9]);
    }

    /// Allocating variant of [`Matrix3::from_column_major_array`].
    pub fn from_column_major_array_new(values: &[f64]) -> Self {
        let mut result = Self::default();
        Self::from_column_major_array(values, &mut result);
        result
    }

    /// Creates a `Matrix3` from a row-major order array.
    ///
    /// Port of `Matrix3.fromRowMajorArray`.
    pub fn from_row_major_array(values: &[f64], result: &mut Self) {
        result.elements[0] = values[0];
        result.elements[1] = values[3];
        result.elements[2] = values[6];
        result.elements[3] = values[1];
        result.elements[4] = values[4];
        result.elements[5] = values[7];
        result.elements[6] = values[2];
        result.elements[7] = values[5];
        result.elements[8] = values[8];
    }

    /// Allocating variant of [`Matrix3::from_row_major_array`].
    pub fn from_row_major_array_new(values: &[f64]) -> Self {
        let mut result = Self::default();
        Self::from_row_major_array(values, &mut result);
        result
    }

    /// Port of `Matrix3.fromQuaternion`.
    pub fn from_quaternion(quaternion: &Quaternion, result: &mut Self) {
        let x2 = quaternion.x * quaternion.x;
        let xy = quaternion.x * quaternion.y;
        let xz = quaternion.x * quaternion.z;
        let xw = quaternion.x * quaternion.w;
        let y2 = quaternion.y * quaternion.y;
        let yz = quaternion.y * quaternion.z;
        let yw = quaternion.y * quaternion.w;
        let z2 = quaternion.z * quaternion.z;
        let zw = quaternion.z * quaternion.w;
        let w2 = quaternion.w * quaternion.w;

        let m00 = x2 - y2 - z2 + w2;
        let m01 = 2.0 * (xy - zw);
        let m02 = 2.0 * (xz + yw);
        let m10 = 2.0 * (xy + zw);
        let m11 = -x2 + y2 - z2 + w2;
        let m12 = 2.0 * (yz - xw);
        let m20 = 2.0 * (xz - yw);
        let m21 = 2.0 * (yz + xw);
        let m22 = -x2 - y2 + z2 + w2;

        // Column-major storage via JS constructor path:
        // new Matrix3(m00, m01, m02, m10, m11, m12, m20, m21, m22)
        // stores as [m00, m10, m20, m01, m11, m21, m02, m12, m22]
        // (transposed relative to standard math layout).
        result.elements[0] = m00;
        result.elements[1] = m10;
        result.elements[2] = m20;
        result.elements[3] = m01;
        result.elements[4] = m11;
        result.elements[5] = m21;
        result.elements[6] = m02;
        result.elements[7] = m12;
        result.elements[8] = m22;
    }

    pub fn from_quaternion_new(quaternion: &Quaternion) -> Self {
        let mut result = Self::default();
        Self::from_quaternion(quaternion, &mut result);
        result
    }

    /// Port of `Matrix3.fromHeadingPitchRoll`.
    ///
    /// Computes a rotation matrix from a [`HeadingPitchRoll`] orientation,
    /// mirroring the JS element-wise trigonometric construction verbatim
    /// (bit-comparable with CesiumJS; earlier revisions routed through
    /// `Quaternion::fromHeadingPitchRoll`, which introduced floating-point
    /// deltas against the JS reference).
    pub fn from_heading_pitch_roll(
        heading_pitch_roll: &HeadingPitchRoll,
        result: &mut Self,
    ) {
        //>>includeStart('debug', pragmas.debug);
        // DEVIATION: JS `Check.typeOf.object("headingPitchRoll", ...)` is
        // statically guaranteed by the non-optional Rust parameter.
        //>>includeEnd('debug');

        let cos_theta = (-heading_pitch_roll.pitch).cos();
        let cos_psi = (-heading_pitch_roll.heading).cos();
        let cos_phi = heading_pitch_roll.roll.cos();
        let sin_theta = (-heading_pitch_roll.pitch).sin();
        let sin_psi = (-heading_pitch_roll.heading).sin();
        let sin_phi = heading_pitch_roll.roll.sin();

        let m00 = cos_theta * cos_psi;
        let m01 = -cos_phi * sin_psi + sin_phi * sin_theta * cos_psi;
        let m02 = sin_phi * sin_psi + cos_phi * sin_theta * cos_psi;

        let m10 = cos_theta * sin_psi;
        let m11 = cos_phi * cos_psi + sin_phi * sin_theta * sin_psi;
        let m12 = -sin_phi * cos_psi + cos_phi * sin_theta * sin_psi;

        let m20 = -sin_theta;
        let m21 = sin_phi * cos_theta;
        let m22 = cos_phi * cos_theta;

        result.elements[0] = m00;
        result.elements[1] = m10;
        result.elements[2] = m20;
        result.elements[3] = m01;
        result.elements[4] = m11;
        result.elements[5] = m21;
        result.elements[6] = m02;
        result.elements[7] = m12;
        result.elements[8] = m22;
    }

    /// Allocating variant of [`Matrix3::from_heading_pitch_roll`].
    pub fn from_heading_pitch_roll_new(heading_pitch_roll: &HeadingPitchRoll) -> Self {
        let mut result = Self::default();
        Self::from_heading_pitch_roll(heading_pitch_roll, &mut result);
        result
    }

    /// Computes a `Matrix3` representing a non-uniform scale.
    ///
    /// Port of `Matrix3.fromScale`.
    pub fn from_scale(scale: &Cartesian3, result: &mut Self) {
        result.elements[0] = scale.x;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = scale.y;
        result.elements[5] = 0.0;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = scale.z;
    }

    /// Allocating variant of [`Matrix3::from_scale`].
    pub fn from_scale_new(scale: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::from_scale(scale, &mut result);
        result
    }

    /// Computes a `Matrix3` representing a uniform scale.
    ///
    /// Port of `Matrix3.fromUniformScale`.
    pub fn from_uniform_scale(scale: f64, result: &mut Self) {
        result.elements[0] = scale;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = scale;
        result.elements[5] = 0.0;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = scale;
    }

    /// Allocating variant of [`Matrix3::from_uniform_scale`].
    pub fn from_uniform_scale_new(scale: f64) -> Self {
        let mut result = Self::default();
        Self::from_uniform_scale(scale, &mut result);
        result
    }

    /// Computes a `Matrix3` representing the cross product equivalent matrix.
    ///
    /// Port of `Matrix3.fromCrossProduct`.
    pub fn from_cross_product(vector: &Cartesian3, result: &mut Self) {
        result.elements[0] = 0.0;
        result.elements[1] = vector.z;
        result.elements[2] = -vector.y;
        result.elements[3] = -vector.z;
        result.elements[4] = 0.0;
        result.elements[5] = vector.x;
        result.elements[6] = vector.y;
        result.elements[7] = -vector.x;
        result.elements[8] = 0.0;
    }

    /// Allocating variant of [`Matrix3::from_cross_product`].
    pub fn from_cross_product_new(vector: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::from_cross_product(vector, &mut result);
        result
    }

    /// Creates a rotation matrix around the x-axis.
    ///
    /// Port of `Matrix3.fromRotationX`.
    pub fn from_rotation_x(angle: f64, result: &mut Self) {
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        result.elements[0] = 1.0;
        result.elements[1] = 0.0;
        result.elements[2] = 0.0;
        result.elements[3] = 0.0;
        result.elements[4] = cos_angle;
        result.elements[5] = sin_angle;
        result.elements[6] = 0.0;
        result.elements[7] = -sin_angle;
        result.elements[8] = cos_angle;
    }

    /// Allocating variant of [`Matrix3::from_rotation_x`].
    pub fn from_rotation_x_new(angle: f64) -> Self {
        let mut result = Self::default();
        Self::from_rotation_x(angle, &mut result);
        result
    }

    /// Creates a rotation matrix around the y-axis.
    ///
    /// Port of `Matrix3.fromRotationY`.
    pub fn from_rotation_y(angle: f64, result: &mut Self) {
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        result.elements[0] = cos_angle;
        result.elements[1] = 0.0;
        result.elements[2] = -sin_angle;
        result.elements[3] = 0.0;
        result.elements[4] = 1.0;
        result.elements[5] = 0.0;
        result.elements[6] = sin_angle;
        result.elements[7] = 0.0;
        result.elements[8] = cos_angle;
    }

    /// Allocating variant of [`Matrix3::from_rotation_y`].
    pub fn from_rotation_y_new(angle: f64) -> Self {
        let mut result = Self::default();
        Self::from_rotation_y(angle, &mut result);
        result
    }

    /// Creates a rotation matrix around the z-axis.
    ///
    /// Port of `Matrix3.fromRotationZ`.
    pub fn from_rotation_z(angle: f64, result: &mut Self) {
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        result.elements[0] = cos_angle;
        result.elements[1] = sin_angle;
        result.elements[2] = 0.0;
        result.elements[3] = -sin_angle;
        result.elements[4] = cos_angle;
        result.elements[5] = 0.0;
        result.elements[6] = 0.0;
        result.elements[7] = 0.0;
        result.elements[8] = 1.0;
    }

    /// Allocating variant of [`Matrix3::from_rotation_z`].
    pub fn from_rotation_z_new(angle: f64) -> Self {
        let mut result = Self::default();
        Self::from_rotation_z(angle, &mut result);
        result
    }

    /// Creates an array from the provided `Matrix3`.
    ///
    /// Port of `Matrix3.toArray`.
    pub fn to_array(matrix: &Self, result: &mut [f64]) {
        result[..9].copy_from_slice(&matrix.elements);
    }

    /// Allocating variant of [`Matrix3::to_array`].
    pub fn to_array_new(matrix: &Self) -> [f64; 9] {
        let mut result = [0.0; 9];
        Self::to_array(matrix, &mut result);
        result
    }

    /// Computes the array index of the element at the provided row and column.
    ///
    /// Port of `Matrix3.getElementIndex`.
    pub fn get_element_index(column: usize, row: usize) -> usize {
        column * 3 + row
    }

    /// Retrieves a column at the provided index as a `Cartesian3`.
    ///
    /// Port of `Matrix3.getColumn`.
    pub fn get_column(matrix: &Self, index: usize, result: &mut Cartesian3) {
        let start = index * 3;
        result.x = matrix.elements[start];
        result.y = matrix.elements[start + 1];
        result.z = matrix.elements[start + 2];
    }

    /// Allocating variant of [`Matrix3::get_column`].
    pub fn get_column_new(matrix: &Self, index: usize) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::get_column(matrix, index, &mut result);
        result
    }

    /// Replaces the specified column in the provided matrix.
    ///
    /// Port of `Matrix3.setColumn`.
    pub fn set_column(matrix: &Self, index: usize, cartesian: &Cartesian3, result: &mut Self) {
        *result = *matrix;
        let start = index * 3;
        result.elements[start] = cartesian.x;
        result.elements[start + 1] = cartesian.y;
        result.elements[start + 2] = cartesian.z;
    }

    /// Retrieves a row at the provided index as a `Cartesian3`.
    ///
    /// Port of `Matrix3.getRow`.
    pub fn get_row(matrix: &Self, index: usize, result: &mut Cartesian3) {
        result.x = matrix.elements[index];
        result.y = matrix.elements[index + 3];
        result.z = matrix.elements[index + 6];
    }

    /// Allocating variant of [`Matrix3::get_row`].
    pub fn get_row_new(matrix: &Self, index: usize) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::get_row(matrix, index, &mut result);
        result
    }

    /// Replaces the specified row in the provided matrix.
    ///
    /// Port of `Matrix3.setRow`.
    pub fn set_row(matrix: &Self, index: usize, cartesian: &Cartesian3, result: &mut Self) {
        *result = *matrix;
        result.elements[index] = cartesian.x;
        result.elements[index + 3] = cartesian.y;
        result.elements[index + 6] = cartesian.z;
    }

    /// Extracts the non-uniform scale assuming the matrix is an affine transformation.
    ///
    /// Port of `Matrix3.getScale`.
    pub fn get_scale(matrix: &Self, result: &mut Cartesian3) {
        let col0 = Cartesian3::new(
            matrix.elements[0],
            matrix.elements[1],
            matrix.elements[2],
        );
        let col1 = Cartesian3::new(
            matrix.elements[3],
            matrix.elements[4],
            matrix.elements[5],
        );
        let col2 = Cartesian3::new(
            matrix.elements[6],
            matrix.elements[7],
            matrix.elements[8],
        );
        result.x = Cartesian3::magnitude(&col0);
        result.y = Cartesian3::magnitude(&col1);
        result.z = Cartesian3::magnitude(&col2);
    }

    /// Allocating variant of [`Matrix3::get_scale`].
    pub fn get_scale_new(matrix: &Self) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::get_scale(matrix, &mut result);
        result
    }

    /// Computes the maximum scale assuming the matrix is an affine transformation.
    ///
    /// Port of `Matrix3.getMaximumScale`.
    pub fn get_maximum_scale(matrix: &Self) -> f64 {
        let scale = Self::get_scale_new(matrix);
        Cartesian3::maximum_component(&scale)
    }

    /// Computes a new matrix that replaces the scale with the provided scale.
    ///
    /// Port of `Matrix3.setScale`.
    pub fn set_scale(matrix: &Self, scale: &Cartesian3, result: &mut Self) {
        let existing_scale = Self::get_scale_new(matrix);
        let ratio_x = scale.x / existing_scale.x;
        let ratio_y = scale.y / existing_scale.y;
        let ratio_z = scale.z / existing_scale.z;

        result.elements[0] = matrix.elements[0] * ratio_x;
        result.elements[1] = matrix.elements[1] * ratio_x;
        result.elements[2] = matrix.elements[2] * ratio_x;
        result.elements[3] = matrix.elements[3] * ratio_y;
        result.elements[4] = matrix.elements[4] * ratio_y;
        result.elements[5] = matrix.elements[5] * ratio_y;
        result.elements[6] = matrix.elements[6] * ratio_z;
        result.elements[7] = matrix.elements[7] * ratio_z;
        result.elements[8] = matrix.elements[8] * ratio_z;
    }

    /// Computes a new matrix that replaces the scale with a uniform scale.
    ///
    /// Port of `Matrix3.setUniformScale`.
    pub fn set_uniform_scale(matrix: &Self, scale: f64, result: &mut Self) {
        let existing_scale = Self::get_scale_new(matrix);
        let ratio_x = scale / existing_scale.x;
        let ratio_y = scale / existing_scale.y;
        let ratio_z = scale / existing_scale.z;

        result.elements[0] = matrix.elements[0] * ratio_x;
        result.elements[1] = matrix.elements[1] * ratio_x;
        result.elements[2] = matrix.elements[2] * ratio_x;
        result.elements[3] = matrix.elements[3] * ratio_y;
        result.elements[4] = matrix.elements[4] * ratio_y;
        result.elements[5] = matrix.elements[5] * ratio_y;
        result.elements[6] = matrix.elements[6] * ratio_z;
        result.elements[7] = matrix.elements[7] * ratio_z;
        result.elements[8] = matrix.elements[8] * ratio_z;
    }

    /// Sets the rotation assuming the matrix is an affine transformation.
    ///
    /// Port of `Matrix3.setRotation`.
    pub fn set_rotation(matrix: &Self, rotation: &Self, result: &mut Self) {
        let scale = Self::get_scale_new(matrix);

        result.elements[0] = rotation.elements[0] * scale.x;
        result.elements[1] = rotation.elements[1] * scale.x;
        result.elements[2] = rotation.elements[2] * scale.x;
        result.elements[3] = rotation.elements[3] * scale.y;
        result.elements[4] = rotation.elements[4] * scale.y;
        result.elements[5] = rotation.elements[5] * scale.y;
        result.elements[6] = rotation.elements[6] * scale.z;
        result.elements[7] = rotation.elements[7] * scale.z;
        result.elements[8] = rotation.elements[8] * scale.z;
    }

    /// Extracts the rotation matrix.
    ///
    /// Port of `Matrix3.getRotation`.
    pub fn get_rotation(matrix: &Self, result: &mut Self) {
        let scale = Self::get_scale_new(matrix);

        result.elements[0] = matrix.elements[0] / scale.x;
        result.elements[1] = matrix.elements[1] / scale.x;
        result.elements[2] = matrix.elements[2] / scale.x;
        result.elements[3] = matrix.elements[3] / scale.y;
        result.elements[4] = matrix.elements[4] / scale.y;
        result.elements[5] = matrix.elements[5] / scale.y;
        result.elements[6] = matrix.elements[6] / scale.z;
        result.elements[7] = matrix.elements[7] / scale.z;
        result.elements[8] = matrix.elements[8] / scale.z;
    }

    /// Allocating variant of [`Matrix3::get_rotation`].
    pub fn get_rotation_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::get_rotation(matrix, &mut result);
        result
    }

    /// Computes the product of two matrices.
    ///
    /// Port of `Matrix3.multiply`.
    pub fn multiply(left: &Self, right: &Self, result: &mut Self) {
        let c0r0 = left.elements[0] * right.elements[0] + left.elements[3] * right.elements[1] + left.elements[6] * right.elements[2];
        let c0r1 = left.elements[1] * right.elements[0] + left.elements[4] * right.elements[1] + left.elements[7] * right.elements[2];
        let c0r2 = left.elements[2] * right.elements[0] + left.elements[5] * right.elements[1] + left.elements[8] * right.elements[2];

        let c1r0 = left.elements[0] * right.elements[3] + left.elements[3] * right.elements[4] + left.elements[6] * right.elements[5];
        let c1r1 = left.elements[1] * right.elements[3] + left.elements[4] * right.elements[4] + left.elements[7] * right.elements[5];
        let c1r2 = left.elements[2] * right.elements[3] + left.elements[5] * right.elements[4] + left.elements[8] * right.elements[5];

        let c2r0 = left.elements[0] * right.elements[6] + left.elements[3] * right.elements[7] + left.elements[6] * right.elements[8];
        let c2r1 = left.elements[1] * right.elements[6] + left.elements[4] * right.elements[7] + left.elements[7] * right.elements[8];
        let c2r2 = left.elements[2] * right.elements[6] + left.elements[5] * right.elements[7] + left.elements[8] * right.elements[8];

        result.elements[0] = c0r0;
        result.elements[1] = c0r1;
        result.elements[2] = c0r2;
        result.elements[3] = c1r0;
        result.elements[4] = c1r1;
        result.elements[5] = c1r2;
        result.elements[6] = c2r0;
        result.elements[7] = c2r1;
        result.elements[8] = c2r2;
    }

    /// Allocating variant of [`Matrix3::multiply`].
    pub fn multiply_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply(left, right, &mut result);
        result
    }

    /// Computes the sum of two matrices.
    ///
    /// Port of `Matrix3.add`.
    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = left.elements[i] + right.elements[i];
        }
    }

    /// Allocating variant of [`Matrix3::add`].
    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    /// Computes the difference of two matrices.
    ///
    /// Port of `Matrix3.subtract`.
    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = left.elements[i] - right.elements[i];
        }
    }

    /// Allocating variant of [`Matrix3::subtract`].
    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    /// Computes the product of a matrix and a column vector.
    ///
    /// Port of `Matrix3.multiplyByVector`.
    pub fn multiply_by_vector(matrix: &Self, cartesian: &Cartesian3, result: &mut Cartesian3) {
        let x = matrix.elements[0] * cartesian.x + matrix.elements[3] * cartesian.y + matrix.elements[6] * cartesian.z;
        let y = matrix.elements[1] * cartesian.x + matrix.elements[4] * cartesian.y + matrix.elements[7] * cartesian.z;
        let z = matrix.elements[2] * cartesian.x + matrix.elements[5] * cartesian.y + matrix.elements[8] * cartesian.z;
        result.x = x;
        result.y = y;
        result.z = z;
    }

    /// Allocating variant of [`Matrix3::multiply_by_vector`].
    pub fn multiply_by_vector_new(matrix: &Self, cartesian: &Cartesian3) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::multiply_by_vector(matrix, cartesian, &mut result);
        result
    }

    /// Computes the product of a matrix and a scalar.
    ///
    /// Port of `Matrix3.multiplyByScalar`.
    pub fn multiply_by_scalar(matrix: &Self, scalar: f64, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = matrix.elements[i] * scalar;
        }
    }

    /// Allocating variant of [`Matrix3::multiply_by_scalar`].
    pub fn multiply_by_scalar_new(matrix: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(matrix, scalar, &mut result);
        result
    }

    /// Computes the product of a matrix times a non-uniform scale.
    ///
    /// Port of `Matrix3.multiplyByScale`.
    pub fn multiply_by_scale(matrix: &Self, scale: &Cartesian3, result: &mut Self) {
        result.elements[0] = matrix.elements[0] * scale.x;
        result.elements[1] = matrix.elements[1] * scale.x;
        result.elements[2] = matrix.elements[2] * scale.x;
        result.elements[3] = matrix.elements[3] * scale.y;
        result.elements[4] = matrix.elements[4] * scale.y;
        result.elements[5] = matrix.elements[5] * scale.y;
        result.elements[6] = matrix.elements[6] * scale.z;
        result.elements[7] = matrix.elements[7] * scale.z;
        result.elements[8] = matrix.elements[8] * scale.z;
    }

    /// Computes the product of a matrix times a uniform scale.
    ///
    /// Port of `Matrix3.multiplyByUniformScale`.
    pub fn multiply_by_uniform_scale(matrix: &Self, scale: f64, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = matrix.elements[i] * scale;
        }
    }

    /// Creates a negated copy of the provided matrix.
    ///
    /// Port of `Matrix3.negate`.
    pub fn negate(matrix: &Self, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = -matrix.elements[i];
        }
    }

    /// Allocating variant of [`Matrix3::negate`].
    pub fn negate_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(matrix, &mut result);
        result
    }

    /// Computes the transpose of the provided matrix.
    ///
    /// Port of `Matrix3.transpose`.
    pub fn transpose(matrix: &Self, result: &mut Self) {
        let c0r0 = matrix.elements[0];
        let c0r1 = matrix.elements[3];
        let c0r2 = matrix.elements[6];
        let c1r0 = matrix.elements[1];
        let c1r1 = matrix.elements[4];
        let c1r2 = matrix.elements[7];
        let c2r0 = matrix.elements[2];
        let c2r1 = matrix.elements[5];
        let c2r2 = matrix.elements[8];

        result.elements[0] = c0r0;
        result.elements[1] = c0r1;
        result.elements[2] = c0r2;
        result.elements[3] = c1r0;
        result.elements[4] = c1r1;
        result.elements[5] = c1r2;
        result.elements[6] = c2r0;
        result.elements[7] = c2r1;
        result.elements[8] = c2r2;
    }

    /// Allocating variant of [`Matrix3::transpose`].
    pub fn transpose_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::transpose(matrix, &mut result);
        result
    }

    /// Computes a matrix with absolute values of the provided matrix's elements.
    ///
    /// Port of `Matrix3.abs`.
    pub fn abs(matrix: &Self, result: &mut Self) {
        for i in 0..9 {
            result.elements[i] = matrix.elements[i].abs();
        }
    }

    /// Allocating variant of [`Matrix3::abs`].
    pub fn abs_new(matrix: &Self) -> Self {
        let mut result = Self::default();
        Self::abs(matrix, &mut result);
        result
    }

    /// Computes the determinant of the provided matrix.
    ///
    /// Port of `Matrix3.determinant`.
    pub fn determinant(matrix: &Self) -> f64 {
        let m11 = matrix.elements[0]; // col0row0
        let m21 = matrix.elements[3]; // col1row0
        let m31 = matrix.elements[6]; // col2row0
        let m12 = matrix.elements[1]; // col0row1
        let m22 = matrix.elements[4]; // col1row1
        let m32 = matrix.elements[7]; // col2row1
        let m13 = matrix.elements[2]; // col0row2
        let m23 = matrix.elements[5]; // col1row2
        let m33 = matrix.elements[8]; // col2row2

        m11 * (m22 * m33 - m23 * m32) + m12 * (m23 * m31 - m21 * m33) + m13 * (m21 * m32 - m22 * m31)
    }

    /// Computes the inverse of the provided matrix.
    ///
    /// Port of `Matrix3.inverse`.
    /// Returns `None` if the matrix is not invertible (determinant ≈ 0).
    pub fn inverse(matrix: &Self, result: &mut Self) -> bool {
        let m11 = matrix.elements[0];
        let m21 = matrix.elements[1];
        let m31 = matrix.elements[2];
        let m12 = matrix.elements[3];
        let m22 = matrix.elements[4];
        let m32 = matrix.elements[5];
        let m13 = matrix.elements[6];
        let m23 = matrix.elements[7];
        let m33 = matrix.elements[8];

        let det = Self::determinant(matrix);

        if det.abs() <= 1e-15 {
            return false;
        }

        result.elements[0] = m22 * m33 - m23 * m32;
        result.elements[1] = m23 * m31 - m21 * m33;
        result.elements[2] = m21 * m32 - m22 * m31;
        result.elements[3] = m13 * m32 - m12 * m33;
        result.elements[4] = m11 * m33 - m13 * m31;
        result.elements[5] = m12 * m31 - m11 * m32;
        result.elements[6] = m12 * m23 - m13 * m22;
        result.elements[7] = m13 * m21 - m11 * m23;
        result.elements[8] = m11 * m22 - m12 * m21;

        let scale_factor = 1.0 / det;
        for i in 0..9 {
            result.elements[i] *= scale_factor;
        }
        true
    }

    /// Allocating variant of [`Matrix3::inverse`].
    pub fn inverse_new(matrix: &Self) -> Option<Self> {
        let mut result = Self::default();
        if Self::inverse(matrix, &mut result) {
            Some(result)
        } else {
            None
        }
    }

    /// Computes the inverse transpose of a matrix.
    ///
    /// Port of `Matrix3.inverseTranspose`.
    pub fn inverse_transpose(matrix: &Self, result: &mut Self) -> bool {
        let mut transposed = Self::default();
        Self::transpose(matrix, &mut transposed);
        Self::inverse(&transposed, result)
    }

    /// Compares two matrices componentwise.
    ///
    /// Port of `Matrix3.equals`.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.elements == right.elements
    }

    /// Compares two matrices componentwise within epsilon.
    ///
    /// Port of `Matrix3.equalsEpsilon`.
    pub fn equals_epsilon(left: &Self, right: &Self, epsilon: f64) -> bool {
        for i in 0..9 {
            if (left.elements[i] - right.elements[i]).abs() > epsilon {
                return false;
            }
        }
        true
    }

    /// Duplicates a `Matrix3` instance.
    ///
    /// Port of `Matrix3.clone`.
    pub fn clone(matrix: &Self, result: &mut Self) {
        result.elements = matrix.elements;
    }

    /// Allocating variant of [`Matrix3::clone`].
    pub fn clone_new(matrix: &Self) -> Self {
        Self {
            elements: matrix.elements,
        }
    }

    /// Computes the eigenvectors and eigenvalues of a symmetric matrix.
    ///
    /// Port of `Matrix3.computeEigenDecomposition`.
    pub fn compute_eigen_decomposition(matrix: &Self, result: Option<EigenDecompositionResult>) -> EigenDecompositionResult {
        let tolerance = 1e-20;
        let max_sweeps = 10;

        let mut count = 0;
        let mut sweep = 0;

        let mut unitary = Self::IDENTITY;
        let mut diagonal = *matrix;

        let epsilon = tolerance * compute_frobenius_norm(&diagonal);

        // row/col pivot indices for 3x3 symmetric Jacobi
        let row_val = [1, 0, 0];
        let col_val = [2, 2, 1];

        while sweep < max_sweeps && off_diagonal_frobenius_norm(&diagonal, &row_val, &col_val) > epsilon {
            let (j_mat, j_trans) = shur_decomposition(&diagonal, &row_val, &col_val);
            let mut temp = Self::default();
            Self::multiply(&diagonal, &j_mat, &mut temp);
            diagonal = temp;
            let mut temp2 = Self::default();
            Self::multiply(&j_trans, &diagonal, &mut temp2);
            diagonal = temp2;
            let mut temp3 = Self::default();
            Self::multiply(&unitary, &j_mat, &mut temp3);
            unitary = temp3;

            count += 1;
            if count > 2 {
                sweep += 1;
                count = 0;
            }
        }

        let _ = result; // unused for now
        EigenDecompositionResult {
            unitary,
            diagonal,
        }
    }
}

impl PartialEq for Matrix3 {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}

impl std::fmt::Display for Matrix3 {
    /// Port of `Matrix3.prototype.toString`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {})\n({}, {}, {})\n({}, {}, {})",
            self.elements[0], self.elements[3], self.elements[6],
            self.elements[1], self.elements[4], self.elements[7],
            self.elements[2], self.elements[5], self.elements[8],
        )
    }
}

// --- Private helper functions for eigen decomposition ---

fn compute_frobenius_norm(matrix: &Matrix3) -> f64 {
    let mut norm = 0.0;
    for i in 0..9 {
        norm += matrix.elements[i] * matrix.elements[i];
    }
    norm.sqrt()
}

fn off_diagonal_frobenius_norm(matrix: &Matrix3, row_val: &[usize; 3], col_val: &[usize; 3]) -> f64 {
    let mut norm = 0.0;
    for i in 0..3 {
        let idx = Matrix3::get_element_index(col_val[i], row_val[i]);
        let temp = matrix.elements[idx];
        norm += 2.0 * temp * temp;
    }
    norm.sqrt()
}

fn shur_decomposition(matrix: &Matrix3, row_val: &[usize; 3], col_val: &[usize; 3]) -> (Matrix3, Matrix3) {
    let tolerance = 1e-15;

    let mut max_diagonal = 0.0;
    let mut rot_axis = 1;

    for i in 0..3 {
        let idx = Matrix3::get_element_index(col_val[i], row_val[i]);
        let temp = matrix.elements[idx].abs();
        if temp > max_diagonal {
            rot_axis = i;
            max_diagonal = temp;
        }
    }

    let mut c = 1.0;
    let mut s = 0.0;

    let p = row_val[rot_axis];
    let q = col_val[rot_axis];

    let qp_idx = Matrix3::get_element_index(q, p);
    if matrix.elements[qp_idx].abs() > tolerance {
        let qq_idx = Matrix3::get_element_index(q, q);
        let pp_idx = Matrix3::get_element_index(p, p);

        let qq = matrix.elements[qq_idx];
        let pp = matrix.elements[pp_idx];
        let qp = matrix.elements[qp_idx];

        let tau = (qq - pp) / 2.0 / qp;
        let t = if tau < 0.0 {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        } else {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        };

        c = 1.0 / (1.0 + t * t).sqrt();
        s = t * c;
    }

    let mut result = Matrix3::IDENTITY;
    let pp_idx = Matrix3::get_element_index(p, p);
    let qq_idx = Matrix3::get_element_index(q, q);
    let qp_idx = Matrix3::get_element_index(q, p);
    let pq_idx = Matrix3::get_element_index(p, q);

    result.elements[pp_idx] = c;
    result.elements[qq_idx] = c;
    result.elements[qp_idx] = s;
    result.elements[pq_idx] = -s;

    let mut result_transpose = Matrix3::default();
    Matrix3::transpose(&result, &mut result_transpose);

    (result, result_transpose)
}
