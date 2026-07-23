//! Math utilities - maps to CesiumJS `Core/Math.js` (CesiumMath)
//! All constants and helper functions used across the geospatial domain.

use std::f64::consts::PI;

/// PI constant
pub const PI_F64: f64 = PI;

/// 2 * PI
pub const TWO_PI: f64 = 2.0 * PI;

/// PI / 2
pub const PI_OVER_TWO: f64 = PI / 2.0;

/// PI / 3
pub const PI_OVER_THREE: f64 = PI / 3.0;

/// PI / 4
pub const PI_OVER_FOUR: f64 = PI / 4.0;

/// PI / 6
pub const PI_OVER_SIX: f64 = PI / 6.0;

/// 3 * PI / 2
pub const THREE_PI_OVER_TWO: f64 = 3.0 * PI / 2.0;

/// 1e-1 epsilon
pub const EPSILON1: f64 = 1e-1;
/// 1e-2 epsilon
pub const EPSILON2: f64 = 1e-2;
/// 1e-3 epsilon
pub const EPSILON3: f64 = 1e-3;
/// 1e-4 epsilon
pub const EPSILON4: f64 = 1e-4;
/// 1e-5 epsilon
pub const EPSILON5: f64 = 1e-5;
/// 1e-6 epsilon
pub const EPSILON6: f64 = 1e-6;
/// 1e-7 epsilon
pub const EPSILON7: f64 = 1e-7;
/// 1e-8 epsilon
pub const EPSILON8: f64 = 1e-8;
/// 1e-9 epsilon
pub const EPSILON9: f64 = 1e-9;
/// 1e-10 epsilon
pub const EPSILON10: f64 = 1e-10;
/// 1e-11 epsilon
pub const EPSILON11: f64 = 1e-11;
/// 1e-12 epsilon
pub const EPSILON12: f64 = 1e-12;
/// 1e-13 epsilon
pub const EPSILON13: f64 = 1e-13;
/// 1e-14 epsilon
pub const EPSILON14: f64 = 1e-14;
/// 1e-15 epsilon
pub const EPSILON15: f64 = 1e-15;
/// 1e-16 epsilon
pub const EPSILON16: f64 = 1e-16;
/// 1e-17 epsilon
pub const EPSILON17: f64 = 1e-17;
/// 1e-18 epsilon
pub const EPSILON18: f64 = 1e-18;
/// 1e-19 epsilon
pub const EPSILON19: f64 = 1e-19;
/// 1e-20 epsilon
pub const EPSILON20: f64 = 1e-20;
/// 1e-21 epsilon
pub const EPSILON21: f64 = 1e-21;

/// The number used to determine if a value is zero.
pub const ZERO: f64 = 0.0;

/// Converts degrees to radians.
/// Maps to CesiumMath.toRadians
#[inline]
pub fn to_radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

/// Converts radians to degrees.
/// Maps to CesiumMath.toDegrees
#[inline]
pub fn to_degrees(radians: f64) -> f64 {
    radians * 180.0 / PI
}

/// Constrains a value to lie between two values.
/// Maps to CesiumMath.clamp
#[inline]
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// Returns the sign of the value: 1 if positive, -1 if negative, 0 if zero.
/// Maps to CesiumMath.sign
#[inline]
pub fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Returns the sign of the value using signNotZero:
/// 1 if >= 0, -1 if < 0.
/// Maps to CesiumMath.signNotZero
#[inline]
pub fn sign_not_zero(value: f64) -> f64 {
    if value < 0.0 { -1.0 } else { 1.0 }
}

/// Linearly interpolates between two values.
/// Maps to CesiumMath.lerp
#[inline]
pub fn lerp(p: f64, q: f64, time: f64) -> f64 {
    (1.0 - time) * p + time * q
}

/// Returns the angle in radians normalized to [-PI, PI].
/// Maps to CesiumMath.negativePiToPi
pub fn negative_pi_to_pi(angle: f64) -> f64 {
    if (-PI..=PI).contains(&angle) {
        return angle;
    }
    angle - TWO_PI * ((angle + PI) / TWO_PI).floor()
}

/// Returns the angle in radians normalized to [0, 2*PI].
/// Maps to CesiumMath.zeroToTwoPi
pub fn zero_to_two_pi(angle: f64) -> f64 {
    let mod_val = angle % TWO_PI;
    if (mod_val.abs() < EPSILON14 && angle.abs() > EPSILON14) || mod_val < 0.0 {
        mod_val + TWO_PI
    } else {
        mod_val
    }
}

/// Determines if two values are equal within an epsilon.
/// Maps to CesiumMath.equalsEpsilon
#[inline]
pub fn equals_epsilon(left: f64, right: f64, relative_epsilon: f64, absolute_epsilon: f64) -> bool {
    let diff = (left - right).abs();
    diff <= absolute_epsilon || diff <= relative_epsilon * left.abs().max(right.abs())
}

/// Computes the factorial of a number.
pub fn factorial(n: u32) -> u64 {
    (1..=n as u64).product()
}

/// Computes the cosine of the angle between two vectors given their magnitudes and dot product.
#[inline]
pub fn cos_angle(dot: f64, mag_a: f64, mag_b: f64) -> f64 {
    clamp(dot / (mag_a * mag_b), -1.0, 1.0)
}

/// Converts a longitude in radians to the range [-PI, PI].
#[inline]
pub fn convert_longitude_range(longitude: f64) -> f64 {
    negative_pi_to_pi(longitude)
}

/// Computes the log base of a value.
#[inline]
pub fn log_base(value: f64, base: f64) -> f64 {
    value.ln() / base.ln()
}

/// Computes the cube root of a value.
#[inline]
pub fn cbrt(value: f64) -> f64 {
    value.cbrt()
}

/// Computes the remainder of a division using floored division.
#[inline]
pub fn mod_f64(m: f64, n: f64) -> f64 {
    ((m % n) + n) % n
}

/// Determines if a value is within the given epsilon of zero.
#[inline]
pub fn is_zero(value: f64) -> bool {
    value.abs() < EPSILON14
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_radians() {
        assert!((to_radians(180.0) - PI).abs() < EPSILON15);
        assert!((to_radians(90.0) - PI_OVER_TWO).abs() < EPSILON15);
        assert!((to_radians(0.0)).abs() < EPSILON15);
    }

    #[test]
    fn test_to_degrees() {
        assert!((to_degrees(PI) - 180.0).abs() < EPSILON13);
        assert!((to_degrees(PI_OVER_TWO) - 90.0).abs() < EPSILON13);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_zero_to_two_pi() {
        assert!((zero_to_two_pi(0.0)).abs() < EPSILON14);
        assert!((zero_to_two_pi(TWO_PI) - TWO_PI).abs() < EPSILON14 || zero_to_two_pi(TWO_PI).abs() < EPSILON14);
        assert!((zero_to_two_pi(-PI_OVER_TWO) - THREE_PI_OVER_TWO).abs() < EPSILON14);
    }

    #[test]
    fn test_negative_pi_to_pi() {
        assert!((negative_pi_to_pi(0.0)).abs() < EPSILON14);
        assert!((negative_pi_to_pi(PI) - PI).abs() < EPSILON14);
        assert!((negative_pi_to_pi(THREE_PI_OVER_TWO) - (-PI_OVER_TWO)).abs() < EPSILON14);
    }
}
