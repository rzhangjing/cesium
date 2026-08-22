//! Ported from packages/engine/Source/Core/Math.js
//!
//! Math functions. `CesiumMath` mirrors the JS `CesiumMath` singleton
//! object as a unit struct with associated constants and functions.

use crate::developer_error::throw_developer_error;
use crate::mersenne_twister::MersenneTwister;
use std::sync::Mutex;

/// Math functions.
///
/// Mirrors `CesiumMath` (`@exports CesiumMath`, `@alias Math`).
#[allow(non_snake_case)]
pub struct CesiumMath;

impl CesiumMath {
    /// 0.1
    pub const EPSILON1: f64 = 0.1;
    /// 0.01
    pub const EPSILON2: f64 = 0.01;
    /// 0.001
    pub const EPSILON3: f64 = 0.001;
    /// 0.0001
    pub const EPSILON4: f64 = 0.0001;
    /// 0.00001
    pub const EPSILON5: f64 = 0.00001;
    /// 0.000001
    pub const EPSILON6: f64 = 0.000001;
    /// 0.0000001
    pub const EPSILON7: f64 = 0.0000001;
    /// 0.00000001
    pub const EPSILON8: f64 = 0.00000001;
    /// 0.000000001
    pub const EPSILON9: f64 = 0.000000001;
    /// 0.0000000001
    pub const EPSILON10: f64 = 0.0000000001;
    /// 0.00000000001
    pub const EPSILON11: f64 = 0.00000000001;
    /// 0.000000000001
    pub const EPSILON12: f64 = 0.000000000001;
    /// 0.0000000000001
    pub const EPSILON13: f64 = 0.0000000000001;
    /// 0.00000000000001
    pub const EPSILON14: f64 = 0.00000000000001;
    /// 0.000000000000001
    pub const EPSILON15: f64 = 0.000000000000001;
    /// 0.0000000000000001
    pub const EPSILON16: f64 = 0.0000000000000001;
    /// 0.00000000000000001
    pub const EPSILON17: f64 = 0.00000000000000001;
    /// 0.000000000000000001
    pub const EPSILON18: f64 = 0.000000000000000001;
    /// 0.0000000000000000001
    pub const EPSILON19: f64 = 0.0000000000000000001;
    /// 0.00000000000000000001
    pub const EPSILON20: f64 = 0.00000000000000000001;
    /// 0.000000000000000000001
    pub const EPSILON21: f64 = 0.000000000000000000001;

    /// The gravitational parameter of the Earth in meters cubed
    /// per second squared as defined by the WGS84 model: 3.986004418e14
    pub const GRAVITATIONALPARAMETER: f64 = 3.986004418e14;

    /// Radius of the sun in meters: 6.955e8
    pub const SOLAR_RADIUS: f64 = 6.955e8;

    /// The mean radius of the moon, according to the "Report of the
    /// IAU/IAG Working Group on Cartographic Coordinates and Rotational
    /// Elements of the Planets and satellites: 2000",
    /// Celestial Mechanics 82: 83-110, 2002.
    pub const LUNAR_RADIUS: f64 = 1737400.0;

    /// 64 * 1024
    pub const SIXTY_FOUR_KILOBYTES: f64 = 64.0 * 1024.0;

    /// 4 * 1024 * 1024 * 1024
    pub const FOUR_GIGABYTES: f64 = 4.0 * 1024.0 * 1024.0 * 1024.0;

    /// pi
    pub const PI: f64 = std::f64::consts::PI;
    /// 1/pi
    pub const ONE_OVER_PI: f64 = 1.0 / std::f64::consts::PI;
    /// pi/2
    pub const PI_OVER_TWO: f64 = std::f64::consts::PI / 2.0;
    /// pi/3
    pub const PI_OVER_THREE: f64 = std::f64::consts::PI / 3.0;
    /// pi/4
    pub const PI_OVER_FOUR: f64 = std::f64::consts::PI / 4.0;
    /// pi/6
    pub const PI_OVER_SIX: f64 = std::f64::consts::PI / 6.0;
    /// 3pi/2
    pub const THREE_PI_OVER_TWO: f64 = (3.0 * std::f64::consts::PI) / 2.0;
    /// 2pi
    pub const TWO_PI: f64 = 2.0 * std::f64::consts::PI;
    /// 1/2pi
    pub const ONE_OVER_TWO_PI: f64 = 1.0 / (2.0 * std::f64::consts::PI);
    /// The number of radians in a degree.
    pub const RADIANS_PER_DEGREE: f64 = std::f64::consts::PI / 180.0;
    /// The number of degrees in a radian.
    pub const DEGREES_PER_RADIAN: f64 = 180.0 / std::f64::consts::PI;
    /// The number of radians in an arc second.
    pub const RADIANS_PER_ARCSECOND: f64 =
        std::f64::consts::PI / 180.0 / 3600.0;

    /// Returns the sign of the value; 1 if the value is positive, -1 if the
    /// value is negative, or 0 if the value is 0.
    ///
    /// Mirrors `Math.sign` (NaN is passed through).
    pub fn sign(value: f64) -> f64 {
        if value == 0.0 || value != value {
            // zero or NaN
            return value;
        }
        if value > 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    /// Returns 1.0 if the given value is positive or zero, and -1.0 if it is
    /// negative. This is similar to `sign` except that returns 1.0 instead
    /// of 0.0 when the input value is 0.0.
    pub fn sign_not_zero(value: f64) -> f64 {
        if value < 0.0 {
            -1.0
        } else {
            1.0
        }
    }

    /// Converts a scalar value in the range [-1.0, 1.0] to a SNORM in the
    /// range [0, range_maximum].
    ///
    /// `range_maximum` defaults to 255 (`undefined` in JS).
    pub fn to_snorm(value: f64, range_maximum: Option<f64>) -> f64 {
        let range_maximum = range_maximum.unwrap_or(255.0);
        js_round((CesiumMath::clamp(value, -1.0, 1.0) * 0.5 + 0.5) * range_maximum)
    }

    /// Converts a SNORM value in the range [0, range_maximum] to a scalar in
    /// the range [-1.0, 1.0].
    ///
    /// `range_maximum` defaults to 255 (`undefined` in JS).
    pub fn from_snorm(value: f64, range_maximum: Option<f64>) -> f64 {
        let range_maximum = range_maximum.unwrap_or(255.0);
        (CesiumMath::clamp(value, 0.0, range_maximum) / range_maximum) * 2.0 - 1.0
    }

    /// Converts a scalar value in the range [range_minimum, range_maximum]
    /// to a scalar in the range [0.0, 1.0].
    pub fn normalize(value: f64, range_minimum: f64, range_maximum: f64) -> f64 {
        let range_maximum = (range_maximum - range_minimum).max(0.0);
        if range_maximum == 0.0 {
            0.0
        } else {
            CesiumMath::clamp((value - range_minimum) / range_maximum, 0.0, 1.0)
        }
    }

    /// Returns the hyperbolic sine of a number.
    pub fn sinh(value: f64) -> f64 {
        value.sinh()
    }

    /// Returns the hyperbolic cosine of a number.
    pub fn cosh(value: f64) -> f64 {
        value.cosh()
    }

    /// Computes the linear interpolation of two values.
    pub fn lerp(p: f64, q: f64, time: f64) -> f64 {
        (1.0 - time) * p + time * q
    }

    /// Converts degrees to radians.
    pub fn to_radians(degrees: f64) -> f64 {
        degrees * CesiumMath::RADIANS_PER_DEGREE
    }

    /// Converts radians to degrees.
    pub fn to_degrees(radians: f64) -> f64 {
        radians * CesiumMath::DEGREES_PER_RADIAN
    }

    /// Converts a longitude value, in radians, to the range
    /// [`-Math.PI`, `Math.PI`).
    pub fn convert_longitude_range(angle: f64) -> f64 {
        let two_pi = CesiumMath::TWO_PI;

        let simplified = angle - (angle / two_pi).floor() * two_pi;

        if simplified < -std::f64::consts::PI {
            return simplified + two_pi;
        }
        if simplified >= std::f64::consts::PI {
            return simplified - two_pi;
        }

        simplified
    }

    /// Convenience function that clamps a latitude value, in radians, to the
    /// range [`-Math.PI/2`, `Math.PI/2`).
    pub fn clamp_to_latitude_range(angle: f64) -> f64 {
        CesiumMath::clamp(angle, -1.0 * CesiumMath::PI_OVER_TWO, CesiumMath::PI_OVER_TWO)
    }

    /// Produces an angle in the range -Pi <= angle <= Pi which is equivalent
    /// to the provided angle.
    pub fn negative_pi_to_pi(angle: f64) -> f64 {
        if angle >= -CesiumMath::PI && angle <= CesiumMath::PI {
            // Early exit if the input is already inside the range. This avoids
            // unnecessary math which could introduce floating point error.
            return angle;
        }
        CesiumMath::zero_to_two_pi(angle + CesiumMath::PI) - CesiumMath::PI
    }

    /// Produces an angle in the range 0 <= angle <= 2Pi which is equivalent
    /// to the provided angle.
    pub fn zero_to_two_pi(angle: f64) -> f64 {
        if angle >= 0.0 && angle <= CesiumMath::TWO_PI {
            // Early exit if the input is already inside the range. This avoids
            // unnecessary math which could introduce floating point error.
            return angle;
        }
        let modulo = CesiumMath::r#mod(angle, CesiumMath::TWO_PI);
        if modulo.abs() < CesiumMath::EPSILON14 && angle.abs() > CesiumMath::EPSILON14 {
            return CesiumMath::TWO_PI;
        }
        modulo
    }

    /// The modulo operation that also works for negative dividends.
    pub fn r#mod(m: f64, n: f64) -> f64 {
        if cfg!(debug_assertions) {
            if n == 0.0 {
                throw_developer_error("divisor cannot be 0.");
            }
        }
        if CesiumMath::sign(m) == CesiumMath::sign(n) && m.abs() < n.abs() {
            // Early exit if the input does not need to be modded. This avoids
            // unnecessary math which could introduce floating point error.
            return m;
        }

        ((m % n) + n) % n
    }

    /// Determines if two values are equal using an absolute or relative
    /// tolerance test.
    ///
    /// `relative_epsilon` defaults to 0 and `absolute_epsilon` defaults to
    /// `relative_epsilon` (`undefined` in JS).
    pub fn equals_epsilon(
        left: f64,
        right: f64,
        relative_epsilon: Option<f64>,
        absolute_epsilon: Option<f64>,
    ) -> bool {
        let relative_epsilon = relative_epsilon.unwrap_or(0.0);
        let absolute_epsilon = absolute_epsilon.unwrap_or(relative_epsilon);
        let abs_diff = (left - right).abs();
        abs_diff <= absolute_epsilon
            || abs_diff <= relative_epsilon * left.abs().max(right.abs())
    }

    /// Determines if the left value is less than the right value. If the two
    /// values are within `absolute_epsilon` of each other, they are
    /// considered equal and this function returns false.
    pub fn less_than(left: f64, right: f64, absolute_epsilon: f64) -> bool {
        left - right < -absolute_epsilon
    }

    /// Determines if the left value is less than or equal to the right value.
    pub fn less_than_or_equals(left: f64, right: f64, absolute_epsilon: f64) -> bool {
        left - right < absolute_epsilon
    }

    /// Determines if the left value is greater the right value.
    pub fn greater_than(left: f64, right: f64, absolute_epsilon: f64) -> bool {
        left - right > absolute_epsilon
    }

    /// Determines if the left value is greater than or equal to the right
    /// value.
    pub fn greater_than_or_equals(left: f64, right: f64, absolute_epsilon: f64) -> bool {
        left - right > -absolute_epsilon
    }

    /// Computes the factorial of the provided number.
    ///
    /// DEVIATION: JS returns `undefined` for non-integer `n` (array lookup
    /// with a fractional key); Rust returns `NaN` instead. Registered in
    /// `docs/deviations.md`.
    pub fn factorial(n: f64) -> f64 {
        if cfg!(debug_assertions) {
            if n < 0.0 {
                throw_developer_error("A number greater than or equal to 0 is required.");
            }
        }

        let mut factorials = FACTORIALS.lock().expect("factorials lock poisoned");

        if n.fract() != 0.0 || n.is_infinite() {
            // DEVIATION: mirrors `factorials[n] === undefined` in JS.
            drop(factorials);
            return f64::NAN;
        }

        // Hold the lock across the read as well: the cache is process-global
        // and parallel tests would otherwise observe a partially extended
        // cache (JS is single-threaded, so this has no semantic effect).
        if factorials.is_empty() {
            factorials.push(1.0);
        }
        let length = factorials.len();
        if n >= length as f64 {
            let mut sum = factorials[length - 1];
            for i in length..=(n as usize) {
                let next = sum * i as f64;
                factorials.push(next);
                sum = next;
            }
        }
        factorials[n as usize]
    }

    /// Increments a number with a wrapping to a minimum value if the number
    /// exceeds the maximum value.
    ///
    /// `minimum_value` defaults to 0.0 (`undefined` in JS).
    pub fn increment_wrap(n: f64, maximum_value: f64, minimum_value: Option<f64>) -> f64 {
        let minimum_value = minimum_value.unwrap_or(0.0);

        if cfg!(debug_assertions) {
            if maximum_value <= minimum_value {
                throw_developer_error("maximumValue must be greater than minimumValue.");
            }
        }

        let mut n = n + 1.0;
        if n > maximum_value {
            n = minimum_value;
        }
        n
    }

    /// Determines if a non-negative integer is a power of two.
    /// The maximum allowed input is (2^32)-1 due to 32-bit bitwise operator
    /// limitation in Javascript.
    pub fn is_power_of_two(n: f64) -> bool {
        if cfg!(debug_assertions) {
            if n < 0.0 || n > 4294967295.0 {
                throw_developer_error("A number between 0 and (2^32)-1 is required.");
            }
        }

        // JS bitwise `&` coerces via Int32; bit pattern is preserved.
        n != 0.0 && (to_int32(n) & to_int32(n - 1.0)) == 0
    }

    /// Computes the next power-of-two integer greater than or equal to the
    /// provided non-negative integer.
    /// The maximum allowed input is 2^31 due to 32-bit bitwise operator
    /// limitation in Javascript.
    pub fn next_power_of_two(n: f64) -> f64 {
        if cfg!(debug_assertions) {
            if n < 0.0 || n > 2147483648.0 {
                throw_developer_error("A number between 0 and 2^31 is required.");
            }
        }

        // From http://graphics.stanford.edu/~seander/bithacks.html#RoundUpPowerOf2
        let mut n = n - 1.0;
        let mut m = to_int32(n);
        m |= m >> 1;
        m |= m >> 2;
        m |= m >> 4;
        m |= m >> 8;
        m |= m >> 16;
        n = m as f64 + 1.0;

        n
    }

    /// Computes the previous power-of-two integer less than or equal to the
    /// provided non-negative integer.
    /// The maximum allowed input is (2^32)-1 due to 32-bit bitwise operator
    /// limitation in Javascript.
    pub fn previous_power_of_two(n: f64) -> f64 {
        if cfg!(debug_assertions) {
            if n < 0.0 || n > 4294967295.0 {
                throw_developer_error("A number between 0 and (2^32)-1 is required.");
            }
        }

        // JS `>> 32` shifts by 0 (shift count mod 32), so it is a no-op.
        let mut m = to_int32(n);
        m |= m >> 1;
        m |= m >> 2;
        m |= m >> 4;
        m |= m >> 8;
        m |= m >> 16;

        // The previous bitwise operations implicitly convert to signed
        // 32-bit. `>>>` in JS converts to unsigned.
        let m = m as u32;
        let n = m.wrapping_sub(m >> 1);

        n as f64
    }

    /// Constraint a value to lie between two values.
    pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
        // Check.typeOf.number checks are statically guaranteed in Rust.
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    /// Sets the seed used by the random number generator
    /// in `next_random_number`.
    pub fn set_random_number_seed(seed: f64) {
        let mut generator = RANDOM_NUMBER_GENERATOR
            .lock()
            .expect("random number generator lock poisoned");
        *generator = Some(MersenneTwister::new(Some(seed)));
    }

    /// Generates a random floating point number in the range of [0.0, 1.0)
    /// using a Mersenne twister.
    pub fn next_random_number() -> f64 {
        let mut generator = RANDOM_NUMBER_GENERATOR
            .lock()
            .expect("random number generator lock poisoned");
        // Lazy `new MersenneTwister()` (time-seeded), mirroring the
        // module-level initialization in Math.js.
        generator.get_or_insert_with(|| MersenneTwister::new(None)).random()
    }

    /// Generates a random number between two numbers.
    pub fn random_between(min: f64, max: f64) -> f64 {
        CesiumMath::next_random_number() * (max - min) + min
    }

    /// Computes `acos(value)`, but first clamps `value` to the range
    /// [-1.0, 1.0] so that the function will never return NaN.
    pub fn acos_clamped(value: f64) -> f64 {
        (CesiumMath::clamp(value, -1.0, 1.0)).acos()
    }

    /// Computes `asin(value)`, but first clamps `value` to the range
    /// [-1.0, 1.0] so that the function will never return NaN.
    pub fn asin_clamped(value: f64) -> f64 {
        (CesiumMath::clamp(value, -1.0, 1.0)).asin()
    }

    /// Finds the chord length between two points given the circle's radius
    /// and the angle between the points.
    pub fn chord_length(angle: f64, radius: f64) -> f64 {
        2.0 * radius * (angle * 0.5).sin()
    }

    /// Finds the logarithm of a number to a base.
    pub fn log_base(number: f64, base: f64) -> f64 {
        number.ln() / base.ln()
    }

    /// Finds the cube root of a number.
    pub fn cbrt(number: f64) -> f64 {
        number.cbrt()
    }

    /// Finds the base 2 logarithm of a number.
    pub fn log2(number: f64) -> f64 {
        number.log2()
    }

    /// Calculate the fog impact at a given distance. Useful for culling.
    /// Matches the equation in `fog.glsl`
    pub fn fog(distance_to_camera: f64, density: f64) -> f64 {
        let scalar = distance_to_camera * density;
        1.0 - (-(scalar * scalar)).exp()
    }

    /// Computes a fast approximation of Atan for input in the range [-1, 1].
    ///
    /// Based on Michal Drobot's approximation from ShaderFastLibs,
    /// which in turn is based on "Efficient approximations for the
    /// arctangent function," Rajan, S. Sichun Wang Inkol, R. Joyal, A.,
    /// May 2006. Adapted from ShaderFastLibs under MIT License.
    pub fn fast_approximate_atan(x: f64) -> f64 {
        // Check.typeOf.number is statically guaranteed in Rust.
        x * (-0.1784 * x.abs() - 0.0663 * x * x + 1.0301)
    }

    /// Computes a fast approximation of Atan2(x, y) for arbitrary input
    /// scalars.
    ///
    /// Range reduction math based on nvidia's cg reference implementation:
    /// http://developer.download.nvidia.com/cg/atan2.html
    pub fn fast_approximate_atan2(x: f64, y: f64) -> f64 {
        // atan approximations are usually only reliable over [-1, 1]
        // So reduce the range by flipping whether x or y is on top based on
        // which is bigger.
        let mut opposite;
        let t = x.abs(); // t used as swap and atan result.
        opposite = y.abs();
        let adjacent = t.max(opposite);
        opposite = t.min(opposite);

        let opposite_over_adjacent = opposite / adjacent;
        if cfg!(debug_assertions) {
            if opposite_over_adjacent.is_nan() {
                throw_developer_error("either x or y must be nonzero");
            }
        }
        let mut t = CesiumMath::fast_approximate_atan(opposite_over_adjacent);

        // Undo range reduction
        t = if y.abs() > x.abs() {
            CesiumMath::PI_OVER_TWO - t
        } else {
            t
        };
        t = if x < 0.0 { CesiumMath::PI - t } else { t };
        t = if y < 0.0 { -t } else { t };
        t
    }
}

/// JS `ToInt32` conversion (JS bitwise operators coerce operands).
fn to_int32(x: f64) -> i32 {
    if x.is_nan() || x.is_infinite() {
        return 0;
    }
    let x = x.trunc();
    let mut m = x % 4_294_967_296.0;
    if m < 0.0 {
        m += 4_294_967_296.0;
    }
    // DEVIATION-free note: `as u32` then `as i32` preserves the bit
    // pattern; a direct `f64 as i32` would saturate instead of wrapping.
    m as u32 as i32
}

/// `Math.round` semantics: `floor(x + 0.5)` (half towards +infinity).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Mirrors the module-level `const factorials = [1]` cache in Math.js.
/// Seeded lazily with `1.0` on first access (`vec!` is not usable in a
/// `static` initializer).
static FACTORIALS: Mutex<Vec<f64>> = Mutex::new(Vec::new());

/// Mirrors the module-level `let randomNumberGenerator = new MersenneTwister()`,
/// lazily initialized on first use (the JS original is time-seeded at
/// module load; `Option` keeps this `const`-constructible).
static RANDOM_NUMBER_GENERATOR: Mutex<Option<MersenneTwister>> = Mutex::new(None);
