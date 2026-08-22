//! Mirrors packages/engine/Specs/Core/definedSpec.js

use cesium_core::defined::defined;

// describe("Core/defined")

#[test]
fn works_for_defined_value() {
    assert!(defined(Some(&0)));
}

#[test]
fn works_for_null_value() {
    // JS `null` maps to `None` in the Rust port.
    let missing: Option<&i32> = None;
    assert!(!defined(missing));
}

#[test]
fn works_for_undefined_value() {
    // JS `undefined` maps to `None` in the Rust port.
    let missing: Option<&i32> = None;
    assert!(!defined(missing));
}
