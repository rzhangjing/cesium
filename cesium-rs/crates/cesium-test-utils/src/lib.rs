//! Shared test-support crate for the cesium-rs `specs` mirror test suite.
//!
//! Rust counterpart of the custom Jasmine matchers defined in the CesiumJS
//! repository root `Specs/addDefaultMatchers.js` (e.g. `toEqualEpsilon`,
//! `toThrowDeveloperError`). Everything here is domain-agnostic: numeric
//! assertion macros plus small panic/`DeveloperError` helpers.
//!
//! - [`assert_approx_eq_f64!`] — absolute/relative tolerance comparison
//!   (port of `toEqualEpsilon`), default epsilon `1e-12`.
//! - [`assert_epsilon_eq_f64!`] — ULP-based comparison, default ≤ 2 ULP.
//! - [`expect_to_throw_dev_error`] — port of `toThrowDeveloperError`.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Default tolerance used by [`assert_approx_eq_f64!`] when none is given.
pub const DEFAULT_APPROX_EPSILON: f64 = 1e-12;

/// Default maximum ULP distance used by [`assert_epsilon_eq_f64!`].
pub const DEFAULT_MAX_ULP: u64 = 2;

/// Maps the bit pattern of an `f64` onto a total ordering so that adjacent
/// representable values have adjacent keys (the classic "ordered integer"
/// representation used for ULP comparisons).
#[inline]
fn to_ordered(f: f64) -> i64 {
    let bits = f.to_bits() as i64;
    if bits < 0 {
        // Negative floats are ordered "backwards": flip all bits except sign.
        i64::MIN - bits
    } else {
        bits
    }
}

/// Number of representable `f64` values ("units in the last place") between
/// `a` and `b`.
///
/// - `NaN` vs `NaN` → `0` (treated as equal, matching test-suite semantics)
/// - exactly one `NaN`, or opposite infinities → `u64::MAX`
/// - `+0.0` vs `-0.0` → `0` (they are numerically equal)
pub fn ulp_diff_f64(a: f64, b: f64) -> u64 {
    if a.is_nan() && b.is_nan() {
        return 0;
    }
    if a.is_nan() || b.is_nan() {
        return u64::MAX;
    }
    // +0.0 and -0.0 compare equal; collapse them so their ULP distance is 0.
    let a = if a == 0.0 { 0.0 } else { a };
    let b = if b == 0.0 { 0.0 } else { b };
    if a == b {
        // Handles +inf == +inf without overflowing the subtraction below.
        return 0;
    }
    to_ordered(a).abs_diff(to_ordered(b))
}

/// Combined tolerance check: `a` and `b` are considered equal when their
/// absolute difference is within `abs_eps`, or their relative difference is
/// within `rel_eps` of the larger magnitude.
#[inline]
pub fn approx_eq_f64(a: f64, b: f64, abs_eps: f64, rel_eps: f64) -> bool {
    if a == b {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    let diff = (a - b).abs();
    diff <= abs_eps || diff <= rel_eps * a.abs().max(b.abs())
}

/// Asserts that two `f64` values are approximately equal.
///
/// Port of the CesiumJS `toEqualEpsilon` Jasmine matcher. Three forms:
///
/// ```ignore
/// assert_approx_eq_f64!(a, b);            // abs & rel tolerance = 1e-12
/// assert_approx_eq_f64!(a, b, 1e-9);      // abs & rel tolerance = 1e-9
/// assert_approx_eq_f64!(a, b, 1e-9, 1e-6); // abs tolerance, rel tolerance
/// ```
#[macro_export]
macro_rules! assert_approx_eq_f64 {
    ($left:expr, $right:expr $(,)?) => {{
        let (l, r) = ($left as f64, $right as f64);
        $crate::assert_approx_eq_f64!(l, r, $crate::DEFAULT_APPROX_EPSILON, $crate::DEFAULT_APPROX_EPSILON);
    }};
    ($left:expr, $right:expr, $eps:expr $(,)?) => {{
        let (l, r) = ($left as f64, $right as f64);
        let eps = $eps as f64;
        $crate::assert_approx_eq_f64!(l, r, eps, eps);
    }};
    ($left:expr, $right:expr, $abs_eps:expr, $rel_eps:expr $(,)?) => {{
        let (l, r) = ($left as f64, $right as f64);
        if !$crate::approx_eq_f64(l, r, $abs_eps as f64, $rel_eps as f64) {
            panic!(
                "assertion failed: `approx_eq_f64`\n  left: {:?}\n right: {:?}\n  abs_eps: {:e}\n  rel_eps: {:e}\n  |diff|: {:e}",
                l, r, $abs_eps as f64, $rel_eps as f64, (l - r).abs()
            );
        }
    }};
}

/// Asserts that two `f64` values are identical up to a bounded number of
/// ULPs (default: ≤ 2 ULP).
///
/// ```ignore
/// assert_epsilon_eq_f64!(a, b);      // ≤ 2 ULP
/// assert_epsilon_eq_f64!(a, b, 4);   // ≤ 4 ULP
/// ```
#[macro_export]
macro_rules! assert_epsilon_eq_f64 {
    ($left:expr, $right:expr $(,)?) => {{
        let (l, r) = ($left as f64, $right as f64);
        $crate::assert_epsilon_eq_f64!(l, r, $crate::DEFAULT_MAX_ULP);
    }};
    ($left:expr, $right:expr, $max_ulp:expr $(,)?) => {{
        let (l, r) = ($left as f64, $right as f64);
        let diff = $crate::ulp_diff_f64(l, r);
        let max_ulp = $max_ulp as u64;
        if diff > max_ulp {
            panic!(
                "assertion failed: `epsilon_eq_f64`\n  left: {:e}\n right: {:e}\n  ulp_diff: {}\n  max allowed: {}",
                l, r, diff, max_ulp
            );
        }
    }};
}

/// Rust port of the `toThrowDeveloperError` Jasmine matcher
/// (`Specs/addDefaultMatchers.js`).
///
/// CesiumJS signals precondition violations by throwing `DeveloperError`.
/// The Rust port surfaces them as panics whose message contains the
/// `DeveloperError` prefix (M0 convention; a typed error may be introduced
/// later). This helper runs `f`, requires it to panic, and checks that the
/// panic payload mentions `DeveloperError`.
///
/// # Panics
/// Panics if `f` does not panic, or if the panic message does not look like
/// a `DeveloperError`.
pub fn expect_to_throw_dev_error<F>(f: F) -> String
where
    F: FnOnce(),
{
    let result = catch_unwind(AssertUnwindSafe(f));
    match result {
        Ok(_) => panic!("expected a DeveloperError to be thrown, but the closure completed normally"),
        Err(payload) => {
            let message = panic_message(payload);
            assert!(
                message.contains("DeveloperError"),
                "expected a DeveloperError, got panic message: {message:?}"
            );
            message
        }
    }
}

/// Variant of [`expect_to_throw_dev_error`] that additionally requires the
/// panic message to contain `expected_fragment`.
pub fn expect_to_throw_dev_error_containing<F>(f: F, expected_fragment: &str) -> String
where
    F: FnOnce(),
{
    let message = expect_to_throw_dev_error(f);
    assert!(
        message.contains(expected_fragment),
        "DeveloperError message {message:?} does not contain {expected_fragment:?}"
    );
    message
}

/// Convenience wrapper for code paths that model `DeveloperError` as
/// `Err(...)` instead of a panic: asserts the result is an `Err` whose
/// `Display` mentions `DeveloperError`.
pub fn expect_dev_error_result<T, E: std::fmt::Display>(result: Result<T, E>) -> String {
    match result {
        Ok(_) => panic!("expected a DeveloperError result, got Ok"),
        Err(e) => {
            let message = e.to_string();
            assert!(
                message.contains("DeveloperError"),
                "expected a DeveloperError, got error: {message:?}"
            );
            message
        }
    }
}

/// Throws a canonical `DeveloperError` panic, mirroring the CesiumJS
/// `DeveloperError` constructor usage in precondition checks.
///
/// ```should_panic
/// cesium_test_utils::throw_developer_error("index out of range");
/// ```
#[cold]
pub fn throw_developer_error(message: &str) -> ! {
    panic!("DeveloperError: {message}");
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        String::from("<non-string panic payload>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulp_diff_of_equal_values_is_zero() {
        assert_eq!(ulp_diff_f64(1.0, 1.0), 0);
        assert_eq!(ulp_diff_f64(0.0, -0.0), 0);
        assert_eq!(ulp_diff_f64(f64::INFINITY, f64::INFINITY), 0);
        assert_eq!(ulp_diff_f64(f64::NAN, f64::NAN), 0);
    }

    #[test]
    fn ulp_diff_of_adjacent_values_is_one() {
        let a = 1.0_f64;
        let b = f64::from_bits(a.to_bits() + 1);
        assert_eq!(ulp_diff_f64(a, b), 1);
        assert_eq!(ulp_diff_f64(b, a), 1);
        // Crossing zero stays monotonic: distance between ±smallest subnormals is 2.
        let tiny_pos = f64::from_bits(1);
        let tiny_neg = f64::from_bits((1u64) | (1u64 << 63));
        assert_eq!(ulp_diff_f64(tiny_pos, tiny_neg), 2);
    }

    #[test]
    fn ulp_diff_of_nan_vs_number_is_max() {
        assert_eq!(ulp_diff_f64(f64::NAN, 1.0), u64::MAX);
        assert_eq!(ulp_diff_f64(1.0, f64::NAN), u64::MAX);
    }

    #[test]
    fn approx_macro_default_epsilon() {
        assert_approx_eq_f64!(1.0, 1.0 + 1e-13);
        assert_approx_eq_f64!(0.1 + 0.2, 0.3, 1e-9);
    }

    #[test]
    #[should_panic(expected = "approx_eq_f64")]
    fn approx_macro_fails_outside_tolerance() {
        assert_approx_eq_f64!(1.0, 2.0, 1e-12);
    }

    #[test]
    fn epsilon_macro_within_two_ulps() {
        let a = 1.0_f64;
        let b = f64::from_bits(a.to_bits() + 2);
        assert_epsilon_eq_f64!(a, b);
        assert_epsilon_eq_f64!(a, b, 2);
    }

    #[test]
    #[should_panic(expected = "epsilon_eq_f64")]
    fn epsilon_macro_fails_beyond_ulps() {
        let a = 1.0_f64;
        let b = f64::from_bits(a.to_bits() + 3);
        assert_epsilon_eq_f64!(a, b, 2);
    }

    #[test]
    fn dev_error_panic_is_detected() {
        let msg = expect_to_throw_dev_error(|| throw_developer_error("x must be positive"));
        assert!(msg.contains("x must be positive"));

        let msg = expect_to_throw_dev_error_containing(
            || throw_developer_error("index out of range: 5"),
            "out of range",
        );
        assert!(msg.contains("DeveloperError"));
    }

    #[test]
    #[should_panic(expected = "completed normally")]
    fn dev_error_helper_requires_panic() {
        expect_to_throw_dev_error(|| {});
    }

    #[test]
    fn dev_error_result_helper() {
        let r: Result<(), String> = Err("DeveloperError: bad input".to_string());
        let msg = expect_dev_error_result(r);
        assert!(msg.contains("bad input"));

        let r: Result<(), String> = Err("some other error".to_string());
        let outcome = catch_unwind(AssertUnwindSafe(|| expect_dev_error_result(r)));
        assert!(outcome.is_err());
    }
}
