//! Mirrors packages/engine/Specs/Core/isLeapYearSpec.js

use cesium_core::is_leap_year::is_leap_year;

// describe("Core/isLeapYear")

#[test]
fn check_for_valid_leap_years() {
    assert!(is_leap_year(2000.0));
    assert!(is_leap_year(2004.0));
    assert!(!is_leap_year(2003.0));
    assert!(!is_leap_year(2300.0));
    assert!(is_leap_year(2400.0));
    assert!(!is_leap_year(-1.0));
    assert!(is_leap_year(-2000.0));
}

// JS: "Fail with null value" / "Fail with undefined value" / "Fail with
// non-numerical value" — the Rust signature (`year: f64`) makes these cases
// statically impossible; mirrored as ignored tests for the record.

#[test]
#[ignore = "the Rust year parameter is f64; null values are statically impossible"]
fn fail_with_null_value() {}

#[test]
#[ignore = "the Rust year parameter is f64; undefined values are statically impossible"]
fn fail_with_undefined_value() {}

#[test]
#[ignore = "the Rust year parameter is f64; non-numerical values are statically impossible"]
fn fail_with_non_numerical_value() {}
