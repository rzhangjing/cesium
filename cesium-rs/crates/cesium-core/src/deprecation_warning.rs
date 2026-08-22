//! Ported from packages/engine/Source/Core/deprecationWarning.js

use crate::developer_error::throw_developer_error;
use crate::one_time_warning::one_time_warning;

/// Logs a deprecation message to the console. Use this function instead of
/// logging directly since this does not log duplicate messages unless it is
/// called from multiple workers.
///
/// Port of CesiumJS `deprecationWarning(identifier, message)`.
///
/// # Example
/// ```ignore
/// // Deprecated function or class
/// fn foo() {
///     deprecation_warning("Foo", "Foo was deprecated in Cesium 1.01. It will be removed in 1.03. Use newFoo instead.");
///     // ...
/// }
/// ```
///
/// # Panics
/// In debug builds, panics with `DeveloperError` when `identifier` or
/// `message` is `None`.
pub fn deprecation_warning(identifier: Option<&str>, message: Option<&str>) {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) && (identifier.is_none() || message.is_none()) {
        throw_developer_error("identifier and message are required.");
    }
    // >>includeEnd('debug')

    one_time_warning(identifier, message);
}
