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

/// The mean radius of the Moon, in meters (IAU 2009). Used by CesiumJS
/// `Ellipsoid.MOON` (a sphere of this radius). Maps to `CesiumMath.LUNAR_RADIUS`.
pub const LUNAR_RADIUS: f64 = 1737400.0;

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

/// Returns the sign of the value: 1 if positive, -1 if negative, 0 if zero, NaN if NaN.
/// Maps to CesiumMath.sign
#[inline]
pub fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value // preserves 0.0, -0.0, and NaN
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
    (angle + PI).rem_euclid(TWO_PI) - PI
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

/// Computes the chord length of a circle given an angle and radius.
/// Maps to CesiumMath.chordLength
#[inline]
pub fn chord_length(angle: f64, radius: f64) -> f64 {
    2.0 * radius * (angle * 0.5).sin()
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

/// Computes the base 2 logarithm of a number.
/// Maps to CesiumMath.log2 (`Math.log(number) * Math.LOG2E`).
#[inline]
pub fn log2(number: f64) -> f64 {
    number.ln() * std::f64::consts::LOG2_E
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

/// Converts a scalar in the range [-1.0, 1.0] to a SNORM in [0, range_maximum].
/// Maps to CesiumMath.toSNorm
#[inline]
pub fn to_snorm(value: f64, range_maximum: f64) -> f64 {
    ((clamp(value, -1.0, 1.0) * 0.5 + 0.5) * range_maximum).round()
}

/// Converts a SNORM value in [0, range_maximum] to a scalar in [-1.0, 1.0].
/// Maps to CesiumMath.fromSNorm
#[inline]
pub fn from_snorm(value: f64, range_maximum: f64) -> f64 {
    (clamp(value, 0.0, range_maximum) / range_maximum) * 2.0 - 1.0
}

/// Normalizes a value from [range_minimum, range_maximum] to [0.0, 1.0].
/// Maps to CesiumMath.normalize
#[inline]
pub fn normalize(value: f64, range_minimum: f64, range_maximum: f64) -> f64 {
    let range = (range_maximum - range_minimum).max(0.0);
    if range == 0.0 {
        0.0
    } else {
        clamp((value - range_minimum) / range, 0.0, 1.0)
    }
}

/// Clamps an angle to the latitude range [-PI/2, PI/2].
/// Maps to CesiumMath.clampToLatitudeRange
#[inline]
pub fn clamp_to_latitude_range(angle: f64) -> f64 {
    clamp(angle, -PI_OVER_TWO, PI_OVER_TWO)
}

/// Determines if left < right, considering values within epsilon as equal.
/// Maps to CesiumMath.lessThan
#[inline]
pub fn less_than(left: f64, right: f64, absolute_epsilon: f64) -> bool {
    left - right < -absolute_epsilon
}

/// Determines if left <= right, considering values within epsilon as equal.
/// Maps to CesiumMath.lessThanOrEquals
#[inline]
pub fn less_than_or_equals(left: f64, right: f64, absolute_epsilon: f64) -> bool {
    left - right < absolute_epsilon
}

/// Determines if left > right, considering values within epsilon as equal.
/// Maps to CesiumMath.greaterThan
#[inline]
pub fn greater_than(left: f64, right: f64, absolute_epsilon: f64) -> bool {
    left - right > absolute_epsilon
}

/// Determines if left >= right, considering values within epsilon as equal.
/// Maps to CesiumMath.greaterThanOrEquals
#[inline]
pub fn greater_than_or_equals(left: f64, right: f64, absolute_epsilon: f64) -> bool {
    left - right > -absolute_epsilon
}

/// Increments n and wraps to minimum_value when exceeding maximum_value.
/// Maps to CesiumMath.incrementWrap
#[inline]
pub fn increment_wrap(n: i64, maximum_value: i64, minimum_value: i64) -> i64 {
    let n = n + 1;
    if n > maximum_value { minimum_value } else { n }
}

/// Determines if a non-negative integer is a power of two.
/// Maps to CesiumMath.isPowerOfTwo
#[inline]
pub fn is_power_of_two(n: u32) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Computes the next power-of-two >= n.
/// Maps to CesiumMath.nextPowerOfTwo
pub fn next_power_of_two(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut v = n - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

/// Computes the previous power-of-two <= n.
/// Maps to CesiumMath.previousPowerOfTwo
pub fn previous_power_of_two(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut v = n;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v - (v >> 1)
}

/// Computes acos(clamp(value, -1, 1)), never returns NaN.
/// Maps to CesiumMath.acosClamped
#[inline]
pub fn acos_clamped(value: f64) -> f64 {
    clamp(value, -1.0, 1.0).acos()
}

/// Computes asin(clamp(value, -1, 1)), never returns NaN.
/// Maps to CesiumMath.asinClamped
#[inline]
pub fn asin_clamped(value: f64) -> f64 {
    clamp(value, -1.0, 1.0).asin()
}

/// Fast approximate atan using polynomial approximation.
/// Maps to CesiumMath.fastApproximateAtan
#[inline]
pub fn fast_approximate_atan(x: f64) -> f64 {
    x * (-0.1784 * x.abs() - 0.0663 * x * x + 1.0301)
}

/// Fast approximate atan2 using range reduction + fast_approximate_atan.
/// Maps to CesiumMath.fastApproximateAtan2
pub fn fast_approximate_atan2(x: f64, y: f64) -> f64 {
    let t = x.abs();
    let opposite = y.abs();
    let adjacent = t.max(opposite);
    let opposite = t.min(opposite);
    let opposite_over_adjacent = opposite / adjacent;
    let mut t = fast_approximate_atan(opposite_over_adjacent);
    // Undo range reduction
    t = if y.abs() > x.abs() { PI_OVER_TWO - t } else { t };
    t = if x < 0.0 { PI - t } else { t };
    if y < 0.0 { -t } else { t }
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
