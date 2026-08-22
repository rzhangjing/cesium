//! Ported from packages/engine/Source/Core/DeveloperError.js
//!
//! CesiumJS throws `DeveloperError` instances; the Rust port models the
//! exception both as a structured error type (for `formatError` and tests)
//! and as a panic whose message carries the `DeveloperError: ` prefix
//! (workspace-wide convention used by `cesium-test-utils`).

use std::fmt;

/// Constructs an exception object that is thrown due to a developer error,
/// e.g., invalid argument, argument out of range, etc. This exception should
/// only be thrown during development; it usually indicates a bug in the
/// calling code. This exception should never be caught; instead the calling
/// code should strive not to generate it.
///
/// On the other hand, a [`crate::runtime_error::RuntimeError`] indicates an
/// exception that may be thrown at runtime, e.g., out of memory, that the
/// calling code should be prepared to catch.
#[derive(Debug, Clone)]
pub struct DeveloperError {
    /// 'DeveloperError' indicating that this exception was thrown due to a
    /// developer error.
    pub name: &'static str,
    /// The explanation for why this exception was thrown.
    pub message: String,
    /// The stack trace of this exception, if available.
    // DEVIATION: JS captures the error stack at construction time; the Rust
    // port keeps `stack = None` (native backtraces are provided by the panic
    // infrastructure instead). See docs/deviations.md.
    pub stack: Option<String>,
}

impl DeveloperError {
    /// Port of `new DeveloperError(message)`.
    ///
    /// The JS parameter is optional (`undefined` message allowed).
    #[must_use]
    pub fn new(message: Option<&str>) -> Self {
        Self {
            name: "DeveloperError",
            message: message.unwrap_or("").to_owned(),
            stack: None,
        }
    }

    /// Port of `DeveloperError.throwInstantiationError()`.
    ///
    /// # Panics
    /// Always panics with a `DeveloperError`.
    pub fn throw_instantiation_error() -> ! {
        throw_developer_error(
            "This function defines an interface and should not be called directly.",
        );
    }
}

impl fmt::Display for DeveloperError {
    /// Port of `DeveloperError.prototype.toString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut str = format!("{}: {}", self.name, self.message);
        if let Some(stack) = &self.stack {
            str.push('\n');
            str.push_str(stack);
        }
        write!(f, "{str}")
    }
}

impl std::error::Error for DeveloperError {}

/// Raises a `DeveloperError` as a panic (workspace convention). The panic
/// message is `"DeveloperError: {message}"`, which is what
/// `cesium_test_utils::expect_to_throw_dev_error` matches against.
///
/// # Panics
/// Always panics.
#[cold]
pub fn throw_developer_error(message: &str) -> ! {
    panic!("DeveloperError: {message}");
}
