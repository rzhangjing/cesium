//! Ported from packages/engine/Source/Core/assert.js

use crate::developer_error::throw_developer_error;

/// Checks that a condition is truthy, throwing a specified message if
/// condition fails.
///
/// Port of CesiumJS `assert(condition, msg)`; the JS version throws a
/// `DeveloperError`, the Rust port panics with the equivalent message.
///
/// # Panics
/// Panics with a `DeveloperError` when `condition` is false.
pub fn cesium_assert(condition: bool, msg: &str) {
    if !condition {
        throw_developer_error(msg);
    }
}
