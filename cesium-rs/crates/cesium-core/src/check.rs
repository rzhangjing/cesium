//! Ported from packages/engine/Source/Core/Check.js
//!
//! Contains functions for checking that supplied arguments are of a specified
//! type or meet specified conditions.
//!
//! DEVIATION: CesiumJS `Check.typeOf.*` performs dynamic `typeof` checks.
//! Rust's static type system already guarantees the type of `Some` values, so
//! the Rust port only rejects the "undefined" case (`None`) and keeps the JS
//! error-message shapes verbatim for spec parity. See docs/deviations.md.

use crate::developer_error::throw_developer_error;

// Convenience re-exports so call sites can use `check::number` /
// `check::object` in addition to the JS-faithful `check::type_of::*` paths.
pub use type_of::{bool, func, number, object, string};

fn get_undefined_error_message(name: &str) -> String {
    format!("{name} is required, actual value was undefined")
}

fn get_failed_type_error_message(actual: &str, expected: &str, name: &str) -> String {
    format!("Expected {name} to be typeof {expected}, actual typeof was {actual}")
}

/// Emulates JavaScript `Number.prototype.toString()` formatting, used to keep
/// `DeveloperError` messages byte-identical with CesiumJS (e.g. `1e+21`,
/// `0.0000001` → `1e-7`, `NaN`, `Infinity`).
pub(crate) fn js_number_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            "Infinity".to_owned()
        } else {
            "-Infinity".to_owned()
        };
    }
    if v == 0.0 {
        // JS: (-0).toString() === "0"
        return "0".to_owned();
    }
    let sign = if v < 0.0 { "-" } else { "" };
    let a = v.abs();
    // Rust's shortest round-trip exponential form, e.g. "1.2345e2".
    let s = format!("{:e}", a);
    let (mant, exp) = s.split_once('e').expect("format!(\"{{:e}}\") contains 'e'");
    let exp: i32 = exp.parse().expect("valid exponent");
    let digits: String = mant.chars().filter(|&c| c != '.').collect();
    let k = digits.len() as i32;
    let n = exp + 1;
    let body = if k <= n && n <= 21 {
        format!("{}{}", digits, "0".repeat((n - k) as usize))
    } else if n > 0 && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if n > -6 && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        let e = n - 1;
        let mant = if k == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!("{}e{}{}", mant, if e >= 0 { "+" } else { "-" }, e.abs())
    };
    format!("{sign}{body}")
}

/// Throws if test is not defined.
///
/// Port of `Check.defined(name, test)`.
///
/// # Panics
/// Panics with `DeveloperError` when `test` is `None`.
pub fn defined<T: ?Sized>(name: &str, test: Option<&T>) {
    if test.is_none() {
        throw_developer_error(&get_undefined_error_message(name));
    }
}

/// Contains type checking functions, all using the typeof operator
/// (statically guaranteed in Rust; see module docs).
pub mod type_of {
    use super::get_failed_type_error_message;
    use crate::check::js_number_to_string;
    use crate::developer_error::throw_developer_error;

    /// Throws if test is not typeof 'function'.
    ///
    /// Rust mapping: `is_defined` reflects whether the callable was provided;
    /// the function type itself is guaranteed by the type system.
    pub fn func(name: &str, is_defined: bool) {
        if !is_defined {
            throw_developer_error(&get_failed_type_error_message(
                "undefined",
                "function",
                name,
            ));
        }
    }

    /// Throws if test is not typeof 'string'.
    pub fn string(name: &str, test: Option<&str>) {
        if test.is_none() {
            throw_developer_error(&get_failed_type_error_message(
                "undefined",
                "string",
                name,
            ));
        }
    }

    /// Throws if test is not typeof 'number'.
    pub fn number(name: &str, test: Option<f64>) {
        if test.is_none() {
            throw_developer_error(&get_failed_type_error_message(
                "undefined",
                "number",
                name,
            ));
        }
    }

    /// Throws if test is not typeof 'number' and less than limit.
    ///
    /// Port of `Check.typeOf.number.lessThan`.
    pub fn number_less_than(name: &str, test: f64, limit: f64) {
        if test >= limit {
            throw_developer_error(&format!(
                "Expected {} to be less than {}, actual value was {}",
                name,
                js_number_to_string(limit),
                js_number_to_string(test)
            ));
        }
    }

    /// Throws if test is not typeof 'number' and less than or equal to limit.
    ///
    /// Port of `Check.typeOf.number.lessThanOrEquals`.
    pub fn number_less_than_or_equals(name: &str, test: f64, limit: f64) {
        if test > limit {
            throw_developer_error(&format!(
                "Expected {} to be less than or equal to {}, actual value was {}",
                name,
                js_number_to_string(limit),
                js_number_to_string(test)
            ));
        }
    }

    /// Throws if test is not typeof 'number' and greater than limit.
    ///
    /// Port of `Check.typeOf.number.greaterThan`.
    pub fn number_greater_than(name: &str, test: f64, limit: f64) {
        if test <= limit {
            throw_developer_error(&format!(
                "Expected {} to be greater than {}, actual value was {}",
                name,
                js_number_to_string(limit),
                js_number_to_string(test)
            ));
        }
    }

    /// Throws if test is not typeof 'number' and greater than or equal to
    /// limit.
    ///
    /// Port of `Check.typeOf.number.greaterThanOrEquals`.
    pub fn number_greater_than_or_equals(name: &str, test: f64, limit: f64) {
        if test < limit {
            throw_developer_error(&format!(
                "Expected {} to be greater than or equal to {}, actual value was {}",
                name,
                js_number_to_string(limit),
                js_number_to_string(test)
            ));
        }
    }

    /// Throws if test is not typeof 'object'.
    pub fn object<T: ?Sized>(name: &str, test: Option<&T>) {
        if test.is_none() {
            throw_developer_error(&get_failed_type_error_message(
                "undefined",
                "object",
                name,
            ));
        }
    }

    /// Throws if test is not typeof 'boolean'.
    pub fn bool(name: &str, test: Option<bool>) {
        if test.is_none() {
            throw_developer_error(&get_failed_type_error_message(
                "undefined",
                "boolean",
                name,
            ));
        }
    }

    /// Throws if test is not typeof 'bigint'.
    ///
    /// Rust mapping: `i128`/`i64` integer arguments are statically typed;
    /// only the undefined case is rejected.
    pub fn bigint(name: &str, is_defined: bool) {
        if !is_defined {
            throw_developer_error(&get_failed_type_error_message(
                "undefined",
                "bigint",
                name,
            ));
        }
    }

    /// Throws if test1 and test2 is not typeof 'number' and not equal in
    /// value.
    ///
    /// Port of `Check.typeOf.number.equals`.
    pub fn number_equals(name1: &str, name2: &str, test1: f64, test2: f64) {
        if test1 != test2 {
            throw_developer_error(&format!(
                "{} must be equal to {}, the actual values are {} and {}",
                name1,
                name2,
                js_number_to_string(test1),
                js_number_to_string(test2)
            ));
        }
    }
}
