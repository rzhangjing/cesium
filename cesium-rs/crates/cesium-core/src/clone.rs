//! Ported from packages/engine/Source/Core/clone.js
//!
//! DEVIATION: CesiumJS `clone` operates on dynamic objects (property bags).
//! Rust expresses cloning through the `Clone` trait; the `deep` flag has no
//! counterpart because Rust's `Clone` implementations define their own
//! copying semantics value-by-value. See docs/deviations.md.

/// Clones an object, returning a new object containing the same properties.
///
/// Port of CesiumJS `clone(object, deep)`. `deep` is accepted for API
/// parity but is a no-op: each Rust type's `Clone` impl decides its own
/// deep/shallow semantics.
#[must_use]
pub fn clone<T: Clone>(object: &T, _deep: bool) -> T {
    object.clone()
}
