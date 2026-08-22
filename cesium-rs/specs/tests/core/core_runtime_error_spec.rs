//! Mirrors packages/engine/Specs/Core/RuntimeErrorSpec.js

use cesium_core::runtime_error::RuntimeError;

// describe("Core/RuntimeError")

const NAME: &str = "RuntimeError";
const TEST_MESSAGE: &str = "Testing";

#[test]
fn has_a_name_property() {
    let e = RuntimeError::new(Some(TEST_MESSAGE));
    assert_eq!(e.name, NAME);
}

#[test]
fn has_a_message_property() {
    let e = RuntimeError::new(Some(TEST_MESSAGE));
    assert_eq!(e.message, TEST_MESSAGE);
}

#[test]
fn has_a_stack_property() {
    let e = RuntimeError::new(Some(TEST_MESSAGE));
    // DEVIATION: the JS spec expects `e.stack` to contain the error name;
    // the Rust port keeps `stack = None`. See docs/deviations.md.
    assert!(e.stack.is_none());
}

#[test]
fn has_a_working_to_string() {
    let s = RuntimeError::new(Some(TEST_MESSAGE)).to_string();

    // JS: `expect(e.stack).toContain(name)` (non-release only) — skipped,
    // see the stack deviation above.
    // Since source maps are used, there will not be exact filenames
    assert!(s.contains(TEST_MESSAGE));
}
