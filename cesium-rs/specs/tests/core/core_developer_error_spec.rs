//! Mirrors packages/engine/Specs/Core/DeveloperErrorSpec.js

use cesium_core::developer_error::DeveloperError;

// describe("Core/DeveloperError")

const NAME: &str = "DeveloperError";
const TEST_MESSAGE: &str = "Testing";

#[test]
fn has_a_name_property() {
    let e = DeveloperError::new(Some(TEST_MESSAGE));
    assert_eq!(e.name, NAME);
}

#[test]
fn has_a_message_property() {
    let e = DeveloperError::new(Some(TEST_MESSAGE));
    assert_eq!(e.message, TEST_MESSAGE);
}

#[test]
fn has_a_stack_property() {
    let e = DeveloperError::new(Some(TEST_MESSAGE));
    // DEVIATION: the JS spec expects `e.stack` to contain the error name
    // (captured by the browser at construction time). The Rust port keeps
    // `stack = None`; native backtraces come from the panic infrastructure.
    // See docs/deviations.md.
    assert!(e.stack.is_none());
}

#[test]
fn has_a_working_to_string() {
    let s = DeveloperError::new(Some(TEST_MESSAGE)).to_string();

    // JS: non-release builds expect "DeveloperError: Testing" in toString.
    assert!(s.contains(&format!("{NAME}: {TEST_MESSAGE}")));
}
