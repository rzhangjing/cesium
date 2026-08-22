//! Ported from packages/engine/Source/Core/defined.js

/// Port of CesiumJS `defined(value)`: returns true if the object is defined,
/// returns false otherwise.
///
/// CesiumJS checks `value !== undefined && value !== null`; in Rust the
/// optionality of a value is modeled with `Option`, so `defined` maps to
/// `Option::is_some`.
///
/// # Example
/// ```ignore
/// if defined(positions.as_ref()) {
///     do_something();
/// } else {
///     do_something_else();
/// }
/// ```
#[inline]
pub fn defined<T>(value: Option<&T>) -> bool {
    value.is_some()
}
