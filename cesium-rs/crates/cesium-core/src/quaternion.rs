//! Ported from packages/engine/Source/Core/Quaternion.js
//!
//! A set of 4-dimensional coordinates used to represent rotation in 3-dimensional space.

use crate::cartesian3::Cartesian3;
use crate::heading_pitch_roll::HeadingPitchRoll;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;

/// A quaternion representing a rotation in 3D space.
#[derive(Clone, Copy, Debug, Default)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Quaternion {
    pub const PACKED_LENGTH: usize = 4;

    pub const ZERO: Quaternion = Quaternion { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    pub const IDENTITY: Quaternion = Quaternion { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };

    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    // --- fromAxisAngle ---

    /// Port of `Quaternion.fromAxisAngle`.
    pub fn from_axis_angle(axis: &Cartesian3, angle: f64, result: &mut Self) {
        let half_angle = angle / 2.0;
        let s = half_angle.sin();
        let normalized = Cartesian3::normalize_new(axis);
        result.x = normalized.x * s;
        result.y = normalized.y * s;
        result.z = normalized.z * s;
        result.w = half_angle.cos();
    }

    pub fn from_axis_angle_new(axis: &Cartesian3, angle: f64) -> Self {
        let mut result = Self::default();
        Self::from_axis_angle(axis, angle, &mut result);
        result
    }

    // --- fromRotationMatrix ---

    /// Port of `Quaternion.fromRotationMatrix`.
    pub fn from_rotation_matrix(matrix: &Matrix3, result: &mut Self) {
        let m00 = matrix.elements[Matrix3::COLUMN0ROW0];
        let m11 = matrix.elements[Matrix3::COLUMN1ROW1];
        let m22 = matrix.elements[Matrix3::COLUMN2ROW2];
        let trace = m00 + m11 + m22;

        if trace > 0.0 {
            let root = (trace + 1.0).sqrt(); // 2w
            let w = 0.5 * root;
            let inv = 0.5 / root; // 1/(4w)

            let x = (matrix.elements[Matrix3::COLUMN1ROW2] - matrix.elements[Matrix3::COLUMN2ROW1]) * inv;
            let y = (matrix.elements[Matrix3::COLUMN2ROW0] - matrix.elements[Matrix3::COLUMN0ROW2]) * inv;
            let z = (matrix.elements[Matrix3::COLUMN0ROW1] - matrix.elements[Matrix3::COLUMN1ROW0]) * inv;

            result.x = x;
            result.y = y;
            result.z = z;
            result.w = w;
        } else {
            let next = [1usize, 2, 0];

            let i = if m11 > m00 {
                if m22 > m11 { 2 } else { 1 }
            } else if m22 > m00 {
                2
            } else {
                0
            };
            let j = next[i];
            let k = next[j];

            let root = (matrix.elements[Matrix3::get_element_index(i, i)]
                - matrix.elements[Matrix3::get_element_index(j, j)]
                - matrix.elements[Matrix3::get_element_index(k, k)]
                + 1.0)
                .sqrt();

            let mut quat = [0.0f64; 3];
            quat[i] = 0.5 * root;
            let inv = 0.5 / root;

            let w = (matrix.elements[Matrix3::get_element_index(k, j)]
                - matrix.elements[Matrix3::get_element_index(j, k)])
                * inv;
            quat[j] = (matrix.elements[Matrix3::get_element_index(j, i)]
                + matrix.elements[Matrix3::get_element_index(i, j)])
                * inv;
            quat[k] = (matrix.elements[Matrix3::get_element_index(k, i)]
                + matrix.elements[Matrix3::get_element_index(i, k)])
                * inv;

            result.x = -quat[0];
            result.y = -quat[1];
            result.z = -quat[2];
            result.w = w;
        }
    }

    pub fn from_rotation_matrix_new(matrix: &Matrix3) -> Self {
        let mut result = Self::default();
        Self::from_rotation_matrix(matrix, &mut result);
        result
    }

    // --- fromHeadingPitchRoll ---

    /// Port of `Quaternion.fromHeadingPitchRoll`.
    pub fn from_heading_pitch_roll(heading_pitch_roll: &HeadingPitchRoll, result: &mut Self) {
        // Roll rotation about +X
        let roll_quat = Self::from_axis_angle_new(&Cartesian3::UNIT_X, heading_pitch_roll.roll);
        // Pitch rotation about -Y
        let pitch_quat = Self::from_axis_angle_new(&Cartesian3::UNIT_Y, -heading_pitch_roll.pitch);
        // Heading rotation about -Z
        let heading_quat = Self::from_axis_angle_new(&Cartesian3::UNIT_Z, -heading_pitch_roll.heading);

        // combined = heading * (pitch * roll)
        let pitch_roll = Self::multiply_new(&pitch_quat, &roll_quat);
        let combined = Self::multiply_new(&heading_quat, &pitch_roll);
        *result = combined;
    }

    pub fn from_heading_pitch_roll_new(heading_pitch_roll: &HeadingPitchRoll) -> Self {
        let mut result = Self::default();
        Self::from_heading_pitch_roll(heading_pitch_roll, &mut result);
        result
    }

    // --- pack / unpack ---

    pub fn pack(value: &Self, array: &mut [f64], starting_index: usize) {
        array[starting_index] = value.x;
        array[starting_index + 1] = value.y;
        array[starting_index + 2] = value.z;
        array[starting_index + 3] = value.w;
    }

    pub fn unpack(array: &[f64], starting_index: usize, result: &mut Self) {
        result.x = array[starting_index];
        result.y = array[starting_index + 1];
        result.z = array[starting_index + 2];
        result.w = array[starting_index + 3];
    }

    pub fn unpack_new(array: &[f64], starting_index: usize) -> Self {
        let mut result = Self::default();
        Self::unpack(array, starting_index, &mut result);
        result
    }

    // --- clone ---

    pub fn clone_quaternion(quaternion: &Self, result: &mut Self) {
        result.x = quaternion.x;
        result.y = quaternion.y;
        result.z = quaternion.z;
        result.w = quaternion.w;
    }

    pub fn clone_new(quaternion: &Self) -> Self {
        Self { x: quaternion.x, y: quaternion.y, z: quaternion.z, w: quaternion.w }
    }

    // --- conjugate ---

    pub fn conjugate(quaternion: &Self, result: &mut Self) {
        result.x = -quaternion.x;
        result.y = -quaternion.y;
        result.z = -quaternion.z;
        result.w = quaternion.w;
    }

    pub fn conjugate_new(quaternion: &Self) -> Self {
        let mut result = Self::default();
        Self::conjugate(quaternion, &mut result);
        result
    }

    // --- magnitude / magnitudeSquared ---

    pub fn magnitude_squared(quaternion: &Self) -> f64 {
        quaternion.x * quaternion.x
            + quaternion.y * quaternion.y
            + quaternion.z * quaternion.z
            + quaternion.w * quaternion.w
    }

    pub fn magnitude(quaternion: &Self) -> f64 {
        Self::magnitude_squared(quaternion).sqrt()
    }

    // --- normalize ---

    pub fn normalize(quaternion: &Self, result: &mut Self) {
        let inv_mag = 1.0 / Self::magnitude(quaternion);
        result.x = quaternion.x * inv_mag;
        result.y = quaternion.y * inv_mag;
        result.z = quaternion.z * inv_mag;
        result.w = quaternion.w * inv_mag;
    }

    pub fn normalize_new(quaternion: &Self) -> Self {
        let mut result = Self::default();
        Self::normalize(quaternion, &mut result);
        result
    }

    // --- inverse ---

    pub fn inverse(quaternion: &Self, result: &mut Self) {
        let mag_sq = Self::magnitude_squared(quaternion);
        Self::conjugate(quaternion, result);
        let inv_mag_sq = 1.0 / mag_sq;
        result.x *= inv_mag_sq;
        result.y *= inv_mag_sq;
        result.z *= inv_mag_sq;
        result.w *= inv_mag_sq;
    }

    pub fn inverse_new(quaternion: &Self) -> Self {
        let mut result = Self::default();
        Self::inverse(quaternion, &mut result);
        result
    }

    // --- dot ---

    pub fn dot(left: &Self, right: &Self) -> f64 {
        left.x * right.x + left.y * right.y + left.z * right.z + left.w * right.w
    }

    // --- multiply ---

    pub fn multiply(left: &Self, right: &Self, result: &mut Self) {
        let lx = left.x;
        let ly = left.y;
        let lz = left.z;
        let lw = left.w;
        let rx = right.x;
        let ry = right.y;
        let rz = right.z;
        let rw = right.w;

        result.x = lw * rx + lx * rw + ly * rz - lz * ry;
        result.y = lw * ry - lx * rz + ly * rw + lz * rx;
        result.z = lw * rz + lx * ry - ly * rx + lz * rw;
        result.w = lw * rw - lx * rx - ly * ry - lz * rz;
    }

    pub fn multiply_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::multiply(left, right, &mut result);
        result
    }

    // --- add / subtract ---

    pub fn add(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x + right.x;
        result.y = left.y + right.y;
        result.z = left.z + right.z;
        result.w = left.w + right.w;
    }

    pub fn add_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::add(left, right, &mut result);
        result
    }

    pub fn subtract(left: &Self, right: &Self, result: &mut Self) {
        result.x = left.x - right.x;
        result.y = left.y - right.y;
        result.z = left.z - right.z;
        result.w = left.w - right.w;
    }

    pub fn subtract_new(left: &Self, right: &Self) -> Self {
        let mut result = Self::default();
        Self::subtract(left, right, &mut result);
        result
    }

    // --- multiplyByScalar / divideByScalar ---

    pub fn multiply_by_scalar(quaternion: &Self, scalar: f64, result: &mut Self) {
        result.x = quaternion.x * scalar;
        result.y = quaternion.y * scalar;
        result.z = quaternion.z * scalar;
        result.w = quaternion.w * scalar;
    }

    pub fn multiply_by_scalar_new(quaternion: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::multiply_by_scalar(quaternion, scalar, &mut result);
        result
    }

    pub fn divide_by_scalar(quaternion: &Self, scalar: f64, result: &mut Self) {
        result.x = quaternion.x / scalar;
        result.y = quaternion.y / scalar;
        result.z = quaternion.z / scalar;
        result.w = quaternion.w / scalar;
    }

    pub fn divide_by_scalar_new(quaternion: &Self, scalar: f64) -> Self {
        let mut result = Self::default();
        Self::divide_by_scalar(quaternion, scalar, &mut result);
        result
    }

    // --- negate ---

    pub fn negate(quaternion: &Self, result: &mut Self) {
        result.x = -quaternion.x;
        result.y = -quaternion.y;
        result.z = -quaternion.z;
        result.w = -quaternion.w;
    }

    pub fn negate_new(quaternion: &Self) -> Self {
        let mut result = Self::default();
        Self::negate(quaternion, &mut result);
        result
    }

    // --- computeAxis / computeAngle ---

    /// Port of `Quaternion.computeAxis`.
    pub fn compute_axis(quaternion: &Self, result: &mut Cartesian3) {
        let w = quaternion.w;
        if (w - 1.0).abs() < CesiumMath::EPSILON6 || (w + 1.0).abs() < CesiumMath::EPSILON6 {
            result.x = 1.0;
            result.y = 0.0;
            result.z = 0.0;
            return;
        }
        let scalar = 1.0 / (1.0 - w * w).sqrt();
        result.x = quaternion.x * scalar;
        result.y = quaternion.y * scalar;
        result.z = quaternion.z * scalar;
    }

    pub fn compute_axis_new(quaternion: &Self) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::compute_axis(quaternion, &mut result);
        result
    }

    /// Port of `Quaternion.computeAngle`.
    pub fn compute_angle(quaternion: &Self) -> f64 {
        if (quaternion.w - 1.0).abs() < CesiumMath::EPSILON6 {
            return 0.0;
        }
        2.0 * quaternion.w.acos()
    }

    // --- lerp ---

    pub fn lerp(start: &Self, end: &Self, t: f64, result: &mut Self) {
        let scaled_end = Self::multiply_by_scalar_new(end, t);
        let scaled_start = Self::multiply_by_scalar_new(start, 1.0 - t);
        Self::add(&scaled_end, &scaled_start, result);
    }

    pub fn lerp_new(start: &Self, end: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::lerp(start, end, t, &mut result);
        result
    }

    // --- slerp ---

    pub fn slerp(start: &Self, end: &Self, t: f64, result: &mut Self) {
        let mut dot = Self::dot(start, end);

        let r = if dot < 0.0 {
            dot = -dot;
            Self::negate_new(end)
        } else {
            *end
        };

        if 1.0 - dot < CesiumMath::EPSILON6 {
            Self::lerp(start, &r, t, result);
            return;
        }

        let theta = dot.acos();
        let scaled_p = Self::multiply_by_scalar_new(start, ((1.0 - t) * theta).sin());
        let scaled_r = Self::multiply_by_scalar_new(&r, (t * theta).sin());
        Self::add(&scaled_p, &scaled_r, result);
        let inv_sin = 1.0 / theta.sin();
        let tmp = *result;
        Self::multiply_by_scalar(&tmp, inv_sin, result);
    }

    pub fn slerp_new(start: &Self, end: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::slerp(start, end, t, &mut result);
        result
    }

    // --- log / exp ---

    /// Port of `Quaternion.log`.
    pub fn log(quaternion: &Self, result: &mut Cartesian3) {
        let theta = CesiumMath::acos_clamped(quaternion.w);
        let theta_over_sin = if theta != 0.0 { theta / theta.sin() } else { 0.0 };
        Cartesian3::multiply_by_scalar(
            &Cartesian3::new(quaternion.x, quaternion.y, quaternion.z),
            theta_over_sin,
            result,
        );
    }

    pub fn log_new(quaternion: &Self) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::log(quaternion, &mut result);
        result
    }

    /// Port of `Quaternion.exp`.
    pub fn exp(cartesian: &Cartesian3, result: &mut Self) {
        let theta = Cartesian3::magnitude(cartesian);
        let sin_theta_over_theta = if theta != 0.0 { theta.sin() / theta } else { 0.0 };
        result.x = cartesian.x * sin_theta_over_theta;
        result.y = cartesian.y * sin_theta_over_theta;
        result.z = cartesian.z * sin_theta_over_theta;
        result.w = theta.cos();
    }

    pub fn exp_new(cartesian: &Cartesian3) -> Self {
        let mut result = Self::default();
        Self::exp(cartesian, &mut result);
        result
    }

    // --- computeInnerQuadrangle ---

    /// Port of `Quaternion.computeInnerQuadrangle`.
    pub fn compute_inner_quadrangle(q0: &Self, q1: &Self, q2: &Self, result: &mut Self) {
        let q_inv = Self::conjugate_new(q1);
        let prod1 = Self::multiply_new(&q_inv, q2);
        let cart0 = Self::log_new(&prod1);

        let prod2 = Self::multiply_new(&q_inv, q0);
        let cart1 = Self::log_new(&prod2);

        let mut sum = Cartesian3::default();
        Cartesian3::add(&cart0, &cart1, &mut sum);
        let scaled = Cartesian3::multiply_by_scalar_new(&sum, 0.25);
        let negated = Cartesian3::negate_new(&scaled);

        let exp_result = Self::exp_new(&negated);
        Self::multiply(q1, &exp_result, result);
    }

    pub fn compute_inner_quadrangle_new(q0: &Self, q1: &Self, q2: &Self) -> Self {
        let mut result = Self::default();
        Self::compute_inner_quadrangle(q0, q1, q2, &mut result);
        result
    }

    // --- squad ---

    pub fn squad(q0: &Self, q1: &Self, s0: &Self, s1: &Self, t: f64, result: &mut Self) {
        let slerp0 = Self::slerp_new(q0, q1, t);
        let slerp1 = Self::slerp_new(s0, s1, t);
        Self::slerp(&slerp0, &slerp1, 2.0 * t * (1.0 - t), result);
    }

    pub fn squad_new(q0: &Self, q1: &Self, s0: &Self, s1: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::squad(q0, q1, s0, s1, t, &mut result);
        result
    }

    // --- fastSlerp ---

    const OPMU: f64 = 1.90110745351730037;

    fn compute_fast_slerp_coeffs(x: f64, t: f64) -> [f64; 8] {
        let u = [
            1.0 / (1.0 * 3.0),
            1.0 / (2.0 * 5.0),
            1.0 / (3.0 * 7.0),
            1.0 / (4.0 * 9.0),
            1.0 / (5.0 * 11.0),
            1.0 / (6.0 * 13.0),
            1.0 / (7.0 * 15.0),
            Self::OPMU / (8.0 * 17.0),
        ];
        let v = [
            1.0 / 3.0,
            2.0 / 5.0,
            3.0 / 7.0,
            4.0 / 9.0,
            5.0 / 11.0,
            6.0 / 13.0,
            7.0 / 15.0,
            (Self::OPMU * 8.0) / 17.0,
        ];

        let xm1 = x - 1.0;
        let sqr_t = t * t;
        let mut b = [0.0f64; 8];
        for i in 0..8 {
            b[i] = (u[i] * sqr_t - v[i]) * xm1;
        }
        b
    }

    /// Port of `Quaternion.fastSlerp`.
    pub fn fast_slerp(start: &Self, end: &Self, t: f64, result: &mut Self) {
        let mut x = Self::dot(start, end);

        let sign = if x >= 0.0 { 1.0 } else { x = -x; -1.0 };

        let d = 1.0 - t;

        let b_t = Self::compute_fast_slerp_coeffs(x, t);
        let b_d = Self::compute_fast_slerp_coeffs(x, d);

        let c_t = sign * t
            * (1.0
                + b_t[0]
                    * (1.0
                        + b_t[1]
                            * (1.0
                                + b_t[2]
                                    * (1.0
                                        + b_t[3]
                                            * (1.0 + b_t[4] * (1.0 + b_t[5] * (1.0 + b_t[6] * (1.0 + b_t[7]))))))));
        let c_d = d
            * (1.0
                + b_d[0]
                    * (1.0
                        + b_d[1]
                            * (1.0
                                + b_d[2]
                                    * (1.0
                                        + b_d[3]
                                            * (1.0 + b_d[4] * (1.0 + b_d[5] * (1.0 + b_d[6] * (1.0 + b_d[7]))))))));

        let temp = Self::multiply_by_scalar_new(start, c_d);
        let scaled_end = Self::multiply_by_scalar_new(end, c_t);
        Self::add(&temp, &scaled_end, result);
    }

    pub fn fast_slerp_new(start: &Self, end: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::fast_slerp(start, end, t, &mut result);
        result
    }

    // --- fastSquad ---

    pub fn fast_squad(q0: &Self, q1: &Self, s0: &Self, s1: &Self, t: f64, result: &mut Self) {
        let slerp0 = Self::fast_slerp_new(q0, q1, t);
        let slerp1 = Self::fast_slerp_new(s0, s1, t);
        Self::fast_slerp(&slerp0, &slerp1, 2.0 * t * (1.0 - t), result);
    }

    pub fn fast_squad_new(q0: &Self, q1: &Self, s0: &Self, s1: &Self, t: f64) -> Self {
        let mut result = Self::default();
        Self::fast_squad(q0, q1, s0, s1, t, &mut result);
        result
    }

    // --- equals / equalsEpsilon ---

    pub fn equals(left: &Self, right: &Self) -> bool {
        left.x == right.x && left.y == right.y && left.z == right.z && left.w == right.w
    }

    pub fn equals_epsilon(left: &Self, right: &Self, epsilon: f64) -> bool {
        (left.x - right.x).abs() <= epsilon
            && (left.y - right.y).abs() <= epsilon
            && (left.z - right.z).abs() <= epsilon
            && (left.w - right.w).abs() <= epsilon
    }
}

impl PartialEq for Quaternion {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}

impl std::fmt::Display for Quaternion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}
