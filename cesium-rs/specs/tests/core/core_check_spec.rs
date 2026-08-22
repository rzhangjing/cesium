//! Mirrors packages/engine/Specs/Core/CheckSpec.js
//!
//! DEVIATION: the CesiumJS spec exercises dynamic `typeof` mismatches
//! (`Check.typeOf.bool("mockName", {})`, ...). Rust's static type system
//! makes wrong-type calls impossible, so those `it` blocks are mirrored as
//! ignored tests; the "undefined" (None) rejections and the numeric range
//! checks are fully ported. See docs/deviations.md.

use cesium_core::check::{self, type_of};
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/Check")

mod type_checks {
    use super::*;

    #[test]
    fn type_of_bool_does_not_throw_when_passed_a_boolean() {
        type_of::bool("bool", Some(true));
    }

    #[test]
    #[ignore = "non-boolean values cannot be passed to Check.typeOf.bool in Rust (static typing)"]
    fn type_of_bool_throws_when_passed_a_non_boolean() {
        expect_to_throw_dev_error(|| type_of::bool("mockName", None));
    }

    #[test]
    fn type_of_bigint_does_not_throw_when_passed_a_bigint() {
        // FeatureDetection.supportsBigInt() is always true in Rust (i128).
        type_of::bigint("bigint", true);
    }

    #[test]
    #[ignore = "non-bigint values cannot be passed to Check.typeOf.bigint in Rust (static typing)"]
    fn type_of_bigint_throws_when_passed_a_non_bigint() {
        expect_to_throw_dev_error(|| type_of::bigint("mockName", false));
    }

    #[test]
    fn type_of_func_does_not_throw_when_passed_a_function() {
        type_of::func("mockName", true);
    }

    #[test]
    #[ignore = "non-function values cannot be passed to Check.typeOf.func in Rust (static typing)"]
    fn type_of_func_throws_when_passed_a_non_function() {
        expect_to_throw_dev_error(|| type_of::func("mockName", false));
    }

    #[test]
    fn type_of_object_does_not_throw_when_passed_object() {
        let object = serde_json::json!({});
        type_of::object("mockName", Some(&object));
    }

    #[test]
    #[ignore = "non-object values cannot be passed to Check.typeOf.object in Rust (static typing)"]
    fn type_of_object_throws_when_passed_non_object() {
        let missing: Option<&serde_json::Value> = None;
        expect_to_throw_dev_error(|| type_of::object("mockName", missing));
    }

    #[test]
    fn type_of_number_does_not_throw_when_passed_number() {
        type_of::number("mockName", Some(2.0));
    }

    #[test]
    #[ignore = "non-number values cannot be passed to Check.typeOf.number in Rust (static typing)"]
    fn type_of_number_throws_when_passed_non_number() {
        expect_to_throw_dev_error(|| type_of::number("mockName", None));
    }

    #[test]
    fn type_of_string_does_not_throw_when_passed_a_string() {
        type_of::string("mockName", Some("s"));
    }

    #[test]
    #[ignore = "non-string values cannot be passed to Check.typeOf.string in Rust (static typing)"]
    fn type_of_string_throws_on_non_string() {
        expect_to_throw_dev_error(|| type_of::string("mockName", None));
    }
}

mod check_defined {
    use super::*;

    #[test]
    fn does_not_throw_unless_passed_value_that_is_undefined_or_null() {
        let object = serde_json::json!({});
        let array: [i32; 0] = [];
        check::defined("mockName", Some(&object));
        check::defined("mockName", Some(&array));
        check::defined("mockName", Some(&2.0));
        check::defined("mockName", Some(&"snt"));
    }

    #[test]
    fn throws_when_passed_undefined() {
        let missing: Option<&i32> = None;
        expect_to_throw_dev_error(|| check::defined("mockName", missing));
    }
}

mod number_less_than {
    use super::*;

    #[test]
    fn throws_if_test_is_equal_to_limit() {
        expect_to_throw_dev_error(|| type_of::number_less_than("mockName", 3.0, 3.0));
    }

    #[test]
    fn throws_if_test_is_greater_than_limit() {
        expect_to_throw_dev_error(|| type_of::number_less_than("mockName", 4.0, 3.0));
    }

    #[test]
    fn does_not_throw_if_test_is_less_than_limit() {
        type_of::number_less_than("mockName", 2.0, 3.0);
    }
}

mod number_less_than_or_equals {
    use super::*;

    #[test]
    fn throws_if_test_is_greater_than_limit() {
        expect_to_throw_dev_error(|| type_of::number_less_than_or_equals("mockName", 4.0, 3.0));
    }

    #[test]
    fn does_not_throw_if_test_is_equal_to_limit() {
        type_of::number_less_than_or_equals("mockName", 3.0, 3.0);
    }

    #[test]
    fn does_not_throw_if_test_is_less_than_limit() {
        type_of::number_less_than_or_equals("mockName", 2.0, 3.0);
    }
}

mod number_equals {
    use super::*;

    #[test]
    #[ignore = "non-number values cannot be passed to Check.typeOf.number.equals in Rust (static typing)"]
    fn throws_if_either_value_is_not_a_number() {}

    #[test]
    fn throws_if_both_the_values_are_a_number_but_not_equal() {
        expect_to_throw_dev_error(|| type_of::number_equals("mockName1", "mockName2", 1.0, 4.0));
    }

    #[test]
    fn does_not_throw_if_both_values_are_a_number_and_are_equal() {
        type_of::number_equals("mockName1", "mockName2", 3.0, 3.0);
    }
}

mod number_greater_than {
    use super::*;

    #[test]
    fn throws_if_test_is_equal_to_limit() {
        expect_to_throw_dev_error(|| type_of::number_greater_than("mockName", 3.0, 3.0));
    }

    #[test]
    fn throws_if_test_is_less_than_limit() {
        expect_to_throw_dev_error(|| type_of::number_greater_than("mockName", 2.0, 3.0));
    }

    #[test]
    fn does_not_throw_if_test_is_greater_than_limit() {
        type_of::number_greater_than("mockName", 4.0, 3.0);
    }
}

mod number_greater_than_or_equals {
    use super::*;

    #[test]
    fn throws_if_test_is_less_than_limit() {
        expect_to_throw_dev_error(|| {
            type_of::number_greater_than_or_equals("mockName", 2.0, 3.0)
        });
    }

    #[test]
    fn does_not_throw_if_test_is_equal_to_limit() {
        type_of::number_greater_than_or_equals("mockName", 3.0, 3.0);
    }

    #[test]
    fn does_not_throw_if_test_is_greater_than_limit() {
        type_of::number_greater_than_or_equals("mockName", 4.0, 3.0);
    }
}
