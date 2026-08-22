//! Mirrors packages/engine/Specs/Core/MathSpec.js

use cesium_core::math::CesiumMath;
use cesium_test_utils::{assert_approx_eq_f64, assert_epsilon_eq_f64, expect_to_throw_dev_error};

const PI: f64 = std::f64::consts::PI;

// describe("Core/Math")

#[test]
fn sign_of_negative_two() {
    assert_eq!(CesiumMath::sign(-2.0), -1.0);
}

#[test]
fn sign_of_two() {
    assert_eq!(CesiumMath::sign(2.0), 1.0);
}

#[test]
fn sign_of_zero() {
    assert_eq!(CesiumMath::sign(0.0), 0.0);
}

#[test]
fn sign_of_negative_zero() {
    // JS `Math.sign(-0)` returns `-0`; mirrored bit-exactly.
    assert_eq!(CesiumMath::sign(-0.0).to_bits(), (-0.0f64).to_bits());
}

#[test]
fn sign_of_nan() {
    assert!(CesiumMath::sign(f64::NAN).is_nan());
}

#[test]
fn sign_not_zero_of_negative_two() {
    assert_eq!(CesiumMath::sign_not_zero(-2.0), -1.0);
}

#[test]
fn sign_not_zero_of_two() {
    assert_eq!(CesiumMath::sign_not_zero(2.0), 1.0);
}

#[test]
fn sign_not_zero_of_zero() {
    assert_eq!(CesiumMath::sign_not_zero(0.0), 1.0);
}

//////////////////////////////////////////////////////////////////////

#[test]
fn to_snorm_negative_one() {
    assert_eq!(CesiumMath::to_snorm(-1.0, None), 0.0);
}

#[test]
fn to_snorm_one() {
    assert_eq!(CesiumMath::to_snorm(1.0, None), 255.0);
}

#[test]
fn to_snorm_below_negative_one() {
    assert_eq!(CesiumMath::to_snorm(-1.0001, None), 0.0);
}

#[test]
fn to_snorm_above_one() {
    assert_eq!(CesiumMath::to_snorm(1.0001, None), 255.0);
}

#[test]
fn to_snorm_zero() {
    // JS Math.round(127.5) === 128
    assert_eq!(CesiumMath::to_snorm(0.0, None), 128.0);
}

#[test]
fn from_snorm_zero() {
    assert_eq!(CesiumMath::from_snorm(0.0, None), -1.0);
}

#[test]
fn from_snorm_255() {
    assert_eq!(CesiumMath::from_snorm(255.0, None), 1.0);
}

#[test]
fn from_snorm_negative_tiny() {
    assert_eq!(CesiumMath::from_snorm(-0.0001, None), -1.0);
}

#[test]
fn from_snorm_above_255() {
    assert_eq!(CesiumMath::from_snorm(255.00001, None), 1.0);
}

#[test]
fn from_snorm_128() {
    assert_eq!(CesiumMath::from_snorm(255.0 / 2.0, None), 0.0);
}

//////////////////////////////////////////////////////////////////////

#[test]
fn normalize_0_with_max_10_min_negative_10() {
    assert_eq!(CesiumMath::normalize(0.0, -10.0, 10.0), 0.5);
}

#[test]
fn normalize_10_with_max_10_min_negative_10() {
    assert_eq!(CesiumMath::normalize(10.0, -10.0, 10.0), 1.0);
}

#[test]
fn normalize_negative_10_with_max_10_min_negative_10() {
    assert_eq!(CesiumMath::normalize(-10.0, -10.0, 10.0), 0.0);
}

#[test]
fn normalize_below_min_with_max_10_min_negative_10() {
    assert_eq!(CesiumMath::normalize(-10.0001, -10.0, 10.0), 0.0);
}

#[test]
fn normalize_above_max_with_max_10_min_negative_10() {
    assert_eq!(CesiumMath::normalize(10.00001, -10.0, 10.0), 1.0);
}

//////////////////////////////////////////////////////////////////////

#[test]
fn cosh_works() {
    assert_eq!(CesiumMath::cosh(0.0), 1.0);
    assert!(CesiumMath::cosh(-1.0) > 1.0);
    assert!(CesiumMath::cosh(1.0) > 1.0);
}

#[test]
fn cosh_nan() {
    assert!(CesiumMath::cosh(f64::NAN).is_nan());
}

#[test]
fn cosh_infinity() {
    assert_eq!(CesiumMath::cosh(f64::INFINITY), f64::INFINITY);
    assert_eq!(CesiumMath::cosh(f64::NEG_INFINITY), f64::INFINITY);
}

#[test]
fn sinh_works() {
    assert_eq!(CesiumMath::sinh(0.0), 0.0);
    assert!(CesiumMath::sinh(-1.0) < 1.0);
    assert!(CesiumMath::sinh(1.0) > 1.0);
}

#[test]
fn sinh_nan() {
    assert!(CesiumMath::sinh(f64::NAN).is_nan());
}

#[test]
fn sinh_infinity() {
    assert_eq!(CesiumMath::sinh(f64::INFINITY), f64::INFINITY);
    assert_eq!(CesiumMath::sinh(f64::NEG_INFINITY), f64::NEG_INFINITY);
}

///////////////////////////////////////////////////////////////////////

#[test]
fn lerps_at_time_0() {
    assert_eq!(CesiumMath::lerp(1.0, 2.0, 0.0), 1.0);
}

#[test]
fn lerps_at_time_half() {
    assert_eq!(CesiumMath::lerp(1.0, 2.0, 0.5), 1.5);
}

#[test]
fn lerps_at_time_1() {
    assert_eq!(CesiumMath::lerp(1.0, 2.0, 1.0), 2.0);
}

///////////////////////////////////////////////////////////////////////

#[test]
fn to_radians_works() {
    assert_eq!(CesiumMath::to_radians(360.0), 2.0 * PI);
}

#[test]
#[ignore = "toRadians throws for undefined — statically impossible in Rust"]
fn to_radians_throws_for_undefined() {}

#[test]
fn to_degrees_works() {
    assert_eq!(CesiumMath::to_degrees(PI), 180.0);
}

#[test]
#[ignore = "toDegrees throws for undefined — statically impossible in Rust"]
fn to_degrees_throws_for_undefined() {}

#[test]
fn convert_longitude_range_1() {
    assert_approx_eq_f64!(
        CesiumMath::convert_longitude_range(CesiumMath::THREE_PI_OVER_TWO),
        -CesiumMath::PI_OVER_TWO,
        0.0,
        CesiumMath::EPSILON16
    );
}

#[test]
fn convert_longitude_range_2() {
    assert_approx_eq_f64!(
        CesiumMath::convert_longitude_range(-PI),
        -PI,
        0.0,
        CesiumMath::EPSILON16
    );
}

#[test]
fn convert_longitude_range_3() {
    assert_approx_eq_f64!(
        CesiumMath::convert_longitude_range(PI),
        -PI,
        0.0,
        CesiumMath::EPSILON16
    );
}

#[test]
#[ignore = "convertLongitudeRange throws for undefined — statically impossible in Rust"]
fn convert_longitude_range_throws_for_undefined() {}

#[test]
fn clamp_to_latitude_range_1() {
    assert_eq!(CesiumMath::clamp_to_latitude_range(PI), CesiumMath::PI_OVER_TWO);
}

#[test]
fn clamp_to_latitude_range_2() {
    assert_eq!(CesiumMath::clamp_to_latitude_range(-PI), -CesiumMath::PI_OVER_TWO);
}

#[test]
#[ignore = "clampToLatitudeRange throws for undefined — statically impossible in Rust"]
fn clamp_to_latitude_range_throws_for_undefined() {}

#[test]
fn negative_pi_to_pi_works() {
    assert_eq!(CesiumMath::negative_pi_to_pi(0.0), 0.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(PI), PI);
    assert_eq!(CesiumMath::negative_pi_to_pi(-PI), -PI);
    assert_eq!(CesiumMath::negative_pi_to_pi(PI - 1.0), PI - 1.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(-PI + 1.0), -PI + 1.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(PI - 0.1), PI - 0.1);
    assert_eq!(CesiumMath::negative_pi_to_pi(-PI + 0.1), -PI + 0.1);
    assert_approx_eq_f64!(
        CesiumMath::negative_pi_to_pi(PI + 0.1),
        -PI + 0.1,
        0.0,
        CesiumMath::EPSILON15
    );
    assert_approx_eq_f64!(
        CesiumMath::negative_pi_to_pi(-PI - 0.1),
        PI - 0.1,
        0.0,
        CesiumMath::EPSILON15
    );

    assert_eq!(CesiumMath::negative_pi_to_pi(2.0 * PI), 0.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(-2.0 * PI), 0.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(3.0 * PI), PI);
    assert_eq!(CesiumMath::negative_pi_to_pi(-3.0 * PI), PI);
    assert_eq!(CesiumMath::negative_pi_to_pi(4.0 * PI), 0.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(-4.0 * PI), 0.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(5.0 * PI), PI);
    assert_eq!(CesiumMath::negative_pi_to_pi(-5.0 * PI), PI);
    assert_eq!(CesiumMath::negative_pi_to_pi(6.0 * PI), 0.0);
    assert_eq!(CesiumMath::negative_pi_to_pi(-6.0 * PI), 0.0);
}

#[test]
#[ignore = "negativePiToPi throws for undefined — statically impossible in Rust"]
fn negative_pi_to_pi_throws_for_undefined() {}

#[test]
fn zero_to_two_pi_works() {
    assert_eq!(CesiumMath::zero_to_two_pi(0.0), 0.0);
    assert_eq!(CesiumMath::zero_to_two_pi(PI), PI);
    assert_eq!(CesiumMath::zero_to_two_pi(-PI), PI);
    assert_eq!(CesiumMath::zero_to_two_pi(PI - 1.0), PI - 1.0);
    assert_approx_eq_f64!(
        CesiumMath::zero_to_two_pi(-PI + 1.0),
        PI + 1.0,
        0.0,
        CesiumMath::EPSILON15
    );
    assert_eq!(CesiumMath::zero_to_two_pi(PI - 0.1), PI - 0.1);
    assert_approx_eq_f64!(
        CesiumMath::zero_to_two_pi(-PI + 0.1),
        PI + 0.1,
        0.0,
        CesiumMath::EPSILON15
    );
    assert_eq!(CesiumMath::zero_to_two_pi(PI + 0.1), PI + 0.1);
    assert_approx_eq_f64!(
        CesiumMath::zero_to_two_pi(-PI - 0.1),
        PI - 0.1,
        0.0,
        CesiumMath::EPSILON15
    );

    assert_eq!(CesiumMath::zero_to_two_pi(2.0 * PI), 2.0 * PI);
    assert_eq!(CesiumMath::zero_to_two_pi(-2.0 * PI), 2.0 * PI);
    assert_eq!(CesiumMath::zero_to_two_pi(3.0 * PI), PI);
    assert_eq!(CesiumMath::zero_to_two_pi(-3.0 * PI), PI);
    assert_eq!(CesiumMath::zero_to_two_pi(4.0 * PI), 2.0 * PI);
    assert_eq!(CesiumMath::zero_to_two_pi(-4.0 * PI), 2.0 * PI);
    assert_eq!(CesiumMath::zero_to_two_pi(5.0 * PI), PI);
    assert_eq!(CesiumMath::zero_to_two_pi(-5.0 * PI), PI);
    assert_eq!(CesiumMath::zero_to_two_pi(6.0 * PI), 2.0 * PI);
    assert_eq!(CesiumMath::zero_to_two_pi(-6.0 * PI), 2.0 * PI);
}

#[test]
#[ignore = "zeroToTwoPi throws for undefined — statically impossible in Rust"]
fn zero_to_two_pi_throws_for_undefined() {}

#[test]
fn mod_works() {
    assert_eq!(CesiumMath::r#mod(0.0, 1.0), 0.0);
    assert_eq!(CesiumMath::r#mod(0.1, 1.0), 0.1);
    assert_eq!(CesiumMath::r#mod(0.5, 1.0), 0.5);
    assert_eq!(CesiumMath::r#mod(1.0, 1.0), 0.0);
    assert_approx_eq_f64!(CesiumMath::r#mod(1.1, 1.0), 0.1, 0.0, CesiumMath::EPSILON15);

    assert_eq!(CesiumMath::r#mod(-0.0, 1.0), 0.0);
    assert_eq!(CesiumMath::r#mod(-0.1, 1.0), 0.9);
    assert_eq!(CesiumMath::r#mod(-0.5, 1.0), 0.5);
    assert_eq!(CesiumMath::r#mod(-1.0, 1.0), 0.0);
    assert_approx_eq_f64!(CesiumMath::r#mod(-1.1, 1.0), 0.9, 0.0, CesiumMath::EPSILON15);

    assert_eq!(CesiumMath::r#mod(0.0, -1.0), -0.0);
    assert_eq!(CesiumMath::r#mod(0.1, -1.0), -0.9);
    assert_eq!(CesiumMath::r#mod(0.5, -1.0), -0.5);
    assert_eq!(CesiumMath::r#mod(1.0, -1.0), -0.0);
    assert_approx_eq_f64!(CesiumMath::r#mod(1.1, -1.0), -0.9, 0.0, CesiumMath::EPSILON15);

    assert_eq!(CesiumMath::r#mod(-0.0, -1.0), -0.0);
    assert_eq!(CesiumMath::r#mod(-0.1, -1.0), -0.1);
    assert_eq!(CesiumMath::r#mod(-0.5, -1.0), -0.5);
    assert_eq!(CesiumMath::r#mod(-1.0, -1.0), -0.0);
    assert_approx_eq_f64!(CesiumMath::r#mod(-1.1, -1.0), -0.1, 0.0, CesiumMath::EPSILON15);
}

#[test]
fn mod_throws_for_divisor_of_0() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::r#mod(1.0, 0.0);
    });
}

#[test]
fn equals_epsilon_works() {
    assert!(CesiumMath::equals_epsilon(1.0, 1.0, Some(0.0), None));
    assert!(CesiumMath::equals_epsilon(1.0, 1.0, Some(1.0), None));
    assert!(CesiumMath::equals_epsilon(
        1.0,
        1.0 + CesiumMath::EPSILON7,
        Some(CesiumMath::EPSILON7),
        None
    ));
    assert!(!CesiumMath::equals_epsilon(
        1.0,
        1.0 + CesiumMath::EPSILON7,
        Some(CesiumMath::EPSILON9),
        None
    ));

    assert!(CesiumMath::equals_epsilon(3000000.0, 3000000.0, Some(0.0), None));
    assert!(CesiumMath::equals_epsilon(
        3000000.0,
        3000000.0,
        Some(CesiumMath::EPSILON7),
        None
    ));
    assert!(CesiumMath::equals_epsilon(
        3000000.0,
        3000000.2,
        Some(CesiumMath::EPSILON7),
        None
    ));
    assert!(!CesiumMath::equals_epsilon(
        3000000.0,
        3000000.2,
        Some(CesiumMath::EPSILON9),
        None
    ));
}

#[test]
#[ignore = "equalsEpsilon throws for undefined left — statically impossible in Rust"]
fn equals_epsilon_throws_for_undefined_left() {}

#[test]
#[ignore = "equalsEpsilon throws for undefined right — statically impossible in Rust"]
fn equals_epsilon_throws_for_undefined_right() {}

#[test]
#[ignore = "equalsEpsilon throws for undefined — statically impossible in Rust"]
fn equals_epsilon_throws_for_undefined() {}

#[test]
fn less_than_works() {
    assert!(CesiumMath::less_than(1.0, 2.0, 0.2));
    assert!(!CesiumMath::less_than(2.0, 1.0, 0.2));
    assert!(!CesiumMath::less_than(1.0, 1.0, 0.2));
    assert!(!CesiumMath::less_than(1.0, 1.2, 0.2));
    assert!(!CesiumMath::less_than(1.2, 1.0, 0.2));
}

#[test]
#[ignore = "lessThan throws for undefined left — statically impossible in Rust"]
fn less_than_throws_for_undefined_left() {}

#[test]
#[ignore = "lessThan throws for undefined right — statically impossible in Rust"]
fn less_than_throws_for_undefined_right() {}

#[test]
#[ignore = "lessThan throws for undefined absoluteEpsilon — statically impossible in Rust"]
fn less_than_throws_for_undefined_absolute_epsilon() {}

#[test]
fn less_than_or_equals_works() {
    assert!(CesiumMath::less_than_or_equals(1.0, 2.0, 0.2));
    assert!(!CesiumMath::less_than_or_equals(2.0, 1.0, 0.2));
    assert!(CesiumMath::less_than_or_equals(1.0, 1.0, 0.2));
    assert!(CesiumMath::less_than_or_equals(1.0, 1.2, 0.2));
    assert!(CesiumMath::less_than_or_equals(1.2, 1.0, 0.2));
}

#[test]
#[ignore = "lessThanOrEquals throws for undefined left — statically impossible in Rust"]
fn less_than_or_equals_throws_for_undefined_left() {}

#[test]
#[ignore = "lessThanOrEquals throws for undefined right — statically impossible in Rust"]
fn less_than_or_equals_throws_for_undefined_right() {}

#[test]
#[ignore = "lessThanOrEquals throws for undefined absoluteEpsilon — statically impossible in Rust"]
fn less_than_or_equals_throws_for_undefined_absolute_epsilon() {}

#[test]
fn greater_than_works() {
    assert!(!CesiumMath::greater_than(1.0, 2.0, 0.2));
    assert!(CesiumMath::greater_than(2.0, 1.0, 0.2));
    assert!(!CesiumMath::greater_than(1.0, 1.0, 0.2));
    assert!(!CesiumMath::greater_than(1.0, 1.2, 0.2));
    assert!(!CesiumMath::greater_than(1.2, 1.0, 0.2));
}

#[test]
#[ignore = "greaterThan throws for undefined left — statically impossible in Rust"]
fn greater_than_throws_for_undefined_left() {}

#[test]
#[ignore = "greaterThan throws for undefined right — statically impossible in Rust"]
fn greater_than_throws_for_undefined_right() {}

#[test]
#[ignore = "greaterThan throws for undefined absoluteEpsilon — statically impossible in Rust"]
fn greater_than_throws_for_undefined_absolute_epsilon() {}

#[test]
fn greater_than_or_equals_works() {
    assert!(!CesiumMath::greater_than_or_equals(1.0, 2.0, 0.2));
    assert!(CesiumMath::greater_than_or_equals(2.0, 1.0, 0.2));
    assert!(CesiumMath::greater_than_or_equals(1.0, 1.0, 0.2));
    assert!(CesiumMath::greater_than_or_equals(1.0, 1.2, 0.2));
    assert!(CesiumMath::greater_than_or_equals(1.2, 1.0, 0.2));
}

#[test]
#[ignore = "greaterThanOrEquals throws for undefined left — statically impossible in Rust"]
fn greater_than_or_equals_throws_for_undefined_left() {}

#[test]
#[ignore = "greaterThanOrEquals throws for undefined right — statically impossible in Rust"]
fn greater_than_or_equals_throws_for_undefined_right() {}

#[test]
#[ignore = "greaterThanOrEquals throws for undefined absoluteEpsilon — statically impossible in Rust"]
fn greater_than_or_equals_throws_for_undefined_absolute_epsilon() {}

#[test]
fn factorial_produces_the_correct_results() {
    let factorials: [f64; 25] = [
        1.0,
        1.0,
        2.0,
        6.0,
        24.0,
        120.0,
        720.0,
        5040.0,
        40320.0,
        362880.0,
        3628800.0,
        39916800.0,
        479001600.0,
        6227020800.0,
        87178291200.0,
        1307674368000.0,
        20922789888000.0,
        355687428096000.0,
        6402373705728000.0,
        121645100408832000.0,
        2432902008176640000.0,
        51090942171709440000.0,
        1124000727777607680000.0,
        25852016738884976640000.0,
        620448401733239439360000.0,
    ];

    let length = factorials.len();
    let mut indices: Vec<usize> = (0..length).collect();

    // Randomize the indices array (deterministic LCG standing in for
    // `Math.random()`; the shuffled order exercises the cache extension).
    let mut seed: u64 = 0x5EED_5EED_5EED_5EED;
    let mut next_random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as usize) % length
    };
    for i in 0..length {
        let tmp = indices[i];
        let random_index = next_random();
        indices[i] = indices[random_index];
        indices[random_index] = tmp;
    }

    for i in 0..length {
        let index = indices[i];
        assert_eq!(CesiumMath::factorial(index as f64), factorials[index]);
    }
}

#[test]
fn increment_wrap_correctly_increments_and_wraps() {
    assert_eq!(CesiumMath::increment_wrap(5.0, 10.0, Some(0.0)), 6.0);
    assert_eq!(CesiumMath::increment_wrap(10.0, 10.0, Some(0.0)), 0.0);
    assert_eq!(CesiumMath::increment_wrap(10.0, 10.0, None), 0.0);
}

#[test]
#[ignore = "incrementWrap throws for undefined — statically impossible in Rust"]
fn increment_wrap_throws_for_undefined() {}

#[test]
fn is_power_of_two_finds_powers_of_two() {
    // Test all power of twos from 1 to 2^31
    for i in 0..32u32 {
        let power_of_two = (1u32 << i) as f64;
        assert!(CesiumMath::is_power_of_two(power_of_two));
    }
}

#[test]
fn is_power_of_two_does_not_find_powers_of_two() {
    assert!(!CesiumMath::is_power_of_two(0.0));
    assert!(!CesiumMath::is_power_of_two(3.0));
    assert!(!CesiumMath::is_power_of_two(5.0));
    assert!(!CesiumMath::is_power_of_two(12.0));
    assert!(!CesiumMath::is_power_of_two(4294967295.0)); // (2^32)-1
}

#[test]
fn next_power_of_two_finds_next_power_of_two() {
    assert_eq!(CesiumMath::next_power_of_two(0.0), 0.0);
    assert_eq!(CesiumMath::next_power_of_two(1.0), 1.0);
    assert_eq!(CesiumMath::next_power_of_two(2.0), 2.0);
    assert_eq!(CesiumMath::next_power_of_two(3.0), 4.0);
    assert_eq!(CesiumMath::next_power_of_two(257.0), 512.0);
    assert_eq!(CesiumMath::next_power_of_two(512.0), 512.0);
    assert_eq!(CesiumMath::next_power_of_two(1023.0), 1024.0);
    // (2^30)+1 -> 2^31
    assert_eq!(CesiumMath::next_power_of_two(1073741825.0), 2147483648.0);
    // (2^31)-1 -> 2^31
    assert_eq!(CesiumMath::next_power_of_two(2147483647.0), 2147483648.0);
    // 2^31 -> 2^31
    assert_eq!(CesiumMath::next_power_of_two(2147483648.0), 2147483648.0);
}

#[test]
fn previous_power_of_two_finds_previous_power_of_two() {
    assert_eq!(CesiumMath::previous_power_of_two(0.0), 0.0);
    assert_eq!(CesiumMath::previous_power_of_two(1.0), 1.0);
    assert_eq!(CesiumMath::previous_power_of_two(2.0), 2.0);
    assert_eq!(CesiumMath::previous_power_of_two(3.0), 2.0);
    assert_eq!(CesiumMath::previous_power_of_two(257.0), 256.0);
    assert_eq!(CesiumMath::previous_power_of_two(512.0), 512.0);
    assert_eq!(CesiumMath::previous_power_of_two(1023.0), 512.0);
    // 2^31 -> 2^31
    assert_eq!(CesiumMath::previous_power_of_two(2147483648.0), 2147483648.0);
    // (2^31)+1 -> 2^31
    assert_eq!(CesiumMath::previous_power_of_two(2147483649.0), 2147483648.0);
    // (2^32)-1 -> 2^31
    assert_eq!(CesiumMath::previous_power_of_two(4294967295.0), 2147483648.0);
}

#[test]
#[ignore = "factorial throws for non-numbers — statically impossible in Rust"]
fn factorial_throws_for_non_numbers() {}

#[test]
fn factorial_throws_for_negative_numbers() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::factorial(-1.0);
    });
}

#[test]
#[ignore = "factorial throws for undefined — statically impossible in Rust"]
fn factorial_throws_for_undefined() {}

#[test]
fn increment_wrap_throws_for_minimum_value_greater_or_equal_maximum_value() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::increment_wrap(5.0, 0.0, Some(10.0));
    });
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::increment_wrap(5.0, 10.0, Some(10.0));
    });
}

#[test]
#[ignore = "isPowerOfTwo throws for non-numbers — statically impossible in Rust"]
fn is_power_of_two_throws_for_non_numbers() {}

#[test]
fn is_power_of_two_throws_for_negative_numbers() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::is_power_of_two(-1.0);
    });
}

#[test]
fn is_power_of_two_throws_for_numbers_that_exceed_maximum_32_bit_unsigned_int() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::is_power_of_two(4294967296.0); // 2^32
    });
}

#[test]
#[ignore = "isPowerOfTwo throws for undefined — statically impossible in Rust"]
fn is_power_of_two_throws_for_undefined() {}

#[test]
#[ignore = "nextPowerOfTwo throws for non-numbers — statically impossible in Rust"]
fn next_power_of_two_throws_for_non_numbers() {}

#[test]
fn next_power_of_two_throws_for_negative_numbers() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::next_power_of_two(-1.0);
    });
}

#[test]
fn next_power_of_two_throws_for_results_that_would_exceed_maximum_32_bit_unsigned_int() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::next_power_of_two(2147483649.0); // (2^31)+1
    });
}

#[test]
#[ignore = "nextPowerOfTwo throws for undefined — statically impossible in Rust"]
fn next_power_of_two_throws_for_undefined() {}

#[test]
#[ignore = "previousPowerOfTwo throws for non-numbers — statically impossible in Rust"]
fn previous_power_of_two_throws_for_non_numbers() {}

#[test]
fn previous_power_of_two_throws_for_negative_numbers() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::previous_power_of_two(-1.0);
    });
}

#[test]
fn previous_power_of_two_throws_for_results_that_would_exceed_maximum_32_bit_unsigned_int() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::previous_power_of_two(4294967296.0); // 2^32
    });
}

#[test]
#[ignore = "previousPowerOfTwo throws for undefined — statically impossible in Rust"]
fn previous_power_of_two_throws_for_undefined() {}

#[test]
#[ignore = "clamp throws for undefined — statically impossible in Rust (Check.typeOf.number)"]
fn clamp_throws_for_undefined() {}

#[test]
fn acos_clamped_returns_acos_for_normal_values() {
    // toBe mirrored with <= 2 ULP tolerance (transcendentals allowance).
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(0.5), 0.5f64.acos(), 2);
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(0.123), 0.123f64.acos(), 2);
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(-0.123), (-0.123f64).acos(), 2);
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(-1.0), (-1.0f64).acos(), 2);
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(1.0), 1.0f64.acos(), 2);
}

#[test]
fn acos_clamped_returns_acos_of_clamped_value_when_value_is_outside_the_valid_range() {
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(-1.01), (-1.0f64).acos(), 2);
    assert_epsilon_eq_f64!(CesiumMath::acos_clamped(1.01), 1.0f64.acos(), 2);
}

#[test]
#[ignore = "acosClamped throws without value — statically impossible in Rust"]
fn acos_clamped_throws_without_value() {}

#[test]
fn asin_clamped_returns_asin_for_normal_values() {
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(0.5), 0.5f64.asin(), 2);
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(0.123), 0.123f64.asin(), 2);
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(-0.123), (-0.123f64).asin(), 2);
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(-1.0), (-1.0f64).asin(), 2);
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(1.0), 1.0f64.asin(), 2);
}

#[test]
fn asin_clamped_returns_asin_of_clamped_value_when_value_is_outside_the_valid_range() {
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(-1.01), (-1.0f64).asin(), 2);
    assert_epsilon_eq_f64!(CesiumMath::asin_clamped(1.01), 1.0f64.asin(), 2);
}

#[test]
#[ignore = "asinClamped throws without value — statically impossible in Rust"]
fn asin_clamped_throws_without_value() {}

#[test]
fn chord_length_finds_the_chord_length() {
    assert_approx_eq_f64!(
        CesiumMath::chord_length(CesiumMath::PI_OVER_THREE, 1.0),
        1.0,
        0.0,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        CesiumMath::chord_length(CesiumMath::PI_OVER_THREE, 5.0),
        5.0,
        0.0,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        CesiumMath::chord_length(2.0 * CesiumMath::PI_OVER_THREE, 1.0),
        3.0f64.sqrt(),
        0.0,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        CesiumMath::chord_length(2.0 * CesiumMath::PI_OVER_THREE, 5.0),
        5.0 * 3.0f64.sqrt(),
        0.0,
        CesiumMath::EPSILON14
    );
    assert_approx_eq_f64!(
        CesiumMath::chord_length(PI, 10.0),
        2.0 * 10.0,
        0.0,
        CesiumMath::EPSILON14
    );
}

#[test]
#[ignore = "chordLength throws without angle — statically impossible in Rust"]
fn chord_length_throws_without_angle() {}

#[test]
#[ignore = "chordLength throws without radius — statically impossible in Rust"]
fn chord_length_throws_without_radius() {}

#[test]
fn log_base_works() {
    // toEqual mirrored with approx tolerance (ln is transcendental, <= 2 ULP).
    assert_approx_eq_f64!(CesiumMath::log_base(64.0, 4.0), 3.0);
}

#[test]
#[ignore = "logBase throws without number — statically impossible in Rust"]
fn log_base_throws_without_number() {}

#[test]
#[ignore = "logBase throws without base — statically impossible in Rust"]
fn log_base_throws_without_base() {}

#[test]
fn cbrt_works() {
    assert_eq!(CesiumMath::cbrt(27.0), 3.0);
    assert_eq!(CesiumMath::cbrt(-27.0), -3.0);
    assert_eq!(CesiumMath::cbrt(0.0), 0.0);
    assert_eq!(CesiumMath::cbrt(1.0), 1.0);
    // JS: `cbrt()` with undefined returns NaN; NaN input mirrors that path.
    assert!(CesiumMath::cbrt(f64::NAN).is_nan());
}

#[test]
fn fast_approximate_atan_works() {
    assert_approx_eq_f64!(
        CesiumMath::fast_approximate_atan(0.0),
        0.0,
        0.0,
        CesiumMath::EPSILON3
    );
    assert_approx_eq_f64!(
        CesiumMath::fast_approximate_atan(1.0),
        CesiumMath::PI_OVER_FOUR,
        0.0,
        CesiumMath::EPSILON3
    );
    assert_approx_eq_f64!(
        CesiumMath::fast_approximate_atan(-1.0),
        -CesiumMath::PI_OVER_FOUR,
        0.0,
        CesiumMath::EPSILON3
    );
}

#[test]
fn fast_approximate_atan2_works() {
    assert_approx_eq_f64!(
        CesiumMath::fast_approximate_atan2(1.0, 0.0),
        0.0,
        0.0,
        CesiumMath::EPSILON3
    );
    assert_approx_eq_f64!(
        CesiumMath::fast_approximate_atan2(1.0, 1.0),
        CesiumMath::PI_OVER_FOUR,
        0.0,
        CesiumMath::EPSILON3
    );
    assert_approx_eq_f64!(
        CesiumMath::fast_approximate_atan2(-1.0, 1.0),
        CesiumMath::PI_OVER_FOUR + CesiumMath::PI_OVER_TWO,
        0.0,
        CesiumMath::EPSILON3
    );
}

#[test]
fn fast_approximate_atan2_throws_if_both_arguments_are_zero() {
    expect_to_throw_dev_error(|| {
        let _ = CesiumMath::fast_approximate_atan2(0.0, 0.0);
    });
}
