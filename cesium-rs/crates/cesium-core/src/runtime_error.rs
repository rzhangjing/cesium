//! Ported from packages/engine/Source/Core/RuntimeError.js

use std::fmt;

/// Constructs an exception object that is thrown due to an error that can
/// occur at runtime, e.g., out of memory, could not compile shader, etc. If
/// a function may throw this exception, the calling code should be prepared
/// to catch it.
///
/// On the other hand, a [`crate::developer_error::DeveloperError`] indicates
/// an exception due to a developer error, e.g., invalid argument, that
/// usually indicates a bug in the calling code.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// 'RuntimeError' indicating that this exception was thrown due to a
    /// runtime error.
    pub name: &'static str,
    /// The explanation for why this exception was thrown.
    pub message: String,
    /// The stack trace of this exception, if available.
    // DEVIATION: JS captures the error stack at construction time; the Rust
    // port keeps `stack = None`. See docs/deviations.md.
    pub stack: Option<String>,
}

impl RuntimeError {
    /// Port of `new RuntimeError(message)`; the JS message parameter is
    /// optional.
    #[must_use]
    pub fn new(message: Option<&str>) -> Self {
        Self {
            name: "RuntimeError",
            message: message.unwrap_or("").to_owned(),
            stack: None,
        }
    }
}

impl fmt::Display for RuntimeError {
    /// Port of `RuntimeError.prototype.toString`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut str = format!("{}: {}", self.name, self.message);
        if let Some(stack) = &self.stack {
            str.push('\n');
            str.push_str(stack);
        }
        write!(f, "{str}")
    }
}

impl std::error::Error for RuntimeError {}
